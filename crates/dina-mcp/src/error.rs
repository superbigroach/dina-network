use thiserror::Error;

/// Errors that can occur in the MCP integration layer.
#[derive(Error, Debug, Clone)]
pub enum McpError {
    #[error("unknown tool: {0}")]
    UnknownTool(String),

    #[error("invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("device error: {0}")]
    DeviceError(String),

    #[error("channel error: {0}")]
    ChannelError(String),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("server error: {0}")]
    ServerError(String),

    #[error("attestation verification failed: {0}")]
    AttestationFailed(String),

    #[error("witness chain invalid: {0}")]
    WitnessChainInvalid(String),
}

pub type McpResult<T> = Result<T, McpError>;
