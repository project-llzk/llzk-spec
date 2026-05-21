//! Functions for emitting MLIR IR representing specifications using LLZK's `verif` dialect.
//!
//! Only emitting IR to a separate file is currently supported. In the future we want to support
//! emitting IR inlined with an existing LLZK module.

use std::collections::HashMap;

use llzk::{
    builder::OpBuilder,
    dialect::{bool, felt, function, llzk::nondet},
    prelude::*,
};
use melior::{
    dialect::{arith, scf},
    ir::{BlockRef, Module},
};

use crate::{
    ast::{self, Spanned, Visitable},
    diagnostic::CompileError,
    ir::{Context, MlirTypeSystem},
    type_analysis::TypeChecker,
};

// Typed AST entities
type TypedDocument<'ast, 'ctx> = ast::Document<'ast, Type<'ctx>>;
type TypedItem<'ast, 'ctx> = ast::Item<'ast, Type<'ctx>>;
type TypedContractDecl<'ast, 'ctx> = ast::ContractDecl<'ast, Type<'ctx>>;
type TypedPredicateDecl<'ast, 'ctx> = ast::PredicateDecl<'ast, Type<'ctx>>;
type TypedBlock<'ast, 'ctx> = ast::Block<'ast, Type<'ctx>>;
type TypedStatement<'ast, 'ctx> = ast::Statement<'ast, Type<'ctx>>;
type TypedExpression<'ast, 'ctx> = ast::Expression<'ast, Type<'ctx>>;
type TypedIdentifier<'ast, 'ctx> = ast::Identifier<'ast, Type<'ctx>>;

/// Generates IR for the given [`Document`] on a fresh module.
pub fn emit_on_empty_module<'ctx>(
    ctx: &'ctx Context,
    filename: &str,
    document: &ast::Document,
) -> Result<Module<'ctx>, CompileError> {
    let typed_document = TypeChecker::check(MlirTypeSystem::new(ctx), filename, document)?;
    let module = ctx.fresh_module(filename, document.span());
    SpecCodegen::new(ctx, &module, filename.to_owned()).emit_ir(&typed_document)?;
    Ok(module)
}

/// Code generator of specifications.
struct SpecCodegen<'ast, 'ctx, 'blk> {
    ctx: &'ctx Context,
    scope: Vec<Scope<'ast, 'ctx, 'blk>>,
    filename: String,
    builder: OpBuilder<'ctx>,
}

impl<'ctx, 'blk> SpecCodegen<'_, 'ctx, 'blk>
where
    'blk: 'ctx,
{
    /// Creates a new code generator.
    fn new(ctx: &'ctx Context, module: &'blk Module<'ctx>, filename: String) -> Self {
        Self {
            ctx,
            scope: vec![Scope::root(module)],
            filename,
            builder: OpBuilder::new(&ctx.context),
        }
    }
}

