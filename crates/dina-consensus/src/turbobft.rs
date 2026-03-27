use dina_core::{Block, BlockHeader, Hash, Transaction};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::leader::LeaderSchedule;
use crate::view_change::{ViewChange, ViewChangeCollector};
use crate::vote::{CommitCertificate, Proposal, Vote, VoteSet, VoteType};

/// Configuration for the TurboBFT consensus engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Public keys (ed25519) of all validators in the network. Order matters
    /// for leader rotation.
    pub validator_keys: Vec<[u8; 32]>,
    /// Target block time in milliseconds (e.g. 2000 for 2-second blocks).
    pub block_time_ms: u64,
    /// Timeout in milliseconds before triggering a view change. Should be
    /// significantly larger than block_time_ms to account for network latency.
    pub timeout_ms: u64,
}

/// The current step in the consensus round state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusStep {
    /// Waiting for or creating a proposal.
    Propose,
    /// Collecting prevotes from validators.
    Prevote,
    /// Collecting precommit votes from validators.
    Precommit,
    /// Block has been committed; ready to advance height.
    Commit,
}

/// Mutable consensus state that evolves as the protocol progresses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusState {
    /// Current blockchain height being decided.
    pub height: u64,
    /// Current round within this height (increments on view change).
    pub round: u32,
    /// Current step in the round state machine.
    pub step: ConsensusStep,
    /// Block we are locked on (received 2/3+ prevotes for it).
    pub locked_block: Option<Block>,
    /// The round in which we locked on the block.
    pub locked_round: Option<u32>,
}

/// Messages emitted by the consensus engine for the network layer to broadcast.
#[derive(Debug, Clone)]
pub enum ConsensusOutput {
    /// Broadcast a proposal to all validators.
    BroadcastProposal(Proposal),
    /// Broadcast a vote to all validators.
    BroadcastVote(Vote),
    /// Broadcast a view change message to all validators.
    BroadcastViewChange(ViewChange),
    /// A block has been committed with a certificate.
    BlockCommitted {
        block: Block,
        certificate: CommitCertificate,
    },
}

/// The TurboBFT consensus engine.
///
/// Implements a pipelined BFT consensus protocol for 3-7 validators based on
/// Tendermint/HotStuff principles:
///
/// 1. **Propose**: Leader proposes a block.
/// 2. **Prevote**: Validators prevote on the proposal (locks the block if 2/3+).
/// 3. **Precommit**: Validators precommit if they see 2/3+ prevotes.
/// 4. **Commit**: Block is committed if 2/3+ precommits are received.
///
/// View changes rotate the leader if a round times out.
pub struct TurboBFT {
    config: ConsensusConfig,
    state: ConsensusState,
    signing_key: SigningKey,
    my_pubkey: [u8; 32],

    /// Current proposal for this round (if any).
    current_proposal: Option<Proposal>,

    /// Prevote set for current height/round.
    prevote_set: VoteSet,
    /// Precommit set for current height/round.
    precommit_set: VoteSet,

    /// View change collector for the current height.
    view_change_collector: ViewChangeCollector,

    /// When the current round started (for timeout detection).
    round_start: Instant,

    /// Channel for emitting consensus outputs to the network layer.
    output_tx: mpsc::UnboundedSender<ConsensusOutput>,
}

impl TurboBFT {
    /// Create a new TurboBFT consensus engine.
    ///
    /// # Arguments
    /// * `config` - Consensus configuration with validator keys and timing.
    /// * `my_key` - This validator's ed25519 signing key.
    /// * `output_tx` - Channel to emit consensus outputs for network broadcast.
    pub fn new(
        config: ConsensusConfig,
        my_key: SigningKey,
        output_tx: mpsc::UnboundedSender<ConsensusOutput>,
    ) -> Self {
        let my_pubkey = my_key.verifying_key().to_bytes();
        let n = config.validator_keys.len();

        info!(
            validators = n,
            my_key = hex::encode(my_pubkey),
            block_time_ms = config.block_time_ms,
            timeout_ms = config.timeout_ms,
            "Initializing TurboBFT consensus engine"
        );

        let state = ConsensusState {
            height: 1,
            round: 0,
            step: ConsensusStep::Propose,
            locked_block: None,
            locked_round: None,
        };

        let prevote_set = VoteSet::new(1, 0, VoteType::Prevote, n);
        let precommit_set = VoteSet::new(1, 0, VoteType::Precommit, n);
        let view_change_collector = ViewChangeCollector::new(1, &config.validator_keys);

        TurboBFT {
            config,
            state,
            signing_key: my_key,
            my_pubkey,
            current_proposal: None,
            prevote_set,
            precommit_set,
            view_change_collector,
            round_start: Instant::now(),
            output_tx,
        }
    }

