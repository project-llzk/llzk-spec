//! Diagnostic and error types for the compiler.

use crate::ast::Span;
use std::{
    fmt::{self, Display},
    str::Utf8Error,
};
use thiserror::Error;

/// User-facing diagnostic emitted during parsing or semantic verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub source: String,
    pub message: String,
    pub span: Option<Span>,
}

impl Diagnostic {
    /// Creates a new diagnostic message.
    pub fn new(source: impl Into<String>, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            source: source.into(),
            message: message.into(),
            span,
        }
    }
}

impl Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.span {
            Some(span) => write!(
                f,
                "{}:{}:{}: {}",
                self.source, span.line, span.column, self.message
            ),
            None => write!(f, "{}: {}", self.source, self.message),
        }
    }
}

/// A collection of structured diagnostics with CLI-oriented display formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostics(Vec<Diagnostic>);

impl Diagnostics {
    /// Returns the diagnostics as a slice for structured consumers.
    pub fn as_slice(&self) -> &[Diagnostic] {
        &self.0
    }

    /// Iterates over the diagnostics in emission order.
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.0.iter()
    }
}

impl From<Vec<Diagnostic>> for Diagnostics {
    fn from(diagnostics: Vec<Diagnostic>) -> Self {
        Self(diagnostics)
    }
}

impl Display for Diagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        )?;
        Ok(())
    }
}

/// Top-level compilation error categories.
#[derive(Debug, Error)]
pub enum CompileError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Ir(String),
    #[error("failed to serialize AST: {0}")]
    AstSerialization(#[source] serde_json::Error),
    #[error("{0}")]
    Syntax(Diagnostic),
    #[error("{0}")]
    Diagnostics(Diagnostics),
    #[error(transparent)]
    Llzk(#[from] llzk::error::Error),
    #[error(transparent)]
    Mlir(#[from] melior::Error),
    #[error(transparent)]
    Utf8(#[from] Utf8Error),
    #[error(transparent)]
    BigUint(#[from] num_bigint::ParseBigIntError),
}

impl CompileError {
    /// Returns any structured diagnostics carried by the error.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Self::Syntax(diagnostic) => std::slice::from_ref(diagnostic),
            Self::Diagnostics(diagnostics) => diagnostics.as_slice(),
            _ => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CompileError, Diagnostic, Diagnostics};

    #[test]
    fn compile_error_displays_multiple_diagnostics_on_separate_lines() {
        let error = CompileError::Diagnostics(Diagnostics::from(vec![
            Diagnostic::new("first.spec", "first failure", None),
            Diagnostic::new("second.spec", "second failure", None),
        ]));

        assert_eq!(
            error.to_string(),
            "first.spec: first failure\nsecond.spec: second failure"
        );
    }
}
