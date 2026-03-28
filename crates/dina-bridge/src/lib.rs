pub mod attestation;
pub mod cctp;
pub mod types;

pub use attestation::{decode_cctp_message, encode_cctp_message, verify_cctp_attestation};
pub use cctp::{CctpAttestation, CctpBridge, CctpMessage};
pub use types::{BridgeStatus, BridgeTransfer, ChainId};
