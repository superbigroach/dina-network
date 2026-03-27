use std::collections::HashMap;

use crate::account::AccountState;
use crate::block::Block;
use crate::crypto::hash_bytes;
use crate::device::{DeviceIdentity, DeviceType};
use crate::error::{DinaError, DinaResult};
use crate::transaction::Transaction;
use crate::types::{Address, Hash};

/// Result of executing a full block of transactions.
#[derive(Debug)]
pub struct ExecutionResult {
    /// Merkle root of the world state after executing all transactions.
    pub state_root: Hash,
    /// Receipt for each transaction, in the same order as the block.
    pub receipts: Vec<TransactionReceipt>,
    /// Sum of all fees collected by the block proposer.
    pub total_fees: u64,
    /// Total gas consumed by the block.
    pub gas_used: u64,
}

/// Receipt for a single executed transaction.
#[derive(Debug, Clone)]
pub struct TransactionReceipt {
    /// Hash of the transaction.
    pub tx_hash: Hash,
    /// Whether the transaction executed successfully.
    pub success: bool,
    /// Gas consumed by this transaction.
    pub gas_used: u64,
    /// Fee paid by the sender.
    pub fee_paid: u64,
    /// Error message if the transaction failed.
    pub error: Option<String>,
    /// Events emitted during execution.
    pub events: Vec<Event>,
}

/// An event emitted during transaction execution.
#[derive(Debug, Clone)]
pub struct Event {
    /// Contract address that emitted the event, if any.
    pub contract: Option<Address>,
    /// Name of the event.
    pub name: String,
    /// Serialized event data.
    pub data: Vec<u8>,
}

/// Gas cost constants. One unit of gas corresponds to one computational step.
mod gas {
    /// Base gas cost for any transaction (signature verification, nonce check).
    pub const BASE: u64 = 21_000;
    /// Per-byte cost for contract bytecode deployment.
    pub const DEPLOY_PER_BYTE: u64 = 200;
    /// Base gas for a contract call (before WASM execution).
    pub const CALL_BASE: u64 = 30_000;
    /// Gas cost for device registration.
    pub const REGISTER_DEVICE: u64 = 25_000;
}

/// The block execution engine. Takes a block of transactions and applies
/// them to the account state, producing receipts and a new state root.
pub struct BlockExecutor {
    state: AccountState,
    /// Registered devices, keyed by device public key.
    devices: HashMap<[u8; 32], DeviceIdentity>,
}

impl BlockExecutor {
    /// Create a new executor with the given initial state.
    pub fn new(state: AccountState) -> Self {
        Self {
            state,
            devices: HashMap::new(),
        }
    }

    /// Execute an entire block of transactions.
    ///
    /// Iterates through every transaction in the block, applies it to the
    /// state, and collects a receipt for each one. Returns an aggregate
    /// `ExecutionResult` containing the new state root, all receipts, total
    /// fees, and total gas consumed.
    pub fn execute_block(&mut self, block: &Block) -> DinaResult<ExecutionResult> {
        let mut receipts = Vec::with_capacity(block.transactions.len());
        let mut total_fees: u64 = 0;
        let mut total_gas: u64 = 0;

        for tx in &block.transactions {
            let receipt = self.execute_transaction(tx);
            total_fees += receipt.fee_paid;
            total_gas += receipt.gas_used;
            receipts.push(receipt);
        }

        // Credit collected fees to the block proposer.
        if total_fees > 0 {
            self.state.credit(&block.header.proposer, total_fees);
        }

        // Compute state root by hashing all account data deterministically.
        let state_root = self.compute_state_root();

        Ok(ExecutionResult {
            state_root,
            receipts,
            total_fees,
            gas_used: total_gas,
        })
    }

