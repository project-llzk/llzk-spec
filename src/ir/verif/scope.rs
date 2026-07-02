//! Lexical scopes handling.

use llzk::{
    builder::InsertPoint,
    prelude::{FuncDefOpRef, OperationLike as _},
};
use melior::ir::{BlockLike as _, BlockRef, Module, Operation, OperationRef, Value};

use crate::{
    diagnostic::CompileError,
    ir::llzk::LlzkLoopTarget,
    type_analysis::scope::{Scope, ScopeStack},
};

/// Optional tag for annotating scopes.
#[derive(Debug, Copy, Clone)]
pub enum ScopeTag {
    /// Tag for the root scope.
    Root,
    /// Tag for predicate scopes.
    Predicate,
    /// Tag for contract scopes.
    Contract,
    /// Tag for compute scopes.
    Compute,
    /// Tag for constrain scopes.
    Constrain,
    /// Tag for invariant scopes.
    Invariant,
}

impl ScopeTag {
    /// Returns whether the tagged scope can append `function.def` operations.
    pub fn accepts_function_def_ops(self) -> bool {
        matches!(self, ScopeTag::Root)
    }

    /// Returns whether the tagged scope accepts condition statements for the compute function.
    pub fn compute_condition_scope(self) -> bool {
        !matches!(self, ScopeTag::Constrain)
    }

    /// Returns whether the tagged scope accepts condition statements for the constrain function.
    pub fn constrain_condition_scope(self) -> bool {
        !matches!(self, ScopeTag::Compute)
    }
}

/// Additional payload appended to each scope with information specific to emitting MLIR
/// operations.
pub(super) struct ScopeData<'ctx, 'blk> {
    /// Current insertion block.
    block: BlockRef<'ctx, 'blk>,
    /// Saved insert point.
    previous: Option<InsertPoint<'ctx, 'blk>>,
    /// Optional tag.
    tag: Option<ScopeTag>,
}

impl<'ctx, 'blk> ScopeData<'ctx, 'blk> {
    /// Creates a root scope based on the given module.
    ///
    /// Uses the module's body for insertion.
    pub fn root<'m>(module: &'blk Module<'ctx>) -> Self
    where
        'blk: 'ctx,
    {
        Self {
            block: module.body(),
            tag: Some(ScopeTag::Root),
            previous: None,
        }
    }

    /// Creates an untagged scope.
    pub fn new(block: BlockRef<'ctx, 'blk>, previous: InsertPoint<'ctx, 'blk>) -> Self {
        Self {
            block,
            tag: None,
            previous: Some(previous),
        }
    }

    /// Creates a tagged scope.
    pub fn new_with_tag(
        block: BlockRef<'ctx, 'blk>,
        previous: InsertPoint<'ctx, 'blk>,
        tag: ScopeTag,
    ) -> Self {
        Self {
            block,
            tag: Some(tag),
            previous: Some(previous),
        }
    }

    #[allow(unused)]
    pub fn block(&self) -> BlockRef<'ctx, 'blk> {
        self.block
    }

    pub fn previous(&self) -> Option<InsertPoint<'ctx, 'blk>> {
        self.previous
    }
}

/// Stack of scopes predefined with the types used for emitting IR.
pub type CodegenScopeStack<'ast, 'ctx, 'blk> = ScopeStack<
    'ast,
    Value<'ctx, 'blk>,
    FuncDefOpRef<'ctx, 'blk>,
    LlzkLoopTarget<'ctx, 'blk>,
    ScopeData<'ctx, 'blk>,
>;
/// Scope predefined with the types used for emitting IR.
///
/// Has some extra methods that make sense to have in this kind of scope but not on the generic
/// scope type.
pub type CodegenScope<'ast, 'ctx, 'blk> = Scope<
    'ast,
    Value<'ctx, 'blk>,
    FuncDefOpRef<'ctx, 'blk>,
    LlzkLoopTarget<'ctx, 'blk>,
    ScopeData<'ctx, 'blk>,
>;

impl<'ast, 'ctx, 'blk> CodegenScope<'ast, 'ctx, 'blk> {
    /// Appends the operation into the block.
    pub fn append_operation(&mut self, op: impl Into<Operation<'ctx>>) -> OperationRef<'ctx, 'blk> {
        let block = self.payload().block;
        let op = op.into();
        match block.terminator() {
            Some(terminator) => block.insert_operation_before(terminator, op),
            None => block.append_operation(op),
        }
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
            .payload()
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

    /// Returns the tag of the scope, if available.
    pub fn tag(&self) -> Option<ScopeTag> {
        self.payload().tag
    }
}
