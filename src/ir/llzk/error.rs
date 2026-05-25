//! Error type for LLZK related operations.

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("struct '{0}' not found")]
    StructNotFound(String),
}