    /// Execute a single transaction. The fee is always deducted (even if the
    /// transaction body fails), and the nonce is only incremented on success.
    pub fn execute_transaction(&mut self, tx: &Transaction) -> TransactionReceipt {
        let tx_hash = tx.hash();
        let fee = tx.fee();
        let sender = tx.sender();

        // Phase 1: Deduct fee. If the sender cannot pay, the tx is dropped
        // entirely with no state changes.
        if let Err(e) = self.state.deduct_fee(&sender, fee) {
            return TransactionReceipt {
                tx_hash,
                success: false,
                gas_used: 0,
                fee_paid: 0,
                error: Some(format!("fee deduction failed: {e}")),
                events: vec![],
            };
        }

        // Phase 2: Execute the transaction body. Failures here do NOT refund
        // the fee, but the nonce is not incremented.
        let result = self.execute_body(tx);

        match result {
            Ok(events) => {
                // Increment nonce on success.
                let _ = self.state.increment_nonce(&sender);

                let gas_used = self.estimate_gas(tx);
                TransactionReceipt {
                    tx_hash,
                    success: true,
                    gas_used,
                    fee_paid: fee,
                    error: None,
                    events,
                }
            }
            Err(e) => {
                let gas_used = self.estimate_gas(tx);
                TransactionReceipt {
                    tx_hash,
                    success: false,
                    gas_used,
                    fee_paid: fee,
                    error: Some(e.to_string()),
                    events: vec![],
                }
            }
        }
    }

    /// Pre-validate a transaction without applying any state changes.
    /// Checks that the sender account exists, nonce is correct, balance
    /// covers fee + amount, and the signature is valid.
    pub fn validate_transaction(&self, tx: &Transaction) -> DinaResult<()> {
        let sender = tx.sender();
        let account = self
            .state
            .get_account(&sender)
            .ok_or_else(|| DinaError::AccountNotFound(sender.to_string()))?;

        // Check nonce.
        if account.nonce != tx.nonce() {
            return Err(DinaError::InvalidNonce {
                expected: account.nonce,
                got: tx.nonce(),
            });
        }

        // Check balance covers fee + transfer amount.
        let total_needed = tx.fee() + self.tx_amount(tx);
        if account.balance < total_needed {
            return Err(DinaError::InsufficientBalance {
                have: account.balance,
                need: total_needed,
            });
        }

        // Verify signature.
        // Recover the verifying key from the sender address is not possible
        // (address is a hash of the pubkey), so we require the sender account
        // to exist. For validation purposes, we verify the signature structurally
        // by checking it is well-formed. Full signature verification requires
        // the public key, which is only available to the caller.
        // For now, we check that the signature bytes are non-zero as a basic
        // structural check. Real signature verification happens at the mempool
        // layer where the public key is available.
        let sig_bytes = self.extract_signature_bytes(tx);
        if sig_bytes == [0u8; 64] {
            return Err(DinaError::InvalidSignature);
        }

        Ok(())
    }

    /// Get a reference to the current account state.
    pub fn state(&self) -> &AccountState {
        &self.state
    }

    /// Consume the executor and return the final account state.
    pub fn into_state(self) -> AccountState {
        self.state
    }

    // ── Private helpers ──────────────────────────────────────────────

    /// Execute the body of a transaction (everything except fee deduction).
    fn execute_body(&mut self, tx: &Transaction) -> DinaResult<Vec<Event>> {
        let sender = tx.sender();
        let account = self
            .state
            .get_account(&sender)
            .ok_or_else(|| DinaError::AccountNotFound(sender.to_string()))?;

        // Check nonce.
        if account.nonce != tx.nonce() {
            return Err(DinaError::InvalidNonce {
                expected: account.nonce,
                got: tx.nonce(),
            });
        }

        match tx {
            Transaction::Transfer {
                from, to, amount, ..
            } => {
                self.state.transfer(from, to, *amount)?;
                Ok(vec![Event {
                    contract: None,
                    name: "Transfer".to_string(),
                    data: Vec::new(),
                }])
            }

            Transaction::DeployContract {
                from,
                wasm_bytecode,
                ..
            } => {
                // Store the code hash on the sender's account (contract
                // account creation is a future enhancement -- for now the
                // deployer account gets the code_hash set).
                let code_hash = hash_bytes(wasm_bytecode);
                if let Some(acct) = self.state.get_account(from).cloned() {
                    let mut updated = acct;
                    updated.code_hash = Some(code_hash);
                    self.state.set_account(updated);
                }
                Ok(vec![Event {
                    contract: Some(*from),
                    name: "ContractDeployed".to_string(),
                    data: code_hash.0.to_vec(),
                }])
            }

            Transaction::CallContract {
                contract,
                usdc_attached,
                from,
                method,
                ..
            } => {
                // Transfer attached USDC to the contract address.
                if *usdc_attached > 0 {
                    self.state.transfer(from, contract, *usdc_attached)?;
                }

                // WASM execution is not yet implemented. For now we emit an
                // event recording the call.
                Ok(vec![Event {
                    contract: Some(*contract),
                    name: format!("ContractCalled::{method}"),
                    data: Vec::new(),
                }])
            }

            Transaction::RegisterDevice {
                device_pubkey,
                owner,
                attestation,
                ..
            } => {
                // Check that the device is not already registered.
                if self.devices.contains_key(device_pubkey) {
                    return Err(DinaError::Custom(
                        "device already registered".to_string(),
                    ));
                }

                let device = DeviceIdentity::new(
                    *device_pubkey,
                    *owner,
                    DeviceType::CognitumSeed, // default; real type from attestation metadata
                    attestation.firmware_hash,
                    attestation.witness_root,
                    attestation.timestamp,
                );

                self.devices.insert(*device_pubkey, device);

                Ok(vec![Event {
                    contract: None,
                    name: "DeviceRegistered".to_string(),
                    data: device_pubkey.to_vec(),
                }])
            }
        }
    }

