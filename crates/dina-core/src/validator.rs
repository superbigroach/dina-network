use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::{DinaError, DinaResult};
use crate::types::Address;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default minimum stake: 10,000 USDC (in micro-USDC).
pub const DEFAULT_MIN_STAKE: u64 = 10_000_000_000; // 10,000 * 1_000_000

/// Default maximum number of active validators.
pub const DEFAULT_MAX_VALIDATORS: usize = 7;

/// Default epoch length in blocks.
pub const DEFAULT_EPOCH_LENGTH: u64 = 1_000;

/// Number of epochs a validator must wait while unbonding.
pub const UNBONDING_EPOCHS: u64 = 3;

/// Maximum commission in basis points (50%).
pub const MAX_COMMISSION_BPS: u16 = 5_000;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// The set of all validators on the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSet {
    validators: BTreeMap<Address, ValidatorInfo>,
    min_stake: u64,
    max_validators: usize,
    epoch_length_blocks: u64,
    current_epoch: u64,
}

/// Information about a single validator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub address: Address,
    pub pubkey: [u8; 32],
    pub stake: u64,
    pub commission_bps: u16,
    pub blocks_proposed: u64,
    pub blocks_missed: u64,
    pub rewards_earned: u64,
    pub slashed_amount: u64,
    pub joined_epoch: u64,
    pub status: ValidatorStatus,
    pub metadata: ValidatorMetadata,
}

/// Current status of a validator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidatorStatus {
    Active,
    Inactive,
    Jailed,
    Unbonding { until_epoch: u64 },
}

/// Human-readable metadata about a validator.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorMetadata {
    pub name: String,
    pub website: String,
    pub description: String,
    pub logo_url: String,
}

