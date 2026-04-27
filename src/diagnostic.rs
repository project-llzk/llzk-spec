//! Diagnostic and error types for the compiler.

use crate::ast::Span;
use std::fmt::{self, Display};
use thiserror::Error;

/// User-facing diagnostic emitted during parsing or semantic verification.
///
/// The compiler keeps this custom type as its public diagnostic representation
/// so parser, verifier, and IR-loading failures share one stable output format.
/// Raw MLIR diagnostics remain an internal source of detail that can be folded
/// into these messages later without changing the compiler-facing API.
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

/// Top-level compilation error categories.
#[derive(Debug, Error)]
pub enum CompileError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Ir(String),
    #[error("{0}")]
    Usage(String),
    #[error("{0}")]
    Syntax(Diagnostic),
    #[error("compilation failed")]
    Diagnostics(Vec<Diagnostic>),
}

impl CompileError {
    /// Returns any structured diagnostics carried by the error.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Self::Syntax(diagnostic) => std::slice::from_ref(diagnostic),
            Self::Diagnostics(diagnostics) => diagnostics,
            _ => &[],
        }
    }

    /// Prints the error in the same diagnostic-oriented format used by the CLI.
    pub fn print(&self) {
        let diagnostics = self.diagnostics();
        if diagnostics.is_empty() {
            eprintln!("{self}");
        } else {
            for diagnostic in diagnostics {
                eprintln!("{diagnostic}");
            }
        }
    }
}
