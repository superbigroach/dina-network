use thiserror::Error;

/// Errors that can occur during relay operations.
#[derive(Debug, Clone, Error)]
pub enum RelayError {
    #[error("invalid company ID: expected 0x{expected:04X}, got 0x{got:04X}")]
    InvalidCompanyId { expected: u16, got: u16 },

    #[error("invalid relay blob: {0}")]
    InvalidBlob(String),

    #[error("blob has expired (created at {created_at}, ttl {ttl_secs}s)")]
    BlobExpired { created_at: u64, ttl_secs: u64 },

    #[error("invalid signature on relay blob")]
    InvalidSignature,

    #[error("payload too large: {size} bytes (max {max} bytes)")]
    PayloadTooLarge { size: usize, max: usize },

    #[error("queue is full: {current}/{max} items")]
    QueueFull { current: usize, max: usize },

    #[error("submission failed: {0}")]
    SubmissionFailed(String),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("QR encoding error: {0}")]
    QrError(String),

    #[error("network error: {0}")]
    NetworkError(String),
}

pub type Result<T> = std::result::Result<T, RelayError>;
