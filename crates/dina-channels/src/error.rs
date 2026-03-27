use thiserror::Error;

/// Errors that can occur during payment channel operations.
#[derive(Debug, Clone, Error)]
pub enum ChannelError {
    #[error("invalid signature")]
    InvalidSignature,

    #[error("insufficient balance: need {need}, have {have}")]
    InsufficientBalance { need: u64, have: u64 },

    #[error("channel not found: 0x{}", hex::encode(.0))]
    ChannelNotFound([u8; 32]),

    #[error("channel is closed")]
    ChannelClosed,

    #[error("invalid sequence: got {got}, expected > {current}")]
    InvalidSequence { got: u64, current: u64 },

    #[error("challenge period is still active")]
    ChallengePeriodActive,

    #[error("not a party to this channel")]
    NotPartyToChannel,

    #[error("invalid relay blob")]
    InvalidRelayBlob,

    #[error("serialization error: {0}")]
    SerializationError(String),
}

pub type Result<T> = std::result::Result<T, ChannelError>;
