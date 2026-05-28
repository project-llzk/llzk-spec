//! Errors related to LLZK.

/// Error type for LLZK related operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("contract target '{0}' not found")]
    ContractTargetNotFound(String),
}
