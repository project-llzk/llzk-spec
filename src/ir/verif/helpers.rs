//! Helper methods for emitting IR.

use crate::{
    ast::{self, Span, Spanned, Visitable, Visitor},
    diagnostic::CompileError,
    ir::{
        llzk::{LlzkContractTarget, function_input_name},
        verif::{
            SpecCodegen, TypedExpression, TypedIdentifier, TypedPredicateDecl,
            affine::{AffineExpr, AffineMap},
            scope::{CodegenScope, ScopeData, ScopeTag},
        },
    },
};
use ::melior::ir::Block;
use llzk::{
    builder::{OpBuilder, OpBuilderLike},
    dialect::{
        array, cast, function,
        poly::{self, unifiable_cast},
        r#struct,
    },
    prelude::{
        ArrayType, FlatSymbolRefAttribute, FuncDefOp, FuncDefOpLike as _, FuncDefOpRef,
        FunctionType, IntegerAttribute, LlzkContext, MemberDefOpLike as _, OperationLike as _,
        StringAttribute, StructDefOpLike as _, StructDefOpRef, SymbolRefAttribute,
        TemplateOpLike as _, TemplateOpRef, TemplateSymbolBindingOpLike as _, is_felt_type,
        melior_dialects::scf,
    },
    value_ext::OwningValueRange,
};
use melior::{
    dialect::arith,
    ir::{
        BlockLike as _, BlockRef, Location, Module, Operation, OperationRef, Region,
        RegionLike as _, Type, TypeLike, Value, ValueLike,
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

        let block = func_op.body()?.append_block(Block::new(&block_args));

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
    pub fn builder(&self) -> &OpBuilder<'ctx, 'ctx> {
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
    /// Sets the insertion point of the builder to the given block.
    ///
    /// For pushing tagged scopes see [`Self::push_tagged`].
    pub fn push(&mut self, block: BlockRef<'ctx, 'blk>) {
        let previous = self.builder().save_insertion_point();
        self.scope.push(ScopeData::new(block, previous));
        self.builder().set_insertion_point_at_start(block);
    }

    /// Pushes a new tagged scope.
    ///
    /// Sets the insertion point of the builder to the given block.
    ///
    /// For pushing without a tag see [`Self::push`].
    pub fn push_tagged(&mut self, block: BlockRef<'ctx, 'blk>, tag: ScopeTag) {
        let previous = self.builder().save_insertion_point();
        self.scope
            .push(ScopeData::new_with_tag(block, previous, tag));
        self.builder().set_insertion_point_at_start(block);
    }

    /// Pops the top of the scope stack.
    ///
    /// Sets the insertion point of the builder to the end of the block on the new top.
    pub fn pop(&mut self) {
        if let Some(previous) = self.scope.top().payload().previous() {
            self.builder().restore_insertion_point(previous);
        }
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
        accept_in_new_scope(
            region,
            self,
            expr,
            |mut result, scope: &mut Self| {
                if result.r#type() != expected_type {
                    result = scope.insert_op_with_result(unifiable_cast(
                        location,
                        result,
                        expected_type,
                    ))?;
                }
                let op = scf::r#yield(&[result], location);
                scope.insert_op(op);
                Ok(result)
            },
            None,
        )
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

    /// Creates the correct symbol for the given identifier.
    pub fn symbolize_target(&self, name: &TypedIdentifier<'ast, 'ctx>) -> SymbolRefAttribute<'ctx> {
        let parts: Vec<_> = name.value().split("::").collect();
        SymbolRefAttribute::new_from_str(self.context(), parts[0], &parts[1..])
    }

    /// Creates a contract name for an anonymous contract targeting the given symbol.
    pub fn anon_contract_name(&mut self, name: SymbolRefAttribute<'ctx>) -> StringAttribute<'ctx> {
        let id = format!("contract${}", self.anon_contracts);
        self.anon_contracts += 1;
        let nested = name.nested();
        let mut buf = String::with_capacity(
            name.root().to_raw().length
                + id.len()
                + nested.iter().map(|n| n.value().len()).sum::<usize>()
                + (nested.len() + 2),
        );
        buf.push_str(name.root().as_str().unwrap());
        buf.push('$');
        for part in nested {
            buf.push_str(part.value());
            buf.push('$');
        }
        buf.push_str(&id);

        StringAttribute::new(self.context(), &buf)
    }

    /// Creates an AST identifier.
    pub fn create_ident(&self, symbol: &str, spanned: &dyn Spanned) -> ast::Identifier<'ast> {
        ast::Identifier::new(self.ast.symbol(symbol), spanned.span())
    }

    /// Binds poly expressions and constants within the scope of a template op.
    ///
    /// If the `parent` operation is a `poly.template` then it iterates over all the
    /// `poly.const` and `poly.expr` ops inside it and binds their names to a `poly.read_const`
    /// operation's result.
    pub fn bind_template_consts(
        &mut self,
        parent_op: OperationRef<'ctx, '_>,
        span: &dyn Spanned,
        location: Location<'ctx>,
    ) -> Result<(), CompileError> {
        if let Ok(template_op) = TemplateOpRef::try_from(parent_op) {
            template_op
                .const_binding_ops()
                .into_iter()
                .try_for_each(|param_op| {
                    let symbol = param_op.sym_name().to_string();
                    let param_type = param_op.type_opt().unwrap_or_else(|| self.felt_type());
                    let read_value = self
                        .insert_op_with_result(poly::read_const(location, &symbol, param_type))?;
                    let name = self.create_ident(&symbol, span);
                    self.scope
                        .top()
                        .bind_local(&name, read_value)
                        .map_err(|err| {
                            err.into_compile_error(
                                &self.filename,
                                Some(span.span()),
                                format!("while binding template parameter '{symbol}'"),
                            )
                        })
                })?;
        }
        Ok(())
    }

    /// Inserts the given operation using the builder and returns a single value.
    ///
    /// Fails if the op has no values or more than one value.
    pub fn insert_op_with_result(
        &self,
        op: impl Into<Operation<'ctx>>,
    ) -> Result<Value<'ctx, 'blk>, CompileError> {
        let op = op.into();
        let location = op.location();
        let op_ref = self.builder().insert(location, |_, _| op);
        if op_ref.result_count() != 1 {
            return Err(CompileError::Ir(format!(
                "expected operation '{op_ref}' to have 1 result but has {}",
                op_ref.result_count()
            )));
        }
        // To avoid a lifetime error.
        Ok(unsafe { Value::from_raw(op_ref.result(0)?.to_raw()) })
    }

    /// Inserts the given operation using the builder.
    pub fn insert_op(&self, op: impl Into<Operation<'ctx>>) {
        let op = op.into();
        let location = op.location();
        self.builder().insert(location, |_, _| op);
    }

    /// Binds the member definitions of the given `struct.def` op to locals in the scope.
    ///
    /// The bound value is the result of a `struct.readm` operation reading the member.
    pub fn bind_members(
        &mut self,
        struct_op: StructDefOpRef<'ctx, 'blk>,
        location: Location<'ctx>,
        self_value: Value<'ctx, 'blk>,
        span: &dyn Spanned,
    ) -> Result<(), CompileError> {
        for member in struct_op.member_defs() {
            let member_name = member.member_name().to_string();
            let op = r#struct::readm(
                self.builder(),
                location,
                member.member_type(),
                self_value,
                &member_name,
            )?;
            let value = self.insert_op_with_result(op)?;
            let name = self.create_ident(&member_name, span);
            self.top_mut().bind_local(&name, value).map_err(|err| {
                err.into_compile_error(
                    &self.filename,
                    Some(span.span()),
                    format!("while binding struct member '{}'", member_name),
                )
            })?;
        }
        Ok(())
    }

    /// Binds block arguments as parameters in the scope based on the metadata in the given
    /// reference to a `function.def`.
    ///
    /// If given, the block arguments are offset by the `offset` parameter.
    pub fn bind_inputs(
        &mut self,
        func: FuncDefOpRef<'ctx, 'blk>,
        block: BlockRef<'ctx, 'blk>,
        source_offset: Option<usize>,
        block_offset: Option<usize>,
        span: &dyn Spanned,
    ) -> Result<(), CompileError> {
        let source_offset = source_offset.unwrap_or_default();
        let block_offset = block_offset.unwrap_or_default();
        let arg_count = func
            .function_type()?
            .input_count()
            .saturating_sub(source_offset);
        (0..arg_count).try_for_each(|n| -> Result<(), CompileError> {
            let arg = Value::from(block.argument(n + block_offset)?);
            let source_idx = n + source_offset;
            let positional_name = self.create_ident(&format!("$arg[{n}]"), span);
            self.scope
                .top()
                .bind_parameter(&positional_name, arg, n)
                .map_err(|err| {
                    err.into_compile_error(
                        &self.filename,
                        Some(span.span()),
                        format!("while binding argument #{n} of target"),
                    )
                })?;

            if let Some(arg_name) = function_input_name(func, source_idx) {
                let arg_name_ident = self.create_ident(&arg_name, span);
                self.top_mut()
                    .bind_local(&arg_name_ident, arg)
                    .map_err(|err| {
                        err.into_compile_error(
                            &self.filename,
                            Some(span.span()),
                            format!("while binding named argument '{arg_name}' of target"),
                        )
                    })?;
            }

            Ok(())
        })
    }

    /// Binds block arguments as outputs in the scope based on the metadata in the given
    /// reference to a `function.def`.
    ///
    /// If given, the block arguments are offset by the `offset` parameter.
    pub fn bind_outputs(
        &mut self,
        func: FuncDefOpRef<'ctx, 'blk>,
        block: BlockRef<'ctx, 'blk>,
        offset: Option<usize>,
        span: &dyn Spanned,
    ) -> Result<(), CompileError> {
        let arg_count = func.function_type()?.input_count();
        (0..arg_count).try_for_each(|n| -> Result<(), CompileError> {
            let arg = Value::from(block.argument(n + offset.unwrap_or_default())?);

            let name = self.create_ident(&format!("$res[{n}]"), span);

            self.scope.top().bind_output(&name, arg, n).map_err(|err| {
                err.into_compile_error(
                    &self.filename,
                    Some(span.span()),
                    format!("while binding outptu #{n} of target"),
                )
            })
        })
    }

    /// Binds the loop information
    pub fn bind_loop_info(
        &mut self,
        target: LlzkContractTarget<'ctx, 'blk>,
        span: &dyn Spanned,
    ) -> Result<(), CompileError> {
        for mut loop_target in target.loops() {
            let name = ast::Identifier::new(
                self.ast.new_symbol(loop_target.label().to_string()),
                span.span(),
            );
            self.scope
                .top()
                .bind_loop(&name, loop_target)
                .map_err(|err| {
                    err.into_compile_error(
                        &self.filename,
                        Some(span.span()),
                        format!("while binding loop info for '{}'", name.value()),
                    )
                })?;
            loop_target.ensure_label_is_present();
        }
        Ok(())
    }

    /// Casts the given value to the requested type if they type of the
    /// value does not match.
    ///
    /// Defaults to using `poly.unifiable_cast` with a special case for
    /// casting between `index` and `!felt.type` and vice versa.
    pub fn cast_if_necessary(
        &mut self,
        value: Value<'ctx, 'blk>,
        requested: Type<'ctx>,
        location: Location<'ctx>,
    ) -> Result<Value<'ctx, 'blk>, CompileError> {
        if value.r#type() == requested {
            return Ok(value);
        }

        let op = if is_felt_type(value.r#type()) && requested.is_index() {
            // cast felt -> index
            cast::toindex(location, value)
        } else if value.r#type().is_index() && is_felt_type(requested) {
            // cast index -> felt
            cast::tofelt(location, value, Some(requested.try_into()?))
        } else {
            poly::unifiable_cast(location, value, requested)
        };

        self.insert_op_with_result(op)
    }

    /// Returns a constant index operation.
    pub fn constant_index_op(
        &mut self,
        location: Location<'ctx>,
        value: i64,
    ) -> Result<Value<'ctx, 'blk>, CompileError> {
        let op = arith::constant(
            self.context(),
            IntegerAttribute::new(Type::index(self.context()), value).into(),
            location,
        );
        self.insert_op_with_result(op)
    }

    /// Creates IR that fills an array of felts with values from the range between the two expressions.
    ///
    /// Return a value pointing to the array.
    pub fn fill_array_with_range(
        &mut self,
        location: Location<'ctx>,
        from: Value<'ctx, 'blk>,
        to: Value<'ctx, 'blk>,
    ) -> Result<Value<'ctx, 'blk>, CompileError> {
        // Create an affine map that computes `to` - `from` to get the size of the array.
        let s0 = AffineExpr::symbol(self.context(), 0);
        let s1 = AffineExpr::symbol(self.context(), 1);
        let map = AffineMap::new(self.context(), 0, 2, &[s1 - s0]);
        let arr_type = ArrayType::new(self.felt_type(), &[map.into()]);
        let from_index = self.cast_if_necessary(from, Type::index(self.context()), location)?;
        let to_index = self.cast_if_necessary(to, Type::index(self.context()), location)?;
        let range = OwningValueRange::from([from_index, to_index].as_slice());
        let arr = array::new(
            self.builder(),
            location,
            arr_type,
            array::ArrayCtor::MapDimSlice(&[(&range).try_into().unwrap()], &[0]),
        );
        let arr = self.insert_op_with_result(arr)?;
        let region = Region::new();
        let block = region.append_block(Block::new(&[(Type::index(self.context()), location)]));
        self.push(block);
        {
            // %felt_iv = cast.tofelt %iv
            // %arr[%iv - %from_index] = %felt_iv
            let iv = block.argument(0).unwrap();
            let idx = self.insert_op_with_result(arith::subi(iv.into(), from_index, location))?;
            let felt_iv = self.cast_if_necessary(iv.into(), self.felt_type(), location)?;
            self.insert_op(array::write(location, arr, &[idx], felt_iv));
            self.insert_op(scf::r#yield(&[], location));
        }
        self.pop();
        let const_one = self.constant_index_op(location, 1)?;
        self.insert_op(scf::r#for(
            from_index, to_index, const_one, region, location,
        ));
        Ok(arr)
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
    tag: Option<ScopeTag>,
) -> Result<R, CompileError>
where
    V: Visitable,
    SpecCodegen<'ast, 'ctx, 'blk>: Visitor<V, Output = Result<R, CompileError>>,
{
    let block_ref = region.append_block(melior::ir::Block::new(&[]));
    match tag {
        Some(tag) => scope.push_tagged(block_ref, tag),
        None => scope.push(block_ref),
    }
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

pub fn find_contract_target_on_module<'ctx, 'blk>(
    module: &'blk Module<'ctx>,
    sym: SymbolRefAttribute<'ctx>,
) -> Result<LlzkContractTarget<'ctx, 'blk>, CompileError> {
    fn children<'ctx, 'blk>(
        op: &OperationRef<'ctx, 'blk>,
    ) -> impl Iterator<Item = OperationRef<'ctx, 'blk>> {
        op.regions()
            .flat_map(|r| std::iter::successors(r.first_block(), |b| b.next_in_region()))
            .flat_map(|b| std::iter::successors(b.first_operation(), |o| o.next_in_block()))
    }

    fn find_contract_impl<'ctx, 'blk>(
        parent: OperationRef<'ctx, 'blk>,
        head: &str,
        tail: &[&str],
    ) -> Result<OperationRef<'ctx, 'blk>, CompileError> {
        let head_op = children(&parent)
            .find_map(|o| {
                let sym_name = StringAttribute::try_from(o.attribute("sym_name").ok()?)
                    .expect("'sym_name' to be a StringAttr");
                (sym_name.value() == head).then_some(o)
            })
            .ok_or_else(|| {
                CompileError::Ir(format!("symbol '{head}' not found in operation {parent}"))
            })?;

        if tail.is_empty() {
            Ok(head_op)
        } else {
            find_contract_impl(head_op, tail[0], &tail[1..])
        }
    }

    let op = find_contract_impl(
        module.as_operation(),
        sym.root().as_str()?,
        &sym.nested()
            .into_iter()
            .map(|s| s.value())
            .collect::<Vec<_>>(),
    )?;

    if let Ok(struct_op) = StructDefOpRef::try_from(op) {
        Ok(LlzkContractTarget::Struct(struct_op))
    } else if let Ok(func_op) = FuncDefOpRef::try_from(op) {
        Ok(LlzkContractTarget::Function(func_op))
    } else {
        let name = op.name();

        Err(CompileError::Ir(format!(
            "operation was expected to be either 'struct.def' or 'function.def' but is '{}'",
            name.as_string_ref().as_str().unwrap_or("<unknown>")
        )))
    }
}
