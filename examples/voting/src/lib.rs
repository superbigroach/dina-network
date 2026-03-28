// =============================================================================
// On-Chain Voting Contract — Transparent, Tamper-Proof Polls
// =============================================================================
//
// This contract enables transparent, on-chain voting where:
//   - Anyone can create a poll with multiple options
//   - Each address can vote exactly once per poll (one-person-one-vote)
//   - Vote counts are publicly verifiable at any time
//   - Only the poll creator can close the poll
//   - Polls have an end time after which no new votes are accepted
//
// Use cases on Dina Network:
//   - DAO governance decisions
//   - Agent swarm consensus voting
//   - Community feature prioritization
//   - Multi-sig approval workflows
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// =============================================================================
// Types
// =============================================================================

/// Unique identifier for each poll.
pub type PollId = u64;

/// A single poll with its options, votes, and metadata.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Poll {
    /// The poll's title/question (e.g. "Which protocol upgrade should we ship?")
    pub title: String,

    /// The available options to vote for (e.g. ["Option A", "Option B", "Option C"]).
    /// Voters reference options by their index (0, 1, 2, ...).
    pub options: Vec<String>,

    /// Records which address voted for which option index.
    /// BTreeMap ensures deterministic iteration order for consensus.
    /// Key: voter address, Value: option index they voted for.
    pub votes: BTreeMap<[u8; 32], usize>,

    /// The address that created this poll. Only they can close it.
    pub creator: [u8; 32],

    /// Unix timestamp (seconds) after which the poll stops accepting votes.
    /// Set to 0 for no expiry.
    pub end_time: u64,

    /// Whether the poll has been explicitly closed by its creator.
    pub closed: bool,
}

// =============================================================================
// Contract State
// =============================================================================

/// The complete on-chain state for the Voting contract.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VotingState {
    /// All polls, keyed by their unique ID.
    pub polls: BTreeMap<PollId, Poll>,
    /// Auto-incrementing counter for generating unique poll IDs.
    pub next_id: PollId,
    /// The contract deployer's address.
    pub owner: [u8; 32],
}

// =============================================================================
// Contract Methods
// =============================================================================

impl VotingState {
    pub fn new(owner: [u8; 32]) -> Self {
        Self {
            polls: BTreeMap::new(),
            next_id: 1,
            owner,
        }
    }

    /// Create a new poll.
    ///
    /// # Arguments
    /// * `caller` — The poll creator's address
    /// * `title` — The poll question
    /// * `options` — List of options to vote for (minimum 2)
    /// * `end_time` — Unix timestamp when voting ends (0 for no expiry)
    ///
    /// # Returns
    /// The new poll's unique ID.
    pub fn create_poll(
        &mut self,
        caller: [u8; 32],
        title: String,
        options: Vec<String>,
        end_time: u64,
    ) -> PollId {
        assert!(!title.is_empty(), "Voting: title cannot be empty");
        assert!(
            options.len() >= 2,
            "Voting: must provide at least 2 options"
        );

        let id = self.next_id;
        self.next_id += 1;

        let poll = Poll {
            title,
            options,
            votes: BTreeMap::new(),
            creator: caller,
            end_time,
            closed: false,
        };

        self.polls.insert(id, poll);
        id
    }

    /// Cast a vote on a poll.
    ///
    /// Each address can only vote once per poll. Attempting to vote again
    /// will panic. This prevents ballot stuffing.
    ///
    /// # Arguments
    /// * `caller` — The voter's address
    /// * `poll_id` — Which poll to vote on
    /// * `option_index` — Index into the poll's `options` array (0-based)
    /// * `current_time` — Current Unix timestamp (provided by the VM)
    pub fn vote(
        &mut self,
        caller: [u8; 32],
        poll_id: PollId,
        option_index: usize,
        current_time: u64,
    ) {
        let poll = self
            .polls
            .get_mut(&poll_id)
            .expect("Voting: poll not found");

        // Check the poll is still open
        assert!(!poll.closed, "Voting: poll is closed");

        // Check the poll hasn't expired
        if poll.end_time > 0 {
            assert!(
                current_time <= poll.end_time,
                "Voting: poll has expired (ended at {})",
                poll.end_time
            );
        }

        // Validate the option index
        assert!(
            option_index < poll.options.len(),
            "Voting: invalid option index {} (poll has {} options)",
            option_index,
            poll.options.len()
        );

        // Enforce one vote per address — this is the key invariant
        assert!(
            !poll.votes.contains_key(&caller),
            "Voting: address has already voted on this poll"
        );

        // Record the vote
        poll.votes.insert(caller, option_index);
    }

