pub mod types;
pub mod cctp;
pub mod attestation;

pub use types::{BridgeTransfer, BridgeStatus, ChainId};
pub use cctp::{CctpBridge, CctpMessage, CctpAttestation};
pub use attestation::{verify_cctp_attestation, encode_cctp_message, decode_cctp_message};
