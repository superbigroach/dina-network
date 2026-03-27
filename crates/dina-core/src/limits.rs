//! Protocol-level limits for transactions, blocks, contracts, and other
//! network parameters.
//!
//! These limits protect the network from resource exhaustion attacks and
//! ensure deterministic validation across all nodes. Different profiles
//! (mainnet, testnet, development) allow progressively relaxed limits
//! for testing purposes.

use crate::block::Block;
use crate::error::{DinaError, DinaResult};
use crate::transaction::Transaction;

/// Hard limits enforced by the Dina protocol at the consensus layer.
///
/// Every transaction and block must pass these limits before being accepted
/// into a block or applied to the state. Validators that accept out-of-bounds
/// values will have their blocks rejected by honest nodes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolLimits {
    /// Maximum serialized transaction size in bytes (default: 1 MB).
    pub max_transaction_size: usize,
    /// Maximum serialized block size in bytes (default: 10 MB).
    pub max_block_size: usize,
    /// Maximum number of transactions per block (default: 10,000).
    pub max_transactions_per_block: usize,
    /// Maximum WASM contract bytecode size in bytes (default: 500 KB).
    pub max_contract_size: usize,
    /// Maximum memo size in bytes (default: 4 KB).
    pub max_memo_size: usize,
    /// Maximum contract method name length in characters (default: 128).
    pub max_method_name_length: usize,
    /// Maximum contract call arguments size in bytes (default: 100 KB).
    pub max_args_size: usize,
    /// Maximum events emitted per transaction (default: 50).
    pub max_events_per_tx: usize,
    /// Maximum cross-contract call depth (default: 10).
    pub max_cross_contract_depth: usize,
    /// Maximum storage keys written per transaction (default: 1,000).
    pub max_storage_keys_per_tx: usize,
    /// Minimum transaction fee in micro-USDC (default: 100 = $0.0001).
    pub min_transaction_fee: u64,
    /// Maximum transaction fee in micro-USDC (default: 10,000,000 = $10).
    pub max_transaction_fee: u64,
    /// Maximum transfer amount in micro-USDC (default: 1,000,000,000,000 = $1M).
    pub max_transfer_amount: u64,
    /// Maximum payment channel duration in blocks (default: 864,000 ~30 days at 3s blocks).
    pub max_channel_duration_blocks: u64,
    /// Maximum number of validators in the active set (default: 7).
    pub max_validator_count: usize,
    /// Minimum validator stake in micro-USDC (default: 10,000,000,000 = $10,000).
    pub min_validator_stake: u64,
}

impl ProtocolLimits {
    /// Production mainnet limits -- the most restrictive profile.
    pub fn mainnet() -> Self {
        Self {
            max_transaction_size: 1_048_576,          // 1 MB
            max_block_size: 10_485_760,               // 10 MB
            max_transactions_per_block: 10_000,
            max_contract_size: 512_000,               // 500 KB
            max_memo_size: 4_096,                     // 4 KB
            max_method_name_length: 128,
            max_args_size: 102_400,                   // 100 KB
            max_events_per_tx: 50,
            max_cross_contract_depth: 10,
            max_storage_keys_per_tx: 1_000,
            min_transaction_fee: 100,                 // $0.0001
            max_transaction_fee: 10_000_000,          // $10
            max_transfer_amount: 1_000_000_000_000,   // $1,000,000
            max_channel_duration_blocks: 864_000,     // ~30 days at 3s blocks
            max_validator_count: 7,
            min_validator_stake: 10_000_000_000,      // $10,000
        }
    }

    /// Public testnet limits -- slightly more relaxed for testing.
    pub fn testnet() -> Self {
        Self {
            max_transaction_size: 2_097_152,          // 2 MB
            max_block_size: 20_971_520,               // 20 MB
            max_transactions_per_block: 20_000,
            max_contract_size: 1_048_576,             // 1 MB
            max_memo_size: 8_192,                     // 8 KB
            max_method_name_length: 256,
            max_args_size: 204_800,                   // 200 KB
            max_events_per_tx: 100,
            max_cross_contract_depth: 20,
            max_storage_keys_per_tx: 2_000,
            min_transaction_fee: 10,                  // $0.00001
            max_transaction_fee: 100_000_000,         // $100
            max_transfer_amount: 10_000_000_000_000,  // $10,000,000
            max_channel_duration_blocks: 2_592_000,   // ~90 days
            max_validator_count: 21,
            min_validator_stake: 1_000_000_000,       // $1,000
        }
    }

