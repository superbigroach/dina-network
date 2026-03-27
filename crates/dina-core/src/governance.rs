use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::error::{DinaError, DinaResult};
use crate::types::Address;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default voting period in blocks (~17 minutes at 100ms blocks).
pub const DEFAULT_VOTING_PERIOD: u64 = 10_000;

/// Default quorum: 60% of total staked tokens must participate.
pub const DEFAULT_QUORUM_BPS: u16 = 6_000;

/// Default pass threshold: 50% of votes must be in favor.
pub const DEFAULT_PASS_THRESHOLD_BPS: u16 = 5_000;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// On-chain governance module for protocol parameter changes, slashing
/// proposals, treasury spends, and protocol upgrades.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceModule {
    proposals: BTreeMap<u64, Proposal>,
    next_proposal_id: u64,
    voting_period_blocks: u64,
    quorum_bps: u16,
    pass_threshold_bps: u16,
}

/// A governance proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub title: String,
    pub description: String,
    pub proposal_type: ProposalType,
    pub created_at_block: u64,
    pub voting_end_block: u64,
    pub votes_for: u64,
    pub votes_against: u64,
    pub voters: BTreeSet<Address>,
    pub status: ProposalStatus,
}

/// What a proposal intends to do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalType {
    ParameterChange {
        key: String,
        old_value: String,
        new_value: String,
    },
    ValidatorSlash {
        validator: Address,
        amount: u64,
        reason: String,
    },
    TreasurySpend {
        recipient: Address,
        amount: u64,
        purpose: String,
    },
    ProtocolUpgrade {
        version: String,
        description: String,
    },
    Custom {
        data: String,
    },
}

