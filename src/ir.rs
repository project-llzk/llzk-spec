//! LLZK IR emission and handling.

pub mod llzk;
pub mod verif;

use ::llzk::prelude::*;
use melior::ir::Module;

use crate::{ast::Span, diagnostic::CompileError};

/// Context supporting IR handling and generation .
pub struct Context {
    context: LlzkContext,
}

impl Context {
    /// Creates a new context.
    pub fn new() -> Self {
        Self {
            context: LlzkContext::new_no_log(),
        }
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
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
