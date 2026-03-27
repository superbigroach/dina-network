use dina_consensus::{
    CommitCertificate, LeaderSchedule, Proposal, Vote, VoteSet, VoteType,
    ViewChange, ViewChangeCollector,
};
use dina_core::{Address, Block, BlockHeader, Hash};
use ed25519_dalek::SigningKey;

// ============================================================
// Helpers
// ============================================================

fn make_signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn make_validators(count: usize) -> Vec<[u8; 32]> {
    (0..count)
        .map(|i| make_signing_key(i as u8 + 1).verifying_key().to_bytes())
        .collect()
}

fn make_signing_keys(count: usize) -> Vec<SigningKey> {
    (0..count)
        .map(|i| make_signing_key(i as u8 + 1))
        .collect()
}

fn dummy_block(height: u64) -> Block {
    Block {
        header: BlockHeader {
            block_number: height,
            timestamp: 1_700_000_000,
            parent_hash: Hash::ZERO,
            state_root: Hash::ZERO,
            transactions_root: Hash::ZERO,
            proposer: Address::ZERO,
            signature: [0u8; 64],
        },
        transactions: Vec::new(),
    }
}

// ============================================================
// Leader rotation tests
// ============================================================

#[test]
fn leader_round_robin_produces_correct_leader_for_each_height() {
    let validators = make_validators(4);

    assert_eq!(LeaderSchedule::leader_for(0, 0, &validators), validators[0]);
    assert_eq!(LeaderSchedule::leader_for(1, 0, &validators), validators[1]);
    assert_eq!(LeaderSchedule::leader_for(2, 0, &validators), validators[2]);
    assert_eq!(LeaderSchedule::leader_for(3, 0, &validators), validators[3]);
}

#[test]
fn leader_changes_with_round_increment() {
    let validators = make_validators(4);

    let leader_r0 = LeaderSchedule::leader_for(0, 0, &validators);
    let leader_r1 = LeaderSchedule::leader_for(0, 1, &validators);
    let leader_r2 = LeaderSchedule::leader_for(0, 2, &validators);

    assert_eq!(leader_r0, validators[0]);
    assert_eq!(leader_r1, validators[1]);
    assert_eq!(leader_r2, validators[2]);
    assert_ne!(leader_r0, leader_r1);
    assert_ne!(leader_r1, leader_r2);
}

#[test]
fn single_validator_is_always_leader() {
    let validators = make_validators(1);

    for height in 0..10 {
        for round in 0..5 {
            assert_eq!(
                LeaderSchedule::leader_for(height, round, &validators),
                validators[0]
            );
        }
    }
}

#[test]
fn leader_wraps_around_validator_set() {
    let validators = make_validators(3);

    // height 3, round 0 => (3 + 0) % 3 = 0 => wraps back
    assert_eq!(LeaderSchedule::leader_for(3, 0, &validators), validators[0]);
    // height 4, round 0 => (4 + 0) % 3 = 1
    assert_eq!(LeaderSchedule::leader_for(4, 0, &validators), validators[1]);
    // height 5, round 0 => (5 + 0) % 3 = 2
    assert_eq!(LeaderSchedule::leader_for(5, 0, &validators), validators[2]);
    // height 6, round 0 => (6 + 0) % 3 = 0 => wraps again
    assert_eq!(LeaderSchedule::leader_for(6, 0, &validators), validators[0]);
}

// ============================================================
// Vote tests
// ============================================================

#[test]
fn vote_sign_and_verify_roundtrip() {
    let key = make_signing_key(1);
    let block_hash = Hash([0xAB; 32]);

    let vote = Vote::new(1, 0, block_hash, VoteType::Prevote, &key);

    assert!(vote.verify_signature());
    assert_eq!(vote.height, 1);
    assert_eq!(vote.round, 0);
    assert_eq!(vote.block_hash, block_hash);
    assert_eq!(vote.vote_type, VoteType::Prevote);
    assert_eq!(vote.voter, key.verifying_key().to_bytes());
}

#[test]
fn vote_invalid_signature_rejected() {
    let key = make_signing_key(1);
    let block_hash = Hash([0xAB; 32]);

    let mut vote = Vote::new(1, 0, block_hash, VoteType::Prevote, &key);
    // Corrupt the signature
    vote.signature[0] ^= 0xFF;

    assert!(!vote.verify_signature());
}

#[test]
fn voteset_deduplicates_by_voter() {
    let keys = make_signing_keys(3);
    let _validators = make_validators(3);
    let block_hash = Hash([0xAB; 32]);

    let mut vs = VoteSet::new(1, 0, VoteType::Prevote, 3);

    let vote1 = Vote::new(1, 0, block_hash, VoteType::Prevote, &keys[0]);
    let vote1_dup = Vote::new(1, 0, block_hash, VoteType::Prevote, &keys[0]);

    assert!(vs.add_vote(vote1));
    assert!(!vs.add_vote(vote1_dup)); // Duplicate rejected
    assert_eq!(vs.count(), 1);
}

