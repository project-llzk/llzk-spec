//! Functions for emitting MLIR IR representing specifications using LLZK's `verif` dialect.
//!
//! Only emitting IR to a separate file is currently supported. In the future we want to support
//! emitting IR inlined with an existing LLZK module.

use std::{any::Any, slice};

use llzk::{
    builder::OpBuilder,
    dialect::{array, bool, felt, function, llzk::nondet, pod, poly, r#struct, verif},
    prelude::*,
};
use melior::{
    dialect::{arith, scf},
    ir::Module,
};

use crate::{
    ast::{self, AstContext, Spanned, Visitable},
    diagnostic::CompileError,
    ir::{
        Context, MlirTypeSystem,
        llzk::{LlzkContractTarget, LlzkInfo, LoopKind},
        verif::{
            helpers::{accept_in_new_scope, find_contract_target_on_module},
            scope::{CodegenScopeStack, ScopeData, ScopeTag},
        },
    },
    type_analysis::TypeChecker,
};

mod helpers;
mod scope;

/// Typed AST document.
type TypedDocument<'ast, 'ctx> = ast::Document<'ast, Type<'ctx>>;
/// Typed AST item.
type TypedItem<'ast, 'ctx> = ast::Item<'ast, Type<'ctx>>;
/// Typed AST predicate declaration.
type TypedPredicateDecl<'ast, 'ctx> = ast::PredicateDecl<'ast, Type<'ctx>>;
/// Typed AST contract declaration.
type TypedContractDecl<'ast, 'ctx> = ast::ContractDecl<'ast, Type<'ctx>>;
/// Typed AST invariant declaration.
type TypedInvariantDecl<'ast, 'ctx> = ast::InvariantDecl<'ast, Type<'ctx>>;
/// Typed AST block.
type TypedBlock<'ast, 'ctx> = ast::Block<'ast, Type<'ctx>>;
/// Typed AST statement.
type TypedStatement<'ast, 'ctx> = ast::Statement<'ast, Type<'ctx>>;
/// Typed AST expression.
type TypedExpression<'ast, 'ctx> = ast::Expression<'ast, Type<'ctx>>;
/// Typed AST identifier.
type TypedIdentifier<'ast, 'ctx> = ast::Identifier<'ast, Type<'ctx>>;

/// Generates IR for the given [`Document`] on a fresh module.
pub fn emit_on_module<'ctx, 'ast>(
    ctx: &'ctx Context,
    ast: &'ast AstContext,
    filename: &str,
    document: &ast::Document<'ast>,
    circuit: &'ctx Module,
) -> Result<Module<'ctx>, CompileError> {
    let typed_document = TypeChecker::check(
        MlirTypeSystem::new(ctx, circuit),
        &LlzkInfo::new(circuit),
        ast,
        filename,
        document,
    )?;
    SpecCodegen::new(ctx, ast, circuit, filename.to_owned()).emit_ir(&typed_document)
}

/// Code generator of specifications.
struct SpecCodegen<'ast, 'ctx, 'blk> {
    ctx: &'ctx Context,
    ast: &'ast AstContext,
    scope: CodegenScopeStack<'ast, 'ctx, 'blk>,
    filename: String,
    builder: OpBuilder<'ctx, 'ctx>,
    /// Number of anonymous contracts encountered so far.
    anon_contracts: u32,
    /// Reference to the module.
    module: &'blk Module<'ctx>,
}

impl<'ast, 'ctx, 'blk> SpecCodegen<'ast, 'ctx, 'blk>
where
    'blk: 'ctx,
{
    /// Creates a new code generator.
    fn new(
        ctx: &'ctx Context,
        ast: &'ast AstContext,
        module: &'blk Module<'ctx>,
        filename: String,
    ) -> Self {
        Self {
            ctx,
            ast,
            scope: CodegenScopeStack::new(ScopeData::root(module)),
            filename,
            builder: OpBuilder::at_block_begin(&ctx.context, module.body()),
            anon_contracts: 0,
            module,
        }
    }

    /// Emits the IR for the document.
    fn emit_ir(mut self, document: &TypedDocument<'ast, 'ctx>) -> Result<(), CompileError> {
        document.accept(&mut self)
    }
}

impl<'ast, 'ctx> ast::Visitor<TypedDocument<'ast, 'ctx>> for SpecCodegen<'ast, 'ctx, '_> {
    type Output = Result<(), CompileError>;

    fn visit(&mut self, document: &TypedDocument<'ast, 'ctx>) -> Self::Output {
        for item in document {
            item.accept(self)?;
        }
        Ok(())
    }
}

