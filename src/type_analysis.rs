//! Type analysis of the AST.

use crate::{
    ast::{Document, Item, Spanned as _, Visitable, Visitor},
    diagnostic::{CompileError, Diagnostic},
    type_analysis::{
        contract::ContractTypeChecker, ctx::TypeInferenceCtx, helpers::check_many,
        predicate::PredicateTypeChecker,
    },
};

mod block;
mod contract;
mod ctx;
mod error;
mod expression;
mod helpers;
mod predicate;
pub mod scope;

type TypingResult<T> = Result<T, Vec<Diagnostic>>;

pub struct TypeChecker<'ast, T: TypeSystem> {
    ctx: TypeInferenceCtx<'ast, T>,
    source_name: &'ast str,
}

impl<'ast, T: TypeSystem> TypeChecker<'ast, T> {
    /// Creates a new type checker.
    fn new(ts: T, source_name: &'ast str) -> Self {
        Self {
            ctx: TypeInferenceCtx::new(ts),
            source_name,
        }
    }

    /// Typechecks the document using the provided type system.
    pub fn check(
        ts: T,
        source_name: &'ast str,
        document: &Document<'ast>,
    ) -> Result<Document<'ast, T::Type>, CompileError> {
        let mut checker = Self::new(ts, source_name);
        document
            .accept(&mut checker)
            .map_err(|diags| CompileError::Diagnostics(diags.into()))
    }
}

impl<'ast, T: TypeSystem> Visitor<Document<'ast>> for TypeChecker<'ast, T> {
    type Output = TypingResult<Document<'ast, T::Type>>;

    fn visit(&mut self, document: &Document<'ast>) -> Self::Output {
        check_many(self, document, |items| {
            Document::new(items, document.span())
        })
    }
}

impl<'ast, T: TypeSystem> Visitor<Item<'ast>> for TypeChecker<'ast, T> {
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
    /// Type used for representing any type.
    type Type: Clone
        + PartialEq
        + std::fmt::Display
        + std::fmt::Debug
        + TypeProperties<FnType = Self::FnType>;

    /// Type used for representing function types.
    type FnType: Clone
        + PartialEq
        + std::fmt::Display
        + std::fmt::Debug
        + Into<Self::Type>
        + FnTypeProperties<Type = Self::Type>;

    /// Type used for representing array types.
    type ArrayType: Clone
        + PartialEq
        + std::fmt::Display
        + std::fmt::Debug
        + Into<Self::Type>
        + ArrayTypeProperties<Type = Self::Type>;

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
    /// Type used to represent generic types.
    type Type;

    /// Returns the inputs of the function type.
    fn inputs(&self) -> Vec<Self::Type>;

    /// Returns the outputs of the function type.
    fn outputs(&self) -> Vec<Self::Type>;

    /// Returns true if the function has type variables on either its inputs or outputs.
    fn contains_type_vars(&self) -> bool;
}

/// Trait for obtaining information about types.
pub trait TypeProperties {
    /// Type used to represent function types.
    type FnType: FnTypeProperties<Type = Self>;

    /// Type used for representing type variables.
    type VarId: Copy + Clone + PartialEq + Eq + std::fmt::Debug + std::hash::Hash;

    /// Type used to represent array types.
    type ArrayType: ArrayTypeProperties<Type = Self>;

    /// Returns true if the type is representing a type variable.
    fn is_var_type(&self) -> bool;

    /// Returns the id of the type variable, if available.
    fn var_id(&self) -> Option<Self::VarId>;

    /// Returns true if the type is a function type.
    fn is_func_type(&self) -> bool;

    /// Converts the type into the concrete function type representation.
    fn to_func_type(&self) -> Option<Self::FnType>;

    /// Returns true if the type contains type variables.
    fn contains_type_vars(&self) -> bool {
        self.is_var_type()
            || (self.is_func_type() && self.to_func_type().is_some_and(|f| f.contains_type_vars()))
            || (self.is_array_type()
                && self.to_array_type().is_some_and(|a| a.contains_type_vars()))
    }

    /// Returns true if the type is an array type.
    fn is_array_type(&self) -> bool;

    /// Converts the type into the concrete array type representation.
    fn to_array_type(&self) -> Option<Self::ArrayType>;
}

/// Trait for obtaining information about array types.
pub trait ArrayTypeProperties {
    /// Type used to represent generic types.
    type Type;

    /// Returns the inner type of the array.
    fn inner_type(&self) -> Self::Type;

    /// Returns true if the array has type vars.
    fn contains_type_vars(&self) -> bool;
}

#[cfg(test)]
mod tests;