    /// Get the current vote counts for a poll.
    ///
    /// Returns a vector of (option_name, vote_count) pairs, one for each option.
    /// Results are available in real-time — you don't have to wait for the poll
    /// to close.
    pub fn get_results(&self, poll_id: PollId) -> Vec<(String, u64)> {
        let poll = self.polls.get(&poll_id).expect("Voting: poll not found");

        // Count votes for each option
        let mut counts = vec![0u64; poll.options.len()];
        for &option_index in poll.votes.values() {
            counts[option_index] += 1;
        }

        // Zip option names with their vote counts
        poll.options
            .iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), counts[i]))
            .collect()
    }

    /// Close a poll. Only the poll creator can do this.
    ///
    /// After closing, no more votes are accepted. Results remain queryable.
    pub fn close_poll(&mut self, caller: [u8; 32], poll_id: PollId) {
        let poll = self
            .polls
            .get_mut(&poll_id)
            .expect("Voting: poll not found");

        assert!(
            caller == poll.creator,
            "Voting: only the poll creator can close it"
        );
        assert!(!poll.closed, "Voting: poll is already closed");

        poll.closed = true;
    }

    /// Get a poll by ID.
    pub fn get_poll(&self, poll_id: PollId) -> Option<&Poll> {
        self.polls.get(&poll_id)
    }
}

// =============================================================================
// Dispatch Argument Types
// =============================================================================

