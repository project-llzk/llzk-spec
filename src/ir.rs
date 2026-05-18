//! LLZK IR emission and handling.

pub mod llzk;
pub mod verif;

use ::llzk::prelude::*;
use melior::ir::Module;

use crate::{ast::Span, diagnostic::CompileError};

/// Context supporting IR handling and generation .
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