    /// Local development limits -- very relaxed for rapid iteration.
    pub fn development() -> Self {
        Self {
            max_transaction_size: 10_485_760,         // 10 MB
            max_block_size: 104_857_600,              // 100 MB
            max_transactions_per_block: 100_000,
            max_contract_size: 10_485_760,            // 10 MB
            max_memo_size: 65_536,                    // 64 KB
            max_method_name_length: 512,
            max_args_size: 1_048_576,                 // 1 MB
            max_events_per_tx: 1_000,
            max_cross_contract_depth: 50,
            max_storage_keys_per_tx: 10_000,
            min_transaction_fee: 1,                   // essentially free
            max_transaction_fee: 1_000_000_000,       // $1,000
            max_transfer_amount: u64::MAX,            // unlimited
            max_channel_duration_blocks: u64::MAX,
            max_validator_count: 100,
            min_validator_stake: 1_000_000,           // $1
        }
    }

    /// Validate a transaction against all applicable protocol limits.
    ///
    /// This should be called before adding a transaction to a block.
    pub fn validate_transaction(&self, tx: &Transaction) -> DinaResult<()> {
        // Check fee bounds
        let fee = tx.fee();
        if fee < self.min_transaction_fee {
            return Err(DinaError::Custom(format!(
                "transaction fee {fee} is below minimum {}", self.min_transaction_fee
            )));
        }
        if fee > self.max_transaction_fee {
            return Err(DinaError::Custom(format!(
                "transaction fee {fee} exceeds maximum {}", self.max_transaction_fee
            )));
        }

        // Check serialized size
        let tx_bytes = bincode::serialize(tx)
            .map_err(|e| DinaError::SerializationError(e.to_string()))?;
        if tx_bytes.len() > self.max_transaction_size {
            return Err(DinaError::Custom(format!(
                "transaction size {} exceeds limit {}",
                tx_bytes.len(),
                self.max_transaction_size
            )));
        }

        // Type-specific checks
        match tx {
            Transaction::Transfer { amount, memo, from, to, .. } => {
                if *amount > self.max_transfer_amount {
                    return Err(DinaError::Custom(format!(
                        "transfer amount {amount} exceeds limit {}",
                        self.max_transfer_amount
                    )));
                }
                if let Some(m) = memo {
                    if m.len() > self.max_memo_size {
                        return Err(DinaError::Custom(format!(
                            "memo size {} exceeds limit {}",
                            m.len(),
                            self.max_memo_size
                        )));
                    }
                }
                // Prevent transfers to the zero address
                if *to == crate::types::Address::ZERO {
                    return Err(DinaError::Custom(
                        "cannot transfer to zero address".to_string(),
                    ));
                }
                // Prevent self-transfers
                if from == to {
                    return Err(DinaError::Custom(
                        "self-transfers are not allowed".to_string(),
                    ));
                }
            }
            Transaction::DeployContract { wasm_bytecode, .. } => {
                if wasm_bytecode.len() > self.max_contract_size {
                    return Err(DinaError::Custom(format!(
                        "contract bytecode size {} exceeds limit {}",
                        wasm_bytecode.len(),
                        self.max_contract_size
                    )));
                }
                if wasm_bytecode.is_empty() {
                    return Err(DinaError::Custom(
                        "contract bytecode is empty".to_string(),
                    ));
                }
            }
            Transaction::CallContract { method, args, .. } => {
                if method.len() > self.max_method_name_length {
                    return Err(DinaError::Custom(format!(
                        "method name length {} exceeds limit {}",
                        method.len(),
                        self.max_method_name_length
                    )));
                }
                if args.len() > self.max_args_size {
                    return Err(DinaError::Custom(format!(
                        "args size {} exceeds limit {}",
                        args.len(),
                        self.max_args_size
                    )));
                }
                // Validate method name characters
                if !method.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    return Err(DinaError::Custom(format!(
                        "method name contains invalid characters: {method:?}"
                    )));
                }
            }
            Transaction::RegisterDevice { .. } => {
                // No additional type-specific limits beyond fee and size
            }
        }

