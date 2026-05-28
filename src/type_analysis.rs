//! Type analysis of the AST.
//!
//! Type-checking is performed in a generic manner via the [`TypeSystem`] trait. Implementations of
//! that trait define what types are used for representing the different types in the language and
//! how they are constructed. This way, the type-checker only needs to focus on the semantic rules
//! of the language.

use std::marker::PhantomData;

use crate::{
    ast::{AstContext, Document, Identifier, Item, Spanned as _, Visitable, Visitor},
    diagnostic::{CompileError, Diagnostic},
    type_analysis::{
        contract::ContractTypeChecker, ctx::TypeInferenceCtx, helpers::check_many, loops::LoopInfo,
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
pub mod loops;
mod predicate;
pub mod scope;

/// Shorthand for a typing result, whose error type is a collection of diagnostics.
type TypingResult<T> = Result<T, Vec<Diagnostic>>;

/// Top-level type checker.
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

    // We copy the AST at this point since it's currently reused in other parts of the tool. If we
    // remove those parts and only rely on either the returned AST or the MLIR IR we can replace
    // the logic to moving the AST. It will still effectively copy the whole tree since is adding
    // the types but we won't have two copies of the AST in memory at the same time.
    // We'll have to add a visitor that moves the visited entity (i.e. `VisitableOnce` and
    // `VisitorOnce` similar to `Fn` and `FnOnce`)
    /// Typechecks the document using the provided type system and circuit information provider.
    ///
    /// Returns a copy of the AST with associated metadata that represents the types deduced for
    /// the particular entity.
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

    /// Type used for representing struct-like types.
    type StructType: Clone
        + PartialEq
        + std::fmt::Display
        + std::fmt::Debug
        + Into<Self::Type>
        + StructTypeProperties<Type = Self::Type>;

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

/// Trait for obtaining information about struct types.
pub trait StructTypeProperties {
    /// Type used to represent generic types.
    type Type;

    /// Returns true if the array has type vars.
    fn contains_type_vars(&self) -> bool;

    /// Returns the type of a member in the struct.
    ///
    /// If the type does not have a member by that name returns `None`.
    fn get_member(&self, name: &str) -> Option<Self::Type>;

    /// Returns the list of members in the struct.
    fn member_types(&self) -> Vec<Self::Type>;

    /// Changes the types of the members.
    fn map_members(&self, map: impl FnMut(&Self::Type) -> Self::Type) -> Self;
}

/// Trait for obtaining information about types.
pub trait TypeProperties: Sized {
    /// Type used to represent function types.
    type FnType: FnTypeProperties<Type = Self>;

    /// Type used for representing type variables.
    type VarId: Copy + Clone + PartialEq + Eq + std::fmt::Debug + std::hash::Hash;

    /// Type used to represent array types.
    type ArrayType: ArrayTypeProperties<Type = Self> + Into<Self> + std::fmt::Display;

    /// Type used to represent struct types.
    type StructType: StructTypeProperties<Type = Self> + Into<Self> + std::fmt::Display;

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
            || (self.is_struct_like_type()
                && self
                    .to_struct_like_type()
                    .is_some_and(|s| s.contains_type_vars()))
    }

    /// Returns true if the type is an array type.
    fn is_array_type(&self) -> bool;

    /// Converts the type into the concrete array type representation.
    fn to_array_type(&self) -> Option<Self::ArrayType>;

    /// Returns true if the type is a struct-like type.
    fn is_struct_like_type(&self) -> bool;

    /// Converts the type into the concrete struct-like type representation.
    fn to_struct_like_type(&self) -> Option<Self::StructType>;

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

    /// Changes the inner type of the array.
    fn map_inner(&self, inner: Self::Type) -> Self;
}

/// Trait that gives information about the circuit the spec is targeting.
pub trait CircuitInfo<'info>: Copy {
    /// Error type.
    type Error: std::fmt::Display;
    /// Type system used by the information provider.
    type TypeSystem: TypeSystem;

    /// Looks up a contract target definition by name.
    fn find_contract_target(
        &self,
        name: &Identifier,
    ) -> Result<impl ContractTargetInfo<'info, TypeSystem = Self::TypeSystem>, Self::Error>;
}

/// Trait that gives information about a contract target in a circuit.
///
/// Implementations of this trait should return data that helps build the locals environment when
/// type-checking a contract's body.
pub trait ContractTargetInfo<'info> {
    /// Type system used by the information provider.
    type TypeSystem: TypeSystem;

    /// Returns the list of input arguments of the target in declaration order.
    fn inputs(
        &self,
    ) -> impl Iterator<Item = InputInfo<'info, <Self::TypeSystem as TypeSystem>::Type>>;

    /// Returns the list of outputs of the target in declaration order.
    fn outputs(
        &self,
    ) -> impl Iterator<Item = OutputInfo<'info, <Self::TypeSystem as TypeSystem>::Type>>;

    /// Returns the list of member's of the target.
    fn members(
        &self,
    ) -> impl Iterator<Item = MemberInfo<'info, <Self::TypeSystem as TypeSystem>::Type>>;

    /// Returns the list of template parameters associated with the target.
    fn template_params(
        &self,
    ) -> impl Iterator<Item = ParamInfo<'info, <Self::TypeSystem as TypeSystem>::Type>>;

    /// Returns the list of loops defined inside the target.
    fn loops(
        &self,
        ts: &mut Self::TypeSystem,
    ) -> Vec<LoopInfo<<Self::TypeSystem as TypeSystem>::Type>>;

    /// Returns the type of the target.
    fn self_type(&self) -> Option<<Self::TypeSystem as TypeSystem>::Type>;
}

/// Information about a contract target's input argument.
pub struct InputInfo<'ctx, T> {
    name: Option<&'ctx str>,
    r#type: T,
}

impl<'ctx, T> InputInfo<'ctx, T> {
    /// Creates a new info struct for a named input.
    pub fn named(name: &'ctx str, r#type: T) -> Self {
        Self {
            name: Some(name),
            r#type,
        }
    }

    /// Creates a new info struct for an unnamed input.
    pub fn unnamed(r#type: T) -> Self {
        Self { name: None, r#type }
    }
}

/// Information about a contract target's output.
pub struct OutputInfo<'ctx, T> {
    name: Option<&'ctx str>,
    r#type: T,
}

impl<'ctx, T> OutputInfo<'ctx, T> {
    /// Creates a new info struct for a named output.
    pub fn named(name: &'ctx str, r#type: T) -> Self {
        Self {
            name: Some(name),
            r#type,
        }
    }

    /// Creates a new info struct for an unnamed output.
    pub fn unnamed(r#type: T) -> Self {
        Self { name: None, r#type }
    }
}

/// Information about a contract target's member.
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