    /// Start the consensus loop. This runs indefinitely, processing timeouts
    /// on a tick interval. Proposals and votes arrive via explicit method calls
    /// from the network layer.
    ///
    /// The loop checks for timeouts and, if this node is the leader for the
    /// current round, waits for transactions before proposing.
    pub async fn start(&mut self, mut tx_rx: mpsc::UnboundedReceiver<Vec<Transaction>>) {
        info!(height = self.state.height, "Starting TurboBFT consensus loop");

        loop {
            let timeout_duration = Duration::from_millis(self.config.timeout_ms);
            let elapsed = self.round_start.elapsed();

            if self.state.step == ConsensusStep::Propose && self.is_leader(self.state.height, self.state.round) {
                // We are the leader: wait for transactions (with timeout)
                let remaining = timeout_duration.saturating_sub(elapsed);
                tokio::select! {
                    Some(txs) = tx_rx.recv() => {
                        let proposal = self.create_proposal(txs);
                        info!(
                            height = proposal.height,
                            round = proposal.round,
                            "Created and broadcasting proposal"
                        );
                        let _ = self.output_tx.send(ConsensusOutput::BroadcastProposal(proposal.clone()));
                        // Process our own proposal
                        self.on_proposal(proposal);
                    }
                    _ = tokio::time::sleep(remaining) => {
                        warn!(
                            height = self.state.height,
                            round = self.state.round,
                            "Timeout waiting for transactions as leader"
                        );
                        self.on_timeout();
                    }
                }
            } else if elapsed >= timeout_duration {
                // Not the leader or not in propose step: check for timeout
                self.on_timeout();
            } else {
                // Sleep briefly before checking again (avoid busy loop)
                let check_interval = Duration::from_millis(self.config.block_time_ms / 10).min(Duration::from_millis(100));
                tokio::time::sleep(check_interval).await;
            }

            // If we committed, yield briefly before starting next height
            if self.state.step == ConsensusStep::Commit {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }

    /// Handle an incoming proposal from the network.
    ///
    /// Validates the proposal and, if accepted, transitions to Prevote step
    /// and broadcasts our prevote.
    pub fn on_proposal(&mut self, proposal: Proposal) {
        // Must be in Propose step
        if self.state.step != ConsensusStep::Propose {
            debug!(
                step = ?self.state.step,
                "Ignoring proposal: not in Propose step"
            );
            return;
        }

        // Must match current height/round
        if proposal.height != self.state.height || proposal.round != self.state.round {
            debug!(
                expected_height = self.state.height,
                expected_round = self.state.round,
                got_height = proposal.height,
                got_round = proposal.round,
                "Ignoring proposal: height/round mismatch"
            );
            return;
        }

        // Verify the proposer is the expected leader for this round
        let expected_leader =
            LeaderSchedule::leader_for(self.state.height, self.state.round, &self.config.validator_keys);
        if proposal.proposer != expected_leader {
            warn!(
                expected = hex::encode(expected_leader),
                got = hex::encode(proposal.proposer),
                "Ignoring proposal: wrong leader"
            );
            return;
        }

        // Verify signature
        if !proposal.verify_signature() {
            warn!("Ignoring proposal: invalid signature");
            return;
        }

        // If we are locked on a block from a previous round, only accept the
        // proposal if it matches our locked block or the proposal's round is
        // newer than our locked round.
        if let (Some(ref locked_block), Some(locked_round)) =
            (&self.state.locked_block, self.state.locked_round)
        {
            let locked_hash = locked_block.header.hash();
            let proposal_hash = proposal.block_hash();
            if proposal_hash != locked_hash && proposal.round <= locked_round {
                warn!(
                    locked_round,
                    proposal_round = proposal.round,
                    "Ignoring proposal: conflicts with locked block from earlier round"
                );
                return;
            }
        }

        info!(
            height = proposal.height,
            round = proposal.round,
            proposer = hex::encode(proposal.proposer),
            "Accepted proposal"
        );

        self.current_proposal = Some(proposal.clone());

        // Transition to Prevote step
        self.state.step = ConsensusStep::Prevote;

        // Cast our prevote for this block
        let block_hash = proposal.block_hash();
        let vote = Vote::new(
            self.state.height,
            self.state.round,
            block_hash,
            VoteType::Prevote,
            &self.signing_key,
        );

        let _ = self.output_tx.send(ConsensusOutput::BroadcastVote(vote.clone()));

        // Process our own vote
        self.on_vote(vote);
    }

    /// Handle an incoming vote from the network.
    ///
    /// Depending on the vote type and current step, this may:
    /// - Collect prevotes and transition to Precommit if quorum is reached.
    /// - Collect precommits and commit the block if quorum is reached.
    pub fn on_vote(&mut self, vote: Vote) {
        // Validate height/round
        if vote.height != self.state.height || vote.round != self.state.round {
            debug!(
                expected_height = self.state.height,
                expected_round = self.state.round,
                got_height = vote.height,
                got_round = vote.round,
                "Ignoring vote: height/round mismatch"
            );
            return;
        }

        // Verify the voter is a known validator
        if !self.config.validator_keys.contains(&vote.voter) {
            warn!(
                voter = hex::encode(vote.voter),
                "Ignoring vote: unknown validator"
            );
            return;
        }

        match vote.vote_type {
            VoteType::Prevote => self.handle_prevote(vote),
            VoteType::Precommit => self.handle_precommit(vote),
        }
    }

    /// Process a prevote. If we reach 2/3+ prevotes for a block, lock on it
    /// and broadcast our precommit.
    fn handle_prevote(&mut self, vote: Vote) {
        if self.state.step != ConsensusStep::Prevote && self.state.step != ConsensusStep::Propose {
            // Can still accept prevotes in Propose step (might arrive before proposal)
            // but only process the quorum check when in Prevote step
        }

        let block_hash = vote.block_hash;
        if !self.prevote_set.add_vote(vote) {
            return;
        }

        debug!(
            count = self.prevote_set.count(),
            quorum = self.prevote_set.quorum_size(),
            "Prevote added"
        );

        // Check if we have quorum for any specific block
        if self.state.step == ConsensusStep::Prevote && self.prevote_set.has_quorum_for(&block_hash)
        {
            info!(
                height = self.state.height,
                round = self.state.round,
                block_hash = hex::encode(block_hash.as_bytes()),
                "Prevote quorum reached — locking block and sending precommit"
            );

            // Lock on this block
            if let Some(ref proposal) = self.current_proposal {
                if proposal.block_hash() == block_hash {
                    self.state.locked_block = Some(proposal.block.clone());
                    self.state.locked_round = Some(self.state.round);
                }
            }

            // Transition to Precommit step
            self.state.step = ConsensusStep::Precommit;

            // Cast our precommit
            let precommit = Vote::new(
                self.state.height,
                self.state.round,
                block_hash,
                VoteType::Precommit,
                &self.signing_key,
            );

            let _ = self.output_tx.send(ConsensusOutput::BroadcastVote(precommit.clone()));

            // Process our own precommit
            self.handle_precommit(precommit);
        }
    }

    /// Process a precommit. If we reach 2/3+ precommits for a block, commit it.
    fn handle_precommit(&mut self, vote: Vote) {
        let block_hash = vote.block_hash;
        if !self.precommit_set.add_vote(vote) {
            return;
        }

        debug!(
            count = self.precommit_set.count(),
            quorum = self.precommit_set.quorum_size(),
            "Precommit added"
        );

        // Check for precommit quorum
        if self.state.step == ConsensusStep::Precommit
            && self.precommit_set.has_quorum_for(&block_hash)
        {
            info!(
                height = self.state.height,
                round = self.state.round,
                block_hash = hex::encode(block_hash.as_bytes()),
                "Precommit quorum reached — committing block"
            );

            // Build commit certificate
            if let Some(certificate) =
                CommitCertificate::from_vote_set(&self.precommit_set, &block_hash)
            {
                // Emit committed block
                if let Some(ref proposal) = self.current_proposal {
                    if proposal.block_hash() == block_hash {
                        let _ = self.output_tx.send(ConsensusOutput::BlockCommitted {
                            block: proposal.block.clone(),
                            certificate,
                        });
                    }
                }
            }

            // Transition to Commit and advance height
            self.state.step = ConsensusStep::Commit;
            self.advance_height();
        }
    }

    /// Handle a timeout. Initiates a view change to rotate to the next leader.
    pub fn on_timeout(&mut self) {
        if self.state.step == ConsensusStep::Commit {
            // Already committed; no timeout handling needed
            return;
        }

        let old_round = self.state.round;
        let new_round = old_round + 1;

        warn!(
            height = self.state.height,
            old_round,
            new_round,
            step = ?self.state.step,
            "Round timed out — initiating view change"
        );

        // Create and broadcast view change message
        let vc = ViewChange::new(self.state.height, old_round, new_round, &self.signing_key);
        let _ = self.output_tx.send(ConsensusOutput::BroadcastViewChange(vc.clone()));

        // Process our own view change
        self.on_view_change(vc);
    }

    /// Handle an incoming view change message from the network.
    pub fn on_view_change(&mut self, vc: ViewChange) {
        if let Some(new_round) = self.view_change_collector.add_view_change(vc) {
            info!(
                height = self.state.height,
                new_round,
                "View change triggered — advancing to new round"
            );
            self.advance_round(new_round);
        }
    }

    /// Create a proposal containing the given transactions.
    ///
    /// Builds a new block at the current height with a proper block header,
    /// signs it, and returns the proposal.
    pub fn create_proposal(&self, transactions: Vec<Transaction>) -> Proposal {
        // Compute a block hash from the transactions and height
        let block_hash = self.compute_block_hash(self.state.height, self.state.round, &transactions);

        let header = BlockHeader {
            block_number: self.state.height,
            timestamp: chrono::Utc::now().timestamp() as u64,
            parent_hash: Hash::ZERO, // Filled by block storage layer
            state_root: Hash::ZERO,     // Filled after execution
            transactions_root: self.compute_transactions_root(&transactions),
            proposer: dina_core::Address::from_pubkey(&self.signing_key.verifying_key()),
            signature: [0u8; 64],
        };

        let block = Block {
            header,
            transactions: transactions.clone(),
        };

        Proposal::new(self.state.height, self.state.round, block, &self.signing_key)
    }

    /// Check whether the given validator is the leader for the specified height and round.
    pub fn is_leader(&self, height: u64, round: u32) -> bool {
        let leader = LeaderSchedule::leader_for(height, round, &self.config.validator_keys);
        leader == self.my_pubkey
    }

    /// Return the quorum size: 2/3 + 1 of the total validator count.
    pub fn quorum_size(&self) -> usize {
        let n = self.config.validator_keys.len();
        (n * 2 + 2) / 3
    }

    /// Check if a set of votes constitutes a quorum.
    pub fn has_quorum(&self, votes: &[Vote]) -> bool {
        votes.len() >= self.quorum_size()
    }

    /// Get the current consensus state (read-only snapshot).
    pub fn state(&self) -> &ConsensusState {
        &self.state
    }

    /// Get the current height.
    pub fn height(&self) -> u64 {
        self.state.height
    }

    /// Get the current round.
    pub fn round(&self) -> u32 {
        self.state.round
    }

    // ── Internal helpers ──────────────────────────────────────────────

    /// Advance to the next height after a successful commit.
    fn advance_height(&mut self) {
        let old_height = self.state.height;
        self.state.height += 1;
        self.state.round = 0;
        self.state.step = ConsensusStep::Propose;
        self.state.locked_block = None;
        self.state.locked_round = None;
        self.current_proposal = None;

        let n = self.config.validator_keys.len();
        self.prevote_set = VoteSet::new(self.state.height, 0, VoteType::Prevote, n);
        self.precommit_set = VoteSet::new(self.state.height, 0, VoteType::Precommit, n);
        self.view_change_collector.reset(self.state.height);
        self.round_start = Instant::now();

        info!(
            old_height,
            new_height = self.state.height,
            "Advanced to new height"
        );
    }

    /// Advance to a new round (view change) without changing height.
    fn advance_round(&mut self, new_round: u32) {
        let old_round = self.state.round;
        self.state.round = new_round;
        self.state.step = ConsensusStep::Propose;
        self.current_proposal = None;

        // Keep locked_block and locked_round — these persist across rounds
        // within the same height (safety property).

        let n = self.config.validator_keys.len();
        self.prevote_set = VoteSet::new(self.state.height, new_round, VoteType::Prevote, n);
        self.precommit_set = VoteSet::new(self.state.height, new_round, VoteType::Precommit, n);
        self.round_start = Instant::now();

        info!(
            height = self.state.height,
            old_round,
            new_round,
            leader = hex::encode(LeaderSchedule::leader_for(
                self.state.height,
                new_round,
                &self.config.validator_keys
            )),
            "Advanced to new round"
        );
    }

    /// Compute a block hash from height, round, and transactions.
    fn compute_block_hash(
        &self,
        height: u64,
        round: u32,
        transactions: &[Transaction],
    ) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(b"BLOCK");
        hasher.update(height.to_le_bytes());
        hasher.update(round.to_le_bytes());
        for tx in transactions {
            // Hash the serialized transaction
            let tx_bytes = serde_json::to_vec(tx).unwrap_or_default();
            hasher.update(&tx_bytes);
        }
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Hash(hash)
    }

    /// Compute a Merkle root of the transaction hashes (simplified).
    fn compute_transactions_root(&self, transactions: &[Transaction]) -> Hash {
        if transactions.is_empty() {
            return Hash::ZERO;
        }

        let mut hasher = Sha256::new();
        hasher.update(b"TX_ROOT");
        for tx in transactions {
            let tx_bytes = serde_json::to_vec(tx).unwrap_or_default();
            let mut tx_hasher = Sha256::new();
            tx_hasher.update(&tx_bytes);
            hasher.update(tx_hasher.finalize());
        }
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Hash(hash)
    }
}
