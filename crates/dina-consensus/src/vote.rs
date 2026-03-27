use dina_core::{Block, Hash};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use sha2::{Digest, Sha256};
use tracing::{debug, warn};

/// Type of vote in the BFT protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteType {
    Prevote,
    Precommit,
}

/// A proposal broadcast by the round leader containing a candidate block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub height: u64,
    pub round: u32,
    pub block: Block,
    pub proposer: [u8; 32],
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
}

impl Proposal {
    /// Create a new signed proposal.
    pub fn new(height: u64, round: u32, block: Block, signing_key: &SigningKey) -> Self {
        let proposer = signing_key.verifying_key().to_bytes();
        let sign_bytes = Self::sign_bytes(height, round, &block);
        let signature = signing_key.sign(&sign_bytes);

        Proposal {
            height,
            round,
            block,
            proposer,
            signature: signature.to_bytes(),
        }
    }

    /// Compute the bytes to sign for a proposal: SHA-256("PROPOSAL" || height || round || block_hash).
    fn sign_bytes(height: u64, round: u32, block: &Block) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"PROPOSAL");
        hasher.update(height.to_le_bytes());
        hasher.update(round.to_le_bytes());
        hasher.update(block.header.hash().as_bytes());
        hasher.finalize().to_vec()
    }

    /// Verify the proposer's ed25519 signature on this proposal.
    pub fn verify_signature(&self) -> bool {
        let verifying_key = match VerifyingKey::from_bytes(&self.proposer) {
            Ok(k) => k,
            Err(_) => {
                warn!("Invalid proposer public key");
                return false;
            }
        };
        let sign_bytes = Self::sign_bytes(self.height, self.round, &self.block);
        let signature = Signature::from_bytes(&self.signature);
        verifying_key.verify(&sign_bytes, &signature).is_ok()
    }

    /// Return the hash of the proposed block.
    pub fn block_hash(&self) -> Hash {
        self.block.header.hash()
    }
}

/// A vote (prevote or precommit) cast by a validator on a specific block hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub height: u64,
    pub round: u32,
    pub block_hash: Hash,
    pub vote_type: VoteType,
    pub voter: [u8; 32],
    #[serde(with = "BigArray")]
    pub signature: [u8; 64],
}

impl Vote {
    /// Create a new signed vote.
    pub fn new(
        height: u64,
        round: u32,
        block_hash: Hash,
        vote_type: VoteType,
        signing_key: &SigningKey,
    ) -> Self {
        let voter = signing_key.verifying_key().to_bytes();
        let sign_bytes = Self::sign_bytes(height, round, &block_hash, vote_type);
        let signature = signing_key.sign(&sign_bytes);

        Vote {
            height,
            round,
            block_hash,
            vote_type,
            voter,
            signature: signature.to_bytes(),
        }
    }

    /// Compute the bytes to sign: SHA-256(vote_type_tag || height || round || block_hash).
    fn sign_bytes(height: u64, round: u32, block_hash: &Hash, vote_type: VoteType) -> Vec<u8> {
        let mut hasher = Sha256::new();
        match vote_type {
            VoteType::Prevote => hasher.update(b"PREVOTE"),
            VoteType::Precommit => hasher.update(b"PRECOMMIT"),
        }
        hasher.update(height.to_le_bytes());
        hasher.update(round.to_le_bytes());
        hasher.update(block_hash.as_bytes());
        hasher.finalize().to_vec()
    }

    /// Verify the voter's ed25519 signature on this vote.
    pub fn verify_signature(&self) -> bool {
        let verifying_key = match VerifyingKey::from_bytes(&self.voter) {
            Ok(k) => k,
            Err(_) => {
                warn!(voter = hex::encode(self.voter), "Invalid voter public key");
                return false;
            }
        };
        let sign_bytes = Self::sign_bytes(self.height, self.round, &self.block_hash, self.vote_type);
        let signature = Signature::from_bytes(&self.signature);
        verifying_key.verify(&sign_bytes, &signature).is_ok()
    }
}

/// A collection of votes for a specific height/round, partitioned by vote type.
/// Used to determine when quorum has been reached.
#[derive(Debug, Clone)]
pub struct VoteSet {
    pub height: u64,
    pub round: u32,
    pub vote_type: VoteType,
    /// Set of votes keyed by voter public key for dedup.
    votes: std::collections::HashMap<[u8; 32], Vote>,
    /// Total number of validators in the network.
    total_validators: usize,
}

