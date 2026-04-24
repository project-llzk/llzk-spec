use crate::ast::Span;
use std::fmt::{self, Display};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub source: String,
    pub message: String,
    pub span: Option<Span>,
}

impl Diagnostic {
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
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Self::Syntax(diagnostic) => std::slice::from_ref(diagnostic),
            Self::Diagnostics(diagnostics) => diagnostics,
            _ => &[],
        }
    }
}
