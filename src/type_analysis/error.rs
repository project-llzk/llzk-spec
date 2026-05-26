use crate::{
    ast::Span,
    diagnostic::{CompileError, Diagnostic},
};
use thiserror::Error;

/// Top-level compilation error categories.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TypeAnalysisError {
    #[error("duplicate predicate '{0}'")]
    DuplicatePredicate(String),
    #[error("duplicate local '{0}'")]
    DuplicateLocal(String),
    #[error("duplicate loop '{0}'")]
    DuplicateLoop(String),
    #[error("unknown predicate '{0}'")]
    UnknownPredicate(String),
    #[error("unknown local '{0}'")]
    UnknownLocal(String),
    #[error("unknown loop '{0}'")]
    UnknownLoop(String),
    #[error("expected type '{0}' but got '{1}'")]
    UnexpectedTypes(String, String),
    #[error("type '{0}' is infinite")]
    InfiniteType(String),
}

impl TypeAnalysisError {
    /// Converts the error into a list of diagnostics.
    pub fn into_diags(
        self,
        source_name: &str,
        span: Option<Span>,
        context: impl std::fmt::Display,
    ) -> Vec<Diagnostic> {
        vec![Diagnostic::new(
            source_name,
            format!("{context}: {self}"),
            span,
        )]
    }

    /// Converts the error into a general compilation error.
    pub fn into_compile_error(
        self,
        source_name: &str,
        span: Option<Span>,
        context: impl std::fmt::Display,
    ) -> CompileError {
        CompileError::Diagnostics(self.into_diags(source_name, span, context).into())
    }
}
