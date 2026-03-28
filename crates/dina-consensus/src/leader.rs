use tracing::debug;

/// Deterministic leader rotation schedule using round-robin.
///
/// The leader for a given (height, round) is selected by indexing into the
/// ordered validator set. This ensures a fair, predictable rotation that all
/// nodes can independently compute without communication.
#[derive(Debug, Clone)]
pub struct LeaderSchedule;

impl LeaderSchedule {
    /// Determine the leader for a given height and round using round-robin.
    ///
    /// The formula is: `validators[(height + round) % n]`
    ///
    /// This means:
    /// - At round 0, the leader rotates through validators as height increases.
    /// - If a round times out and increments, the next validator takes over,
    ///   providing liveness even when a leader is unresponsive.
    ///
    /// # Panics
    /// Panics if `validators` is empty.
    pub fn leader_for(height: u64, round: u32, validators: &[[u8; 32]]) -> [u8; 32] {
        assert!(!validators.is_empty(), "Validator set must not be empty");

        let n = validators.len() as u64;
        let index = ((height + round as u64) % n) as usize;

        debug!(
            height,
            round,
            index,
            leader = hex::encode(validators[index]),
            "Selected leader for round"
        );

        validators[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_validators(count: usize) -> Vec<[u8; 32]> {
        (0..count)
            .map(|i| {
                let mut key = [0u8; 32];
                key[0] = i as u8;
                key
            })
            .collect()
    }

    #[test]
    fn test_round_robin_basic() {
        let validators = make_validators(4);

        // height=0, round=0 => index 0
        assert_eq!(LeaderSchedule::leader_for(0, 0, &validators), validators[0]);
        // height=1, round=0 => index 1
        assert_eq!(LeaderSchedule::leader_for(1, 0, &validators), validators[1]);
        // height=4, round=0 => wraps back to index 0
        assert_eq!(LeaderSchedule::leader_for(4, 0, &validators), validators[0]);
    }

    #[test]
    fn test_round_increment_changes_leader() {
        let validators = make_validators(4);

        // height=0, round=0 => index 0
        assert_eq!(LeaderSchedule::leader_for(0, 0, &validators), validators[0]);
        // height=0, round=1 => index 1 (view change rotates leader)
        assert_eq!(LeaderSchedule::leader_for(0, 1, &validators), validators[1]);
        // height=0, round=2 => index 2
        assert_eq!(LeaderSchedule::leader_for(0, 2, &validators), validators[2]);
    }

    #[test]
    fn test_single_validator() {
        let validators = make_validators(1);
        // Single validator is always leader
        assert_eq!(LeaderSchedule::leader_for(0, 0, &validators), validators[0]);
        assert_eq!(
            LeaderSchedule::leader_for(100, 5, &validators),
            validators[0]
        );
    }

    #[test]
    #[should_panic(expected = "Validator set must not be empty")]
    fn test_empty_validators_panics() {
        let validators: Vec<[u8; 32]> = vec![];
        LeaderSchedule::leader_for(0, 0, &validators);
    }
}
