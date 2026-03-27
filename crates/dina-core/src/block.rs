use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::crypto::hash_bytes;
use crate::merkle::MerkleTree;
use crate::transaction::Transaction;
use crate::types::{Address, Hash};

/// Header of a block in the Dina blockchain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Sequential block number (height).
    pub block_number: u64,
    /// Hash of the parent block's header.
    pub parent_hash: Hash,
    /// Merkle root of the world state after applying this block.
    pub state_root: Hash,
    /// Merkle root of the transactions in this block.
    pub transactions_root: Hash,
    /// Unix timestamp when the block was proposed.
    pub timestamp: u64,
    /// Address of the block proposer/validator.
    pub proposer: Address,
    /// Ed25519 signature of the block header hash by the proposer.
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
}

impl BlockHeader {
    /// Compute the SHA-256 hash of this block header (all fields except the signature).
    pub fn hash(&self) -> Hash {
        let payload = HeaderPayload {
            block_number: self.block_number,
            parent_hash: &self.parent_hash,
            state_root: &self.state_root,
            transactions_root: &self.transactions_root,
            timestamp: self.timestamp,
            proposer: &self.proposer,
        };
        let bytes = bincode::serialize(&payload).expect("block header serialization cannot fail");
        hash_bytes(&bytes)
    }

    /// Verify the block header signature against the proposer's public key.
    pub fn verify(&self, verifying_key: &ed25519_dalek::VerifyingKey) -> bool {
        use ed25519_dalek::{Signature, Verifier};
        let hash = self.hash();
        let sig = Signature::from_bytes(&self.signature);
        verifying_key.verify(hash.as_bytes(), &sig).is_ok()
    }
}

#[derive(Serialize)]
struct HeaderPayload<'a> {
    block_number: u64,
    parent_hash: &'a Hash,
    state_root: &'a Hash,
    transactions_root: &'a Hash,
    timestamp: u64,
    proposer: &'a Address,
}

/// A full block in the Dina blockchain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    /// Compute the hash of this block (delegates to header).
    pub fn hash(&self) -> Hash {
        self.header.hash()
    }

    /// Verify the block header signature.
    pub fn verify(&self, verifying_key: &ed25519_dalek::VerifyingKey) -> bool {
        self.header.verify(verifying_key)
    }

    /// Return the number of transactions in this block.
    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }

    /// Compute the Merkle root of the block's transactions.
    pub fn compute_transactions_root(&self) -> Hash {
        if self.transactions.is_empty() {
            return Hash::ZERO;
        }
        let mut tree = MerkleTree::new();
        for tx in &self.transactions {
            let tx_hash = tx.hash();
            tree.insert(tx_hash.as_bytes());
        }
        tree.root()
    }

    /// Create the genesis block with no transactions and zero hashes.
    pub fn genesis(proposer: Address, timestamp: u64) -> Self {
        let header = BlockHeader {
            block_number: 0,
            parent_hash: Hash::ZERO,
            state_root: Hash::ZERO,
            transactions_root: Hash::ZERO,
            timestamp,
            proposer,
            signature: [0u8; 64],
        };
        Block {
            header,
            transactions: Vec::new(),
        }
    }

    /// Create a genesis block signed by the given key.
    pub fn signed_genesis(
        signing_key: &ed25519_dalek::SigningKey,
        timestamp: u64,
    ) -> Self {
        use crate::crypto;

        let verifying_key = signing_key.verifying_key();
        let proposer = Address::from_pubkey(&verifying_key);

        let mut block = Self::genesis(proposer, timestamp);
        let header_hash = block.header.hash();
        block.header.signature = crypto::sign(signing_key, header_hash.as_bytes());
        block
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto;

    #[test]
    fn genesis_block() {
        let genesis = Block::genesis(Address::ZERO, 1_700_000_000);
        assert_eq!(genesis.header.block_number, 0);
        assert_eq!(genesis.header.parent_hash, Hash::ZERO);
        assert_eq!(genesis.transaction_count(), 0);
    }

    #[test]
    fn signed_genesis_verifies() {
        let (sk, vk) = crypto::generate_keypair();
        let genesis = Block::signed_genesis(&sk, 1_700_000_000);
        assert!(genesis.verify(&vk));
        assert_eq!(genesis.header.block_number, 0);
    }

    #[test]
    fn hash_is_deterministic() {
        let genesis = Block::genesis(Address::ZERO, 1_700_000_000);
        assert_eq!(genesis.hash(), genesis.hash());
    }

    #[test]
    fn compute_transactions_root_empty() {
        let genesis = Block::genesis(Address::ZERO, 0);
        assert_eq!(genesis.compute_transactions_root(), Hash::ZERO);
    }
}
