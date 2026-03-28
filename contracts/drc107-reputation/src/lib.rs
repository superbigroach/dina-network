use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-107  Reputation Score
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum InteractionOutcome {
    Success,
    Failure,
    Disputed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InteractionRecord {
    pub counterparty: [u8; 32],
    pub outcome: InteractionOutcome,
    pub rating: u16, // 0-10000 (basis points)
    pub volume: u64,
    pub timestamp: u64,
    pub category: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReputationDetails {
    pub overall_score: u16, // 0-10000
    pub total_interactions: u64,
    pub successful: u64,
    pub failed: u64,
    pub disputed: u64,
    pub total_volume: u64,
    pub average_rating: u16, // 0-10000
    pub categories: BTreeMap<String, CategoryScore>,
    pub interactions: Vec<InteractionRecord>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CategoryScore {
    pub score: u16,
    pub count: u64,
    pub total_rating: u64,
}

impl Default for ReputationDetails {
    fn default() -> Self {
        Self::new()
    }
}

impl ReputationDetails {
    pub fn new() -> Self {
        Self {
            overall_score: 5000, // start at neutral 50%
            total_interactions: 0,
            successful: 0,
            failed: 0,
            disputed: 0,
            total_volume: 0,
            average_rating: 5000,
            categories: BTreeMap::new(),
            interactions: Vec::new(),
        }
    }

    /// Recalculate overall_score weighted by recency and volume.
    fn recalculate(&mut self) {
        if self.interactions.is_empty() {
            self.overall_score = 5000;
            self.average_rating = 5000;
            return;
        }

        // Sort interactions by timestamp (oldest first) so most recent get
        // highest weight.
        let mut sorted = self.interactions.clone();
        sorted.sort_by_key(|i| i.timestamp);

        let n = sorted.len() as f64;
        let mut weighted_sum: f64 = 0.0;
        let mut weight_total: f64 = 0.0;

        for (idx, interaction) in sorted.iter().enumerate() {
            // Recency weight: linearly from 1.0 (oldest) to 2.0 (newest)
            let recency_weight = 1.0 + (idx as f64) / n;

            // Volume weight: log2(volume + 1) capped at 20
            let volume_weight = ((interaction.volume as f64 + 1.0).log2()).clamp(1.0, 20.0);

            // Outcome multiplier
            let outcome_score: f64 = match interaction.outcome {
                InteractionOutcome::Success => interaction.rating as f64,
                InteractionOutcome::Failure => {
                    // Failed interactions get a penalty: rating is halved
                    (interaction.rating as f64) * 0.5
                }
                InteractionOutcome::Disputed => {
                    // Disputed interactions get moderate penalty
                    (interaction.rating as f64) * 0.7
                }
            };

            let w = recency_weight * volume_weight;
            weighted_sum += outcome_score * w;
            weight_total += w;
        }

        self.overall_score = if weight_total > 0.0 {
            ((weighted_sum / weight_total) as u16).min(10000)
        } else {
            5000
        };

        // Simple average rating (unweighted)
        let total_rating: u64 = self.interactions.iter().map(|i| i.rating as u64).sum();
        self.average_rating = (total_rating / self.interactions.len() as u64) as u16;
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReputationState {
    pub reputations: BTreeMap<[u8; 32], ReputationDetails>,
}

impl Default for ReputationState {
    fn default() -> Self {
        Self::new()
    }
}

impl ReputationState {
    pub fn new() -> Self {
        Self {
            reputations: BTreeMap::new(),
        }
    }

    pub fn reputation_of(&self, account: &[u8; 32]) -> u16 {
        self.reputations
            .get(account)
            .map(|r| r.overall_score)
            .unwrap_or(5000) // default neutral score
    }

    pub fn reputation_details(&self, account: &[u8; 32]) -> ReputationDetails {
        self.reputations
            .get(account)
            .cloned()
            .unwrap_or_else(ReputationDetails::new)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_interaction(
        &mut self,
        caller: [u8; 32],
        subject: [u8; 32],
        counterparty: [u8; 32],
        outcome: InteractionOutcome,
        rating: u16,
        volume: u64,
        timestamp: u64,
        category: String,
    ) {
        assert!(rating <= 10000, "DRC107: rating must be 0-10000");
        assert!(subject != counterparty, "DRC107: cannot rate yourself");
        // In a real deployment, `caller` would be verified as an authorised
        // oracle or the service-agreement contract. For now we accept any caller.
        let _ = caller;

        let details = self.reputations.entry(subject).or_default();

        details.total_interactions += 1;
        details.total_volume += volume;

        match outcome {
            InteractionOutcome::Success => details.successful += 1,
            InteractionOutcome::Failure => details.failed += 1,
            InteractionOutcome::Disputed => details.disputed += 1,
        }

        // Update category score
        let cat = details
            .categories
            .entry(category.clone())
            .or_insert(CategoryScore {
                score: 5000,
                count: 0,
                total_rating: 0,
            });
        cat.count += 1;
        cat.total_rating += rating as u64;
        cat.score = (cat.total_rating / cat.count) as u16;

        details.interactions.push(InteractionRecord {
            counterparty,
            outcome,
            rating,
            volume,
            timestamp,
            category,
        });

        details.recalculate();
    }

    pub fn meets_threshold(&self, account: &[u8; 32], threshold: u16) -> bool {
        self.reputation_of(account) >= threshold
    }
}

// ---------------------------------------------------------------------------
// Dispatch arg types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct AccountArgs {
    account: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct RecordInteractionArgs {
    subject: [u8; 32],
    counterparty: [u8; 32],
    outcome: InteractionOutcome,
    rating: u16,
    volume: u64,
    timestamp: u64,
    category: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct MeetsThresholdArgs {
    account: [u8; 32],
    threshold: u16,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<ReputationState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC107: already initialised");
            *state = Some(ReputationState::new());
            serde_json::to_vec("ok").unwrap()
        }

        "reputation_of" => {
            let s = state.as_ref().expect("DRC107: not initialised");
            let a: AccountArgs =
                serde_json::from_slice(args).expect("DRC107: bad reputation_of args");
            serde_json::to_vec(&s.reputation_of(&a.account)).unwrap()
        }

        "reputation_details" => {
            let s = state.as_ref().expect("DRC107: not initialised");
            let a: AccountArgs =
                serde_json::from_slice(args).expect("DRC107: bad reputation_details args");
            serde_json::to_vec(&s.reputation_details(&a.account)).unwrap()
        }

        "record_interaction" => {
            let s = state.as_mut().expect("DRC107: not initialised");
            let a: RecordInteractionArgs =
                serde_json::from_slice(args).expect("DRC107: bad record_interaction args");
            s.record_interaction(
                caller,
                a.subject,
                a.counterparty,
                a.outcome,
                a.rating,
                a.volume,
                a.timestamp,
                a.category,
            );
            serde_json::to_vec("ok").unwrap()
        }

        "meets_threshold" => {
            let s = state.as_ref().expect("DRC107: not initialised");
            let a: MeetsThresholdArgs =
                serde_json::from_slice(args).expect("DRC107: bad meets_threshold args");
            serde_json::to_vec(&s.meets_threshold(&a.account, a.threshold)).unwrap()
        }

        _ => panic!("DRC107: unknown method '{method}'"),
    }
}
