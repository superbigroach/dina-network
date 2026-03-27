//! Unified chain state for the Dina node.
//!
//! [`ChainState`] combines the [`ChainManager`] (block storage), the
//! [`AccountState`] (world state), and a pending transaction pool into a
//! single structure that the node binary uses to process blocks.

use dina_core::account::AccountState;
use dina_core::block::Block;
use dina_core::chain::ChainManager;
use dina_core::error::DinaResult;
use dina_core::transaction::Transaction;
use dina_core::types::Address;

/// The result of executing all transactions in a block.
#[derive(Debug)]
pub struct ExecutionResult {
    /// Number of transactions that were successfully executed.
    pub successful_txs: usize,
    /// Number of transactions that failed (but did not halt the block).
    pub failed_txs: usize,
    /// Total fees collected from executed transactions.
    pub total_fees: u64,
}

/// Unified chain state managed by the node.
///
/// This struct owns the canonical chain, the account world-state, and a
/// buffer of pending transactions waiting to be included in a block.
pub struct ChainState {
    /// The canonical chain of blocks.
    pub chain: ChainManager,
    /// In-memory account state (balances, nonces, contract metadata).
    pub accounts: AccountState,
    /// Transactions waiting to be included in the next block.
    #[allow(dead_code)]
    pub pending_txs: Vec<Transaction>,
}

impl ChainState {
    /// Create a new chain state initialized with the given genesis block.
    pub fn new(genesis: Block, chain_id: String) -> Self {
        ChainState {
            chain: ChainManager::new(genesis, chain_id),
            accounts: AccountState::new(),
            pending_txs: Vec::new(),
        }
    }

    /// Validate, execute, and append a block to the chain.
    ///
    /// Each transaction in the block is executed against the current account
    /// state. Transfer transactions move funds between accounts. All
    /// transaction fees are deducted from senders. Failed transactions are
    /// counted but do not prevent the block from being applied.
    ///
    /// Returns an [`ExecutionResult`] summarizing the outcome.
    pub fn apply_block(&mut self, block: Block) -> DinaResult<ExecutionResult> {
        // Validate the block against the current chain tip first
        self.chain.is_valid_next_block(&block)?;

        let mut successful_txs = 0;
        let mut failed_txs = 0;
        let mut total_fees = 0u64;

        for tx in &block.transactions {
            match self.execute_transaction(tx) {
                Ok(fee) => {
                    successful_txs += 1;
                    total_fees += fee;
                }
                Err(_) => {
                    failed_txs += 1;
                }
            }
        }

        // Append the block to the chain (we already validated above)
        self.chain.add_block(block)?;

        Ok(ExecutionResult {
            successful_txs,
            failed_txs,
            total_fees,
        })
    }

    /// Execute a single transaction against the account state.
    ///
    /// Returns the fee paid on success.
    fn execute_transaction(&mut self, tx: &Transaction) -> DinaResult<u64> {
        let sender = tx.sender();
        let fee = tx.fee();

        // Faucet/coinbase transactions (from zero address) skip fee and nonce
        let is_coinbase = sender == Address([0u8; 32]);

        if !is_coinbase {
            // Deduct the fee from the sender
            self.accounts.deduct_fee(&sender, fee)?;

            // Increment the sender's nonce
            self.accounts.increment_nonce(&sender)?;
        }

        // Execute the transaction-specific logic
        match tx {
            Transaction::Transfer {
                from, to, amount, ..
            } => {
                if *from == Address([0u8; 32]) {
                    // Coinbase/faucet: mint (credit without debit)
                    self.accounts.credit(to, *amount);
                } else {
                    self.accounts.transfer(from, to, *amount)?;
                }
            }
            Transaction::DeployContract { .. } => {
                // Contract deployment is handled by the WASM runtime;
                // the fee has already been deducted above.
            }
            Transaction::CallContract {
                from,
                contract,
                usdc_attached,
                ..
            } => {
                if *usdc_attached > 0 {
                    self.accounts.transfer(from, contract, *usdc_attached)?;
                }
            }
            Transaction::RegisterDevice { .. } => {
                // Device registration is handled by the device registry;
                // the fee has already been deducted above.
            }
        }

        Ok(fee)
    }