impl<'ast, 'ctx, 'blk> SpecCodegen<'ast, 'ctx, 'blk> {
    fn builder(&self) -> &OpBuilder<'ctx> {
        &self.builder
    }

    fn context(&self) -> &'ctx melior::Context {
        &self.ctx.context
    }

    fn emit_ir(mut self, document: &TypedDocument<'ast, 'ctx>) -> Result<(), CompileError> {
        document.accept(&mut self)
    }

    fn top_mut(&mut self) -> &mut Scope<'ast, 'ctx, 'blk> {
        self.scope.last_mut().unwrap()
    }

    fn push(&mut self, block: BlockRef<'ctx, 'blk>) {
        self.scope.push(Scope::new(block))
    }

    fn pop(&mut self) {
        self.scope.pop();
    }

    /// Creates a `function.def` operation.
    fn create_func_def_op(
        &self,
        span: ast::Span,
        name: &TypedIdentifier<'ast, 'ctx>,
    ) -> Result<FuncDefOp<'ctx>, CompileError> {
        let op = function::def(
            self.location(span),
            name.as_ref(),
            (*name.meta()).try_into()?,
            &[],
            None,
        )?;
        op.set_allow_non_native_field_ops_attr(true);
        Ok(op)
    }

    fn func_type(&self, ins: &[Type<'ctx>], outs: &[Type<'ctx>]) -> FunctionType<'ctx> {
        self.ctx.func_type(ins, outs)
    }

    fn bool_type(&self) -> Type<'ctx> {
        self.ctx.bool_type()
    }

    fn felt_type(&self) -> Type<'ctx> {
        self.ctx.felt_type()
    }

    fn location(&self, span: ast::Span) -> Location<'ctx> {
        self.ctx.location_from_span(&self.filename, span)
    }

    /// Traverses the scope from the top until it finds what is looking for or returns an error
    /// otherwise.
    fn find_in_scope<R>(
        &self,
        find_cb: impl FnMut(&Scope<'ast, 'ctx, 'blk>) -> Option<R>,
        on_error: impl FnOnce() -> CompileError,
    ) -> Result<R, CompileError> {
        self.scope
            .iter()
            .rev()
            .find_map(find_cb)
            .ok_or_else(on_error)
    }

    fn find_symbol(
        &self,
        name: &TypedIdentifier<'ast, 'ctx>,
    ) -> Result<Value<'ctx, 'blk>, CompileError> {
        self.find_in_scope(
            |scope| scope.locals.get(&name.symbol()).copied(),
            || CompileError::Ir(format!("local symbol '{}' not found", name.value())),
        )
    }

    fn find_predicate(
        &self,
        name: &TypedIdentifier<'ast, 'ctx>,
    ) -> Result<FuncDefOpRef<'ctx, 'blk>, CompileError> {
        self.find_in_scope(
            |scope| scope.predicates.get(&name.symbol()).copied(),
            || CompileError::Ir(format!("predicate symbol '{}' not found", name.value())),
        )
    }
}

fn accept_in_new_scope<'ast, 'ctx, 'blk, V, R>(
    region: &Region<'ctx>,
    scope: &mut SpecCodegen<'ast, 'ctx, 'blk>,
    target: &V,
    tail_cb: impl FnOnce(R, &mut SpecCodegen<'ast, 'ctx, 'blk>) -> Result<R, CompileError>,
) -> Result<R, CompileError>
where
    V: Visitable,
    SpecCodegen<'ast, 'ctx, 'blk>: ast::Visitor<V, Output = Result<R, CompileError>>,
{
    let block_ref = region.append_block(melior::ir::Block::new(&[]));
    scope.push(block_ref);
    let result = target.accept(scope)?;
    let result = tail_cb(result, scope)?;
    scope.pop();
    Ok(result)
}

impl<'ast, 'ctx> ast::Visitor<TypedDocument<'ast, 'ctx>> for SpecCodegen<'ast, 'ctx, '_> {
    type Output = Result<(), CompileError>;

    fn visit(&mut self, document: &TypedDocument<'ast, 'ctx>) -> Self::Output {
        document
            .items()
            .iter()
            .try_for_each(|item| item.accept(self))
    }
}

impl<'ast, 'ctx> ast::Visitor<TypedItem<'ast, 'ctx>> for SpecCodegen<'ast, 'ctx, '_> {
    type Output = Result<(), CompileError>;

    fn visit(&mut self, item: &TypedItem<'ast, 'ctx>) -> Self::Output {
        match item {
            TypedItem::Contract(_) => todo!("lowering contracts it not currently supported"),
            TypedItem::Predicate(decl) => decl.accept(self),
        }
    }
}

/// Returns the inputs of a function type in a `Vec`.
fn func_type_inputs<'ctx>(func_type: FunctionType<'ctx>) -> Result<Vec<Type<'ctx>>, melior::Error> {
    (0..(func_type.input_count()))
        .map(|n| func_type.input(n))
        .collect()
}

impl<'ast, 'ctx, 'blk> SpecCodegen<'ast, 'ctx, 'blk> {
    /// Pushes a block where to emit the body of a predicate.
    fn bind_and_push_predicate_block(
        &mut self,
        decl: &TypedPredicateDecl<'ast, 'ctx>,
    ) -> Result<(), CompileError> {
        let param_types = func_type_inputs((*decl.name().meta()).try_into()?)?;
        let param_locations = decl.params().iter().map(|i| self.location(i.span()));

        let block_args = std::iter::zip(param_types, param_locations).collect::<Vec<_>>();

        // Insert the function into the predicates' bindings of the current scope.
        let func_op = self.create_func_def_op(decl.span(), decl.name())?;
        let func_op = self.top_mut().bind_predicate(decl.name(), func_op)?;

        let block = func_op
            .region(0)?
            .append_block(::melior::ir::Block::new(&block_args));

        self.push(block);

        // Bind the formals to their block arguments.
        decl.params()
            .iter()
            .enumerate()
            .map(|(n, formal)| -> Result<(_, Value), CompileError> {
                let value = Value::from(block.argument(n)?);
                Ok((formal, value))
            })
            .try_for_each(|r| {
                let (name, value) = r?;
                self.top_mut().bind_local(name, value)
            })
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
        block
            .statements()
            .iter()
            .try_for_each(|stmt| stmt.accept(self))
    }
}

