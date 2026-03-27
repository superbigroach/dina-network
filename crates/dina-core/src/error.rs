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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_invalid_signature() {
        let err = DinaError::InvalidSignature;
        assert_eq!(format!("{err}"), "invalid signature");
    }

    #[test]
    fn error_display_insufficient_balance() {
        let err = DinaError::InsufficientBalance { have: 50, need: 100 };
        assert_eq!(format!("{err}"), "insufficient balance: have 50, need 100");
    }

    #[test]
    fn error_display_invalid_nonce() {
        let err = DinaError::InvalidNonce { expected: 5, got: 3 };
        assert_eq!(format!("{err}"), "invalid nonce: expected 5, got 3");
    }

    #[test]
    fn error_display_account_not_found() {
        let err = DinaError::AccountNotFound("0xabc".to_string());
        assert_eq!(format!("{err}"), "account not found: 0xabc");
    }

    #[test]
    fn error_display_contract_not_found() {
        let err = DinaError::ContractNotFound("0xdef".to_string());
        assert_eq!(format!("{err}"), "contract not found: 0xdef");
    }

    #[test]
    fn error_display_device_not_found() {
        let err = DinaError::DeviceNotFound("device-1".to_string());
        assert_eq!(format!("{err}"), "device not found: device-1");
    }

    #[test]
    fn error_display_invalid_attestation() {
        let err = DinaError::InvalidAttestation("bad firmware".to_string());
        assert_eq!(format!("{err}"), "invalid attestation: bad firmware");
    }

    #[test]
    fn error_display_wasm_execution() {
        let err = DinaError::WasmExecutionError("out of gas".to_string());
        assert_eq!(format!("{err}"), "WASM execution error: out of gas");
    }

    #[test]
    fn error_display_consensus() {
        let err = DinaError::ConsensusError("fork detected".to_string());
        assert_eq!(format!("{err}"), "consensus error: fork detected");
    }

    #[test]
    fn error_display_network() {
        let err = DinaError::NetworkError("timeout".to_string());
        assert_eq!(format!("{err}"), "network error: timeout");
    }

    #[test]
    fn error_display_storage() {
        let err = DinaError::StorageError("disk full".to_string());
        assert_eq!(format!("{err}"), "storage error: disk full");
    }

    #[test]
    fn error_display_serialization() {
        let err = DinaError::SerializationError("invalid bytes".to_string());
        assert_eq!(format!("{err}"), "serialization error: invalid bytes");
    }

    #[test]
    fn error_display_custom() {
        let err = DinaError::Custom("something went wrong".to_string());
        assert_eq!(format!("{err}"), "something went wrong");
    }

    #[test]
    fn error_clone() {
        let err = DinaError::InsufficientBalance { have: 1, need: 2 };
        let cloned = err.clone();
        assert_eq!(format!("{err}"), format!("{cloned}"));
    }

    #[test]
    fn error_debug() {
        let err = DinaError::InvalidSignature;
        let debug = format!("{err:?}");
        assert!(debug.contains("InvalidSignature"));
    }

    #[test]
    fn dina_result_ok() {
        let result: DinaResult<u64> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn dina_result_err() {
        let result: DinaResult<u64> = Err(DinaError::InvalidSignature);
        assert!(result.is_err());
    }
}
