//! Helper methods for emitting IR.

use ::melior::ir::Block;
use llzk::{
    builder::OpBuilder,
    dialect::{function, poly::unifiable_cast},
    prelude::{
        FlatSymbolRefAttribute, FuncDefOp, FuncDefOpLike as _, FuncDefOpRef, FunctionType,
        LlzkContext, OperationLike as _, StringAttribute, melior_dialects::scf,
    },
};
use melior::ir::{
    BlockLike as _, BlockRef, Location, Region, RegionLike as _, Type, Value, ValueLike,
};

use crate::{
    ast::{Span, Spanned as _, Visitable, Visitor},
    diagnostic::CompileError,
    ir::verif::{
        SpecCodegen, TypedExpression, TypedIdentifier, TypedPredicateDecl,
        scope::{CodegenScope, ScopeData, ScopeTag},
    },
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
        let func_op = self.bind_predicate(decl.name(), func_op)?;

        let block = func_op.get_body()?.append_block(Block::new(&block_args));

        self.push_tagged(block, ScopeTag::Predicate);

        // Bind the formals to their block arguments.
        for (index, formal) in decl.params().iter().enumerate() {
            let value = block.argument(index)?;
            self.top_mut()
                .bind_parameter(formal, value.into(), index)
                .map_err(|err| {
                    err.into_compile_error(
                        &self.filename,
                        Some(decl.span()),
                        format!(
                            "on parameter #{index} '{}' of predicate '{}'",
                            formal.value(),
                            decl.name().value()
                        ),
                    )
                })?;
        }
        Ok(())
    }

    /// Binds a predicate into the top of the stack. However, the actual operation is inserted on
    /// the first scope that can accept `function.def` operations.
    ///
    /// Since predicates can be defined locally inside contracts or other predicates the top of the
    /// stack may be a block whose parent operation is not the kind expected by `function.def`
    /// operations.
    fn bind_predicate(
        &mut self,
        name: &TypedIdentifier<'ast, 'ctx>,
        func_op: FuncDefOp<'ctx>,
    ) -> Result<FuncDefOpRef<'ctx, 'blk>, CompileError> {
        // Append the operation into the correct scope.
        let scope = self
            .scope
            .ordered_scopes_mut()
            .find(|scope| scope.tag().is_some_and(ScopeTag::accepts_function_def_ops))
            .unwrap();
        let op_ref = scope.append_with_symbol_uniquing(func_op);
        // But bind it in the top of the stack.
        self.top_mut()
            .bind_predicate(name, op_ref.try_into()?)
            .map_err(|err| {
                err.into_compile_error(
                    &self.filename,
                    Some(name.span()),
                    format!("on declaration of predicate '{}'", name.value()),
                )
            })?;
        Ok(op_ref.try_into()?)
    }

    /// Returns a reference to an [`OpBuilder`].
    pub fn builder(&self) -> &OpBuilder<'ctx> {
        &self.builder
    }

    /// Returns a reference to the LLZK context.
    pub fn context(&self) -> &'ctx LlzkContext {
        &self.ctx.context
    }

    /// Returns a mutable reference to the top of the scope stack.
    pub fn top_mut(&mut self) -> &mut CodegenScope<'ast, 'ctx, 'blk> {
        self.scope.top()
    }

    /// Pushes a new, untagged, scope.
    ///
    /// For pushing tagged scopes see [`Self::push_tagged`].
    pub fn push(&mut self, block: BlockRef<'ctx, 'blk>) {
        self.scope.push(ScopeData::new(block))
    }

    /// Pushes a new tagged scope.
    ///
    /// For pushing without a tag see [`Self::push`].
    pub fn push_tagged(&mut self, block: BlockRef<'ctx, 'blk>, tag: ScopeTag) {
        self.scope.push(ScopeData::new_with_tag(block, tag))
    }

    /// Pops the top of the scope stack.
    pub fn pop(&mut self) {
        self.scope.pop();
    }

    /// Returns the tag closest to the top of the stack.
    ///
    /// If the top scope is not tagged checks the next one, repeating until one is found.
    /// The root scope must always be tagged with [`ScopeTag::Root`].
    ///
    /// # Panics
    ///
    /// If the root scope is not tagged.
    pub fn closest_tag(&self) -> ScopeTag {
        self.find_in_scope(
            |scope| scope.tag(),
            || panic!("at least one scope must be tagged"),
        )
        .unwrap()
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

    /// Return the type representing felts.
    pub fn felt_type(&self) -> Type<'ctx> {
        self.ctx.felt_type()
    }

    /// Creates a MLIR location pointing to the given span.
    pub fn location(&self, span: Span) -> Location<'ctx> {
        self.ctx.location_from_span(&self.filename, span)
    }

    /// Traverses the scope from the top until it finds what is looking for or returns an error
    /// otherwise.
    pub fn find_in_scope<R>(
        &self,
        find_cb: impl FnMut(&CodegenScope<'ast, 'ctx, 'blk>) -> Option<R>,
        on_error: impl FnOnce() -> CompileError,
    ) -> Result<R, CompileError> {
        self.scope
            .ordered_scopes()
            .find_map(find_cb)
            .ok_or_else(on_error)
    }

    /// Looks for a SSA value bound to the given local symbol.
    pub fn find_symbol(
        &self,
        name: &TypedIdentifier<'ast, 'ctx>,
    ) -> Result<Value<'ctx, 'blk>, CompileError> {
        self.find_in_scope(
            |scope| scope.locals().get(&name.symbol()).copied(),
            || CompileError::Ir(format!("local symbol '{}' not found", name.value())),
        )
    }

    /// Looks for a `function.def` operation bound to the given predicate symbol.
    pub fn find_predicate(
        &self,
        name: &TypedIdentifier<'ast, 'ctx>,
    ) -> Result<FuncDefOpRef<'ctx, 'blk>, CompileError> {
        self.find_in_scope(
            |scope| scope.predicates().get(&name.symbol()).copied(),
            || CompileError::Ir(format!("predicate symbol '{}' not found", name.value())),
        )
    }

    /// Lowers the given expression in the context of a conditional branch.
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

    /// Looks for the actual name of the predicate bound by the given symbol.
    ///
    /// If two predicates on different scopes have the same name they may get inserted on the same
    /// block, requiring that one of them changes its name to ensure uniqueness. When emitting
    /// `function.call` ops we need to use the actual MLIR name and not the symbol given by the
    /// callee in the AST.
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

    /// Visits a collection of entities that this type can visit, collecting the result of each
    /// visit in a list.
    pub fn visit_many<V, R>(&mut self, entities: &[V]) -> Result<Vec<R>, CompileError>
    where
        Self: Visitor<V, Output = Result<R, CompileError>>,
        V: Visitable,
    {
        entities.iter().map(|e| e.accept(self)).collect()
    }
}

/// Visits an entity inside a fresh scope.
///
/// The scope uses a block that is added to the given region and popped before returning.
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