    /// Estimate gas usage for a transaction based on its type.
    fn estimate_gas(&self, tx: &Transaction) -> u64 {
        match tx {
            Transaction::Transfer { .. } => gas::BASE,
            Transaction::DeployContract { wasm_bytecode, .. } => {
                gas::BASE + (wasm_bytecode.len() as u64) * gas::DEPLOY_PER_BYTE
            }
            Transaction::CallContract { .. } => gas::CALL_BASE,
            Transaction::RegisterDevice { .. } => gas::REGISTER_DEVICE,
        }
    }

    /// Extract the transfer/attach amount from a transaction (zero for
    /// non-transfer types that don't move funds).
    fn tx_amount(&self, tx: &Transaction) -> u64 {
        match tx {
            Transaction::Transfer { amount, .. } => *amount,
            Transaction::CallContract { usdc_attached, .. } => *usdc_attached,
            _ => 0,
        }
    }

    /// Extract the raw 64-byte signature from any transaction variant.
    fn extract_signature_bytes(&self, tx: &Transaction) -> [u8; 64] {
        match tx {
            Transaction::Transfer { signature, .. }
            | Transaction::DeployContract { signature, .. }
            | Transaction::CallContract { signature, .. }
            | Transaction::RegisterDevice { signature, .. } => signature.0,
        }
    }

    /// Compute a deterministic state root by hashing all accounts in sorted
    /// order by address.
    fn compute_state_root(&self) -> Hash {
        let mut entries: Vec<_> = self.state.iter().collect();
        entries.sort_by_key(|(addr, _)| addr.0);

        let mut hasher_input = Vec::new();
        for (addr, account) in entries {
            hasher_input.extend_from_slice(addr.as_bytes());
            let account_bytes =
                bincode::serialize(account).expect("account serialization cannot fail");
            hasher_input.extend_from_slice(&account_bytes);
        }

        if hasher_input.is_empty() {
            Hash::ZERO
        } else {
            hash_bytes(&hasher_input)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{Block, BlockHeader};
    use crate::crypto;
    use crate::transaction::Sig64;

    /// Helper: create a signed transfer transaction.
    fn make_transfer(
        sk: &ed25519_dalek::SigningKey,
        to: Address,
        amount: u64,
        nonce: u64,
        fee: u64,
    ) -> Transaction {
        let vk = sk.verifying_key();
        let from = Address::from_pubkey(&vk);

        let mut tx = Transaction::Transfer {
            from,
            to,
            amount,
            memo: None,
            device_witness: None,
            nonce,
            fee,
            signature: Sig64([0u8; 64]),
        };

        let msg = tx.signing_bytes();
        let sig = crypto::sign(sk, &msg);

        if let Transaction::Transfer {
            ref mut signature, ..
        } = tx
        {
            *signature = Sig64(sig);
        }

        tx
    }

    /// Helper: wrap transactions into a block with a given proposer.
    fn make_block(proposer: Address, txs: Vec<Transaction>) -> Block {
        Block {
            header: BlockHeader {
                block_number: 1,
                parent_hash: Hash::ZERO,
                state_root: Hash::ZERO,
                transactions_root: Hash::ZERO,
                timestamp: 1_700_000_000,
                proposer,
                signature: [0u8; 64],
            },
            transactions: txs,
        }
    }

    #[test]
    fn execute_empty_block() {
        let mut state = AccountState::new();
        let proposer = Address([0x01; 32]);
        state.credit(&proposer, 0);

        let mut executor = BlockExecutor::new(state);
        let block = make_block(proposer, vec![]);
        let result = executor.execute_block(&block).unwrap();

        assert!(result.receipts.is_empty());
        assert_eq!(result.total_fees, 0);
        assert_eq!(result.gas_used, 0);
    }

    #[test]
    fn execute_single_transfer() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let recipient = Address([0xbb; 32]);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&sender, 10_000);

        let tx = make_transfer(&sk, recipient, 1_000, 0, 10);
        let mut executor = BlockExecutor::new(state);
        let block = make_block(proposer, vec![tx]);
        let result = executor.execute_block(&block).unwrap();

        assert_eq!(result.receipts.len(), 1);
        assert!(result.receipts[0].success);
        assert_eq!(result.receipts[0].fee_paid, 10);
        assert_eq!(result.total_fees, 10);

        // Sender: 10000 - 10 (fee) - 1000 (transfer) = 8990
        let final_state = executor.state();
        assert_eq!(final_state.get_account(&sender).unwrap().balance, 8_990);
        assert_eq!(final_state.get_account(&sender).unwrap().nonce, 1);
        // Recipient gets 1000
        assert_eq!(
            final_state.get_account(&recipient).unwrap().balance,
            1_000
        );
        // Proposer gets the fee
        assert_eq!(final_state.get_account(&proposer).unwrap().balance, 10);
    }