impl<'ast, 'ctx> ast::Visitor<TypedItem<'ast, 'ctx>> for SpecCodegen<'ast, 'ctx, '_> {
    type Output = Result<(), CompileError>;

    fn visit(&mut self, item: &TypedItem<'ast, 'ctx>) -> Self::Output {
        match item {
            TypedItem::Contract(decl) => decl.accept(self),
            TypedItem::Predicate(decl) => decl.accept(self),
        }
    }
}

impl<'ast, 'ctx, 'blk> ast::Visitor<TypedContractDecl<'ast, 'ctx>>
    for SpecCodegen<'ast, 'ctx, 'blk>
{
    type Output = Result<(), CompileError>;

    fn visit(&mut self, decl: &TypedContractDecl<'ast, 'ctx>) -> Self::Output {
        let location = self.location(decl.span());
        let sym = self.symbolize_target(decl.target());
        let name = self.anon_contract_name(sym);
        let target = find_contract_target_on_module(self.module, sym)?;
        let parent_op = target.parent_operation().ok_or_else(|| {
            CompileError::Ir(format!(
                "expected target '{}' to be contained in another operation",
                target.fully_qualified_name()
            ))
        })?;
        // Push into the parent block, this is where we will insert the contract op.
        self.push(target.block().ok_or_else(|| {
            CompileError::Ir(format!(
                "expected target '{}' to be contained in a block",
                target.fully_qualified_name()
            ))
        })?);
        // Create a function def op pretending to be the contract for now.
        let block = verif::contract(
            self.builder(),
            location,
            name.value(),
            target.fully_qualified_name(),
        )?
        .body()?
        .first_block()
        .unwrap();
        self.push_tagged(block, ScopeTag::Contract);
        self.bind_template_consts(parent_op, decl, location)?;
        match target {
            LlzkContractTarget::Struct(target_op) => {
                // Bind the members as `struct.readm` operations reading from argument #0
                self.bind_members(target_op, location, Value::from(block.argument(0)?), decl)?;
                // Bind the inputs from the rest of the arguments of the function.
                self.bind_inputs(target_op.compute_func().unwrap(), block, Some(1), decl)?;
            }
            LlzkContractTarget::Function(target_op) => {
                // Bind the inputs (the first N arguments of the contract)
                self.bind_inputs(target_op, block, None, decl)?;
                // Bind the outputs (the rest of the arguments of the contract)
                self.bind_outputs(
                    target_op,
                    block,
                    Some(target_op.function_type()?.input_count()),
                    decl,
                )?;
            }
        }
        self.bind_loop_info(target, decl)?;

        decl.body().accept(self)?;
        // We pop twice: the body of the contract and the parent of the target.
        self.pop();
        self.pop();
        Ok(())
    }
}

impl<'ast, 'ctx> ast::Visitor<TypedPredicateDecl<'ast, 'ctx>> for SpecCodegen<'ast, 'ctx, '_> {
    type Output = Result<(), CompileError>;

    fn visit(&mut self, decl: &TypedPredicateDecl<'ast, 'ctx>) -> Self::Output {
        self.bind_and_push_predicate_block(decl)?;
        // Lower the body of the predicate.
        decl.body().accept(self)?;
        self.pop();
        Ok(())
    }
}

impl<'ast, 'ctx> ast::Visitor<TypedBlock<'ast, 'ctx>> for SpecCodegen<'ast, 'ctx, '_> {
    type Output = Result<(), CompileError>;

    fn visit(&mut self, block: &TypedBlock<'ast, 'ctx>) -> Self::Output {
        for stmt in block {
            stmt.accept(self)?;
        }
        Ok(())
    }
}

/// This macro call [`unreachable!`] with a message stating that the given statement cannot be
/// defined in the given scope.
///
/// The type-checking pass must have emitted the correct errors for those cases, thus making them
/// impossible during IR emission.
macro_rules! stmt_not_allowed {
    ($stmt:literal, $scope:literal) => {
        unreachable!(concat!(
            $stmt,
            " statements are not allowed inside ",
            $scope
        ))
    };
}

