//! # dina-channels
//!
//! Offline payment channel system for the Dina Network.
//!
//! Two Cognitum Seeds can transact locally without internet using bidirectional
//! payment channels, then settle on-chain when connectivity is restored.
//!
//! ## Architecture
//!
//! - **Channel**: A bidirectional payment channel between two parties with locked funds.
//! - **State**: Signed state updates representing the current balance distribution.
//! - **Relay**: Compact blobs for mesh relay of settlement data via QR or BLE.
//! - **Manager**: High-level API for managing multiple channels on a single device.

pub mod channel;
pub mod error;
pub mod manager;
pub mod relay;
pub mod state;

pub use channel::{ChannelStatus, PaymentChannel};
pub use error::ChannelError;
pub use manager::ChannelManager;
pub use relay::RelayBlob;
pub use state::{SignedState, StateUpdate};
