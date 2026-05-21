//! Helper methods for emitting IR.

use ::melior::ir::Block;
use llzk::{
    builder::OpBuilder,
    dialect::{function, poly::unifiable_cast},
    prelude::{
        FlatSymbolRefAttribute, FuncDefOp, FuncDefOpLike as _, FuncDefOpRef, FunctionType,
        OperationLike as _, StringAttribute, melior_dialects::scf,
    },
};
use melior::ir::{
    BlockLike as _, BlockRef, Location, Region, RegionLike as _, Type, Value, ValueLike,
};

use crate::{
    ast::{self, Span, Spanned as _, Visitable, Visitor},
    diagnostic::CompileError,
    ir::verif::{Scope, SpecCodegen, TypedExpression, TypedIdentifier, TypedPredicateDecl},
};

impl<'ast, 'ctx, 'blk> SpecCodegen<'ast, 'ctx, 'blk> {
    /// Pushes a block where to emit the body of a predicate.
    pub fn bind_and_push_predicate_block(
        &mut self,
        decl: &TypedPredicateDecl<'ast, 'ctx>,
    ) -> Result<(), CompileError> {
        let param_types = func_type_inputs((*decl.name().meta()).try_into()?)?;
        let param_locations = decl.params().iter().map(|i| self.location(i.span()));

        let block_args = std::iter::zip(param_types, param_locations).collect::<Vec<_>>();

        // Insert the function into the predicates' bindings of the current scope.
        let func_op = self.create_func_def_op(decl.span(), decl.name())?;
        let func_op = self.top_mut().bind_predicate(decl.name(), func_op)?;

        let block = func_op.region(0)?.append_block(Block::new(&block_args));

        self.push(block);

        // Bind the formals to their block arguments.
        decl.params()
            .iter()
            .enumerate()
            .map(|(n, formal)| -> Result<(_, Value), CompileError> {
                Ok((formal, block.argument(n)?.into()))
            })
            .try_for_each(|r| {
                let (name, value) = r?;
                self.top_mut().bind_local(name, value)
            })
    }

    pub fn builder(&self) -> &OpBuilder<'ctx> {
        &self.builder
    }

    pub fn context(&self) -> &'ctx melior::Context {
        &self.ctx.context
    }

    pub fn top_mut(&mut self) -> &mut Scope<'ast, 'ctx, 'blk> {
        self.scope.last_mut().unwrap()
    }

    pub fn push(&mut self, block: BlockRef<'ctx, 'blk>) {
        self.scope.push(Scope::new(block))
    }

    pub fn pop(&mut self) {
        self.scope.pop();
    }

    /// Creates a `function.def` operation.
    pub fn create_func_def_op(
        &self,
        span: Span,
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

    pub fn felt_type(&self) -> Type<'ctx> {
        self.ctx.felt_type()
    }

    pub fn location(&self, span: Span) -> Location<'ctx> {
        self.ctx.location_from_span(&self.filename, span)
    }

    /// Traverses the scope from the top until it finds what is looking for or returns an error
    /// otherwise.
    pub fn find_in_scope<R>(
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

    pub fn find_symbol(
        &self,
        name: &TypedIdentifier<'ast, 'ctx>,
    ) -> Result<Value<'ctx, 'blk>, CompileError> {
        self.find_in_scope(
            |scope| scope.locals().get(&name.symbol()).copied(),
            || CompileError::Ir(format!("local symbol '{}' not found", name.value())),
        )
    }

    pub fn find_predicate(
        &self,
        name: &TypedIdentifier<'ast, 'ctx>,
    ) -> Result<FuncDefOpRef<'ctx, 'blk>, CompileError> {
        self.find_in_scope(
            |scope| scope.predicates().get(&name.symbol()).copied(),
            || CompileError::Ir(format!("predicate symbol '{}' not found", name.value())),
        )
    }

    pub fn lower_conditional_branch(
        &mut self,
        region: &Region<'ctx>,
        expr: &TypedExpression<'ast, 'ctx>,
        location: Location<'ctx>,
        expected_type: Type<'ctx>,
    ) -> Result<Value<'ctx, 'blk>, CompileError> {
        accept_in_new_scope(region, self, expr, |mut result, scope: &mut Self| {
            if result.r#type() != expected_type {
                result = scope
                    .top_mut()
                    .append_operation_with_result(unifiable_cast(
                        location,
                        result,
                        expected_type,
                    ))?;
            }
            let op = scf::r#yield(&[result], location);
            scope.top_mut().append_operation(op);
            Ok(result)
        })
    }

    pub fn find_actual_function_name(
        &self,
        name: &TypedIdentifier<'ast, 'ctx>,
    ) -> Result<FlatSymbolRefAttribute<'ctx>, CompileError> {
        let name = StringAttribute::try_from(
            self.find_predicate(name)?
                .attribute("sym_name")
                .expect("'function.def' has attribute 'sym_name'"),
        )?;
        Ok(FlatSymbolRefAttribute::new(self.context(), name.value()))
    }

    pub fn visit_many<V, R>(&mut self, entities: &[V]) -> Result<Vec<R>, CompileError>
    where
        Self: Visitor<V, Output = Result<R, CompileError>>,
        V: Visitable,
    {
        entities.iter().map(|e| e.accept(self)).collect()
    }
}

pub fn accept_in_new_scope<'ast, 'ctx, 'blk, V, R>(
    region: &Region<'ctx>,
    scope: &mut SpecCodegen<'ast, 'ctx, 'blk>,
    target: &V,
    tail_cb: impl FnOnce(R, &mut SpecCodegen<'ast, 'ctx, 'blk>) -> Result<R, CompileError>,
) -> Result<R, CompileError>
where
    V: Visitable,
    SpecCodegen<'ast, 'ctx, 'blk>: Visitor<V, Output = Result<R, CompileError>>,
{
    let block_ref = region.append_block(melior::ir::Block::new(&[]));
    scope.push(block_ref);
    let result = target.accept(scope)?;
    let result = tail_cb(result, scope)?;
    scope.pop();
    Ok(result)
}

/// Returns the inputs of a function type in a `Vec`.
fn func_type_inputs<'ctx>(func_type: FunctionType<'ctx>) -> Result<Vec<Type<'ctx>>, melior::Error> {
    (0..(func_type.input_count()))
        .map(|n| func_type.input(n))
        .collect()
}
