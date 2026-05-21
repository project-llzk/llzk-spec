//! LLZK IR emission and handling.

pub mod llzk;
pub mod verif;

use ::llzk::{dialect::poly::TVarType, prelude::*};
use melior::ir::Module;

use crate::{
    ast::Span,
    diagnostic::CompileError,
    type_analysis::{FnTypeProperties, TypeProperties, TypeSystem},
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
        llzk_module(self.location_from_span(filename, span))
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
    pub fn bool_type(&self) -> Type {
        IntegerType::new(self.context(), 1).into()
    }

    /// Returns a type representing a finite field element.
    pub fn felt_type(&self) -> Type {
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

pub struct MlirTypeSystem<'ctx> {
    ctx: &'ctx Context,
    next_var: usize,
}

impl<'ctx> MlirTypeSystem<'ctx> {
    pub fn new(ctx: &'ctx Context) -> Self {
        Self { ctx, next_var: 0 }
    }
}

impl<'ctx> TypeSystem for MlirTypeSystem<'ctx> {
    type Type = Type<'ctx>;

    type FnType = WrapFunctionType<'ctx>;

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
        TVarType::new(self.ctx.context(), StringRef::new(&format!("τ{id}"))).into()
    }
}

impl<'ctx> TypeProperties for Type<'ctx> {
    type FnType = WrapFunctionType<'ctx>;

    type VarId = &'ctx str;

    fn is_var_type(&self) -> bool {
        is_type_variable(*self)
    }

    fn var_id(&self) -> Option<Self::VarId> {
        TVarType::try_from(*self)
            .ok()
            .and_then(|t| unsafe { std::mem::transmute(t.name().as_str().ok()) })
    }

    fn is_func_type(&self) -> bool {
        self.is_function()
    }

    fn to_func_type(&self) -> Option<Self::FnType> {
        FunctionType::try_from(*self).ok().map(WrapFunctionType)
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
