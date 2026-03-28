//! Chain management for the Dina blockchain.
//!
//! [`ChainManager`] tracks the canonical chain as an ordered sequence of
//! blocks. It validates new blocks against the current tip before appending
//! them and provides fast lookup by height or hash.

use std::collections::HashMap;

use crate::block::Block;
use crate::error::{DinaError, DinaResult};
use crate::types::Hash;

/// Manages the canonical chain of blocks.
///
/// Blocks are stored in-memory in a `Vec` ordered by height, with a secondary
/// hash-to-height index for O(1) lookups by block hash.
pub struct ChainManager {
    /// Blocks ordered by height (index == height).
    blocks: Vec<Block>,
    /// Map from block hash to its height for fast lookup.
    block_index: HashMap<Hash, u64>,
    /// Current chain height (equal to the latest block's block_number).
    current_height: u64,
    /// Hash of the genesis block.
    genesis_hash: Hash,
    /// Identifier for this chain (e.g. "dina-testnet-1").
    chain_id: String,
}

impl ChainManager {
    /// Create a new chain manager initialized with the given genesis block.
    ///
    /// The genesis block must have `block_number == 0`.
    pub fn new(genesis: Block, chain_id: String) -> Self {
        let genesis_hash = genesis.hash();
        let mut block_index = HashMap::new();
        block_index.insert(genesis_hash, 0);

        ChainManager {
            blocks: vec![genesis],
            block_index,
            current_height: 0,
            genesis_hash,
            chain_id,
        }
    }

    /// Validate and append a block to the chain.
    ///
    /// The block must satisfy all of the following:
    /// - Its `parent_hash` matches the hash of the current tip.
    /// - Its `block_number` is exactly `current_height + 1`.
    /// - Its `timestamp` is strictly greater than the previous block's timestamp.
    pub fn add_block(&mut self, block: Block) -> DinaResult<()> {
        self.is_valid_next_block(&block)?;

        let hash = block.hash();
        let height = block.header.block_number;
        self.block_index.insert(hash, height);
        self.blocks.push(block);
        self.current_height = height;
        Ok(())
    }

    /// Get a block by its height. Returns `None` if the height exceeds the chain.
    pub fn get_block(&self, height: u64) -> Option<&Block> {
        self.blocks.get(height as usize)
    }

    /// Get a block by its hash. Returns `None` if no block with that hash exists.
    pub fn get_block_by_hash(&self, hash: &Hash) -> Option<&Block> {
        let height = self.block_index.get(hash)?;
        self.blocks.get(*height as usize)
    }

    /// Return a reference to the latest (tip) block.
    ///
    /// This always succeeds because the chain is initialized with a genesis block.
    pub fn latest_block(&self) -> &Block {
        self.blocks
            .last()
            .expect("chain always has at least the genesis block")
    }

    /// Return the current chain height (the block number of the tip).
    pub fn current_height(&self) -> u64 {
        self.current_height
    }

    /// Return the hash of the genesis block.
    pub fn genesis_hash(&self) -> &Hash {
        &self.genesis_hash
    }

    /// Return the chain identifier.
    pub fn chain_id(&self) -> &str {
        &self.chain_id
    }

    /// Validate that a block is a valid successor to the current chain tip
    /// without mutating the chain.
    ///
    /// Returns `Ok(())` if valid, or a `DinaError::ConsensusError` describing
    /// why the block is invalid.
    pub fn is_valid_next_block(&self, block: &Block) -> DinaResult<()> {
        let latest = self.latest_block();
        let expected_height = self.current_height + 1;

        // Height must be exactly current + 1
        if block.header.block_number != expected_height {
            return Err(DinaError::ConsensusError(format!(
                "invalid block height: expected {}, got {}",
                expected_height, block.header.block_number
            )));
        }

        // Parent hash must match current tip
        let latest_hash = latest.hash();
        if block.header.parent_hash != latest_hash {
            return Err(DinaError::ConsensusError(format!(
                "invalid parent hash: expected {}, got {}",
                latest_hash, block.header.parent_hash
            )));
        }

        // Timestamp must be strictly increasing
        if block.header.timestamp <= latest.header.timestamp {
            return Err(DinaError::ConsensusError(format!(
                "block timestamp {} must be greater than previous block timestamp {}",
                block.header.timestamp, latest.header.timestamp
            )));
        }

        Ok(())
    }

    /// Return all blocks from the given height to the tip (inclusive).
    ///
    /// If `height` exceeds the current chain height, an empty slice is returned.
    pub fn blocks_since(&self, height: u64) -> &[Block] {
        let start = height as usize;
        if start >= self.blocks.len() {
            return &[];
        }
        &self.blocks[start..]
    }