#[test]
fn voteset_quorum_3_validators_at_2() {
    // n=3, quorum = (3*2+2)/3 = 8/3 = 2
    let keys = make_signing_keys(3);
    let block_hash = Hash([0xAB; 32]);

    let mut vs = VoteSet::new(1, 0, VoteType::Prevote, 3);
    assert_eq!(vs.quorum_size(), 2);

    let v0 = Vote::new(1, 0, block_hash, VoteType::Prevote, &keys[0]);
    vs.add_vote(v0);
    assert!(!vs.has_quorum());

    let v1 = Vote::new(1, 0, block_hash, VoteType::Prevote, &keys[1]);
    vs.add_vote(v1);
    assert!(vs.has_quorum());
}

#[test]
fn voteset_quorum_5_validators_at_4() {
    // n=5, quorum = (5*2+2)/3 = 12/3 = 4
    let keys = make_signing_keys(5);
    let block_hash = Hash([0xAB; 32]);

    let mut vs = VoteSet::new(1, 0, VoteType::Prevote, 5);
    assert_eq!(vs.quorum_size(), 4);

    for i in 0..3 {
        let v = Vote::new(1, 0, block_hash, VoteType::Prevote, &keys[i]);
        vs.add_vote(v);
    }
    assert!(!vs.has_quorum()); // 3 < 4

    let v3 = Vote::new(1, 0, block_hash, VoteType::Prevote, &keys[3]);
    vs.add_vote(v3);
    assert!(vs.has_quorum()); // 4 >= 4
}

#[test]
fn voteset_quorum_7_validators_at_5() {
    // n=7, quorum = (7*2+2)/3 = 16/3 = 5
    let keys = make_signing_keys(7);
    let block_hash = Hash([0xAB; 32]);

    let mut vs = VoteSet::new(1, 0, VoteType::Prevote, 7);
    assert_eq!(vs.quorum_size(), 5);

    for i in 0..4 {
        let v = Vote::new(1, 0, block_hash, VoteType::Prevote, &keys[i]);
        vs.add_vote(v);
    }
    assert!(!vs.has_quorum()); // 4 < 5

    let v4 = Vote::new(1, 0, block_hash, VoteType::Prevote, &keys[4]);
    vs.add_vote(v4);
    assert!(vs.has_quorum()); // 5 >= 5
}

// ============================================================
// Proposal tests
// ============================================================

#[test]
fn proposal_sign_and_verify() {
    let key = make_signing_key(1);
    let block = dummy_block(1);

    let proposal = Proposal::new(1, 0, block, &key);

    assert!(proposal.verify_signature());
    assert_eq!(proposal.height, 1);
    assert_eq!(proposal.round, 0);
    assert_eq!(proposal.proposer, key.verifying_key().to_bytes());
}

#[test]
fn proposal_invalid_signature_rejected() {
    let key = make_signing_key(1);
    let block = dummy_block(1);

    let mut proposal = Proposal::new(1, 0, block, &key);
    // Corrupt the signature
    proposal.signature[0] ^= 0xFF;

    assert!(!proposal.verify_signature());
}

// ============================================================
// CommitCertificate tests
// ============================================================

#[test]
fn commit_certificate_valid_with_quorum_votes() {
    let keys = make_signing_keys(3);
    let block = dummy_block(1);
    let block_hash = block.header.hash();

    let mut vs = VoteSet::new(1, 0, VoteType::Precommit, 3);
    // Need quorum of 2 for n=3
    for i in 0..2 {
        let v = Vote::new(1, 0, block_hash, VoteType::Precommit, &keys[i]);
        vs.add_vote(v);
    }
    assert!(vs.has_quorum_for(&block_hash));

    let cert = CommitCertificate::from_vote_set(&vs, &block_hash);
    assert!(cert.is_some());

    let cert = cert.unwrap();
    assert_eq!(cert.height, 1);
    assert_eq!(cert.block_hash, block_hash);
    assert!(cert.verify(3));
}

#[test]
fn commit_certificate_verify_checks_all_signatures() {
    let keys = make_signing_keys(3);
    let block = dummy_block(1);
    let block_hash = block.header.hash();

    let mut vs = VoteSet::new(1, 0, VoteType::Precommit, 3);
    for i in 0..2 {
        let v = Vote::new(1, 0, block_hash, VoteType::Precommit, &keys[i]);
        vs.add_vote(v);
    }

    let mut cert = CommitCertificate::from_vote_set(&vs, &block_hash).unwrap();
    assert!(cert.verify(3));

    // Corrupt one vote's signature
    cert.votes[0].signature[0] ^= 0xFF;
    assert!(!cert.verify(3));
}

