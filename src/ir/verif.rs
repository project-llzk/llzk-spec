//! Functions for emitting MLIR IR representing specifications using LLZK's `verif` dialect.
//!
//! Only emitting IR to a separate file is currently supported. In the future we want to support
//! emitting IR inlined with an existing LLZK module.

use llzk::{
    builder::OpBuilder,
    dialect::{bool, felt, function, llzk::nondet},
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
        llzk::LlzkInfo,
        verif::{
            helpers::accept_in_new_scope,
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
pub fn emit_on_empty_module<'ctx, 'ast>(
    ctx: &'ctx Context,
    ast: &'ast AstContext,
    filename: &str,
    document: &ast::Document<'ast>,
    circuit: &'ctx Module,
) -> Result<Module<'ctx>, CompileError> {
    let typed_document = TypeChecker::check(
        MlirTypeSystem::new(ctx),
        &LlzkInfo::new(circuit),
        ast,
        filename,
        document,
    )?;
    let module = ctx.fresh_module(filename, document.span());
    SpecCodegen::new(ctx, &module, filename.to_owned()).emit_ir(&typed_document)?;
    Ok(module)
}

/// Code generator of specifications.
struct SpecCodegen<'ast, 'ctx, 'blk> {
    ctx: &'ctx Context,
    scope: CodegenScopeStack<'ast, 'ctx, 'blk>,
    filename: String,
    builder: OpBuilder<'ctx>,
}

impl<'ast, 'ctx, 'blk> SpecCodegen<'ast, 'ctx, 'blk>
where
    'blk: 'ctx,
{
    /// Creates a new code generator.
    fn new(ctx: &'ctx Context, module: &'blk Module<'ctx>, filename: String) -> Self {
        Self {
            ctx,
            scope: CodegenScopeStack::new(ScopeData::root(module)),
            filename,
            builder: OpBuilder::new(&ctx.context),
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

    fn visit(&mut self, _: &TypedContractDecl<'ast, 'ctx>) -> Self::Output {
        todo!("lowering contracts it not currently supported")
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
                _ => todo!("invariant statement is not supported yet"),
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
                let name = self.find_actual_function_name(callee)?;
                let op = function::call(self.builder(), location, name, &args, &[*meta])?;
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
