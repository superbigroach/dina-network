use rs_merkle::{
    algorithms::Sha256 as MerkleSha256, Hasher, MerkleProof, MerkleTree as RsMerkleTree,
};

use crate::types::Hash;

/// Wrapper around `rs_merkle::MerkleTree` for computing state roots and transaction roots.
#[derive(Clone, Debug)]
pub struct MerkleTree {
    leaves: Vec<[u8; 32]>,
}

impl MerkleTree {
    /// Create a new empty Merkle tree.
    pub fn new() -> Self {
        Self { leaves: Vec::new() }
    }

    /// Insert a leaf (32-byte hash) into the tree.
    pub fn insert(&mut self, data: &[u8; 32]) {
        let leaf_hash = MerkleSha256::hash(data);
        self.leaves.push(leaf_hash);
    }

    /// Compute and return the Merkle root. Returns `Hash::ZERO` if the tree is empty.
    pub fn root(&self) -> Hash {
        if self.leaves.is_empty() {
            return Hash::ZERO;
        }

        let tree = RsMerkleTree::<MerkleSha256>::from_leaves(&self.leaves);
        match tree.root() {
            Some(root_bytes) => {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&root_bytes);
                Hash(hash)
            }
            None => Hash::ZERO,
        }
    }

    /// Generate a Merkle proof for the leaf at the given index.
    pub fn proof(&self, index: usize) -> Option<Vec<u8>> {
        if index >= self.leaves.len() {
            return None;
        }
        let tree = RsMerkleTree::<MerkleSha256>::from_leaves(&self.leaves);
        let proof = tree.proof(&[index]);
        Some(proof.to_bytes())
    }

    /// Verify a Merkle proof for a given leaf at a given index against a known root.
    pub fn verify_proof(
        root: &Hash,
        leaf_data: &[u8; 32],
        index: usize,
        total_leaves: usize,
        proof_bytes: &[u8],
    ) -> bool {
        let leaf_hash = MerkleSha256::hash(leaf_data);
        let Ok(proof) = MerkleProof::<MerkleSha256>::from_bytes(proof_bytes) else {
            return false;
        };
        proof.verify(root.0, &[index], &[leaf_hash], total_leaves)
    }

    /// Return the number of leaves in the tree.
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Check if the tree has no leaves.
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function: compute a Merkle root from a slice of byte slices.
pub fn compute_merkle_root(items: &[&[u8]]) -> Hash {
    if items.is_empty() {
        return Hash::ZERO;
    }

    let leaves: Vec<[u8; 32]> = items.iter().map(|data| MerkleSha256::hash(data)).collect();
    let tree = RsMerkleTree::<MerkleSha256>::from_leaves(&leaves);

    match tree.root() {
        Some(root) => {
            let mut bytes = [0u8; 32];
            bytes.copy_from_slice(&root);
            Hash(bytes)
        }
        None => Hash::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tree_returns_zero() {
        let tree = MerkleTree::new();
        assert_eq!(tree.root(), Hash::ZERO);
    }

    #[test]
    fn single_leaf() {
        let mut tree = MerkleTree::new();
        tree.insert(&[0xaa; 32]);
        let root = tree.root();
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn deterministic_root() {
        let mut t1 = MerkleTree::new();
        let mut t2 = MerkleTree::new();
        t1.insert(&[1u8; 32]);
        t1.insert(&[2u8; 32]);
        t2.insert(&[1u8; 32]);
        t2.insert(&[2u8; 32]);
        assert_eq!(t1.root(), t2.root());
    }

    #[test]
    fn different_leaves_different_root() {
        let mut t1 = MerkleTree::new();
        let mut t2 = MerkleTree::new();
        t1.insert(&[1u8; 32]);
        t2.insert(&[2u8; 32]);
        assert_ne!(t1.root(), t2.root());
    }

    #[test]
    fn proof_and_verify() {
        let mut tree = MerkleTree::new();
        let leaf0 = [0x11; 32];
        let leaf1 = [0x22; 32];
        let leaf2 = [0x33; 32];
        tree.insert(&leaf0);
        tree.insert(&leaf1);
        tree.insert(&leaf2);

        let root = tree.root();
        let proof_bytes = tree.proof(1).expect("proof should exist");
        assert!(MerkleTree::verify_proof(&root, &leaf1, 1, 3, &proof_bytes));
        // Wrong leaf should fail verification
        assert!(!MerkleTree::verify_proof(&root, &leaf0, 1, 3, &proof_bytes));
    }

    #[test]
    fn compute_merkle_root_empty() {
        assert_eq!(compute_merkle_root(&[]), Hash::ZERO);
    }

    #[test]
    fn compute_merkle_root_non_empty() {
        let data: Vec<&[u8]> = vec![&[1u8; 32], &[2u8; 32]];
        let root = compute_merkle_root(&data);
        assert_ne!(root, Hash::ZERO);

        // Same inputs should produce the same root.
        let root2 = compute_merkle_root(&data);
        assert_eq!(root, root2);
    }
}
