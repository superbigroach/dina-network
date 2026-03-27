use serde::{Deserialize, Serialize};
use serde_json::json;

/// Definition of an MCP tool exposed by the Dina Network.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpTool {
    /// Unique tool name in namespace/action format (e.g., "dina/transfer").
    pub name: String,
    /// Human-readable description of what the tool does.
    pub description: String,
    /// JSON Schema describing the tool's input parameters.
    pub input_schema: serde_json::Value,
}

/// An incoming MCP tool call from a Cognitum Seed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpToolCall {
    /// The tool name to invoke (must match an available tool).
    pub tool_name: String,
    /// Arguments to pass to the tool, conforming to the tool's input_schema.
    pub arguments: serde_json::Value,
}

/// Result of executing an MCP tool call.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpToolResult {
    /// Whether the tool call succeeded.
    pub success: bool,
    /// Result data on success, or empty object on failure.
    pub data: serde_json::Value,
    /// Error message if the tool call failed.
    pub error: Option<String>,
}

impl McpToolResult {
    /// Create a successful result with the given data.
    pub fn ok(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data,
            error: None,
        }
    }

    /// Create a failed result with the given error message.
    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: json!({}),
            error: Some(message.into()),
        }
    }
}

/// Returns the complete list of MCP tools exposed by the Dina Network.
pub fn all_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "dina/transfer".to_string(),
            description: "Send USDC to an address on the Dina Network. Returns the transaction hash on success.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "to": {
                        "type": "string",
                        "description": "Recipient address (0x-prefixed hex, 32 bytes)"
                    },
                    "amount": {
                        "type": "integer",
                        "description": "Amount in micro-USDC (1 USDC = 1_000_000)",
                        "minimum": 1
                    },
                    "memo": {
                        "type": "string",
                        "description": "Optional memo attached to the transfer (hex-encoded bytes)"
                    },
                    "fee": {
                        "type": "integer",
                        "description": "Transaction fee in micro-USDC",
                        "minimum": 0
                    }
                },
                "required": ["to", "amount"]
            }),
        },
        McpTool {
            name: "dina/balance".to_string(),
            description: "Check the USDC balance of an address on the Dina Network.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "address": {
                        "type": "string",
                        "description": "Address to query (0x-prefixed hex, 32 bytes). Omit to query the device's own address."
                    }
                },
                "required": []
            }),
        },
        McpTool {
            name: "dina/deploy_contract".to_string(),
            description: "Deploy a WASM smart contract to the Dina Network. Returns the deployed contract address.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "wasm_bytecode": {
                        "type": "string",
                        "description": "Hex-encoded WASM bytecode of the contract to deploy"
                    },
                    "init_args": {
                        "type": "string",
                        "description": "Hex-encoded initialization arguments for the contract constructor"
                    },
                    "fee": {
                        "type": "integer",
                        "description": "Transaction fee in micro-USDC",
                        "minimum": 0
                    }
                },
                "required": ["wasm_bytecode"]
            }),
        },
        McpTool {
            name: "dina/call_contract".to_string(),
            description: "Call a method on a deployed smart contract. Returns the method's return value.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "contract": {
                        "type": "string",
                        "description": "Contract address (0x-prefixed hex, 32 bytes)"
                    },
                    "method": {
                        "type": "string",
                        "description": "Method name to call on the contract"
                    },
                    "args": {
                        "type": "string",
                        "description": "Hex-encoded arguments to pass to the method"
                    },
                    "usdc_attached": {
                        "type": "integer",
                        "description": "Amount of micro-USDC to attach to the call",
                        "minimum": 0
                    },
                    "fee": {
                        "type": "integer",
                        "description": "Transaction fee in micro-USDC",
                        "minimum": 0
                    }
                },
                "required": ["contract", "method"]
            }),
        },
        McpTool {
            name: "dina/register_device".to_string(),
            description: "Register a Cognitum device on-chain. The device's attestation is verified and its identity is stored in the network state.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "device_pubkey": {
                        "type": "string",
                        "description": "Device's Ed25519 public key (hex-encoded, 32 bytes)"
                    },
                    "owner": {
                        "type": "string",
                        "description": "Owner address (0x-prefixed hex, 32 bytes)"
                    },
                    "firmware_hash": {
                        "type": "string",
                        "description": "SHA-256 hash of the device firmware (hex-encoded, 32 bytes)"
                    },
                    "witness_root": {
                        "type": "string",
                        "description": "Merkle root of the device's witness history (hex-encoded, 32 bytes)"
                    },
                    "attestation_signature": {
                        "type": "string",
                        "description": "Ed25519 signature over the attestation fields (hex-encoded, 64 bytes)"
                    },
                    "fee": {
                        "type": "integer",
                        "description": "Transaction fee in micro-USDC",
                        "minimum": 0
                    }
                },
                "required": ["device_pubkey", "owner", "firmware_hash", "attestation_signature"]
            }),
        },
        McpTool {
            name: "dina/verify_device".to_string(),
            description: "Verify a device's attestation against its on-chain identity. Returns verification status and device metadata.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "device_id": {
                        "type": "string",
                        "description": "Device ID to verify (0x-prefixed hex, 32 bytes)"
                    },
                    "attestation_pubkey": {
                        "type": "string",
                        "description": "Device's Ed25519 public key for attestation verification (hex-encoded, 32 bytes)"
                    },
                    "firmware_hash": {
                        "type": "string",
                        "description": "Expected firmware hash to verify against (hex-encoded, 32 bytes)"
                    },
                    "witness_root": {
                        "type": "string",
                        "description": "Expected witness root to verify against (hex-encoded, 32 bytes)"
                    },
                    "attestation_signature": {
                        "type": "string",
                        "description": "Attestation signature to verify (hex-encoded, 64 bytes)"
                    }
                },
                "required": ["device_id"]
            }),
        },
        McpTool {
            name: "dina/channel_open".to_string(),
            description: "Open a bidirectional payment channel with another party. Funds are locked on-chain and can be exchanged off-chain.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "counterparty": {
                        "type": "string",
                        "description": "Address of the counterparty to open a channel with (0x-prefixed hex, 32 bytes)"
                    },
                    "deposit": {
                        "type": "integer",
                        "description": "Amount in micro-USDC to lock in the channel",
                        "minimum": 1
                    },
                    "fee": {
                        "type": "integer",
                        "description": "Transaction fee in micro-USDC",
                        "minimum": 0
                    }
                },
                "required": ["counterparty", "deposit"]
            }),
        },
        McpTool {
            name: "dina/channel_pay".to_string(),
            description: "Make an off-chain payment through an open payment channel. Updates the channel state locally.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "string",
                        "description": "Channel identifier (hex-encoded, 32 bytes)"
                    },
                    "amount": {
                        "type": "integer",
                        "description": "Amount in micro-USDC to pay through the channel",
                        "minimum": 1
                    }
                },
                "required": ["channel_id", "amount"]
            }),
        },
        McpTool {
            name: "dina/channel_close".to_string(),
            description: "Close a payment channel and settle the final balances on-chain.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "string",
                        "description": "Channel identifier (hex-encoded, 32 bytes)"
                    },
                    "fee": {
                        "type": "integer",
                        "description": "Transaction fee for the settlement transaction in micro-USDC",
                        "minimum": 0
                    }
                },
                "required": ["channel_id"]
            }),
        },
        McpTool {
            name: "dina/peers".to_string(),
            description: "List discovered peers on the Dina Network. Returns peer IDs, addresses, and connection status.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of peers to return",
                        "minimum": 1,
                        "maximum": 1000
                    }
                },
                "required": []
            }),
        },
        McpTool {
            name: "dina/block_info".to_string(),
            description: "Get information about a specific block by number or hash. Returns block header, transaction count, and proposer.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "block_number": {
                        "type": "integer",
                        "description": "Block number (height) to query. Omit for latest block.",
                        "minimum": 0
                    },
                    "block_hash": {
                        "type": "string",
                        "description": "Block hash to query (0x-prefixed hex, 32 bytes). Overrides block_number if provided."
                    }
                },
                "required": []
            }),
        },
        McpTool {
            name: "dina/network_status".to_string(),
            description: "Get the current network status including chain height, peer count, sync state, and node version.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tools_returns_twelve_tools() {
        let tools = all_tools();
        assert_eq!(tools.len(), 12);
    }

    #[test]
    fn all_tools_have_unique_names() {
        let tools = all_tools();
        let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 12);
    }

    #[test]
    fn all_tools_have_dina_namespace() {
        let tools = all_tools();
        for tool in &tools {
            assert!(
                tool.name.starts_with("dina/"),
                "tool '{}' must start with 'dina/'",
                tool.name
            );
        }
    }

    #[test]
    fn all_tools_have_descriptions() {
        let tools = all_tools();
        for tool in &tools {
            assert!(
                !tool.description.is_empty(),
                "tool '{}' must have a description",
                tool.name
            );
        }
    }

    #[test]
    fn all_tools_have_object_schemas() {
        let tools = all_tools();
        for tool in &tools {
            assert_eq!(
                tool.input_schema["type"], "object",
                "tool '{}' schema must have type 'object'",
                tool.name
            );
        }
    }

    #[test]
    fn tool_result_ok() {
        let result = McpToolResult::ok(json!({"balance": 1000}));
        assert!(result.success);
        assert_eq!(result.data["balance"], 1000);
        assert!(result.error.is_none());
    }

    #[test]
    fn tool_result_err() {
        let result = McpToolResult::err("something failed");
        assert!(!result.success);
        assert!(result.error.is_some());
        assert_eq!(result.error.unwrap(), "something failed");
    }

    #[test]
    fn tool_call_roundtrip() {
        let call = McpToolCall {
            tool_name: "dina/transfer".to_string(),
            arguments: json!({
                "to": "0xabababababababababababababababababababababababababababababababab",
                "amount": 1_000_000
            }),
        };
        let serialized = serde_json::to_string(&call).unwrap();
        let deserialized: McpToolCall = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.tool_name, "dina/transfer");
        assert_eq!(deserialized.arguments["amount"], 1_000_000);
    }

    #[test]
    fn tool_definition_roundtrip() {
        let tools = all_tools();
        let serialized = serde_json::to_string(&tools).unwrap();
        let deserialized: Vec<McpTool> = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.len(), tools.len());
        for (orig, deser) in tools.iter().zip(deserialized.iter()) {
            assert_eq!(orig.name, deser.name);
        }
    }
}
