//! LLZK IR emission and handling.

pub mod llzk;
pub mod verif;

use ::llzk::{dialect::poly::TVarType, prelude::*};
use melior::ir::Module;

use crate::{
    ast::Span,
    diagnostic::CompileError,
    type_analysis::{
        ArrayTypeProperties, FnTypeProperties, StructTypeProperties, TypeProperties, TypeSystem,
    },
};

/// Context supporting IR handling and generation.
pub struct Context {
    context: LlzkContext,
    prime: Option<String>,
}

impl Context {
    /// Creates a new context.
    pub fn new() -> Self {
        Self {
            context: LlzkContext::new_no_log(),
            prime: None,
        }
    }

    /// Creates a new context with the given prime field.
    pub fn with_field(prime: String) -> Self {
        Self {
            context: LlzkContext::new_no_log(),
            prime: Some(prime),
        }
    }

    /// Returns a reference to the MLIR context.
    pub fn context(&self) -> &melior::Context {
        &self.context
    }

    /// Creates an MLIR from the given span and filename.
    #[inline]
    pub fn location_from_span<'ctx>(&'ctx self, filename: &str, span: Span) -> Location<'ctx> {
        Location::new(&self.context, filename, span.line, span.column)
    }

    /// Creates an empty MLIR module.
    #[inline]
    pub fn fresh_module<'ctx>(&'ctx self, filename: &str, span: Span) -> Module<'ctx> {
        llzk_module(self.location_from_span(filename, span), None)
    }

    /// Loads a MLIR module from the given string.
    #[inline]
    pub fn parse_module<'ctx>(
        &'ctx self,
        source_name: &str,
        source: &str,
    ) -> Result<Module<'ctx>, CompileError> {
        Module::parse(&self.context, source)
            .ok_or_else(|| CompileError::Ir(format!("{source_name}: failed to parse LLZK IR")))
    }

    /// Returns a type representing a function.
    pub fn func_type<'ctx>(
        &'ctx self,
        ins: &[Type<'ctx>],
        outs: &[Type<'ctx>],
    ) -> FunctionType<'ctx> {
        FunctionType::new(self.context(), ins, outs)
    }

    /// Returns a type representing a boolean.
    pub fn bool_type(&self) -> Type<'_> {
        IntegerType::new(self.context(), 1).into()
    }

    /// Returns a type representing a machine word.
    pub fn index_type(&self) -> Type<'_> {
        Type::index(self.context())
    }

    /// Returns a type representing a finite field element.
    pub fn felt_type(&self) -> Type<'_> {
        match self.prime() {
            Some(prime) => FeltType::with_field(self.context(), prime),
            None => FeltType::new(self.context()),
        }
        .into()
    }

    pub fn prime(&self) -> Option<&str> {
        self.prime.as_deref()
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

/// Implementation of [`TypeSystem`] based on MLIR.
pub struct MlirTypeSystem<'ctx, 'm> {
    ctx: &'ctx Context,
    module: &'m Module<'ctx>,
    next_var: usize,
}

impl<'ctx, 'm> MlirTypeSystem<'ctx, 'm> {
    /// Creates a new type system.
    pub fn new(ctx: &'ctx Context, module: &'m Module<'ctx>) -> Self {
        Self {
            ctx,
            module,
            next_var: 0,
        }
    }
}

impl<'ctx> TypeSystem for MlirTypeSystem<'ctx, '_> {
    // We are currently using `Type` since that's the obvious thing to do. However, if in the
    // future we want to do things like pretty printing the type in diagnostic messages (i.e.
    // printing `Bool` instead of `i1`) we can replace it with a wrapper.
    //
    // We already have wrappers for other types for other reasons so it's not too far fetched.
    type Type = Type<'ctx>;

    type FnType = WrapFunctionType<'ctx>;

    type ArrayType = ArrayType<'ctx>;

    type StructType = WrapStructLike<'ctx>;

    type Scope = Module<'ctx>;

    fn bool_type(&mut self) -> Self::Type {
        self.ctx.bool_type()
    }

    fn felt_type(&mut self) -> Self::Type {
        self.ctx.felt_type()
    }

    fn func_type(&mut self, ins: &[Self::Type], outs: &[Self::Type]) -> Self::FnType {
        WrapFunctionType(self.ctx.func_type(ins, outs))
    }

    fn fresh_var(&mut self) -> Self::Type {
        let id = self.next_var;
        self.next_var += 1;
        TVarType::new(self.ctx.context(), StringRef::new(&format!("T{id}"))).into()
    }

    fn scope(&mut self) -> &Self::Scope {
        self.module
    }
}