    /// Check whether a block with the given hash exists in the chain.
    pub fn contains_block(&self, hash: &Hash) -> bool {
        self.block_index.contains_key(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Address;

    /// Helper: create a genesis block with the given timestamp.
    fn make_genesis(timestamp: u64) -> Block {
        Block::genesis(Address::ZERO, timestamp)
    }

    /// Helper: create a block that is a valid child of `parent`.
    fn make_child(parent: &Block, timestamp: u64) -> Block {
        let header = crate::block::BlockHeader {
            block_number: parent.header.block_number + 1,
            parent_hash: parent.hash(),
            state_root: Hash::ZERO,
            transactions_root: Hash::ZERO,
            timestamp,
            proposer: Address::ZERO,
            proposer_pubkey: [0u8; 32],
            signature: [0u8; 64],
        };
        Block {
            header,
            transactions: Vec::new(),
        }
    }

    #[test]
    fn new_chain_with_genesis() {
        let genesis = make_genesis(1_000);
        let chain = ChainManager::new(genesis.clone(), "test-chain".to_string());

        assert_eq!(chain.current_height(), 0);
        assert_eq!(chain.chain_id(), "test-chain");
        assert!(chain.contains_block(&genesis.hash()));
        assert_eq!(chain.latest_block().header.block_number, 0);
        assert_eq!(*chain.genesis_hash(), genesis.hash());
    }

    #[test]
    fn add_valid_block() {
        let genesis = make_genesis(1_000);
        let mut chain = ChainManager::new(genesis.clone(), "test".to_string());

        let block1 = make_child(&genesis, 2_000);
        chain.add_block(block1.clone()).unwrap();

        assert_eq!(chain.current_height(), 1);
        assert!(chain.contains_block(&block1.hash()));
        assert_eq!(chain.get_block(1).unwrap().hash(), block1.hash());
        assert_eq!(chain.latest_block().hash(), block1.hash());

        // Add a second block
        let block2 = make_child(&block1, 3_000);
        chain.add_block(block2.clone()).unwrap();

        assert_eq!(chain.current_height(), 2);
        assert_eq!(
            chain.get_block_by_hash(&block2.hash()).unwrap().hash(),
            block2.hash()
        );
    }

    #[test]
    fn reject_invalid_parent_hash() {
        let genesis = make_genesis(1_000);
        let mut chain = ChainManager::new(genesis, "test".to_string());

        // Block with wrong parent hash
        let bad_block = Block {
            header: crate::block::BlockHeader {
                block_number: 1,
                parent_hash: Hash([0xff; 32]), // wrong
                state_root: Hash::ZERO,
                transactions_root: Hash::ZERO,
                timestamp: 2_000,
                proposer: Address::ZERO,
                proposer_pubkey: [0u8; 32],
                signature: [0u8; 64],
            },
            transactions: Vec::new(),
        };

        let err = chain.add_block(bad_block).unwrap_err();
        match err {
            DinaError::ConsensusError(msg) => assert!(msg.contains("invalid parent hash")),
            other => panic!("expected ConsensusError, got: {other}"),
        }
    }

    #[test]
    fn reject_wrong_height() {
        let genesis = make_genesis(1_000);
        let mut chain = ChainManager::new(genesis.clone(), "test".to_string());

        // Block with height 5 instead of 1
        let bad_block = Block {
            header: crate::block::BlockHeader {
                block_number: 5,
                parent_hash: genesis.hash(),
                state_root: Hash::ZERO,
                transactions_root: Hash::ZERO,
                timestamp: 2_000,
                proposer: Address::ZERO,
                proposer_pubkey: [0u8; 32],
                signature: [0u8; 64],
            },
            transactions: Vec::new(),
        };

        let err = chain.add_block(bad_block).unwrap_err();
        match err {
            DinaError::ConsensusError(msg) => assert!(msg.contains("invalid block height")),
            other => panic!("expected ConsensusError, got: {other}"),
        }
    }

    #[test]
    fn reject_old_timestamp() {
        let genesis = make_genesis(5_000);
        let mut chain = ChainManager::new(genesis.clone(), "test".to_string());

        // Block with timestamp <= genesis timestamp
        let bad_block = Block {
            header: crate::block::BlockHeader {
                block_number: 1,
                parent_hash: genesis.hash(),
                state_root: Hash::ZERO,
                transactions_root: Hash::ZERO,
                timestamp: 4_000, // older than genesis
                proposer: Address::ZERO,
                proposer_pubkey: [0u8; 32],
                signature: [0u8; 64],
            },
            transactions: Vec::new(),
        };

        let err = chain.add_block(bad_block).unwrap_err();
        match err {
            DinaError::ConsensusError(msg) => assert!(msg.contains("timestamp")),
            other => panic!("expected ConsensusError, got: {other}"),
        }

        // Also reject equal timestamp
        let equal_ts_block = Block {
            header: crate::block::BlockHeader {
                block_number: 1,
                parent_hash: genesis.hash(),
                state_root: Hash::ZERO,
                transactions_root: Hash::ZERO,
                timestamp: 5_000, // equal to genesis
                proposer: Address::ZERO,
                proposer_pubkey: [0u8; 32],
                signature: [0u8; 64],
            },
            transactions: Vec::new(),
        };

        assert!(chain.add_block(equal_ts_block).is_err());
    }

    #[test]
    fn blocks_since_returns_slice() {
        let genesis = make_genesis(1_000);
        let mut chain = ChainManager::new(genesis.clone(), "test".to_string());

        let block1 = make_child(&genesis, 2_000);
        chain.add_block(block1.clone()).unwrap();

        let block2 = make_child(&block1, 3_000);
        chain.add_block(block2.clone()).unwrap();

        // From height 0: all blocks
        assert_eq!(chain.blocks_since(0).len(), 3);
        // From height 1: blocks 1 and 2
        assert_eq!(chain.blocks_since(1).len(), 2);
        // From height 2: only block 2
        assert_eq!(chain.blocks_since(2).len(), 1);
        // Beyond chain: empty
        assert_eq!(chain.blocks_since(10).len(), 0);
    }

    #[test]
    fn get_block_by_hash_returns_none_for_unknown() {
        let genesis = make_genesis(1_000);
        let chain = ChainManager::new(genesis, "test".to_string());
        assert!(chain.get_block_by_hash(&Hash([0xff; 32])).is_none());
    }

    #[test]
    fn is_valid_next_block_does_not_mutate() {
        let genesis = make_genesis(1_000);
        let chain = ChainManager::new(genesis.clone(), "test".to_string());

        let block1 = make_child(&genesis, 2_000);
        // Calling is_valid_next_block should not change the chain
        chain.is_valid_next_block(&block1).unwrap();
        assert_eq!(chain.current_height(), 0);
    }
}