impl VoteSet {
    /// Create a new empty vote set for a given height, round, and vote type.
    pub fn new(height: u64, round: u32, vote_type: VoteType, total_validators: usize) -> Self {
        VoteSet {
            height,
            round,
            vote_type,
            votes: std::collections::HashMap::new(),
            total_validators,
        }
    }

    /// The quorum size: 2f + 1 where f = floor((n - 1) / 3).
    /// Equivalent to ceil(2n/3) for the standard BFT threshold.
    pub fn quorum_size(&self) -> usize {
        (self.total_validators * 2 + 2) / 3
    }

    /// Add a vote to the set. Returns true if the vote was new (not a duplicate).
    /// Validates the vote's height, round, type, and signature before accepting.
    pub fn add_vote(&mut self, vote: Vote) -> bool {
        // Reject votes for wrong height/round/type
        if vote.height != self.height || vote.round != self.round || vote.vote_type != self.vote_type
        {
            debug!(
                expected_height = self.height,
                expected_round = self.round,
                got_height = vote.height,
                got_round = vote.round,
                "Rejected vote: height/round/type mismatch"
            );
            return false;
        }

        // Reject duplicate votes from the same voter
        if self.votes.contains_key(&vote.voter) {
            debug!(voter = hex::encode(vote.voter), "Rejected duplicate vote");
            return false;
        }

        // Verify signature
        if !vote.verify_signature() {
            warn!(voter = hex::encode(vote.voter), "Rejected vote with invalid signature");
            return false;
        }

        self.votes.insert(vote.voter, vote);
        true
    }

    /// Check whether we have reached quorum (2/3+ votes).
    pub fn has_quorum(&self) -> bool {
        self.votes.len() >= self.quorum_size()
    }

    /// Check whether quorum of votes agree on a specific block hash.
    pub fn has_quorum_for(&self, block_hash: &Hash) -> bool {
        let matching = self
            .votes
            .values()
            .filter(|v| &v.block_hash == block_hash)
            .count();
        matching >= self.quorum_size()
    }

    /// Return all votes collected so far as a Vec.
    pub fn votes(&self) -> Vec<Vote> {
        self.votes.values().cloned().collect()
    }

    /// Return the number of votes collected.
    pub fn count(&self) -> usize {
        self.votes.len()
    }

    /// Return votes that voted for a specific block hash.
    pub fn votes_for(&self, block_hash: &Hash) -> Vec<Vote> {
        self.votes
            .values()
            .filter(|v| &v.block_hash == block_hash)
            .cloned()
            .collect()
    }
}

/// A commit certificate proving that a block was committed by 2/3+ validators.
/// This is the final proof of consensus for a given height.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitCertificate {
    pub height: u64,
    pub block_hash: Hash,
    pub votes: Vec<Vote>,
}

impl CommitCertificate {
    /// Build a commit certificate from a precommit VoteSet that has reached quorum
    /// on a specific block hash.
    pub fn from_vote_set(vote_set: &VoteSet, block_hash: &Hash) -> Option<Self> {
        if vote_set.vote_type != VoteType::Precommit {
            warn!("Cannot create CommitCertificate from non-precommit votes");
            return None;
        }
        if !vote_set.has_quorum_for(block_hash) {
            warn!("Cannot create CommitCertificate without quorum");
            return None;
        }

        let matching_votes = vote_set.votes_for(block_hash);
        Some(CommitCertificate {
            height: vote_set.height,
            block_hash: *block_hash,
            votes: matching_votes,
        })
    }

    /// Verify all vote signatures in the certificate and confirm quorum.
    pub fn verify(&self, total_validators: usize) -> bool {
        let quorum = (total_validators * 2 + 2) / 3;

        if self.votes.len() < quorum {
            warn!(
                votes = self.votes.len(),
                quorum,
                "CommitCertificate has insufficient votes"
            );
            return false;
        }

        // Verify all signatures and that votes are for the correct block hash
        for vote in &self.votes {
            if vote.vote_type != VoteType::Precommit {
                warn!("CommitCertificate contains non-precommit vote");
                return false;
            }
            if vote.block_hash != self.block_hash {
                warn!("CommitCertificate contains vote for wrong block hash");
                return false;
            }
            if !vote.verify_signature() {
                warn!(voter = hex::encode(vote.voter), "CommitCertificate vote has invalid signature");
                return false;
            }
        }

        // Ensure no duplicate voters
        let mut seen_voters = std::collections::HashSet::new();
        for vote in &self.votes {
            if !seen_voters.insert(vote.voter) {
                warn!("CommitCertificate contains duplicate voter");
                return false;
            }
        }

        true
    }
}