impl<'ctx> TypeProperties for Type<'ctx> {
    type FnType = WrapFunctionType<'ctx>;
    type ArrayType = ArrayType<'ctx>;
    type StructType = WrapStructLike<'ctx>;
    type VarId = &'ctx str;

    fn is_var_type(&self) -> bool {
        is_type_variable(*self)
    }

    fn var_id(&self) -> Option<Self::VarId> {
        TVarType::try_from(*self)
            .ok()
            .and_then(|t| t.name().as_str().ok())
    }

    fn is_func_type(&self) -> bool {
        self.is_function()
    }

    fn to_func_type(&self) -> Option<Self::FnType> {
        FunctionType::try_from(*self).ok().map(WrapFunctionType)
    }

    fn is_array_type(&self) -> bool {
        is_array_type(*self)
    }

    fn to_array_type(&self) -> Option<Self::ArrayType> {
        ArrayType::try_from(*self).ok()
    }

    fn can_resolve_unification(&self, other: &Self) -> bool {
        // Special case for pairs of 'index' and '!felt.type'
        if (self.is_index() && is_felt_type(*other)) || (is_felt_type(*self) && other.is_index()) {
            return true;
        }
        // Go by equality
        *self == *other
    }

    fn is_struct_like_type(&self) -> bool {
        is_struct_type(*self) || is_pod_type(*self)
    }

    fn to_struct_like_type(&self) -> Option<Self::StructType> {
        if let Ok(t) = StructType::try_from(*self) {
            Some(WrapStructLike::Struct(t))
        } else if let Ok(t) = PodType::try_from(*self) {
            Some(WrapStructLike::Pod(t))
        } else {
            None
        }
    }
}

impl<'ctx> ArrayTypeProperties for ArrayType<'ctx> {
    type Type = Type<'ctx>;

    fn inner_type(&self) -> Self::Type {
        let dims = self.dims();
        if dims.len() == 1 {
            self.element_type()
        } else {
            ArrayType::new(self.element_type(), &dims[1..]).into()
        }
    }

    fn contains_type_vars(&self) -> bool {
        self.element_type().contains_type_vars()
    }

    fn map_inner(&self, inner: Self::Type) -> Self {
        ArrayType::new(inner, &self.dims())
    }
}

/// Newtype wrapper over [`FunctionType`] for implementing the necessary traits.
#[derive(Copy, Clone)]
pub struct WrapFunctionType<'ctx>(FunctionType<'ctx>);

impl<'ctx> FnTypeProperties for WrapFunctionType<'ctx> {
    type Type = Type<'ctx>;

    fn inputs(&self) -> Vec<Self::Type> {
        (0..self.0.input_count())
            .map(|n| self.0.input(n).unwrap())
            .collect()
    }

    fn outputs(&self) -> Vec<Self::Type> {
        (0..self.0.result_count())
            .map(|n| self.0.result(n).unwrap())
            .collect()
    }

    fn contains_type_vars(&self) -> bool {
        let mut inputs = (0..self.0.input_count()).map(|n| self.0.input(n).unwrap());
        let mut outputs = (0..self.0.result_count()).map(|n| self.0.result(n).unwrap());

        inputs.any(|i| i.contains_type_vars()) || outputs.any(|o| o.contains_type_vars())
    }
}

impl<'ctx> From<WrapFunctionType<'ctx>> for Type<'ctx> {
    fn from(value: WrapFunctionType<'ctx>) -> Self {
        value.0.into()
    }
}

impl PartialEq for WrapFunctionType<'_> {
    fn eq(&self, other: &Self) -> bool {
        unsafe { mlir_sys::mlirTypeEqual(self.0.to_raw(), other.0.to_raw()) }
    }
}

impl std::fmt::Display for WrapFunctionType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::fmt::Debug for WrapFunctionType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

