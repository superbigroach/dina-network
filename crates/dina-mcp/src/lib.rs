//! # dina-mcp
//!
//! Model Context Protocol (MCP) integration for the Dina Network.
//!
//! This crate exposes Dina Network functionality as MCP tools that Cognitum
//! Seeds can call directly. It provides a standardized tool interface for
//! transfers, contract deployment, device registration, payment channels,
//! and network queries.
//!
//! ## Architecture
//!
//! - **Tools**: MCP tool definitions with JSON Schema input specifications.
//! - **Handler**: Routes incoming tool calls to the appropriate JSON-RPC endpoint.
//! - **Server**: HTTP server that accepts MCP protocol messages and returns results.
//! - **Device**: Cognitum device-specific utilities for attestation and witness chains.

pub mod device;
pub mod error;
pub mod handler;
pub mod server;
pub mod tools;

pub use device::{CognitumDevice, WitnessEntry};
pub use error::McpError;
pub use handler::McpHandler;
pub use server::McpServer;
pub use tools::{McpTool, McpToolCall, McpToolResult};
