//! Functions for emitting MLIR IR representing specifications using LLZK's `verif` dialect.
//!
//! Only emitting IR to a separate file is currently supported. In the future we want to support
//! emitting IR inlined with an existing LLZK module.

// TODO: This lowering may act as semantic analysis for the spec language, which means that
// we'll have to collect errors in a nicer way that just giving up with `Result::Err`. Also we may
// want to wire MLIR diagnostics to our own s.t. they show up to the user with the same formatting.

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
    ast::{self, Visitable},
    diagnostic::CompileError,
    ir::Context,
};

/// Generates IR for the given [`Document`] on a fresh module.
pub fn emit_on_empty_module<'ctx>(
    ctx: &'ctx Context,
    filename: &str,
    document: &ast::Document,
    prime: Option<&str>,
) -> Result<Module<'ctx>, CompileError> {
    let module = ctx.fresh_module(filename, document.span);
    SpecCodegen::new(
        ctx,
        &module,
        filename.to_owned(),
        prime.map(ToOwned::to_owned),
    )
    .emit_ir(document)?;
    Ok(module)
}

/// Code generator of specifications.
struct SpecCodegen<'ctx, 'blk> {
    ctx: &'ctx Context,
    scope: Vec<Scope<'ctx, 'blk>>,
    filename: String,
    prime: Option<String>,
    builder: OpBuilder<'ctx>,
}

impl<'ctx, 'blk> SpecCodegen<'ctx, 'blk>
where
    'blk: 'ctx,
{
    /// Creates a new code generator.
    fn new(
        ctx: &'ctx Context,
        module: &'blk Module<'ctx>,
        filename: String,
        prime: Option<String>,
    ) -> Self {
        Self {
            ctx,
            scope: vec![Scope::root(module)],
            filename,
            prime,
            builder: OpBuilder::new(&ctx.context),
        }
    }
}

impl<'ctx, 'blk> SpecCodegen<'ctx, 'blk> {
    fn builder(&self) -> &OpBuilder<'ctx> {
        &self.builder
    }

    fn context(&self) -> &'ctx melior::Context {
        &self.ctx.context
    }

    fn emit_ir(mut self, document: &ast::Document) -> Result<(), CompileError> {
        document.accept(&mut self)
    }

    fn top_mut(&mut self) -> &mut Scope<'ctx, 'blk> {
        self.scope.last_mut().unwrap()
    }

    fn push(&mut self, block: BlockRef<'ctx, 'blk>) {
        self.scope.push(Scope::new(block))
    }

    fn pop(&mut self) {
        self.scope.pop();
    }

    fn create_func_def_op(
        &self,
        span: ast::Span,
        name: &ast::Identifier,
        inputs: &[Type<'ctx>],
        outputs: &[Type<'ctx>],
    ) -> Result<FuncDefOp<'ctx>, CompileError> {
        let op = function::def(
            self.location(span),
            name.as_ref(),
            self.func_type(inputs, outputs),
            &[],
            None,
        )?;
        op.set_allow_non_native_field_ops_attr(true);
        Ok(op)
    }

    fn func_type(&self, ins: &[Type<'ctx>], outs: &[Type<'ctx>]) -> FunctionType<'ctx> {
        FunctionType::new(self.context(), ins, outs)
    }

    fn bool_type(&self) -> Type<'ctx> {
        IntegerType::new(self.context(), 1).into()
    }

    fn felt_type(&self) -> Type<'ctx> {
        match &self.prime {
            Some(prime) => FeltType::with_field(self.context(), prime),
            None => FeltType::new(self.context()),
        }
        .into()
    }

    fn location(&self, span: ast::Span) -> Location<'ctx> {
        self.ctx.location_from_span(&self.filename, span)
    }

    /// Traverses the scope from the top until it finds what is looking for or returns an error
    /// otherwise.
    fn find_in_scope<R>(
        &self,
        find_cb: impl FnMut(&Scope<'ctx, 'blk>) -> Option<R>,
        on_error: impl FnOnce() -> CompileError,
    ) -> Result<R, CompileError> {
        self.scope
            .iter()
            .rev()
            .find_map(find_cb)
            .ok_or_else(on_error)
    }

    fn find_symbol(&self, symbol: &ast::Identifier) -> Result<Value<'ctx, 'blk>, CompileError> {
        self.find_in_scope(
            |scope| scope.locals.get(symbol.as_ref()).copied(),
            || CompileError::Ir(format!("local symbol '{}' not found", symbol.name)),
        )
    }

    fn find_predicate(
        &self,
        symbol: &ast::Identifier,
    ) -> Result<FuncDefOpRef<'ctx, 'blk>, CompileError> {
        self.find_in_scope(
            |scope| scope.predicates.get(symbol.as_ref()).copied(),
            || CompileError::Ir(format!("predicate symbol '{}' not found", symbol.name)),
        )
    }
}

fn accept_in_new_scope<'ctx, 'blk, V, R>(
    region: &Region<'ctx>,
    scope: &mut SpecCodegen<'ctx, 'blk>,
    target: &V,
    tail_cb: impl FnOnce(R, &mut SpecCodegen<'ctx, 'blk>) -> Result<R, CompileError>,
) -> Result<R, CompileError>
where
    V: Visitable,
    SpecCodegen<'ctx, 'blk>: ast::Visitor<V, Output = Result<R, CompileError>>,
{
    let block_ref = region.append_block(melior::ir::Block::new(&[]));
    scope.push(block_ref);
    let result = target.accept(scope)?;
    let result = tail_cb(result, scope)?;
    scope.pop();
    Ok(result)
}