#[derive(Serialize, Deserialize, Debug)]
struct CreatePollArgs {
    title: String,
    options: Vec<String>,
    end_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct VoteArgs {
    poll_id: PollId,
    option_index: usize,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct PollIdArgs {
    poll_id: PollId,
}

/// A single result entry returned from get_results.
#[derive(Serialize, Deserialize, Debug)]
struct ResultEntry {
    option: String,
    votes: u64,
}

// =============================================================================
// Dispatch Function
// =============================================================================

pub fn dispatch(
    state: &mut Option<VotingState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "Voting: already initialised");
            *state = Some(VotingState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "create_poll" => {
            let s = state.as_mut().expect("Voting: not initialised");
            let a: CreatePollArgs =
                serde_json::from_slice(args).expect("Voting: bad create_poll args");
            let id = s.create_poll(caller, a.title, a.options, a.end_time);
            serde_json::to_vec(&id).unwrap()
        }

        "vote" => {
            let s = state.as_mut().expect("Voting: not initialised");
            let a: VoteArgs = serde_json::from_slice(args).expect("Voting: bad vote args");
            s.vote(caller, a.poll_id, a.option_index, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }

        "get_results" => {
            let s = state.as_ref().expect("Voting: not initialised");
            let a: PollIdArgs = serde_json::from_slice(args).expect("Voting: bad get_results args");
            let results: Vec<ResultEntry> = s
                .get_results(a.poll_id)
                .into_iter()
                .map(|(option, votes)| ResultEntry { option, votes })
                .collect();
            serde_json::to_vec(&results).unwrap()
        }

        "close_poll" => {
            let s = state.as_mut().expect("Voting: not initialised");
            let a: PollIdArgs = serde_json::from_slice(args).expect("Voting: bad close_poll args");
            s.close_poll(caller, a.poll_id);
            serde_json::to_vec("ok").unwrap()
        }

        "get_poll" => {
            let s = state.as_ref().expect("Voting: not initialised");
            let a: PollIdArgs = serde_json::from_slice(args).expect("Voting: bad get_poll args");
            serde_json::to_vec(&s.get_poll(a.poll_id)).unwrap()
        }

        _ => panic!("Voting: unknown method '{method}'"),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    #[test]
    fn test_create_and_vote() {
        let mut state: Option<VotingState> = None;
        let creator = addr(1);

        dispatch(&mut state, "init", b"{}", creator);

        // Create a poll
        let create_args = serde_json::to_vec(&CreatePollArgs {
            title: "Best language?".to_string(),
            options: vec!["Rust".to_string(), "Go".to_string(), "Python".to_string()],
            end_time: 0, // no expiry
        })
        .unwrap();
        let result = dispatch(&mut state, "create_poll", &create_args, creator);
        let poll_id: PollId = serde_json::from_slice(&result).unwrap();
        assert_eq!(poll_id, 1);

        // Three voters vote
        for (i, seed) in [10u8, 11, 12].iter().enumerate() {
            let vote_args = serde_json::to_vec(&VoteArgs {
                poll_id: 1,
                option_index: if i < 2 { 0 } else { 1 }, // 2 for Rust, 1 for Go
                current_time: 1000,
            })
            .unwrap();
            dispatch(&mut state, "vote", &vote_args, addr(*seed));
        }

        // Check results
        let results_args = serde_json::to_vec(&PollIdArgs { poll_id: 1 }).unwrap();
        let result = dispatch(&mut state, "get_results", &results_args, creator);
        let results: Vec<ResultEntry> = serde_json::from_slice(&result).unwrap();

        assert_eq!(results[0].option, "Rust");
        assert_eq!(results[0].votes, 2);
        assert_eq!(results[1].option, "Go");
        assert_eq!(results[1].votes, 1);
        assert_eq!(results[2].option, "Python");
        assert_eq!(results[2].votes, 0);
    }

    #[test]
    #[should_panic(expected = "already voted")]
    fn test_double_vote_fails() {
        let mut state: Option<VotingState> = None;
        let creator = addr(1);
        let voter = addr(10);

        dispatch(&mut state, "init", b"{}", creator);

        let create_args = serde_json::to_vec(&CreatePollArgs {
            title: "Test".to_string(),
            options: vec!["A".to_string(), "B".to_string()],
            end_time: 0,
        })
        .unwrap();
        dispatch(&mut state, "create_poll", &create_args, creator);

        let vote_args = serde_json::to_vec(&VoteArgs {
            poll_id: 1,
            option_index: 0,
            current_time: 1000,
        })
        .unwrap();

        // First vote succeeds
        dispatch(&mut state, "vote", &vote_args, voter);
        // Second vote panics
        dispatch(&mut state, "vote", &vote_args, voter);
    }

    #[test]
    #[should_panic(expected = "poll is closed")]
    fn test_vote_on_closed_poll_fails() {
        let mut state: Option<VotingState> = None;
        let creator = addr(1);
        let voter = addr(10);

        dispatch(&mut state, "init", b"{}", creator);

        let create_args = serde_json::to_vec(&CreatePollArgs {
            title: "Test".to_string(),
            options: vec!["A".to_string(), "B".to_string()],
            end_time: 0,
        })
        .unwrap();
        dispatch(&mut state, "create_poll", &create_args, creator);

        // Close the poll
        let close_args = serde_json::to_vec(&PollIdArgs { poll_id: 1 }).unwrap();
        dispatch(&mut state, "close_poll", &close_args, creator);

        // Try to vote — should fail
        let vote_args = serde_json::to_vec(&VoteArgs {
            poll_id: 1,
            option_index: 0,
            current_time: 1000,
        })
        .unwrap();
        dispatch(&mut state, "vote", &vote_args, voter);
    }

    #[test]
    #[should_panic(expected = "poll has expired")]
    fn test_vote_after_expiry_fails() {
        let mut state: Option<VotingState> = None;
        let creator = addr(1);
        let voter = addr(10);

        dispatch(&mut state, "init", b"{}", creator);

        let create_args = serde_json::to_vec(&CreatePollArgs {
            title: "Test".to_string(),
            options: vec!["A".to_string(), "B".to_string()],
            end_time: 5000, // expires at t=5000
        })
        .unwrap();
        dispatch(&mut state, "create_poll", &create_args, creator);

        // Try to vote after expiry
        let vote_args = serde_json::to_vec(&VoteArgs {
            poll_id: 1,
            option_index: 0,
            current_time: 6000, // after end_time
        })
        .unwrap();
        dispatch(&mut state, "vote", &vote_args, voter);
    }
}