    /// Return the current chain height.
    pub fn current_height(&self) -> u64 {
        self.chain.current_height()
    }

    /// Return a reference to the latest block.
    pub fn latest_block(&self) -> &Block {
        self.chain.latest_block()
    }

    /// Look up an account by address.
    #[allow(dead_code)]
    pub fn get_account(&self, addr: &Address) -> Option<&dina_core::Account> {
        self.accounts.get_account(addr)
    }

    /// Get the balance of an account. Returns 0 if the account does not exist.
    #[allow(dead_code)]
    pub fn get_balance(&self, addr: &Address) -> u64 {
        self.accounts
            .get_account(addr)
            .map(|a| a.balance)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dina_core::block::BlockHeader;
    use dina_core::transaction::Sig64;
    use dina_core::types::{Address, Hash};

    fn make_genesis() -> Block {
        Block::genesis(Address::ZERO, 1_000)
    }

    fn make_child(parent: &Block, timestamp: u64) -> Block {
        Block {
            header: BlockHeader {
                block_number: parent.header.block_number + 1,
                parent_hash: parent.hash(),
                state_root: Hash::ZERO,
                transactions_root: Hash::ZERO,
                timestamp,
                proposer: Address::ZERO,
                signature: [0u8; 64],
            },
            transactions: Vec::new(),
        }
    }

    fn addr(byte: u8) -> Address {
        Address([byte; 32])
    }

    #[test]
    fn new_chain_state() {
        let genesis = make_genesis();
        let state = ChainState::new(genesis, "test".to_string());
        assert_eq!(state.current_height(), 0);
    }

    #[test]
    fn apply_empty_block() {
        let genesis = make_genesis();
        let mut state = ChainState::new(genesis.clone(), "test".to_string());

        let block1 = make_child(&genesis, 2_000);
        let result = state.apply_block(block1).unwrap();

        assert_eq!(result.successful_txs, 0);
        assert_eq!(result.failed_txs, 0);
        assert_eq!(result.total_fees, 0);
        assert_eq!(state.current_height(), 1);
    }

    #[test]
    fn apply_block_with_transfer() {
        let genesis = make_genesis();
        let mut state = ChainState::new(genesis.clone(), "test".to_string());

        let from = addr(1);
        let to = addr(2);

        // Credit sender so the transfer can succeed
        state.accounts.credit(&from, 10_000);

        let tx = Transaction::Transfer {
            from,
            to,
            amount: 500,
            memo: None,
            device_witness: None,
            nonce: 0,
            fee: 10,
            signature: Sig64([0u8; 64]),
        };

        let block1 = Block {
            header: BlockHeader {
                block_number: 1,
                parent_hash: genesis.hash(),
                state_root: Hash::ZERO,
                transactions_root: Hash::ZERO,
                timestamp: 2_000,
                proposer: Address::ZERO,
                signature: [0u8; 64],
            },
            transactions: vec![tx],
        };

        let result = state.apply_block(block1).unwrap();
        assert_eq!(result.successful_txs, 1);
        assert_eq!(result.total_fees, 10);

        // Sender: 10000 - 10 (fee) - 500 (transfer) = 9490
        assert_eq!(state.get_balance(&from), 9_490);
        // Receiver: 500
        assert_eq!(state.get_balance(&to), 500);
    }

    #[test]
    fn get_balance_returns_zero_for_unknown() {
        let genesis = make_genesis();
        let state = ChainState::new(genesis, "test".to_string());
        assert_eq!(state.get_balance(&addr(99)), 0);
    }

    #[test]
    fn apply_invalid_block_is_rejected() {
        let genesis = make_genesis();
        let mut state = ChainState::new(genesis.clone(), "test".to_string());

        // Block with wrong height
        let bad_block = Block {
            header: BlockHeader {
                block_number: 5,
                parent_hash: genesis.hash(),
                state_root: Hash::ZERO,
                transactions_root: Hash::ZERO,
                timestamp: 2_000,
                proposer: Address::ZERO,
                signature: [0u8; 64],
            },
            transactions: Vec::new(),
        };

        assert!(state.apply_block(bad_block).is_err());
        assert_eq!(state.current_height(), 0);
    }
}
