//! Lexical scopes handling.

use std::collections::HashMap;

use llzk::prelude::{FuncDefOp, FuncDefOpRef, OperationLike as _};
use melior::ir::{BlockLike as _, BlockRef, Module, Operation, OperationRef, Value};

use crate::{ast::Symbol, diagnostic::CompileError, ir::verif::TypedIdentifier};

/// Entry in the scope stack.
pub(super) struct Scope<'ast, 'ctx, 'blk> {
    // Current insertion block.
    block: BlockRef<'ctx, 'blk>,
    // Binds names to predicates.
    predicates: HashMap<Symbol<'ast>, FuncDefOpRef<'ctx, 'blk>>,
    // Binds local names to SSA values.
    locals: HashMap<Symbol<'ast>, Value<'ctx, 'blk>>,
}

impl<'ast, 'ctx, 'blk> Scope<'ast, 'ctx, 'blk> {
    pub fn root<'m>(module: &'blk Module<'ctx>) -> Self
    where
        'blk: 'ctx,
    {
        Self::new(module.body())
    }

    pub fn new(block: BlockRef<'ctx, 'blk>) -> Self {
        Self {
            block,
            predicates: Default::default(),
            locals: Default::default(),
        }
    }

    pub fn bind_predicate(
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

    pub fn append_operation(&mut self, op: impl Into<Operation<'ctx>>) -> OperationRef<'ctx, 'blk> {
        self.block.append_operation(op.into())
    }

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

    pub fn predicates(&self) -> &HashMap<Symbol<'ast>, FuncDefOpRef<'ctx, 'blk>> {
        &self.predicates
    }

    pub fn locals(&self) -> &HashMap<Symbol<'ast>, Value<'ctx, 'blk>> {
        &self.locals
    }
}