#[test]
fn commit_certificate_fails_without_quorum() {
    let keys = make_signing_keys(3);
    let block = dummy_block(1);
    let block_hash = block.header.hash();

    let mut vs = VoteSet::new(1, 0, VoteType::Precommit, 3);
    // Only 1 vote, need 2
    let v = Vote::new(1, 0, block_hash, VoteType::Precommit, &keys[0]);
    vs.add_vote(v);

    let cert = CommitCertificate::from_vote_set(&vs, &block_hash);
    assert!(cert.is_none());
}

#[test]
fn commit_certificate_requires_precommit_votes() {
    let keys = make_signing_keys(3);
    let block = dummy_block(1);
    let block_hash = block.header.hash();

    // Create a Prevote set instead of Precommit
    let mut vs = VoteSet::new(1, 0, VoteType::Prevote, 3);
    for i in 0..2 {
        let v = Vote::new(1, 0, block_hash, VoteType::Prevote, &keys[i]);
        vs.add_vote(v);
    }

    let cert = CommitCertificate::from_vote_set(&vs, &block_hash);
    assert!(cert.is_none()); // Cannot create from prevote set
}

// ============================================================
// View change tests
// ============================================================

#[test]
fn view_change_sign_and_verify() {
    let key = make_signing_key(1);

    let vc = ViewChange::new(1, 0, 1, &key);

    assert!(vc.verify_signature());
    assert_eq!(vc.height, 1);
    assert_eq!(vc.old_round, 0);
    assert_eq!(vc.new_round, 1);
    assert_eq!(vc.voter, key.verifying_key().to_bytes());
}

#[test]
fn view_change_invalid_signature_rejected() {
    let key = make_signing_key(1);

    let mut vc = ViewChange::new(1, 0, 1, &key);
    vc.signature[0] ^= 0xFF;

    assert!(!vc.verify_signature());
}

#[test]
fn view_change_collector_reaches_quorum() {
    let keys = make_signing_keys(3);
    let validators = make_validators(3);

    let mut collector = ViewChangeCollector::new(1, &validators);
    // quorum for n=3 is 2

    let vc0 = ViewChange::new(1, 0, 1, &keys[0]);
    assert_eq!(collector.add_view_change(vc0), None); // 1/2, no quorum yet

    let vc1 = ViewChange::new(1, 0, 1, &keys[1]);
    assert_eq!(collector.add_view_change(vc1), Some(1)); // 2/2, quorum reached!
}

#[test]
fn view_change_collector_rejects_wrong_height() {
    let keys = make_signing_keys(3);
    let validators = make_validators(3);

    let mut collector = ViewChangeCollector::new(1, &validators);

    // Wrong height
    let vc = ViewChange::new(2, 0, 1, &keys[0]);
    assert_eq!(collector.add_view_change(vc), None);
    assert_eq!(collector.count_for_round(1), 0);
}

#[test]
fn view_change_collector_rejects_invalid_round_progression() {
    let keys = make_signing_keys(3);
    let validators = make_validators(3);

    let mut collector = ViewChangeCollector::new(1, &validators);

    // new_round <= old_round should be rejected
    let vc = ViewChange::new(1, 1, 1, &keys[0]);
    assert_eq!(collector.add_view_change(vc), None);
    assert_eq!(collector.count_for_round(1), 0);
}

#[test]
fn view_change_collector_rejects_duplicate() {
    let keys = make_signing_keys(3);
    let validators = make_validators(3);

    let mut collector = ViewChangeCollector::new(1, &validators);

    let vc0 = ViewChange::new(1, 0, 1, &keys[0]);
    assert_eq!(collector.add_view_change(vc0), None);

    // Same voter again
    let vc0_dup = ViewChange::new(1, 0, 1, &keys[0]);
    assert_eq!(collector.add_view_change(vc0_dup), None);
    assert_eq!(collector.count_for_round(1), 1); // Still just 1
}

#[test]
fn view_change_collector_rejects_unknown_validator() {
    let _keys = make_signing_keys(3);
    let validators = make_validators(3);

    let mut collector = ViewChangeCollector::new(1, &validators);

    // Unknown validator (seed=99)
    let unknown_key = make_signing_key(99);
    let vc = ViewChange::new(1, 0, 1, &unknown_key);
    assert_eq!(collector.add_view_change(vc), None);
    assert_eq!(collector.count_for_round(1), 0);
}
