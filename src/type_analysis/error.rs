use crate::{ast::Span, diagnostic::Diagnostic};
use thiserror::Error;

/// Top-level compilation error categories.
#[derive(Debug, Error)]
pub enum TypeAnalysisError {
    #[error("duplicate predicate '{0}'")]
    DuplicatePredicate(String),
    #[error("duplicate local '{0}'")]
    DuplicateLocal(String),
    #[error("unknown predicate '{0}'")]
    UnknownPredicate(String),
    #[error("unknown local '{0}'")]
    UnknownLocal(String),
    #[error("expected type '{0}' but got '{1}'")]
    UnexpectedTypes(String, String),
    #[error("type '{0}' is infinite")]
    InfiniteType(String),
}

impl TypeAnalysisError {
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
}