impl ast::Visitor<ast::Document> for SpecCodegen<'_, '_> {
    type Output = Result<(), CompileError>;

    fn visit(&mut self, document: &ast::Document) -> Self::Output {
        document.items.iter().try_for_each(|item| item.accept(self))
    }
}

impl ast::Visitor<ast::Item> for SpecCodegen<'_, '_> {
    type Output = Result<(), CompileError>;

    fn visit(&mut self, item: &ast::Item) -> Self::Output {
        match item {
            ast::Item::Contract(_) => todo!("lowering contracts it not currently supported"),
            ast::Item::Predicate(decl) => decl.accept(self),
        }
    }
}

impl ast::Visitor<ast::PredicateDecl> for SpecCodegen<'_, '_> {
    type Output = Result<(), CompileError>;

    fn visit(&mut self, decl: &ast::PredicateDecl) -> Self::Output {
        let bool_type = self.bool_type();
        let param_types = vec![bool_type; decl.params.len()];
        // Create the FuncDefOp and insert it into the current block.
        let func_op = self.create_func_def_op(decl.span, &decl.name, &param_types, &[bool_type])?;
        // Insert the function into the predicates' bindings of the current scope.
        let func_op = self.top_mut().bind_predicate(func_op)?;
        // Push a new scope using the first block of the function
        let param_locations = decl.params.iter().map(|i| self.location(i.span));
        let block_args = std::iter::zip(param_types, param_locations).collect::<Vec<_>>();
        let block = func_op
            .region(0)?
            .append_block(::melior::ir::Block::new(&block_args));
        self.push(block);
        // Bind the formals to their block arguments.
        decl.params
            .iter()
            .enumerate()
            .map(|(n, formal)| -> Result<(String, Value), CompileError> {
                let value = Value::from(block.argument(n)?);
                Ok((formal.name.clone(), value))
            })
            .try_for_each(|r| {
                let (name, value) = r?;
                self.top_mut().bind_local(name, value)
            })?;
        // Lower the body of the predicate.
        decl.body.accept(self)?;
        self.pop();
        Ok(())
    }
}

impl ast::Visitor<ast::Block> for SpecCodegen<'_, '_> {
    type Output = Result<(), CompileError>;

    fn visit(&mut self, block: &ast::Block) -> Self::Output {
        block
            .statements
            .iter()
            .try_for_each(|stmt| stmt.accept(self))
    }
}

impl ast::Visitor<ast::Statement> for SpecCodegen<'_, '_> {
    type Output = Result<(), CompileError>;

    fn visit(&mut self, stmt: &ast::Statement) -> Self::Output {
        use ast::Statement::*;
        match stmt {
            Scoped { .. } => todo!("scoped statement is not supported yet"),
            Block(block) => {
                // Wrap the body of the block to ensure that SSA values don't leak in case of bugs
                // in the scope logic.
                let region = Region::new();
                accept_in_new_scope(&region, self, block, |_, _| Ok(()))?;
                let op = scf::execute_region(&[], region, self.location(block.span));
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
                self.top_mut().bind_local(name.name.clone(), value)
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

impl<'ctx, 'blk> ast::Visitor<ast::Expression> for SpecCodegen<'ctx, 'blk> {
    type Output = Result<Value<'ctx, 'blk>, CompileError>;

    fn visit(&mut self, expr: &ast::Expression) -> Self::Output {
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
                    |result, scope: &mut SpecCodegen<'ctx, 'blk>| {
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
                    |result, scope: &mut SpecCodegen<'ctx, 'blk>| {
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
                let value =
                    FeltConstAttribute::from_biguint(self.context(), value, self.prime.as_deref());
                self.top_mut()
                    .append_operation_with_result(felt::constant(location, value)?)
            }
            Symbol(symbol) => self.find_symbol(symbol),
        }
    }
}

/// Entry in the scope stack.
struct Scope<'ctx, 'blk> {
    // Current insertion block.
    block: BlockRef<'ctx, 'blk>,
    // Binds names to predicates.
    predicates: HashMap<String, FuncDefOpRef<'ctx, 'blk>>,
    // Binds local names to SSA values.
    locals: HashMap<String, Value<'ctx, 'blk>>,
}

impl<'ctx, 'blk> Scope<'ctx, 'blk> {
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
        func_op: FuncDefOp<'ctx>,
    ) -> Result<FuncDefOpRef<'ctx, 'blk>, CompileError> {
        let name = func_op
            .attribute("sym_name")
            .expect("'function.def' has 'sym_name' attribute");
        let name = StringAttribute::try_from(name)?;
        let name = name.value();
        if self.predicates.contains_key(name) {
            return Err(CompileError::Ir(format!("duplicate predicate '{name}'")));
        }
        let op_ref: FuncDefOpRef<'ctx, 'blk> = self.append_operation(func_op).try_into()?;
        self.predicates.insert(name.to_owned(), op_ref);
        Ok(op_ref)
    }

    fn bind_local(&mut self, name: String, value: Value<'ctx, 'blk>) -> Result<(), CompileError> {
        if self.locals.contains_key(&name) {
            return Err(CompileError::Ir(format!("duplicate local '{name}'")));
        }
        self.locals.insert(name, value);
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