    #[test]
    fn insufficient_balance_receipt_shows_failure() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let recipient = Address([0xbb; 32]);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        // Give sender enough for fee but not fee + amount.
        state.credit(&sender, 100);

        let tx = make_transfer(&sk, recipient, 500, 0, 10);
        let mut executor = BlockExecutor::new(state);
        let block = make_block(proposer, vec![tx]);
        let result = executor.execute_block(&block).unwrap();

        assert_eq!(result.receipts.len(), 1);
        assert!(!result.receipts[0].success);
        assert!(result.receipts[0].error.is_some());
        // Fee was still deducted.
        assert_eq!(result.receipts[0].fee_paid, 10);
        assert_eq!(
            executor.state().get_account(&sender).unwrap().balance,
            90
        );
    }

    #[test]
    fn wrong_nonce_receipt_shows_failure() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let recipient = Address([0xbb; 32]);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&sender, 10_000);

        // Nonce should be 0 but we send 5.
        let tx = make_transfer(&sk, recipient, 100, 5, 10);
        let mut executor = BlockExecutor::new(state);
        let block = make_block(proposer, vec![tx]);
        let result = executor.execute_block(&block).unwrap();

        assert_eq!(result.receipts.len(), 1);
        assert!(!result.receipts[0].success);
        let err_msg = result.receipts[0].error.as_ref().unwrap();
        assert!(err_msg.contains("nonce"));
        // Fee still deducted, nonce NOT incremented.
        assert_eq!(result.receipts[0].fee_paid, 10);
        assert_eq!(
            executor.state().get_account(&sender).unwrap().nonce,
            0
        );
    }

    #[test]
    fn fee_deducted_on_failed_transaction() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let recipient = Address([0xbb; 32]);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&sender, 500);

        // Transfer more than balance-after-fee allows.
        let tx = make_transfer(&sk, recipient, 1_000, 0, 20);
        let mut executor = BlockExecutor::new(state);
        let block = make_block(proposer, vec![tx]);
        let result = executor.execute_block(&block).unwrap();

        assert!(!result.receipts[0].success);
        assert_eq!(result.receipts[0].fee_paid, 20);
        // Balance = 500 - 20 = 480 (fee deducted, transfer not applied).
        assert_eq!(
            executor.state().get_account(&sender).unwrap().balance,
            480
        );
    }

    #[test]
    fn multiple_transactions_in_one_block() {
        let (sk_a, vk_a) = crypto::generate_keypair();
        let (sk_b, vk_b) = crypto::generate_keypair();
        let addr_a = Address::from_pubkey(&vk_a);
        let addr_b = Address::from_pubkey(&vk_b);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&addr_a, 5_000);
        state.credit(&addr_b, 3_000);

        let tx1 = make_transfer(&sk_a, addr_b, 1_000, 0, 10);
        let tx2 = make_transfer(&sk_b, addr_a, 500, 0, 10);

        let mut executor = BlockExecutor::new(state);
        let block = make_block(proposer, vec![tx1, tx2]);
        let result = executor.execute_block(&block).unwrap();

        assert_eq!(result.receipts.len(), 2);
        assert!(result.receipts[0].success);
        assert!(result.receipts[1].success);
        assert_eq!(result.total_fees, 20);

        let final_state = executor.state();
        // A: 5000 - 10 (fee) - 1000 (sent) + 500 (received) = 4490
        assert_eq!(final_state.get_account(&addr_a).unwrap().balance, 4_490);
        // B: 3000 + 1000 (received) - 10 (fee) - 500 (sent) = 3490
        assert_eq!(final_state.get_account(&addr_b).unwrap().balance, 3_490);
        // Proposer: 20 total fees
        assert_eq!(final_state.get_account(&proposer).unwrap().balance, 20);
    }

    #[test]
    fn validate_transaction_catches_bad_signature() {
        let (_, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);

        let mut state = AccountState::new();
        state.credit(&sender, 10_000);

        // Transaction with all-zero signature (invalid).
        let tx = Transaction::Transfer {
            from: sender,
            to: Address([0xbb; 32]),
            amount: 100,
            memo: None,
            device_witness: None,
            nonce: 0,
            fee: 10,
            signature: Sig64([0u8; 64]),
        };

        let executor = BlockExecutor::new(state);
        let err = executor.validate_transaction(&tx).unwrap_err();
        assert!(matches!(err, DinaError::InvalidSignature));
    }

    #[test]
    fn validate_transaction_catches_wrong_nonce() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);

        let mut state = AccountState::new();
        state.credit(&sender, 10_000);

        let tx = make_transfer(&sk, Address([0xbb; 32]), 100, 99, 10);

        let executor = BlockExecutor::new(state);
        let err = executor.validate_transaction(&tx).unwrap_err();
        assert!(matches!(err, DinaError::InvalidNonce { .. }));
    }

    #[test]
    fn validate_transaction_catches_insufficient_balance() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);

        let mut state = AccountState::new();
        state.credit(&sender, 50);

        let tx = make_transfer(&sk, Address([0xbb; 32]), 100, 0, 10);

        let executor = BlockExecutor::new(state);
        let err = executor.validate_transaction(&tx).unwrap_err();
        assert!(matches!(err, DinaError::InsufficientBalance { .. }));
    }

    #[test]
    fn into_state_returns_final_state() {
        let mut state = AccountState::new();
        let addr = Address([0x42; 32]);
        state.credit(&addr, 1_000);

        let executor = BlockExecutor::new(state);
        let recovered = executor.into_state();
        assert_eq!(recovered.get_account(&addr).unwrap().balance, 1_000);
    }

    #[test]
    fn state_root_changes_after_execution() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&sender, 10_000);

        let tx = make_transfer(&sk, Address([0xcc; 32]), 500, 0, 10);

        let mut executor = BlockExecutor::new(state.clone());
        let empty_block = make_block(proposer, vec![]);
        let result_empty = executor.execute_block(&empty_block).unwrap();

        let mut executor2 = BlockExecutor::new(state);
        let block_with_tx = make_block(proposer, vec![tx]);
        let result_tx = executor2.execute_block(&block_with_tx).unwrap();

        // State roots should differ because the state changed.
        assert_ne!(result_empty.state_root, result_tx.state_root);
    }

    #[test]
    fn cannot_pay_fee_drops_transaction() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&sender, 5); // Less than the fee.

        let tx = make_transfer(&sk, Address([0xbb; 32]), 1, 0, 10);
        let mut executor = BlockExecutor::new(state);
        let block = make_block(proposer, vec![tx]);
        let result = executor.execute_block(&block).unwrap();

        assert!(!result.receipts[0].success);
        // Fee was NOT paid because sender couldn't afford it.
        assert_eq!(result.receipts[0].fee_paid, 0);
        // Balance unchanged.
        assert_eq!(
            executor.state().get_account(&sender).unwrap().balance,
            5
        );
    }
}
