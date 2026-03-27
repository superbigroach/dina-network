use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum DinaError {
    #[error("invalid signature")]
    InvalidSignature,

    #[error("insufficient balance: have {have}, need {need}")]
    InsufficientBalance { have: u64, need: u64 },

    #[error("invalid nonce: expected {expected}, got {got}")]
    InvalidNonce { expected: u64, got: u64 },

    #[error("account not found: {0}")]
    AccountNotFound(String),

    #[error("contract not found: {0}")]
    ContractNotFound(String),

    #[error("device not found: {0}")]
    DeviceNotFound(String),

    #[error("invalid attestation: {0}")]
    InvalidAttestation(String),

    #[error("WASM execution error: {0}")]
    WasmExecutionError(String),

    #[error("consensus error: {0}")]
    ConsensusError(String),

    #[error("network error: {0}")]
    NetworkError(String),

    #[error("storage error: {0}")]
    StorageError(String),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("{0}")]
    Custom(String),
}

pub type DinaResult<T> = Result<T, DinaError>;