impl<'ast, 'ctx> ast::Visitor<TypedStatement<'ast, 'ctx>> for SpecCodegen<'ast, 'ctx, '_> {
    type Output = Result<(), CompileError>;

    fn visit(&mut self, stmt: &TypedStatement<'ast, 'ctx>) -> Self::Output {
        use ast::Statement::*;
        match stmt {
            Scoped { .. } => todo!("scoped statement is not supported yet"),
            Block(block) => {
                // Wrap the body of the block to ensure that SSA values don't leak in case of bugs
                // in the scope logic.
                let region = Region::new();
                accept_in_new_scope(&region, self, block, |_, _| Ok(()))?;
                let op = scf::execute_region(&[], region, self.location(block.span()));
                self.top_mut().append_operation(op);
                Ok(())
            }
            Require { .. } => todo!("require statement is not supported yet"),
            Ensure { .. } => todo!("ensure statement is not supported yet"),
            // TODO: We are considering that let expressions are not like Rust's in that they
            // won't shadow existing names. If they do, then we need to push a new scope.
            // They way this AST works, pushing a new scope will make it tricky to determine how
            // many pops we will need at the end to reach the baseline scope. This is because this
            // let expression only contains the binding and assumes that everything after will
            // depend on it. To allow shadowing with these AST structures we need to either make
            // this let variant a 'let-in' kind of binding (`let {binding} := {expression} in {block}`) or
            // wrap 'lets' inside blocks (that already handle pushing and poping scopes) with the
            // rest of the AST generated after. And because life is never that simple, this second
            // approach will require some handling of 'return' inside the block because they get
            // wrapped in a `scf.execute_region` and `function.return` ops are terminators of
            // `function.def` ops (meaning, we can't have it inside the `scf.execute_region` op and they must be the
            // last op in the function's body).
            Let { name, value, .. } => {
                let value = value.accept(self)?;
                self.top_mut().bind_local(name, value)
            }
            Unused { .. } => todo!("unused statement is not supported yet"),
            Return { expression, span } => {
                let value = expression.accept(self)?;
                let location = self.location(*span);
                self.top_mut()
                    .append_operation(function::r#return(location, &[value]));
                Ok(())
            }
            Increases { .. } => todo!("increases statement is not supported yet"),
            Decreases { .. } => todo!("decreases statement is not supported yet"),
            Step { .. } => todo!("step statement is not supported yet"),
            Invariant(_) => todo!("invariant decl statement is not supported yet"),
            Predicate(_) => todo!("predicate decl statement is not supported yet"),
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
                ..
            } => {
                let condition = condition.accept(self)?;
                let then_region = Region::new();
                let then_result = accept_in_new_scope(
                    &then_region,
                    self,
                    then_branch,
                    |result, scope: &mut SpecCodegen<'ast, 'ctx, 'blk>| {
                        let op = scf::r#yield(&[result], location);
                        scope.top_mut().append_operation(op);
                        Ok(result)
                    },
                )?;

                let else_region = Region::new();
                let else_result = accept_in_new_scope(
                    &else_region,
                    self,
                    else_branch,
                    |result, scope: &mut SpecCodegen<'ast, 'ctx, 'blk>| {
                        let op = scf::r#yield(&[result], location);
                        scope.top_mut().append_operation(op);
                        Ok(result)
                    },
                )?;
                if then_result.r#type() != else_result.r#type() {
                    return Err(CompileError::Ir(format!(
                        "incompatible type in conditional branches {} != {}",
                        then_result.r#type(),
                        else_result.r#type()
                    )));
                }
                let result_types = [then_result.r#type()];

                let op = scf::r#if(condition, &result_types, then_region, else_region, location);
                self.top_mut().append_operation_with_result(op)
            }
            Binary {
                op, left, right, ..
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
                self.top_mut().append_operation_with_result(op)
            }
            Unary { op, expr, .. } => {
                let value = expr.accept(self)?;
                let op = match op {
                    Not => bool::not(location, value)?,
                    Neg => felt::neg(location, value)?,
                };
                self.top_mut().append_operation_with_result(op)
            }
            Index { .. } => todo!("index expression is not supported yet"),
            Member { .. } => todo!("member expression is not supported yet"),
            Call { callee, args, .. } => {
                let args = args
                    .iter()
                    .map(|expr| expr.accept(self))
                    .collect::<Result<Vec<_>, _>>()?;
                let name = StringAttribute::try_from(
                    self.find_predicate(callee)?
                        .attribute("sym_name")
                        .expect("'function.def' has attribute 'sym_name'"),
                )?;
                let name = FlatSymbolRefAttribute::new(self.context(), name.value());
                let op =
                    function::call(self.builder(), location, name, &args, &[self.bool_type()])?;
                self.top_mut().append_operation_with_result(op)
            }
            Quantifier { .. } => todo!("quantifier expression is not supported yet"),
            Len { .. } => todo!("len expression is not supported yet"),
            Old { .. } => todo!("old expression is not supported yet"),
            Arg { .. } => todo!("arg expression is not supported yet"),
            Nondet { .. } => {
                let felt_type = self.felt_type();
                self.top_mut()
                    .append_operation_with_result(nondet(location, felt_type))
            }
            Boolean { value, .. } => {
                let op = arith::constant(
                    self.context(),
                    IntegerAttribute::new(self.bool_type(), (*value).into()).into(),
                    location,
                );
                self.top_mut().append_operation_with_result(op)
            }
            Number { value, .. } => {
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

/// Entry in the scope stack.
struct Scope<'ast, 'ctx, 'blk> {
    // Current insertion block.
    block: BlockRef<'ctx, 'blk>,
    // Binds names to predicates.
    predicates: HashMap<ast::Symbol<'ast>, FuncDefOpRef<'ctx, 'blk>>,
    // Binds local names to SSA values.
    locals: HashMap<ast::Symbol<'ast>, Value<'ctx, 'blk>>,
}