/// Lifecycle status of a proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Executed,
    Expired,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl GovernanceModule {
    /// Create a new governance module with the given parameters.
    pub fn new(voting_period_blocks: u64, quorum_bps: u16, pass_threshold_bps: u16) -> Self {
        Self {
            proposals: BTreeMap::new(),
            next_proposal_id: 1,
            voting_period_blocks,
            quorum_bps,
            pass_threshold_bps,
        }
    }

    /// Create a governance module with sensible defaults.
    pub fn default_config() -> Self {
        Self::new(
            DEFAULT_VOTING_PERIOD,
            DEFAULT_QUORUM_BPS,
            DEFAULT_PASS_THRESHOLD_BPS,
        )
    }

    /// Create a new proposal. Returns the proposal ID.
    pub fn create_proposal(
        &mut self,
        proposer: Address,
        title: String,
        description: String,
        proposal_type: ProposalType,
        current_block: u64,
    ) -> u64 {
        let id = self.next_proposal_id;
        self.next_proposal_id += 1;

        let proposal = Proposal {
            id,
            proposer,
            title,
            description,
            proposal_type,
            created_at_block: current_block,
            voting_end_block: current_block + self.voting_period_blocks,
            votes_for: 0,
            votes_against: 0,
            voters: BTreeSet::new(),
            status: ProposalStatus::Active,
        };

        self.proposals.insert(id, proposal);
        id
    }

    /// Cast a vote on a proposal. Each address may only vote once.
    /// Votes are weighted by the voter's stake.
    pub fn vote(
        &mut self,
        proposal_id: u64,
        voter: Address,
        vote_for: bool,
        voter_stake: u64,
        current_block: u64,
    ) -> DinaResult<()> {
        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or_else(|| DinaError::GovernanceError("proposal not found".to_string()))?;

        if proposal.status != ProposalStatus::Active {
            return Err(DinaError::GovernanceError(
                "proposal is not active".to_string(),
            ));
        }

        if current_block > proposal.voting_end_block {
            return Err(DinaError::GovernanceError(
                "voting period has ended".to_string(),
            ));
        }

        if proposal.voters.contains(&voter) {
            return Err(DinaError::GovernanceError(
                "voter has already voted".to_string(),
            ));
        }

        if voter_stake == 0 {
            return Err(DinaError::GovernanceError(
                "voter has no stake".to_string(),
            ));
        }

        proposal.voters.insert(voter);

        if vote_for {
            proposal.votes_for += voter_stake;
        } else {
            proposal.votes_against += voter_stake;
        }

        Ok(())
    }

    /// Finalize a proposal: check if quorum is met and whether it passes.
    /// `total_staked` is the total amount of tokens staked across all
    /// validators at the time of finalization.
    pub fn finalize_proposal(
        &mut self,
        proposal_id: u64,
        current_block: u64,
        total_staked: u64,
    ) -> DinaResult<ProposalStatus> {
        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or_else(|| DinaError::GovernanceError("proposal not found".to_string()))?;

        if proposal.status != ProposalStatus::Active {
            return Err(DinaError::GovernanceError(
                "proposal is not active".to_string(),
            ));
        }

        if current_block < proposal.voting_end_block {
            return Err(DinaError::GovernanceError(
                "voting period has not ended".to_string(),
            ));
        }

        let total_votes = proposal.votes_for + proposal.votes_against;

        // Check quorum: total votes must be >= quorum_bps% of total staked
        let quorum_required = total_staked * self.quorum_bps as u64 / 10_000;
        if total_votes < quorum_required {
            proposal.status = ProposalStatus::Expired;
            return Ok(ProposalStatus::Expired);
        }

        // Check threshold: votes_for must be >= pass_threshold_bps% of total votes
        let threshold = total_votes * self.pass_threshold_bps as u64 / 10_000;
        if proposal.votes_for >= threshold {
            proposal.status = ProposalStatus::Passed;
            Ok(ProposalStatus::Passed)
        } else {
            proposal.status = ProposalStatus::Rejected;
            Ok(ProposalStatus::Rejected)
        }
    }

    /// Mark a passed proposal as executed and return the proposal type for
    /// the caller to act on.
    pub fn execute_proposal(&mut self, proposal_id: u64) -> DinaResult<ProposalType> {
        let proposal = self
            .proposals
            .get_mut(&proposal_id)
            .ok_or_else(|| DinaError::GovernanceError("proposal not found".to_string()))?;

        if proposal.status != ProposalStatus::Passed {
            return Err(DinaError::GovernanceError(
                "proposal has not passed".to_string(),
            ));
        }

        proposal.status = ProposalStatus::Executed;
        Ok(proposal.proposal_type.clone())
    }

    /// Return all currently active proposals.
    pub fn active_proposals(&self) -> Vec<&Proposal> {
        self.proposals
            .values()
            .filter(|p| p.status == ProposalStatus::Active)
            .collect()
    }

    /// Get a proposal by ID.
    pub fn get_proposal(&self, id: u64) -> Option<&Proposal> {
        self.proposals.get(&id)
    }

    /// Total number of proposals ever created.
    pub fn proposal_count(&self) -> u64 {
        self.next_proposal_id - 1
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(byte: u8) -> Address {
        Address([byte; 32])
    }

    fn gov() -> GovernanceModule {
        // Short voting period for tests
        GovernanceModule::new(100, 6_000, 5_000)
    }

    fn param_change() -> ProposalType {
        ProposalType::ParameterChange {
            key: "min_stake".to_string(),
            old_value: "10000".to_string(),
            new_value: "20000".to_string(),
        }
    }

    #[test]
    fn create_proposal_returns_id() {
        let mut g = gov();
        let id = g.create_proposal(
            addr(1),
            "Raise minimum stake".to_string(),
            "Double it".to_string(),
            param_change(),
            0,
        );
        assert_eq!(id, 1);
        assert_eq!(g.proposal_count(), 1);
    }

    #[test]
    fn create_multiple_proposals() {
        let mut g = gov();
        let id1 = g.create_proposal(addr(1), "P1".into(), "D1".into(), param_change(), 0);
        let id2 = g.create_proposal(addr(2), "P2".into(), "D2".into(), param_change(), 10);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(g.active_proposals().len(), 2);
    }

    #[test]
    fn vote_for_success() {
        let mut g = gov();
        let id = g.create_proposal(addr(1), "P".into(), "D".into(), param_change(), 0);
        g.vote(id, addr(2), true, 5_000, 50).unwrap();
        let p = g.get_proposal(id).unwrap();
        assert_eq!(p.votes_for, 5_000);
        assert_eq!(p.votes_against, 0);
        assert!(p.voters.contains(&addr(2)));
    }

    #[test]
    fn vote_against_success() {
        let mut g = gov();
        let id = g.create_proposal(addr(1), "P".into(), "D".into(), param_change(), 0);
        g.vote(id, addr(2), false, 3_000, 50).unwrap();
        let p = g.get_proposal(id).unwrap();
        assert_eq!(p.votes_against, 3_000);
    }

    #[test]
    fn double_vote_fails() {
        let mut g = gov();
        let id = g.create_proposal(addr(1), "P".into(), "D".into(), param_change(), 0);
        g.vote(id, addr(2), true, 5_000, 50).unwrap();
        let err = g.vote(id, addr(2), false, 5_000, 50).unwrap_err();
        assert!(matches!(err, DinaError::GovernanceError(_)));
    }

    #[test]
    fn vote_after_period_fails() {
        let mut g = gov();
        let id = g.create_proposal(addr(1), "P".into(), "D".into(), param_change(), 0);
        // voting_end_block = 0 + 100 = 100
        let err = g.vote(id, addr(2), true, 5_000, 101).unwrap_err();
        assert!(matches!(err, DinaError::GovernanceError(_)));
    }

    #[test]
    fn vote_zero_stake_fails() {
        let mut g = gov();
        let id = g.create_proposal(addr(1), "P".into(), "D".into(), param_change(), 0);
        let err = g.vote(id, addr(2), true, 0, 50).unwrap_err();
        assert!(matches!(err, DinaError::GovernanceError(_)));
    }

    #[test]
    fn finalize_passes_with_quorum_and_majority() {
        let mut g = gov();
        let id = g.create_proposal(addr(1), "P".into(), "D".into(), param_change(), 0);
        // total_staked = 10_000, quorum = 60% = 6_000
        g.vote(id, addr(2), true, 4_000, 50).unwrap();
        g.vote(id, addr(3), true, 3_000, 50).unwrap();
        // 7_000 votes for, 0 against. 7_000 >= 6_000 quorum. 7_000 >= 50% of 7_000.
        let status = g.finalize_proposal(id, 101, 10_000).unwrap();
        assert_eq!(status, ProposalStatus::Passed);
    }

    #[test]
    fn finalize_rejected_with_majority_against() {
        let mut g = gov();
        let id = g.create_proposal(addr(1), "P".into(), "D".into(), param_change(), 0);
        g.vote(id, addr(2), true, 2_000, 50).unwrap();
        g.vote(id, addr(3), false, 5_000, 50).unwrap();
        // total 7_000 >= 6_000 quorum, but for=2_000 < 50% of 7_000=3_500
        let status = g.finalize_proposal(id, 101, 10_000).unwrap();
        assert_eq!(status, ProposalStatus::Rejected);
    }

    #[test]
    fn finalize_expired_no_quorum() {
        let mut g = gov();
        let id = g.create_proposal(addr(1), "P".into(), "D".into(), param_change(), 0);
        g.vote(id, addr(2), true, 1_000, 50).unwrap();
        // 1_000 < 6_000 quorum
        let status = g.finalize_proposal(id, 101, 10_000).unwrap();
        assert_eq!(status, ProposalStatus::Expired);
    }

    #[test]
    fn finalize_before_end_fails() {
        let mut g = gov();
        let id = g.create_proposal(addr(1), "P".into(), "D".into(), param_change(), 0);
        let err = g.finalize_proposal(id, 50, 10_000).unwrap_err();
        assert!(matches!(err, DinaError::GovernanceError(_)));
    }

    #[test]
    fn execute_passed_proposal() {
        let mut g = gov();
        let pt = ProposalType::TreasurySpend {
            recipient: addr(10),
            amount: 50_000,
            purpose: "dev fund".to_string(),
        };
        let id = g.create_proposal(addr(1), "Spend".into(), "D".into(), pt.clone(), 0);
        g.vote(id, addr(2), true, 7_000, 50).unwrap();
        g.finalize_proposal(id, 101, 10_000).unwrap();
        let executed = g.execute_proposal(id).unwrap();
        assert_eq!(executed, pt);
        assert_eq!(
            g.get_proposal(id).unwrap().status,
            ProposalStatus::Executed
        );
    }

    #[test]
    fn execute_non_passed_fails() {
        let mut g = gov();
        let id = g.create_proposal(addr(1), "P".into(), "D".into(), param_change(), 0);
        // Still active, not passed
        let err = g.execute_proposal(id).unwrap_err();
        assert!(matches!(err, DinaError::GovernanceError(_)));
    }

    #[test]
    fn active_proposals_excludes_finalized() {
        let mut g = gov();
        let id1 = g.create_proposal(addr(1), "P1".into(), "D".into(), param_change(), 0);
        let _id2 = g.create_proposal(addr(1), "P2".into(), "D".into(), param_change(), 0);
        g.vote(id1, addr(2), true, 7_000, 50).unwrap();
        g.finalize_proposal(id1, 101, 10_000).unwrap();
        assert_eq!(g.active_proposals().len(), 1);
    }

    #[test]
    fn default_config_values() {
        let g = GovernanceModule::default_config();
        assert_eq!(g.voting_period_blocks, DEFAULT_VOTING_PERIOD);
    }
}
