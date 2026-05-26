//! Type analysis of the AST.

use std::marker::PhantomData;

use crate::{
    ast::{AstContext, Document, Identifier, Item, Spanned as _, Visitable, Visitor},
    diagnostic::{CompileError, Diagnostic},
    type_analysis::{
        contract::ContractTypeChecker, ctx::TypeInferenceCtx, helpers::check_many,
        predicate::PredicateTypeChecker,
    },
};

mod base;
mod block;
mod contract;
mod ctx;
mod error;
mod expression;
mod helpers;
mod invariant;
mod predicate;
pub mod scope;

type TypingResult<T> = Result<T, Vec<Diagnostic>>;

pub struct TypeChecker<'ast, 'info, 'c, T, C>
where
    T: TypeSystem,
    C: CircuitInfo<'c, TypeSystem = T>,
{
    ctx: TypeInferenceCtx<'ast, T>,
    info: &'info C,
    source_name: &'ast str,
    _marker: PhantomData<&'c ()>,
}

impl<'ast, 'info, 'c, T, C> TypeChecker<'ast, 'info, 'c, T, C>
where
    T: TypeSystem,
    C: CircuitInfo<'c, TypeSystem = T>,
{
    /// Creates a new type checker.
    fn new(ts: T, info: &'info C, ast: &'ast AstContext, source_name: &'ast str) -> Self {
        Self {
            ctx: TypeInferenceCtx::new(ts, ast),
            info,
            source_name,
            _marker: PhantomData,
        }
    }

    /// Typechecks the document using the provided type system.
    pub fn check(
        ts: T,
        info: &'info C,
        ast: &'ast AstContext,
        source_name: &'ast str,
        document: &Document<'ast>,
    ) -> Result<Document<'ast, T::Type>, CompileError> {
        let mut checker = Self::new(ts, info, ast, source_name);
        document
            .accept(&mut checker)
            .map_err(|diags| CompileError::Diagnostics(diags.into()))
    }
}

impl<'ast, 'c, T, C> Visitor<Document<'ast>> for TypeChecker<'ast, '_, 'c, T, C>
where
    T: TypeSystem,
    C: CircuitInfo<'c, TypeSystem = T>,
{
    type Output = TypingResult<Document<'ast, T::Type>>;

    fn visit(&mut self, document: &Document<'ast>) -> Self::Output {
        check_many(self, document, |items| {
            Document::new(items, document.span())
        })
    }
}

impl<'ast, 'c, T, C> Visitor<Item<'ast>> for TypeChecker<'ast, '_, 'c, T, C>
where
    T: TypeSystem,
    C: CircuitInfo<'c, TypeSystem = T>,
{
    type Output = TypingResult<Item<'ast, T::Type>>;

    fn visit(&mut self, entity: &Item<'ast>) -> Self::Output {
        match entity {
            Item::Contract(decl) => decl
                .accept(&mut ContractTypeChecker::new(
                    &mut self.ctx,
                    self.info,
                    self.source_name,
                ))
                .map(Into::into),
            Item::Predicate(decl) => decl
                .accept(&mut PredicateTypeChecker::new(
                    self.source_name,
                    &mut self.ctx,
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

    /// Return true if the types can resolve their unification.
    ///
    /// For example, an 'index' type can be coerced to a '!felt.type' using the cast operation,
    /// so it returns true for that case.
    fn can_resolve_unification(&self, other: &Self) -> bool;
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

/// Trait that gives information about the circuit the spec is targeting.
pub trait CircuitInfo<'info>: Copy {
    /// Error type.
    type Error: std::fmt::Display;
    /// Type system used by the information provider.
    type TypeSystem: TypeSystem;

    /// Looks up a struct definition by name.
    fn find_struct(
        &self,
        name: &Identifier,
    ) -> Result<impl StructInfo<'info, TypeSystem = Self::TypeSystem>, Self::Error>;
}

/// Trait that gives information about a struct in a circuit.
///
/// Implementations of this trait should return data that helps build the locals environment when
/// type-checking a contract's body.
pub trait StructInfo<'info> {
    /// Type system used by the information provider.
    type TypeSystem: TypeSystem;

    /// Returns the list of input arguments of the struct in declaration order.
    fn inputs(&self) -> impl Iterator<Item = <Self::TypeSystem as TypeSystem>::Type>;

    /// Returns the list of members of the struct.
    fn members(
        &self,
    ) -> impl Iterator<Item = MemberInfo<'info, <Self::TypeSystem as TypeSystem>::Type>>;

    /// Returns the list of template parameters associated with the struct.
    fn template_params(
        &self,
    ) -> impl Iterator<Item = ParamInfo<'info, <Self::TypeSystem as TypeSystem>::Type>>;
}

/// Information about a struct's member.
pub struct MemberInfo<'ctx, T> {
    name: &'ctx str,
    r#type: T,
    public: bool,
}

impl<'ctx, T> MemberInfo<'ctx, T> {
    /// Creates a new info struct.
    pub fn new(name: &'ctx str, r#type: T, public: bool) -> Self {
        Self {
            name,
            r#type,
            public,
        }
    }
}

/// Information about a template parameter.
pub struct ParamInfo<'ctx, T> {
    name: &'ctx str,
    r#type: Option<T>,
}

impl<'ctx, T> ParamInfo<'ctx, T> {
    /// Creates a new info struct.
    pub fn new(name: &'ctx str, r#type: Option<T>) -> Self {
        Self { name, r#type }
    }
}

#[cfg(test)]
mod tests;