        Ok(())
    }

    /// Validate a block against protocol limits.
    ///
    /// This should be called before executing a block.
    pub fn validate_block(&self, block: &Block) -> DinaResult<()> {
        // Check transaction count
        if block.transactions.len() > self.max_transactions_per_block {
            return Err(DinaError::Custom(format!(
                "block contains {} transactions, exceeds limit {}",
                block.transactions.len(),
                self.max_transactions_per_block
            )));
        }

        // Check serialized block size
        let block_bytes = bincode::serialize(block)
            .map_err(|e| DinaError::SerializationError(e.to_string()))?;
        if block_bytes.len() > self.max_block_size {
            return Err(DinaError::Custom(format!(
                "block size {} exceeds limit {}",
                block_bytes.len(),
                self.max_block_size
            )));
        }

        // Validate each transaction individually
        for (i, tx) in block.transactions.iter().enumerate() {
            self.validate_transaction(tx).map_err(|e| {
                DinaError::Custom(format!("transaction {i} in block is invalid: {e}"))
            })?;
        }

        Ok(())
    }
}

impl Default for ProtocolLimits {
    fn default() -> Self {
        Self::mainnet()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{Block, BlockHeader};
    use crate::transaction::Sig64;
    use crate::types::{Address, Hash};

    fn make_transfer(from: Address, to: Address, amount: u64, fee: u64) -> Transaction {
        Transaction::Transfer {
            from,
            to,
            amount,
            memo: None,
            device_witness: None,
            nonce: 0,
            fee,
            signature: Sig64([1u8; 64]),  // non-zero sig
        }
    }

    fn make_block(txs: Vec<Transaction>) -> Block {
        Block {
            header: BlockHeader {
                block_number: 1,
                parent_hash: Hash::ZERO,
                state_root: Hash::ZERO,
                transactions_root: Hash::ZERO,
                timestamp: 1_700_000_000,
                proposer: Address([0x01; 32]),
                signature: [0u8; 64],
            },
            transactions: txs,
        }
    }

    // -- ProtocolLimits profiles --

    #[test]
    fn mainnet_limits_exist() {
        let limits = ProtocolLimits::mainnet();
        assert_eq!(limits.max_transaction_size, 1_048_576);
        assert_eq!(limits.max_validator_count, 7);
    }

    #[test]
    fn testnet_more_relaxed() {
        let mainnet = ProtocolLimits::mainnet();
        let testnet = ProtocolLimits::testnet();
        assert!(testnet.max_transaction_size > mainnet.max_transaction_size);
        assert!(testnet.max_contract_size > mainnet.max_contract_size);
        assert!(testnet.min_transaction_fee < mainnet.min_transaction_fee);
    }

    #[test]
    fn development_most_relaxed() {
        let testnet = ProtocolLimits::testnet();
        let dev = ProtocolLimits::development();
        assert!(dev.max_transaction_size > testnet.max_transaction_size);
        assert!(dev.min_transaction_fee < testnet.min_transaction_fee);
    }

    #[test]
    fn default_is_mainnet() {
        assert_eq!(ProtocolLimits::default(), ProtocolLimits::mainnet());
    }

    // -- validate_transaction --

    #[test]
    fn validate_transaction_ok() {
        let limits = ProtocolLimits::mainnet();
        let tx = make_transfer(
            Address([0x01; 32]),
            Address([0x02; 32]),
            1_000_000,
            200,
        );
        assert!(limits.validate_transaction(&tx).is_ok());
    }

    #[test]
    fn validate_transaction_fee_too_low() {
        let limits = ProtocolLimits::mainnet();
        let tx = make_transfer(
            Address([0x01; 32]),
            Address([0x02; 32]),
            1_000,
            1, // below min_transaction_fee of 100
        );
        assert!(limits.validate_transaction(&tx).is_err());
    }

    #[test]
    fn validate_transaction_fee_too_high() {
        let limits = ProtocolLimits::mainnet();
        let tx = make_transfer(
            Address([0x01; 32]),
            Address([0x02; 32]),
            1_000,
            100_000_000, // above max_transaction_fee of 10,000,000
        );
        assert!(limits.validate_transaction(&tx).is_err());
    }

    #[test]
    fn validate_transaction_transfer_too_large() {
        let limits = ProtocolLimits::mainnet();
        let tx = make_transfer(
            Address([0x01; 32]),
            Address([0x02; 32]),
            u64::MAX, // way above max_transfer_amount
            200,
        );
        assert!(limits.validate_transaction(&tx).is_err());
    }

    #[test]
    fn validate_transaction_self_transfer() {
        let limits = ProtocolLimits::mainnet();
        let addr = Address([0x01; 32]);
        let tx = make_transfer(addr, addr, 1_000, 200);
        assert!(limits.validate_transaction(&tx).is_err());
    }

    #[test]
    fn validate_transaction_zero_address_target() {
        let limits = ProtocolLimits::mainnet();
        let tx = make_transfer(
            Address([0x01; 32]),
            Address::ZERO,
            1_000,
            200,
        );
        assert!(limits.validate_transaction(&tx).is_err());
    }

    #[test]
    fn validate_transaction_memo_too_large() {
        let limits = ProtocolLimits::mainnet();
        let tx = Transaction::Transfer {
            from: Address([0x01; 32]),
            to: Address([0x02; 32]),
            amount: 1_000,
            memo: Some(vec![0u8; 5_000]), // exceeds 4KB
            device_witness: None,
            nonce: 0,
            fee: 200,
            signature: Sig64([1u8; 64]),
        };
        assert!(limits.validate_transaction(&tx).is_err());
    }

    #[test]
    fn validate_transaction_contract_too_large() {
        let limits = ProtocolLimits::mainnet();
        let tx = Transaction::DeployContract {
            from: Address([0x01; 32]),
            wasm_bytecode: vec![0u8; 600_000], // exceeds 500KB
            init_args: vec![],
            nonce: 0,
            fee: 1_000_000,
            signature: Sig64([1u8; 64]),
        };
        assert!(limits.validate_transaction(&tx).is_err());
    }

    #[test]
    fn validate_transaction_empty_contract() {
        let limits = ProtocolLimits::mainnet();
        let tx = Transaction::DeployContract {
            from: Address([0x01; 32]),
            wasm_bytecode: vec![],
            init_args: vec![],
            nonce: 0,
            fee: 1_000_000,
            signature: Sig64([1u8; 64]),
        };
        assert!(limits.validate_transaction(&tx).is_err());
    }

    #[test]
    fn validate_transaction_method_name_too_long() {
        let limits = ProtocolLimits::mainnet();
        let tx = Transaction::CallContract {
            from: Address([0x01; 32]),
            contract: Address([0x02; 32]),
            method: "a".repeat(200),
            args: vec![],
            usdc_attached: 0,
            nonce: 0,
            fee: 10_000,
            signature: Sig64([1u8; 64]),
        };
        assert!(limits.validate_transaction(&tx).is_err());
    }

    #[test]
    fn validate_transaction_invalid_method_chars() {
        let limits = ProtocolLimits::mainnet();
        let tx = Transaction::CallContract {
            from: Address([0x01; 32]),
            contract: Address([0x02; 32]),
            method: "drop(); --".to_string(),
            args: vec![],
            usdc_attached: 0,
            nonce: 0,
            fee: 10_000,
            signature: Sig64([1u8; 64]),
        };
        assert!(limits.validate_transaction(&tx).is_err());
    }

    #[test]
    fn validate_transaction_args_too_large() {
        let limits = ProtocolLimits::mainnet();
        let tx = Transaction::CallContract {
            from: Address([0x01; 32]),
            contract: Address([0x02; 32]),
            method: "call".to_string(),
            args: vec![0u8; 200_000], // exceeds 100KB
            usdc_attached: 0,
            nonce: 0,
            fee: 10_000,
            signature: Sig64([1u8; 64]),
        };
        assert!(limits.validate_transaction(&tx).is_err());
    }

    // -- validate_block --

    #[test]
    fn validate_block_ok() {
        let limits = ProtocolLimits::mainnet();
        let tx = make_transfer(Address([0x01; 32]), Address([0x02; 32]), 1_000, 200);
        let block = make_block(vec![tx]);
        assert!(limits.validate_block(&block).is_ok());
    }

    #[test]
    fn validate_block_empty() {
        let limits = ProtocolLimits::mainnet();
        let block = make_block(vec![]);
        assert!(limits.validate_block(&block).is_ok());
    }

    #[test]
    fn validate_block_invalid_transaction() {
        let limits = ProtocolLimits::mainnet();
        let bad_tx = make_transfer(
            Address([0x01; 32]),
            Address([0x02; 32]),
            1_000,
            1, // fee too low
        );
        let block = make_block(vec![bad_tx]);
        assert!(limits.validate_block(&block).is_err());
    }
}