impl<'ast, 'ctx> ast::Visitor<TypedStatement<'ast, 'ctx>> for SpecCodegen<'ast, 'ctx, '_> {
    type Output = Result<(), CompileError>;

    fn visit(&mut self, stmt: &TypedStatement<'ast, 'ctx>) -> Self::Output {
        use ast::Statement::*;
        match stmt {
            Scoped {
                scope,
                statement,
                span,
            } => match self.closest_tag() {
                ScopeTag::Predicate => stmt_not_allowed!("scoped", "predicates"),
                ScopeTag::Compute | ScopeTag::Constrain => {
                    stmt_not_allowed!("scoped", "other scoped statements")
                }
                _ => {
                    // Wrap the body of the block to ensure that SSA values don't leak in case of bugs
                    // in the scope logic.
                    let region = Region::new();
                    accept_in_new_scope(
                        &region,
                        self,
                        statement,
                        |_, _| Ok(()),
                        Some(match scope {
                            ast::Scope::Compute => ScopeTag::Compute,
                            ast::Scope::Constrain => ScopeTag::Constrain,
                        }),
                    )?;
                    let block = region.first_block().unwrap();
                    if block.terminator().is_none() {
                        block.append_operation(scf::r#yield(&[], self.location(*span)));
                    }
                    let op = scf::execute_region(&[], region, self.location(*span));
                    self.top_mut().append_operation(op);
                    Ok(())
                }
            },
            Block(block) => {
                // Wrap the body of the block to ensure that SSA values don't leak in case of bugs
                // in the scope logic.
                let region = Region::new();
                accept_in_new_scope(&region, self, block, |_, _| Ok(()), None)?;
                let block_ref = region.first_block().unwrap();
                if block_ref.terminator().is_none() {
                    block_ref.append_operation(scf::r#yield(&[], self.location(block.span())));
                }
                let op = scf::execute_region(&[], region, self.location(block.span()));
                self.top_mut().append_operation(op);
                Ok(())
            }
            Require { expression, span } => match self.closest_tag() {
                ScopeTag::Predicate => stmt_not_allowed!("require", "predicates"),
                tag => {
                    let location = self.location(*span);
                    let condition = expression.accept(self)?;
                    if tag.compute_condition_scope() {
                        verif::require_compute(self.builder(), location, condition)?;
                    }
                    if tag.constrain_condition_scope() {
                        verif::require_constrain(self.builder(), location, condition)?;
                    }
                    Ok(())
                }
            },
            Ensure { expression, span } => match self.closest_tag() {
                ScopeTag::Predicate => stmt_not_allowed!("ensure", "predicates"),
                tag => {
                    let location = self.location(*span);
                    let condition = expression.accept(self)?;
                    if tag.compute_condition_scope() {
                        verif::ensure_compute(self.builder(), location, condition)?;
                    }
                    if tag.constrain_condition_scope() {
                        verif::ensure_constrain(self.builder(), location, condition)?;
                    }
                    Ok(())
                }
            },
            Let {
                name, value, span, ..
            } => {
                let value = value.accept(self)?;
                self.top_mut().bind_local(name, value).map_err(|err| {
                    err.into_compile_error(&self.filename, Some(*span), "on let statement")
                })
            }
            Unused { .. } => match self.closest_tag() {
                ScopeTag::Predicate => stmt_not_allowed!("unused", "predicates"),
                _ => todo!("unused statement is not supported yet"),
            },
            Return { expression, span } => {
                let value = expression.accept(self)?;
                let location = self.location(*span);
                self.top_mut()
                    .append_operation(function::r#return(location, &[value]));
                Ok(())
            }
            Increases { .. } => match self.closest_tag() {
                ScopeTag::Predicate => stmt_not_allowed!("increases", "predicates"),
                _ => todo!("increases statement is not supported yet"),
            },
            Decreases { .. } => match self.closest_tag() {
                ScopeTag::Predicate => stmt_not_allowed!("decreases", "predicates"),
                _ => todo!("decreases statement is not supported yet"),
            },
            Step { .. } => match self.closest_tag() {
                ScopeTag::Predicate => stmt_not_allowed!("step", "predicates"),
                _ => todo!("step statement is not supported yet"),
            },
            Invariant(decl) => match self.closest_tag() {
                ScopeTag::Predicate => stmt_not_allowed!("invariant", "predicates"),
                _ => decl.accept(self),
            },
            Predicate(decl) => decl.accept(self),
            Empty { .. } => Ok(()),
        }
    }
}

impl<'ast, 'ctx, 'blk> ast::Visitor<TypedInvariantDecl<'ast, 'ctx>>
    for SpecCodegen<'ast, 'ctx, 'blk>
{
    type Output = Result<(), CompileError>;

    fn visit(&mut self, _: &TypedInvariantDecl<'ast, 'ctx>) -> Self::Output {
        todo!("invariant statement is not supported yet");
    }
}

impl<'ast, 'ctx, 'blk> ast::Visitor<TypedExpression<'ast, 'ctx>> for SpecCodegen<'ast, 'ctx, 'blk> {
    type Output = Result<Value<'ctx, 'blk>, CompileError>;

    fn visit(&mut self, expr: &TypedExpression<'ast, 'ctx>) -> Self::Output {
        use ast::BinaryOp::*;
        use ast::Expression::*;
        use ast::UnaryOp::*;
        let location = self.location(expr.span());
        match expr {
            Conditional {
                condition,
                then_branch,
                else_branch,
                meta,
                ..
            } => {
                let condition = condition.accept(self)?;
                let then_region = Region::new();
                let then_result =
                    self.lower_conditional_branch(&then_region, then_branch, location, *meta)?;

                let else_region = Region::new();
                let else_result =
                    self.lower_conditional_branch(&else_region, else_branch, location, *meta)?;
                assert_eq!(then_result.r#type(), else_result.r#type());

                let op = scf::r#if(condition, &[*meta], then_region, else_region, location);
                self.top_mut().append_operation_with_result(op)
            }
            Binary {
                op,
                left,
                right,
                meta,
                ..
            } => {
                let lhs = left.accept(self)?;
                let rhs = right.accept(self)?;
                let op = match op {
                    Or => bool::or(location, lhs, rhs)?,
                    And => bool::and(location, lhs, rhs)?,
                    Eq => {
                        let lhs = self.cast_if_necessary(lhs, self.ctx.felt_type(), location)?;
                        let rhs = self.cast_if_necessary(rhs, self.ctx.felt_type(), location)?;
                        bool::eq(location, lhs, rhs)?
                    }
                    Ne => {
                        let lhs = self.cast_if_necessary(lhs, self.ctx.felt_type(), location)?;
                        let rhs = self.cast_if_necessary(rhs, self.ctx.felt_type(), location)?;
                        bool::ne(location, lhs, rhs)?
                    }
                    Lt => {
                        let lhs = self.cast_if_necessary(lhs, self.ctx.felt_type(), location)?;
                        let rhs = self.cast_if_necessary(rhs, self.ctx.felt_type(), location)?;
                        bool::lt(location, lhs, rhs)?
                    }
                    Le => {
                        let lhs = self.cast_if_necessary(lhs, self.ctx.felt_type(), location)?;
                        let rhs = self.cast_if_necessary(rhs, self.ctx.felt_type(), location)?;
                        bool::le(location, lhs, rhs)?
                    }
                    Gt => {
                        let lhs = self.cast_if_necessary(lhs, self.ctx.felt_type(), location)?;
                        let rhs = self.cast_if_necessary(rhs, self.ctx.felt_type(), location)?;
                        bool::gt(location, lhs, rhs)?
                    }
                    Ge => {
                        let lhs = self.cast_if_necessary(lhs, self.ctx.felt_type(), location)?;
                        let rhs = self.cast_if_necessary(rhs, self.ctx.felt_type(), location)?;
                        bool::ge(location, lhs, rhs)?
                    }
                    Add => {
                        let lhs = self.cast_if_necessary(lhs, self.ctx.felt_type(), location)?;
                        let rhs = self.cast_if_necessary(rhs, self.ctx.felt_type(), location)?;
                        felt::add(location, lhs, rhs)?
                    }
                    Sub => {
                        let lhs = self.cast_if_necessary(lhs, self.ctx.felt_type(), location)?;
                        let rhs = self.cast_if_necessary(rhs, self.ctx.felt_type(), location)?;
                        felt::sub(location, lhs, rhs)?
                    }
                    Mul => {
                        let lhs = self.cast_if_necessary(lhs, self.ctx.felt_type(), location)?;
                        let rhs = self.cast_if_necessary(rhs, self.ctx.felt_type(), location)?;
                        felt::mul(location, lhs, rhs)?
                    }
                    Div => {
                        let lhs = self.cast_if_necessary(lhs, self.ctx.felt_type(), location)?;
                        let rhs = self.cast_if_necessary(rhs, self.ctx.felt_type(), location)?;
                        felt::div(location, lhs, rhs)?
                    }
                    // felt.smod or felt.umod?
                    Mod => {
                        let lhs = self.cast_if_necessary(lhs, self.ctx.felt_type(), location)?;
                        let rhs = self.cast_if_necessary(rhs, self.ctx.felt_type(), location)?;
                        felt::umod(location, lhs, rhs)?
                    }
                    BitAnd => {
                        let lhs = self.cast_if_necessary(lhs, self.ctx.felt_type(), location)?;
                        let rhs = self.cast_if_necessary(rhs, self.ctx.felt_type(), location)?;
                        felt::bit_and(location, lhs, rhs)?
                    }
                    Pow => {
                        let lhs = self.cast_if_necessary(lhs, self.ctx.felt_type(), location)?;
                        let rhs = self.cast_if_necessary(rhs, self.ctx.felt_type(), location)?;
                        felt::pow(location, lhs, rhs)?
                    }
                };
                let value = self.top_mut().append_operation_with_result(op)?;
                assert_eq!(*meta, value.r#type());
                Ok(value)
            }
            Unary { op, expr, meta, .. } => {
                let value = expr.accept(self)?;
                let op = match op {
                    Not => bool::not(location, value)?,
                    Neg => {
                        let value =
                            self.cast_if_necessary(value, self.ctx.felt_type(), location)?;
                        felt::neg(location, value)?
                    }
                };
                let value = self.top_mut().append_operation_with_result(op)?;
                assert_eq!(*meta, value.r#type());
                Ok(value)
            }
            Index {
                target,
                index,
                span,
                meta,
            } => {
                let location = self.location(*span);
                let target_value = target.accept(self)?;
                let index_value = index.accept(self)?;
                let index_value =
                    self.cast_if_necessary(index_value, self.ctx.index_type(), location)?;
                let Ok(arr_type) = ArrayType::try_from(target_value.r#type()) else {
                    return Err(CompileError::Ir(format!(
                        "expected array type but got {}",
                        target_value.r#type()
                    )));
                };
                self.top_mut()
                    .append_operation_with_result(if arr_type.dims().len() > 1 {
                        array::extract
                    } else {
                        array::read
                    }(
                        location, *meta, target_value, &[index_value]
                    ))
            }
            Member {
                target,
                member,
                span,
                meta,
            } => {
                let location = self.location(*span);
                let target_value = target.accept(self)?;
                if let Ok(_) = StructType::try_from(target_value.r#type()) {
                    let op = r#struct::readm(
                        self.builder(),
                        location,
                        *meta,
                        target_value,
                        member.value(),
                    )?;
                    self.top_mut().append_operation_with_result(op)
                } else if let Ok(_) = PodType::try_from(target_value.r#type()) {
                    let op = pod::read(
                        location,
                        target_value,
                        FlatSymbolRefAttribute::new(self.context(), member.value()),
                        *meta,
                    );
                    self.top_mut().append_operation_with_result(op)
                } else {
                    Err(CompileError::Ir(format!(
                        "was expecting either a struct or pod type but got {}",
                        target_value.r#type()
                    )))
                }
            }
            Call {
                callee, args, meta, ..
            } => {
                let args = self.visit_many(args)?;
                let callee = self.find_actual_function_name(callee)?;
                let op = function::call(
                    self.builder(),
                    location,
                    callee,
                    &args,
                    slice::from_ref(meta),
                )?;
                self.top_mut().append_operation_with_result(op)
            }
            Quantifier { .. } => todo!("quantifier expression is not supported yet"),
            Len { target, span, .. } => {
                let target_value = target.accept(self)?;
                let location = self.location(*span);

                let Ok(_) = ArrayType::try_from(target_value.r#type()) else {
                    return Err(CompileError::Ir(format!(
                        "expected array type but got {}",
                        target_value.r#type()
                    )));
                };
                let op = arith::constant(
                    self.context(),
                    IntegerAttribute::new(self.ctx.index_type(), 0).into(),
                    location,
                );
                let dim = self.top_mut().append_operation_with_result(op)?;
                let op = array::len(location, target_value, dim);
                self.top_mut().append_operation_with_result(op)
            }
            Old { .. } => todo!("old expression is not supported yet"),
            Arg { index, span, .. } => self.scope.find_parameter(index).copied().map_err(|err| {
                err.into_compile_error(&self.filename, Some(*span), format!("on argument #{index}"))
            }),
            Nondet { meta, .. } => self
                .top_mut()
                .append_operation_with_result(nondet(location, *meta)),
            Boolean { value, meta, .. } => {
                let op = arith::constant(
                    self.context(),
                    IntegerAttribute::new(*meta, (*value).into()).into(),
                    location,
                );
                self.top_mut().append_operation_with_result(op)
            }
            Number { value, meta, .. } => {
                assert_eq!(*meta, self.felt_type());
                let value = FeltConstAttribute::from_biguint(
                    self.context(),
                    value.value(),
                    self.ctx.prime(),
                );
                self.top_mut()
                    .append_operation_with_result(felt::constant(location, value)?)
            }
            Symbol(symbol) => self.find_symbol(symbol),
        }
    }
}