impl<'ast, 'ctx, 'blk> Scope<'ast, 'ctx, 'blk> {
    fn root<'m>(module: &'blk Module<'ctx>) -> Self
    where
        'blk: 'ctx,
    {
        Self::new(module.body())
    }

    fn new(block: BlockRef<'ctx, 'blk>) -> Self {
        Self {
            block,
            predicates: Default::default(),
            locals: Default::default(),
        }
    }

    fn bind_predicate(
        &mut self,
        name: &TypedIdentifier<'ast, 'ctx>,
        func_op: FuncDefOp<'ctx>,
    ) -> Result<FuncDefOpRef<'ctx, 'blk>, CompileError> {
        if self.predicates.contains_key(&name.symbol()) {
            return Err(CompileError::Ir(format!(
                "duplicate predicate '{}'",
                name.value()
            )));
        }
        let op_ref: FuncDefOpRef<'ctx, 'blk> = self.append_operation(func_op).try_into()?;
        self.predicates.insert(name.symbol(), op_ref);
        Ok(op_ref)
    }

    fn bind_local(
        &mut self,
        name: &TypedIdentifier<'ast, 'ctx>,
        value: Value<'ctx, 'blk>,
    ) -> Result<(), CompileError> {
        if self.locals.contains_key(&name.symbol()) {
            return Err(CompileError::Ir(format!(
                "duplicate local '{}'",
                name.value()
            )));
        }
        self.locals.insert(name.symbol(), value);
        Ok(())
    }

    fn append_operation(&mut self, op: impl Into<Operation<'ctx>>) -> OperationRef<'ctx, 'blk> {
        self.block.append_operation(op.into())
    }

    fn append_operation_with_result(
        &mut self,
        op: impl Into<Operation<'ctx>>,
    ) -> Result<Value<'ctx, 'blk>, CompileError> {
        let op_ref = self.append_operation(op);
        if op_ref.result_count() != 1 {
            return Err(CompileError::Ir(format!(
                "expected operation '{op_ref}' to have 1 result but has {}",
                op_ref.result_count()
            )));
        }
        Ok(op_ref.result(0)?.into())
    }
}
