//! Functions for emitting MLIR IR representing specifications using LLZK's `verif` dialect.
//!
//! Only emitting IR to a separate file is currently supported. In the future we want to support
//! emitting IR inlined with an existing LLZK module.

use std::slice;

use llzk::{
    builder::OpBuilder,
    dialect::{bool, felt, function, llzk::nondet, poly, r#struct},
    prelude::*,
};
use melior::{
    dialect::{arith, scf},
    ir::{Identifier, Module},
};

use crate::{
    ast::{self, AstContext, Spanned, Visitable},
    diagnostic::CompileError,
    ir::{
        Context, MlirTypeSystem,
        llzk::{LlzkContractTarget, LlzkInfo},
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
) -> Result<(), CompileError> {
    let info = LlzkInfo::new(circuit);
    let typed_document =
        TypeChecker::check(MlirTypeSystem::new(ctx), &info, ast, filename, document)?;
    SpecCodegen::new(ctx, ast, circuit, filename.to_owned()).emit_ir(&typed_document)
}

/// Code generator of specifications.
struct SpecCodegen<'ast, 'ctx, 'blk> {
    ctx: &'ctx Context,
    ast: &'ast AstContext,
    scope: CodegenScopeStack<'ast, 'ctx, 'blk>,
    filename: String,
    builder: OpBuilder<'ctx>,
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
            builder: OpBuilder::new(&ctx.context),
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

impl<'ast, 'ctx> ast::Visitor<TypedContractDecl<'ast, 'ctx>> for SpecCodegen<'ast, 'ctx, '_> {
    type Output = Result<(), CompileError>;

    fn visit(&mut self, decl: &TypedContractDecl<'ast, 'ctx>) -> Self::Output {
        let location = self.location(decl.span());
        let sym = self.symbolize_target(decl.target());
        let name = self.anon_contract_name(sym);
        let target = find_contract_target_on_module(self.module, sym)?;
        let parent_op = target.parent_operation().ok_or_else(|| {
            CompileError::Ir(format!(
                "expected target '{target}' to be contained in another operation"
            ))
        })?;
        let parent_block = target.block().ok_or_else(|| {
            CompileError::Ir(format!(
                "expected target '{target}' to be contained in a block"
            ))
        })?;

        // Push into the parent block, this is where we will insert the contract op.
        self.push(parent_block);
        {
            // Create a function def op pretending to be the contract for now.
            // We will replace this with `verif.contract` once the constructor is fixed.
            let block_args = {
                let t = match target {
                    LlzkContractTarget::Struct(op_ref) => op_ref.constrain_func().unwrap(),
                    LlzkContractTarget::Function(op_ref) => op_ref,
                }
                .function_type()?;
                t.inputs()
                    .chain(t.results())
                    .map(|t| (t, location))
                    .collect::<Vec<_>>()
            };
            let arg_attrs = {
                let (func_op, mut arg_attrs) = match target {
                    LlzkContractTarget::Struct(op_ref) => {
                        // Return the initial vector with an empty padding for the self argument.
                        (op_ref.compute_func().unwrap(), vec![vec![]])
                    }
                    LlzkContractTarget::Function(op_ref) => (op_ref, vec![]),
                };
                arg_attrs.extend(
                    (0..func_op.function_type().unwrap().input_count()).map(|n| {
                        Vec::from_iter(
                            func_op
                                .argument_attr(n, "function.arg_name")
                                .map(|a| (Identifier::new(self.context(), "function.arg_name"), a)),
                        )
                    }),
                );
                arg_attrs
            };
            let op = function::def(
                location,
                name.value(),
                (*decl.target().meta())
                    .try_into()
                    .map_err(|err| CompileError::Ir(format!("{err}")))?,
                &[],
                Some(&arg_attrs),
            )?;
            let op_ref = FuncDefOpRef::try_from(self.scope.top().append_operation(op)).unwrap();
            let block = op_ref.body()?.append_block(Block::new(&block_args));
            let arg0 = Value::from(block.argument(0)?);

            // Push into the block holding the body of the contract.
            self.push_tagged(block, ScopeTag::Contract);

            // If we are encapsulated in a `poly.template` op, bind all the template parameters
            // into the environment.
            // The bindings are `poly.read_const` read ops from each binding (that has a type).
            if let Ok(template_op) = TemplateOpRef::try_from(parent_op) {
                template_op
                    .const_binding_ops()
                    .into_iter()
                    .filter(|param_ops| param_ops.type_opt().is_some())
                    .try_for_each(|param_op| {
                        let symbol = param_op.sym_name();
                        let read_value =
                            self.scope
                                .top()
                                .append_operation_with_result(poly::read_const(
                                    location,
                                    symbol,
                                    param_op.type_opt().unwrap(),
                                ))?;
                        let name = self.create_ident(symbol, decl);
                        self.scope
                            .top()
                            .bind_local(&name, read_value)
                            .map_err(|err| {
                                err.into_compile_error(
                                    &self.filename,
                                    Some(decl.span()),
                                    format!("while binding template parameter '{symbol}'"),
                                )
                            })
                    })?;
            }

            match target {
                LlzkContractTarget::Struct(target_op) => {
                    // If the target is a struct:
                    //   Bind the members are `struct.readm` operations reading from argument #0
                    //   Bind the inputs from the rest of the arguments of the function.
                    for member in target_op.member_defs() {
                        let op = r#struct::readm(
                            self.builder(),
                            location,
                            member.member_type(),
                            arg0,
                            member.member_name(),
                        )?;
                        let value = self.scope.top().append_operation_with_result(op)?;
                        let name = self.create_ident(member.member_name(), decl);
                        self.scope.top().bind_local(&name, value).map_err(|err| {
                            err.into_compile_error(
                                &self.filename,
                                Some(decl.span()),
                                format!("while binding struct member '{}'", member.member_name()),
                            )
                        })?;
                    }
                    let compute_func = target_op.compute_func().unwrap();
                    let arg_count = compute_func.function_type()?.input_count();
                    (0..arg_count).try_for_each(|n| -> Result<(), CompileError> {
                        let arg = Value::from(block.argument(n + 1)?);

                        let name = match compute_func
                            .argument_attr(n, "function.arg_name")
                            .and_then(|a| Ok(StringAttribute::try_from(a)?.value()))
                            .ok()
                        {
                            Some(arg_name) => self.create_ident(arg_name, decl),
                            None => self.create_ident(&format!("$arg[{n}]"), decl),
                        };

                        self.scope
                            .top()
                            .bind_parameter(&name, arg, n)
                            .map_err(|err| {
                                err.into_compile_error(
                                    &self.filename,
                                    Some(decl.span()),
                                    format!("while binding argument #{n} of target"),
                                )
                            })
                    })?;
                }
                LlzkContractTarget::Function(target_op) => {
                    // If the target is a function:
                    //   Bind the inputs (the first N arguments of the contract)
                    let function_type = target_op.function_type()?;
                    let input_count = function_type.input_count();
                    (0..input_count).try_for_each(|n| -> Result<(), CompileError> {
                        let arg = Value::from(block.argument(n)?);

                        let name = match target_op
                            .argument_attr(n, "function.arg_name")
                            .and_then(|a| Ok(StringAttribute::try_from(a)?.value()))
                            .ok()
                        {
                            Some(arg_name) => self.create_ident(arg_name, decl),
                            None => self.create_ident(&format!("$arg[{n}]"), decl),
                        };

                        self.scope
                            .top()
                            .bind_parameter(&name, arg, n)
                            .map_err(|err| {
                                err.into_compile_error(
                                    &self.filename,
                                    Some(decl.span()),
                                    format!("while binding argument #{n} of target"),
                                )
                            })
                    })?;
                    //   Bind the outputs (the rest of the arguments of the contract)
                    let output_count = function_type.result_count();
                    (0..output_count).try_for_each(|n| -> Result<(), CompileError> {
                        let arg = Value::from(block.argument(input_count + n)?);
                        let name = self.create_ident(&format!("$res[{n}]"), decl);
                        self.scope.top().bind_output(&name, arg, n).map_err(|err| {
                            err.into_compile_error(
                                &self.filename,
                                Some(decl.span()),
                                format!("while binding outptu #{n} of target"),
                            )
                        })
                    })?;
                }
            }

            // TODO: Bind loop information.

            // Emit the IR for the body
            decl.body().accept(self)?;

            // Add a `function.return` terminator since we are faking the contract op.
            // We will remove the line below once we are using `verif.contract`
            self.scope
                .top()
                .append_operation(function::r#return(location, &[]));

            // Pop from the block holding the body.
            self.pop();
        }
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
            Scoped { .. } => match self.closest_tag() {
                ScopeTag::Predicate => stmt_not_allowed!("scoped", "predicates"),
                _ => todo!("scoped statement is not supported yet"),
            },
            Block(block) => {
                // Wrap the body of the block to ensure that SSA values don't leak in case of bugs
                // in the scope logic.
                let region = Region::new();
                accept_in_new_scope(&region, self, block, |_, _| Ok(()))?;
                let op = scf::execute_region(&[], region, self.location(block.span()));
                self.top_mut().append_operation(op);
                Ok(())
            }
            Require { .. } => match self.closest_tag() {
                ScopeTag::Predicate => stmt_not_allowed!("require", "predicates"),
                _ => todo!("require statement is not supported yet"),
            },
            Ensure { .. } => match self.closest_tag() {
                ScopeTag::Predicate => stmt_not_allowed!("ensure", "predicates"),
                _ => todo!("ensure statement is not supported yet"),
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
            Invariant(_) => match self.closest_tag() {
                ScopeTag::Predicate => stmt_not_allowed!("invariant", "predicates"),
                _ => {
                    eprintln!("invariant statement is not supported yet");
                    Ok(())
                }
            },
            Predicate(decl) => decl.accept(self),
            Empty { .. } => Ok(()),
        }
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
                    Eq => bool::eq(location, lhs, rhs)?,
                    Ne => bool::ne(location, lhs, rhs)?,
                    Lt => bool::lt(location, lhs, rhs)?,
                    Le => bool::le(location, lhs, rhs)?,
                    Gt => bool::gt(location, lhs, rhs)?,
                    Ge => bool::ge(location, lhs, rhs)?,
                    Add => felt::add(location, lhs, rhs)?,
                    Sub => felt::sub(location, lhs, rhs)?,
                    Mul => felt::mul(location, lhs, rhs)?,
                    Div => felt::div(location, lhs, rhs)?,
                    // felt.smod or felt.umod?
                    Mod => felt::umod(location, lhs, rhs)?,
                    BitAnd => felt::bit_and(location, lhs, rhs)?,
                    Pow => felt::pow(location, lhs, rhs)?,
                };
                let value = self.top_mut().append_operation_with_result(op)?;
                assert_eq!(*meta, value.r#type());
                Ok(value)
            }
            Unary { op, expr, meta, .. } => {
                let value = expr.accept(self)?;
                let op = match op {
                    Not => bool::not(location, value)?,
                    Neg => felt::neg(location, value)?,
                };
                let value = self.top_mut().append_operation_with_result(op)?;
                assert_eq!(*meta, value.r#type());
                Ok(value)
            }
            Index { .. } => todo!("index expression is not supported yet"),
            Member { .. } => todo!("member expression is not supported yet"),
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
            Len { .. } => todo!("len expression is not supported yet"),
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