/// The result of distributing rewards for a single epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpochRewards {
    pub epoch: u64,
    pub total_fees: u64,
    pub validator_rewards: BTreeMap<Address, u64>,
    pub treasury_amount: u64,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl ValidatorSet {
    /// Create a new, empty validator set.
    pub fn new(min_stake: u64, max_validators: usize, epoch_length_blocks: u64) -> Self {
        Self {
            validators: BTreeMap::new(),
            min_stake,
            max_validators,
            epoch_length_blocks,
            current_epoch: 0,
        }
    }

    /// Create a validator set with sensible defaults.
    pub fn default_config() -> Self {
        Self::new(
            DEFAULT_MIN_STAKE,
            DEFAULT_MAX_VALIDATORS,
            DEFAULT_EPOCH_LENGTH,
        )
    }

    /// Register a new validator. Fails if stake is below minimum, if the
    /// validator set is full, or if the address is already registered.
    pub fn register_validator(
        &mut self,
        address: Address,
        pubkey: [u8; 32],
        stake: u64,
        commission_bps: u16,
        metadata: ValidatorMetadata,
    ) -> DinaResult<()> {
        if stake < self.min_stake {
            return Err(DinaError::ValidatorError(format!(
                "stake {} below minimum {}",
                stake, self.min_stake
            )));
        }

        if commission_bps > MAX_COMMISSION_BPS {
            return Err(DinaError::ValidatorError(format!(
                "commission {} bps exceeds maximum {} bps",
                commission_bps, MAX_COMMISSION_BPS
            )));
        }

        if self.validators.contains_key(&address) {
            return Err(DinaError::ValidatorError(
                "validator already registered".to_string(),
            ));
        }

        let active_count = self.active_validators().len();
        if active_count >= self.max_validators {
            return Err(DinaError::ValidatorError(format!(
                "validator set full ({}/{})",
                active_count, self.max_validators
            )));
        }

        let info = ValidatorInfo {
            address,
            pubkey,
            stake,
            commission_bps,
            blocks_proposed: 0,
            blocks_missed: 0,
            rewards_earned: 0,
            slashed_amount: 0,
            joined_epoch: self.current_epoch,
            status: ValidatorStatus::Active,
            metadata,
        };

        self.validators.insert(address, info);
        Ok(())
    }

    /// Begin unbonding a validator. They remain in the set but stop
    /// participating after the current epoch; their stake unlocks after
    /// `UNBONDING_EPOCHS` epochs.
    pub fn unregister_validator(&mut self, address: &Address) -> DinaResult<()> {
        let validator = self
            .validators
            .get_mut(address)
            .ok_or_else(|| DinaError::ValidatorError("validator not found".to_string()))?;

        match validator.status {
            ValidatorStatus::Active => {}
            _ => {
                return Err(DinaError::ValidatorError(
                    "validator is not active".to_string(),
                ));
            }
        }

        validator.status = ValidatorStatus::Unbonding {
            until_epoch: self.current_epoch + UNBONDING_EPOCHS,
        };
        Ok(())
    }

    /// Slash a validator's stake by `amount`. If their remaining stake drops
    /// below the minimum they are automatically jailed.
    pub fn slash_validator(
        &mut self,
        address: &Address,
        amount: u64,
        _reason: &str,
    ) -> DinaResult<()> {
        let validator = self
            .validators
            .get_mut(address)
            .ok_or_else(|| DinaError::ValidatorError("validator not found".to_string()))?;

        let slash = amount.min(validator.stake);
        validator.stake -= slash;
        validator.slashed_amount += slash;

        if validator.stake < self.min_stake {
            validator.status = ValidatorStatus::Jailed;
        }

        Ok(())
    }

    /// Jail a validator (e.g. for misbehavior or excessive missed blocks).
    pub fn jail_validator(&mut self, address: &Address) -> DinaResult<()> {
        let validator = self
            .validators
            .get_mut(address)
            .ok_or_else(|| DinaError::ValidatorError("validator not found".to_string()))?;

        validator.status = ValidatorStatus::Jailed;
        Ok(())
    }

    /// Unjail a validator. They must have at least `min_stake` to rejoin.
    pub fn unjail_validator(&mut self, address: &Address) -> DinaResult<()> {
        let validator = self
            .validators
            .get_mut(address)
            .ok_or_else(|| DinaError::ValidatorError("validator not found".to_string()))?;

        if validator.status != ValidatorStatus::Jailed {
            return Err(DinaError::ValidatorError(
                "validator is not jailed".to_string(),
            ));
        }

        if validator.stake < self.min_stake {
            return Err(DinaError::ValidatorError(format!(
                "stake {} below minimum {} -- must re-stake before unjailing",
                validator.stake, self.min_stake
            )));
        }

        validator.status = ValidatorStatus::Active;
        Ok(())
    }

    /// Record that a validator successfully proposed a block.
    pub fn record_block_proposed(&mut self, address: &Address) {
        if let Some(v) = self.validators.get_mut(address) {
            v.blocks_proposed += 1;
        }
    }

    /// Record that a validator missed their block proposal slot.
    pub fn record_block_missed(&mut self, address: &Address) {
        if let Some(v) = self.validators.get_mut(address) {
            v.blocks_missed += 1;
        }
    }

    /// Distribute `total_fees` to active validators proportional to their
    /// stake. Each validator's commission is subtracted and kept; the
    /// remainder goes to a conceptual "delegator pool" (here, back to the
    /// validator since there is no delegation layer yet). 20% of total fees
    /// goes to the treasury.
    pub fn distribute_epoch_rewards(&mut self, total_fees: u64) -> EpochRewards {
        let treasury_bps: u64 = 2_000; // 20%
        let treasury_amount = total_fees * treasury_bps / 10_000;
        let validator_pool = total_fees - treasury_amount;

        let active: Vec<Address> = self.active_validators().iter().map(|v| v.address).collect();

        let total_stake: u64 = active.iter().map(|a| self.validators[a].stake).sum();

        let mut rewards_map = BTreeMap::new();

        if total_stake > 0 {
            let mut distributed = 0u64;
            for (i, addr) in active.iter().enumerate() {
                let stake = self.validators[addr].stake;
                let reward = if i == active.len() - 1 {
                    // Last validator gets the remainder to avoid rounding loss
                    validator_pool - distributed
                } else {
                    validator_pool * stake / total_stake
                };
                distributed += reward;

                self.validators.get_mut(addr).unwrap().rewards_earned += reward;
                rewards_map.insert(*addr, reward);
            }
        }

        EpochRewards {
            epoch: self.current_epoch,
            total_fees,
            validator_rewards: rewards_map,
            treasury_amount,
        }
    }

    /// Return all active validators, sorted descending by stake.
    pub fn active_validators(&self) -> Vec<&ValidatorInfo> {
        let mut active: Vec<&ValidatorInfo> = self
            .validators
            .values()
            .filter(|v| v.status == ValidatorStatus::Active)
            .collect();
        active.sort_by(|a, b| b.stake.cmp(&a.stake));
        active
    }

    /// Check if a validator is active.
    pub fn is_active(&self, address: &Address) -> bool {
        self.validators
            .get(address)
            .is_some_and(|v| v.status == ValidatorStatus::Active)
    }

    /// Get a validator by address.
    pub fn get_validator(&self, address: &Address) -> Option<&ValidatorInfo> {
        self.validators.get(address)
    }

    /// Total USDC staked across all validators (including jailed/unbonding).
    pub fn total_staked(&self) -> u64 {
        self.validators.values().map(|v| v.stake).sum()
    }

    /// Advance to the next epoch: process unbonding completions, then
    /// distribute rewards from `total_fees` collected during this epoch.
    pub fn advance_epoch(&mut self, total_fees: u64) -> EpochRewards {
        self.current_epoch += 1;

        // Complete any unbonding periods that have finished
        let addrs: Vec<Address> = self.validators.keys().copied().collect();
        for addr in addrs {
            if let ValidatorStatus::Unbonding { until_epoch } = self.validators[&addr].status {
                if self.current_epoch >= until_epoch {
                    self.validators.get_mut(&addr).unwrap().status = ValidatorStatus::Inactive;
                }
            }
        }

        // Distribute rewards
        self.distribute_epoch_rewards(total_fees)
    }

    /// Current epoch number.
    pub fn current_epoch(&self) -> u64 {
        self.current_epoch
    }

    /// Epoch length in blocks.
    pub fn epoch_length_blocks(&self) -> u64 {
        self.epoch_length_blocks
    }

    /// Minimum required stake.
    pub fn min_stake(&self) -> u64 {
        self.min_stake
    }

    /// Number of registered validators (all statuses).
    pub fn validator_count(&self) -> usize {
        self.validators.len()
    }

    /// Add stake to an existing validator.
    pub fn add_stake(&mut self, address: &Address, amount: u64) -> DinaResult<()> {
        let validator = self
            .validators
            .get_mut(address)
            .ok_or_else(|| DinaError::ValidatorError("validator not found".to_string()))?;

        validator.stake += amount;
        Ok(())
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

    fn meta(name: &str) -> ValidatorMetadata {
        ValidatorMetadata {
            name: name.to_string(),
            ..Default::default()
        }
    }

    fn test_set() -> ValidatorSet {
        // Use small stakes for test convenience (1000 micro-USDC minimum)
        ValidatorSet::new(1_000, 7, 100)
    }

    fn register(set: &mut ValidatorSet, byte: u8, stake: u64) {
        set.register_validator(
            addr(byte),
            [byte; 32],
            stake,
            500,
            meta(&format!("V{byte}")),
        )
        .unwrap();
    }

    // -- Registration -------------------------------------------------------

    #[test]
    fn register_validator_success() {
        let mut set = test_set();
        register(&mut set, 1, 5_000);
        assert_eq!(set.validator_count(), 1);
        assert!(set.is_active(&addr(1)));
    }

    #[test]
    fn register_below_min_stake_fails() {
        let mut set = test_set();
        let err = set
            .register_validator(addr(1), [1; 32], 500, 500, meta("V1"))
            .unwrap_err();
        assert!(matches!(err, DinaError::ValidatorError(_)));
    }

    #[test]
    fn register_duplicate_fails() {
        let mut set = test_set();
        register(&mut set, 1, 5_000);
        let err = set
            .register_validator(addr(1), [1; 32], 5_000, 500, meta("V1"))
            .unwrap_err();
        assert!(matches!(err, DinaError::ValidatorError(_)));
    }

    #[test]
    fn register_full_set_fails() {
        let mut set = ValidatorSet::new(1_000, 2, 100);
        register(&mut set, 1, 5_000);
        register(&mut set, 2, 5_000);
        let err = set
            .register_validator(addr(3), [3; 32], 5_000, 500, meta("V3"))
            .unwrap_err();
        assert!(matches!(err, DinaError::ValidatorError(_)));
    }

    #[test]
    fn register_excessive_commission_fails() {
        let mut set = test_set();
        let err = set
            .register_validator(addr(1), [1; 32], 5_000, 6_000, meta("V1"))
            .unwrap_err();
        assert!(matches!(err, DinaError::ValidatorError(_)));
    }

    // -- Unregister / Unbonding ---------------------------------------------

    #[test]
    fn unregister_starts_unbonding() {
        let mut set = test_set();
        register(&mut set, 1, 5_000);
        set.unregister_validator(&addr(1)).unwrap();
        let v = set.get_validator(&addr(1)).unwrap();
        assert!(matches!(v.status, ValidatorStatus::Unbonding { .. }));
        assert!(!set.is_active(&addr(1)));
    }

    #[test]
    fn unregister_nonexistent_fails() {
        let mut set = test_set();
        assert!(set.unregister_validator(&addr(99)).is_err());
    }

    #[test]
    fn unregister_inactive_fails() {
        let mut set = test_set();
        register(&mut set, 1, 5_000);
        set.unregister_validator(&addr(1)).unwrap();
        // Already unbonding, cannot unregister again
        assert!(set.unregister_validator(&addr(1)).is_err());
    }

    // -- Slashing -----------------------------------------------------------

    #[test]
    fn slash_reduces_stake() {
        let mut set = test_set();
        register(&mut set, 1, 5_000);
        set.slash_validator(&addr(1), 1_000, "double sign").unwrap();
        let v = set.get_validator(&addr(1)).unwrap();
        assert_eq!(v.stake, 4_000);
        assert_eq!(v.slashed_amount, 1_000);
        // Still active because 4000 >= 1000 min
        assert_eq!(v.status, ValidatorStatus::Active);
    }

    #[test]
    fn slash_below_minimum_jails() {
        let mut set = test_set();
        register(&mut set, 1, 2_000);
        set.slash_validator(&addr(1), 1_500, "downtime").unwrap();
        let v = set.get_validator(&addr(1)).unwrap();
        assert_eq!(v.stake, 500);
        assert_eq!(v.status, ValidatorStatus::Jailed);
    }

    #[test]
    fn slash_more_than_stake_zeros_out() {
        let mut set = test_set();
        register(&mut set, 1, 2_000);
        set.slash_validator(&addr(1), 100_000, "severe").unwrap();
        let v = set.get_validator(&addr(1)).unwrap();
        assert_eq!(v.stake, 0);
        assert_eq!(v.slashed_amount, 2_000);
    }

    // -- Jail / Unjail ------------------------------------------------------

    #[test]
    fn jail_and_unjail() {
        let mut set = test_set();
        register(&mut set, 1, 5_000);
        set.jail_validator(&addr(1)).unwrap();
        assert!(!set.is_active(&addr(1)));
        set.unjail_validator(&addr(1)).unwrap();
        assert!(set.is_active(&addr(1)));
    }

    #[test]
    fn unjail_below_min_stake_fails() {
        let mut set = test_set();
        register(&mut set, 1, 2_000);
        set.slash_validator(&addr(1), 1_500, "bad").unwrap();
        // Now jailed with 500 stake, can't unjail without re-staking
        let err = set.unjail_validator(&addr(1)).unwrap_err();
        assert!(matches!(err, DinaError::ValidatorError(_)));
    }

    #[test]
    fn unjail_non_jailed_fails() {
        let mut set = test_set();
        register(&mut set, 1, 5_000);
        assert!(set.unjail_validator(&addr(1)).is_err());
    }

    // -- Block recording ----------------------------------------------------

    #[test]
    fn record_blocks() {
        let mut set = test_set();
        register(&mut set, 1, 5_000);
        set.record_block_proposed(&addr(1));
        set.record_block_proposed(&addr(1));
        set.record_block_missed(&addr(1));
        let v = set.get_validator(&addr(1)).unwrap();
        assert_eq!(v.blocks_proposed, 2);
        assert_eq!(v.blocks_missed, 1);
    }

    // -- Reward distribution ------------------------------------------------

    #[test]
    fn distribute_rewards_single_validator() {
        let mut set = test_set();
        register(&mut set, 1, 10_000);
        let rewards = set.distribute_epoch_rewards(100_000);
        // 20% treasury = 20_000, validator gets 80_000
        assert_eq!(rewards.treasury_amount, 20_000);
        assert_eq!(rewards.validator_rewards[&addr(1)], 80_000);
    }

    #[test]
    fn distribute_rewards_proportional() {
        let mut set = test_set();
        register(&mut set, 1, 3_000);
        register(&mut set, 2, 7_000);
        let rewards = set.distribute_epoch_rewards(100_000);
        assert_eq!(rewards.treasury_amount, 20_000);
        // Validator pool = 80_000
        // V1: 80_000 * 3_000 / 10_000 = 24_000
        // V2 gets remainder: 80_000 - 24_000 = 56_000
        let v1_reward = rewards.validator_rewards[&addr(1)];
        let v2_reward = rewards.validator_rewards[&addr(2)];
        assert_eq!(v1_reward, 24_000);
        assert_eq!(v2_reward, 56_000);
        assert_eq!(v1_reward + v2_reward + rewards.treasury_amount, 100_000);
    }

    #[test]
    fn distribute_rewards_zero_fees() {
        let mut set = test_set();
        register(&mut set, 1, 5_000);
        let rewards = set.distribute_epoch_rewards(0);
        assert_eq!(rewards.treasury_amount, 0);
        assert_eq!(rewards.validator_rewards[&addr(1)], 0);
    }

    #[test]
    fn distribute_rewards_no_active_validators() {
        let mut set = test_set();
        let rewards = set.distribute_epoch_rewards(100_000);
        assert_eq!(rewards.treasury_amount, 20_000);
        assert!(rewards.validator_rewards.is_empty());
    }

    // -- Active validators --------------------------------------------------

    #[test]
    fn active_validators_sorted_by_stake() {
        let mut set = test_set();
        register(&mut set, 1, 2_000);
        register(&mut set, 2, 8_000);
        register(&mut set, 3, 5_000);
        let active = set.active_validators();
        assert_eq!(active.len(), 3);
        assert_eq!(active[0].address, addr(2));
        assert_eq!(active[1].address, addr(3));
        assert_eq!(active[2].address, addr(1));
    }

    // -- Epoch advancement --------------------------------------------------

    #[test]
    fn advance_epoch_completes_unbonding() {
        let mut set = test_set();
        register(&mut set, 1, 5_000);
        register(&mut set, 2, 5_000);
        set.unregister_validator(&addr(1)).unwrap();
        // Unbonding until epoch 0 + UNBONDING_EPOCHS = 3

        // Advance through epochs
        for _ in 0..UNBONDING_EPOCHS {
            set.advance_epoch(0);
        }

        let v = set.get_validator(&addr(1)).unwrap();
        assert_eq!(v.status, ValidatorStatus::Inactive);
    }

    #[test]
    fn advance_epoch_distributes_rewards() {
        let mut set = test_set();
        register(&mut set, 1, 5_000);
        let rewards = set.advance_epoch(50_000);
        assert_eq!(rewards.epoch, 1);
        assert_eq!(set.current_epoch(), 1);
        assert_eq!(rewards.total_fees, 50_000);
    }

    // -- Misc ---------------------------------------------------------------

    #[test]
    fn total_staked_includes_all() {
        let mut set = test_set();
        register(&mut set, 1, 3_000);
        register(&mut set, 2, 7_000);
        set.jail_validator(&addr(2)).unwrap();
        // Total includes jailed
        assert_eq!(set.total_staked(), 10_000);
    }

    #[test]
    fn add_stake_works() {
        let mut set = test_set();
        register(&mut set, 1, 5_000);
        set.add_stake(&addr(1), 3_000).unwrap();
        assert_eq!(set.get_validator(&addr(1)).unwrap().stake, 8_000);
    }

    #[test]
    fn add_stake_nonexistent_fails() {
        let mut set = test_set();
        assert!(set.add_stake(&addr(99), 1_000).is_err());
    }

    #[test]
    fn rewards_accumulate_across_epochs() {
        let mut set = test_set();
        register(&mut set, 1, 5_000);
        set.advance_epoch(10_000);
        set.advance_epoch(20_000);
        let v = set.get_validator(&addr(1)).unwrap();
        // epoch 1: 10_000 * 80% = 8_000
        // epoch 2: 20_000 * 80% = 16_000
        assert_eq!(v.rewards_earned, 24_000);
    }

    #[test]
    fn default_config_values() {
        let set = ValidatorSet::default_config();
        assert_eq!(set.min_stake(), DEFAULT_MIN_STAKE);
        assert_eq!(set.epoch_length_blocks(), DEFAULT_EPOCH_LENGTH);
    }
}
