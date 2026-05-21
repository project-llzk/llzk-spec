//! Type analysis of the AST.

// Shut up rust_analyzer for now.
#![allow(unused)]
#![allow(dead_code)]

use std::{collections::HashMap, marker::PhantomData};

use llzk::dialect::bool;

use crate::{
    ast::{
        BinaryOp, Block, ContractDecl, Document, Expression, Identifier, Item, PredicateDecl, Span,
        Spanned as _, Statement, Symbol, UnaryOp, Visitable, Visitor,
    },
    diagnostic::Diagnostic,
    type_analysis::{
        contract::ContractTypeChecker,
        ctx::TypeInferenceCtx,
        helpers::{check_many, extract_result},
        predicate::PredicateTypeChecker,
        scope::ScopeStack,
    },
};

mod block;
mod contract;
mod ctx;
mod error;
mod helpers;
mod predicate;
mod scope;

type TypingResult<T> = Result<T, Vec<Diagnostic>>;

struct TypeChecker<'ctx, 'ast, T: TypeSystem> {
    ctx: TypeInferenceCtx<'ast, T>,
    source_name: &'ast str,
    /// I don't want to get rid of 'ctx yet just in case.
    _marker: PhantomData<&'ctx ()>,
}

impl<'ast, 'ctx, T: TypeSystem> Visitor<Document<'ast>> for TypeChecker<'ctx, 'ast, T> {
    type Output = TypingResult<Document<'ast, T::Type>>;

    fn visit(&mut self, document: &Document<'ast>) -> Self::Output {
        check_many(self, document, |items| {
            Document::new(items, document.span())
        })
    }
}

impl<'ast, 'ctx, T: TypeSystem> Visitor<Item<'ast>> for TypeChecker<'ctx, 'ast, T> {
    type Output = TypingResult<Item<'ast, T::Type>>;

    fn visit(&mut self, entity: &Item<'ast>) -> Self::Output {
        match entity {
            Item::Contract(decl) => decl
                .accept(&mut ContractTypeChecker::new(
                    &mut self.ctx,
                    self.source_name,
                ))
                .map(Into::into),
            Item::Predicate(decl) => decl
                .accept(&mut PredicateTypeChecker::new(
                    &mut self.ctx,
                    self.source_name,
                ))
                .map(Into::into),
        }
    }
}

/// Trait abstracting the actual types used for inference.
///
/// Helps decouple the inference engine from MLIR types, simplifying testing.
pub trait TypeSystem {
    type Type: Clone + PartialEq + std::fmt::Display + TypeProperties<FnType = Self::FnType>;
    type FnType: Clone
        + PartialEq
        + std::fmt::Display
        + Into<Self::Type>
        + FnTypeProperties<Type = Self::Type>;

    /// Create a boolean type.
    fn bool_type(&mut self) -> Self::Type;

    /// Create a felt type.
    fn felt_type(&mut self) -> Self::Type;

    /// Create a function type.
    fn func_type(&mut self, ins: &[Self::Type], outs: &[Self::Type]) -> Self::FnType;

    /// Create a fresh type variable.
    fn fresh_var(&mut self) -> Self::Type;

    /// Create a predicate function type.
    fn predicate_type(&mut self, ins: &[Self::Type]) -> Self::FnType {
        let bool = self.bool_type();
        self.func_type(ins, &[bool])
    }
}

/// Trait for obtaining information about function types.
pub trait FnTypeProperties {
    type Type;
    fn inputs(&self) -> Vec<Self::Type>;

    fn outputs(&self) -> Vec<Self::Type>;
}

/// Trait for obtaining information about types.
pub trait TypeProperties {
    type FnType: FnTypeProperties<Type = Self>;
    type VarId: Copy + Clone + PartialEq + Eq + std::fmt::Debug + std::hash::Hash;

    fn is_var_type(&self) -> bool;

    fn var_id(&self) -> Option<Self::VarId>;

    fn is_func_type(&self) -> bool;

    fn to_func_type(&self) -> Option<Self::FnType>;
}

#[cfg(test)]
mod tests;