/// Wrapper over struct and pod types since from the point of view of the spec language they are
/// equivalent.
#[derive(Copy, Clone)]
pub enum WrapStructLike<'ctx> {
    /// Wraps a [`StructType`].
    Struct(StructType<'ctx>),
    /// Wraps a [`PodType`].
    Pod(PodType<'ctx>),
}

impl WrapStructLike<'_> {
    /// Returns the raw MLIR CAPI representation.
    fn to_raw(&self) -> mlir_sys::MlirType {
        match self {
            WrapStructLike::Struct(t) => t.to_raw(),
            WrapStructLike::Pod(t) => t.to_raw(),
        }
    }
}

impl<'ctx> From<WrapStructLike<'ctx>> for Type<'ctx> {
    fn from(value: WrapStructLike<'ctx>) -> Self {
        match value {
            WrapStructLike::Struct(t) => t.into(),
            WrapStructLike::Pod(t) => t.into(),
        }
    }
}

impl PartialEq for WrapStructLike<'_> {
    fn eq(&self, other: &Self) -> bool {
        unsafe { mlir_sys::mlirTypeEqual(self.to_raw(), other.to_raw()) }
    }
}

impl std::fmt::Display for WrapStructLike<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WrapStructLike::Struct(t) => std::fmt::Display::fmt(t, f),
            WrapStructLike::Pod(t) => std::fmt::Display::fmt(t, f),
        }
    }
}

impl std::fmt::Debug for WrapStructLike<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WrapStructLike::Struct(t) => std::fmt::Debug::fmt(t, f),
            WrapStructLike::Pod(t) => std::fmt::Debug::fmt(t, f),
        }
    }
}

impl<'ctx> StructTypeProperties for WrapStructLike<'ctx> {
    type Type = Type<'ctx>;
    type Scope = Module<'ctx>;

    fn contains_type_vars(&self) -> bool {
        match self {
            WrapStructLike::Struct(t) => t
                .params_vec()
                .into_iter()
                .filter_map(|a| TypeAttribute::try_from(a).ok().map(|t| t.value()))
                .any(|t| t.contains_type_vars()),
            WrapStructLike::Pod(t) => t
                .get_records()
                .iter()
                .any(|r| r.r#type().contains_type_vars()),
        }
    }

    fn member(&self, member: &str, root: &Self::Scope) -> Option<Type<'ctx>> {
        match self {
            WrapStructLike::Struct(t) => {
                let op = t.get_definition_from_module(root).ok()?;
                let op = StructDefOpRef::try_from(op.get_operation()?).ok()?;
                op.get_member_def(member)
                    .and_then(|def| def.has_public_attr().then_some(def.member_type()))
            }
            WrapStructLike::Pod(t) => t.get_type_of_record(member),
        }
    }

    fn member_types(&self, root: &Self::Scope) -> Vec<Type<'ctx>> {
        match self {
            WrapStructLike::Struct(t) => {
                let Ok(op) = t.get_definition_from_module(root) else {
                    return vec![];
                };
                let Some(op) = op
                    .get_operation()
                    .and_then(|op| StructDefOpRef::try_from(op).ok())
                else {
                    return vec![];
                };
                op.get_member_defs()
                    .into_iter()
                    .filter_map(|def| def.has_public_attr().then_some(def.member_type()))
                    .collect()
            }
            WrapStructLike::Pod(t) => t.get_records().into_iter().map(|r| r.r#type()).collect(),
        }
    }

    fn map_members(&self, mut map: impl FnMut(&Self::Type) -> Self::Type) -> Self {
        match self {
            WrapStructLike::Struct(t) => {
                // Map any type attributes that are type variables
                let params = t
                    .params_vec()
                    .into_iter()
                    .map(|a| {
                        if let Ok(ta) = TypeAttribute::try_from(a) {
                            TypeAttribute::new(match TVarType::try_from(ta.value()) {
                                Ok(tvar) => map(&tvar.into()),
                                _ => ta.value(),
                            })
                            .into()
                        } else {
                            a
                        }
                    })
                    .collect::<Vec<_>>();

                WrapStructLike::Struct(StructType::new(t.name(), &params))
            }
            WrapStructLike::Pod(t) => {
                let ctx = t.context();
                let records = t
                    .records()
                    .into_iter()
                    .map(|r| {
                        let name = r.name();
                        let name = name.as_string_ref().as_str().unwrap();
                        PodRecordAttribute::new(name, map(&r.r#type()))
                    })
                    .collect::<Vec<_>>();
                WrapStructLike::Pod(PodType::new(unsafe { ctx.to_ref() }, &records))
            }
        }
    }
}
