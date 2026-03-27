//! # dina-relay
//!
//! Lightweight SDK for passively relaying Dina payment settlements via BLE broadcast.
//!
//! Any app (e.g., Lucilla) integrates this crate to participate in the Dina relay
//! network — conceptually similar to Apple's Find My network, but for payment
//! settlement propagation. Devices scan for, validate, and re-broadcast settlement
//! blobs, earning micro-fees for each successful relay.

pub mod blob;
pub mod broadcaster;
pub mod error;
pub mod qr;
pub mod scanner;
pub mod stats;
pub mod submitter;

pub use blob::RelayBlob;
pub use broadcaster::{BroadcastConfig, RelayBroadcaster};
pub use error::{RelayError, Result};
pub use qr::{blob_from_qr_data, blob_to_qr_data, estimate_qr_version, is_qr_compatible};
pub use scanner::{BleAdvertisement, RelayScanner, RelayScannerConfig, DINA_COMPANY_ID};
pub use stats::RelayStats;
pub use submitter::{RelaySubmitter, SubmissionResult, SubmitterConfig};
