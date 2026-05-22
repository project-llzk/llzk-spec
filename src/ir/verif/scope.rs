//! Lexical scopes handling.

use std::collections::HashMap;

use llzk::prelude::{FuncDefOpRef, OperationLike as _};
use melior::ir::{BlockLike as _, BlockRef, Module, Operation, OperationRef, Value};

use crate::{ast::Symbol, diagnostic::CompileError, ir::verif::TypedIdentifier};

/// Optional tag for annotating scopes.
#[derive(Debug, Copy, Clone)]
pub enum ScopeTag {
    /// Tag for the root scope.
    Root,
    /// Tag for predicate scopes.
    Predicate,
}

impl ScopeTag {
    /// Returns wether the tagged scope can append `function.def` operations.
    pub fn accepts_function_def_ops(self) -> bool {
        matches!(self, ScopeTag::Root)
    }
}

/// Entry in the scope stack.
pub(super) struct Scope<'ast, 'ctx, 'blk> {
    /// Current insertion block.
    block: BlockRef<'ctx, 'blk>,
    /// Binds names to predicates.
    predicates: HashMap<Symbol<'ast>, FuncDefOpRef<'ctx, 'blk>>,
    /// Binds local names to SSA values.
    locals: HashMap<Symbol<'ast>, Value<'ctx, 'blk>>,
    /// Optional tag.
    tag: Option<ScopeTag>,
}

impl<'ast, 'ctx, 'blk> Scope<'ast, 'ctx, 'blk> {
    /// Creates a root scope based on the given module.
    ///
    /// Uses the module's body for insertion.
    pub fn root<'m>(module: &'blk Module<'ctx>) -> Self
    where
        'blk: 'ctx,
    {
        Self::new_with_tag(module.body(), ScopeTag::Root)
    }

    /// Creates an untagged scope.
    pub fn new(block: BlockRef<'ctx, 'blk>) -> Self {
        Self {
            block,
            predicates: Default::default(),
            locals: Default::default(),
            tag: None,
        }
    }

    /// Creates a tagged scope.
    pub fn new_with_tag(block: BlockRef<'ctx, 'blk>, tag: ScopeTag) -> Self {
        Self {
            block,
            predicates: Default::default(),
            locals: Default::default(),
            tag: Some(tag),
        }
    }

    /// Binds a reference to a `function.def` operation to the given symbol.
    ///
    /// The reference must have been inserted somewhere with a lifetime greater or equal to the
    /// block's lifetime. It could be the block pointed by this scope or a parent block.
    pub fn bind_predicate(
        &mut self,
        name: &TypedIdentifier<'ast, 'ctx>,
        func_op: FuncDefOpRef<'ctx, 'blk>,
    ) -> Result<(), CompileError> {
        if self.predicates.contains_key(&name.symbol()) {
            return Err(CompileError::Ir(format!(
                "duplicate predicate '{}'",
                name.value()
            )));
        }
        self.predicates.insert(name.symbol(), func_op);
        Ok(())
    }

    /// Binds a SSA value to the given symbol.
    ///
    /// The SSA value must dominate any future users that point to the symbol.
    pub fn bind_local(
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

    /// Appends the operation into the block.
    pub fn append_operation(&mut self, op: impl Into<Operation<'ctx>>) -> OperationRef<'ctx, 'blk> {
        self.block.append_operation(op.into())
    }

    /// Appends the operation into the scope using MLIR's SymbolTable API to ensure that the
    /// operation has an unique name after insertion.
    ///
    /// The parent op of the block must implement the `SymbolTable` interface and the to be
    /// inserted operation must implement the `Symbol` interface.
    ///
    /// # Panics
    ///
    /// If the block does not have a parent operation (aka is free floating).
    pub fn append_with_symbol_uniquing(
        &mut self,
        op: impl Into<Operation<'ctx>>,
    ) -> OperationRef<'ctx, 'blk> {
        let parent_op = self
            .block
            .parent_operation()
            .expect("scope's block has a parent operation");
        llzk::symbol_table::insert(&parent_op, op.into())
    }

    /// Appends an operation with one SSA result into the block. Then returns the SSA value with
    /// its lifetime tied to the block's.
    pub fn append_operation_with_result(
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

    /// Returns a reference to the predicates namespace.
    pub fn predicates(&self) -> &HashMap<Symbol<'ast>, FuncDefOpRef<'ctx, 'blk>> {
        &self.predicates
    }

    /// Returns a reference to the locals namespace.
    pub fn locals(&self) -> &HashMap<Symbol<'ast>, Value<'ctx, 'blk>> {
        &self.locals
    }

    /// Returns the tag of the scope, if available.
    pub fn tag(&self) -> Option<ScopeTag> {
        self.tag
    }
}
