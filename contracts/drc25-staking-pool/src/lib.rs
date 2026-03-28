use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-25  Staking Pool
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

/// Precision multiplier for rewards_per_share to avoid rounding to zero.
const PRECISION: u128 = 1_000_000_000_000;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StakingPoolState {
    pub owner: Address,
    pub validator: Address,
    pub total_staked: u64,
    pub total_shares: u64,
    pub shares: BTreeMap<Address, u64>,
    /// Accumulated rewards per share (scaled by PRECISION)
    pub rewards_per_share: u128,
    /// Tracks reward debt per user for correct reward accounting
    pub reward_debt: BTreeMap<Address, u128>,
    /// Pending rewards credited during unstake, available for claim.
    pub pending_reward_balance: BTreeMap<Address, u64>,
}

impl StakingPoolState {
    pub fn new(owner: Address, validator: Address) -> Self {
        Self {
            owner,
            validator,
            total_staked: 0,
            total_shares: 0,
            shares: BTreeMap::new(),
            rewards_per_share: 0,
            reward_debt: BTreeMap::new(),
            pending_reward_balance: BTreeMap::new(),
        }
    }

    /// Stake tokens and receive proportional shares.
    /// Returns the number of shares minted.
    pub fn stake(&mut self, caller: Address, amount: u64) -> u64 {
        assert!(amount > 0, "DRC25: stake amount must be positive");

        // Calculate shares: if pool is empty, 1:1; otherwise proportional
        let new_shares = if self.total_staked == 0 || self.total_shares == 0 {
            amount
        } else {
            u64::try_from(
                (amount as u128) * (self.total_shares as u128) / (self.total_staked as u128),
            )
            .expect("DRC25: shares overflow")
        };
        assert!(new_shares > 0, "DRC25: stake too small, zero shares minted");

        let existing_shares = self.shares.get(&caller).copied().unwrap_or(0);
        self.shares.insert(caller, existing_shares + new_shares);
        self.total_shares += new_shares;
        self.total_staked += amount;

        // Set reward debt for new shares so they don't earn past rewards
        let existing_debt = self.reward_debt.get(&caller).copied().unwrap_or(0);
        self.reward_debt.insert(
            caller,
            existing_debt + (new_shares as u128) * self.rewards_per_share / PRECISION,
        );

        new_shares
    }

    /// Unstake by burning shares. Returns the amount of tokens released.
    pub fn unstake(&mut self, caller: Address, share_count: u64) -> u64 {
        assert!(share_count > 0, "DRC25: share count must be positive");
        let user_shares = self.shares.get(&caller).copied().unwrap_or(0);
        assert!(
            user_shares >= share_count,
            "DRC25: insufficient shares ({user_shares} < {share_count})"
        );

        // Calculate token amount proportional to shares
        let amount = ((share_count as u128) * (self.total_staked as u128)
            / (self.total_shares as u128)) as u64;

        // Credit pending rewards to the user's claimable balance before removing shares.
        let pending = self.pending_rewards_internal(caller) as u64;
        if pending > 0 {
            let existing = self
                .pending_reward_balance
                .get(&caller)
                .copied()
                .unwrap_or(0);
            self.pending_reward_balance
                .insert(caller, existing + pending);
        }

        self.shares.insert(caller, user_shares - share_count);
        if user_shares == share_count {
            self.shares.remove(&caller);
            self.reward_debt.remove(&caller);
        } else {
            // Recalculate debt for remaining shares
            let remaining = user_shares - share_count;
            self.reward_debt.insert(
                caller,
                (remaining as u128) * self.rewards_per_share / PRECISION,
            );
        }
        self.total_shares -= share_count;
        self.total_staked -= amount;

        amount
    }

    /// Distribute rewards across all stakers proportionally.
    pub fn distribute_rewards(&mut self, caller: Address, reward_amount: u64) {
        assert!(
            caller == self.owner || caller == self.validator,
            "DRC25: only owner or validator can distribute rewards"
        );
        assert!(reward_amount > 0, "DRC25: reward amount must be positive");
        assert!(
            self.total_shares > 0,
            "DRC25: no shares outstanding to receive rewards"
        );

        self.rewards_per_share += (reward_amount as u128) * PRECISION / (self.total_shares as u128);
        self.total_staked = self
            .total_staked
            .checked_add(reward_amount)
            .expect("DRC25: total_staked overflow");
    }

    /// Claim pending rewards for the caller. Returns amount claimed.
    /// Includes both actively accruing rewards and any rewards credited during unstake.
    pub fn claim_rewards(&mut self, caller: Address) -> u64 {
        let accruing = self.pending_rewards_internal(caller) as u64;
        let buffered = self
            .pending_reward_balance
            .get(&caller)
            .copied()
            .unwrap_or(0);
        let total = accruing + buffered;
        assert!(total > 0, "DRC25: no rewards to claim");

        // Reset debt to current level
        let user_shares = self.shares.get(&caller).copied().unwrap_or(0);
        if user_shares > 0 {
            self.reward_debt.insert(
                caller,
                (user_shares as u128) * self.rewards_per_share / PRECISION,
            );
        }

        // Clear buffered rewards
        self.pending_reward_balance.remove(&caller);

        // Deduct claimed rewards from total_staked
        self.total_staked = self.total_staked.saturating_sub(total);

        total
    }

    pub fn balance_of(&self, account: &Address) -> u64 {
        let user_shares = self.shares.get(account).copied().unwrap_or(0);
        if self.total_shares == 0 {
            return 0;
        }
        ((user_shares as u128) * (self.total_staked as u128) / (self.total_shares as u128)) as u64
    }

    pub fn pending_rewards(&self, account: &Address) -> u64 {
        let accruing = self.pending_rewards_internal(*account) as u64;
        let buffered = self
            .pending_reward_balance
            .get(account)
            .copied()
            .unwrap_or(0);
        accruing + buffered
    }

    fn pending_rewards_internal(&self, account: Address) -> u128 {
        let user_shares = self.shares.get(&account).copied().unwrap_or(0) as u128;
        let debt = self.reward_debt.get(&account).copied().unwrap_or(0);
        let accumulated = user_shares * self.rewards_per_share / PRECISION;
        accumulated.saturating_sub(debt)
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    validator: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct StakeArgs {
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct UnstakeArgs {
    shares: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct DistributeArgs {
    reward_amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AccountArgs {
    account: Address,
}

/// Contract-level dispatch.
pub fn dispatch(
    state: &mut Option<StakingPoolState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC25: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC25: bad init args");
            *state = Some(StakingPoolState::new(caller, a.validator));
            serde_json::to_vec("ok").unwrap()
        }

        "stake" => {
            let s = state.as_mut().expect("DRC25: not initialised");
            let a: StakeArgs = serde_json::from_slice(args).expect("DRC25: bad stake args");
            let shares = s.stake(caller, a.amount);
            serde_json::to_vec(&shares).unwrap()
        }

        "unstake" => {
            let s = state.as_mut().expect("DRC25: not initialised");
            let a: UnstakeArgs = serde_json::from_slice(args).expect("DRC25: bad unstake args");
            let amount = s.unstake(caller, a.shares);
            serde_json::to_vec(&amount).unwrap()
        }

        "claim_rewards" => {
            let s = state.as_mut().expect("DRC25: not initialised");
            let claimed = s.claim_rewards(caller);
            serde_json::to_vec(&claimed).unwrap()
        }

        "distribute_rewards" => {
            let s = state.as_mut().expect("DRC25: not initialised");
            let a: DistributeArgs =
                serde_json::from_slice(args).expect("DRC25: bad distribute_rewards args");
            s.distribute_rewards(caller, a.reward_amount);
            serde_json::to_vec("ok").unwrap()
        }

        "balance_of" => {
            let s = state.as_ref().expect("DRC25: not initialised");
            let a: AccountArgs = serde_json::from_slice(args).expect("DRC25: bad balance_of args");
            serde_json::to_vec(&s.balance_of(&a.account)).unwrap()
        }

        "pending_rewards" => {
            let s = state.as_ref().expect("DRC25: not initialised");
            let a: AccountArgs =
                serde_json::from_slice(args).expect("DRC25: bad pending_rewards args");
            serde_json::to_vec(&s.pending_rewards(&a.account)).unwrap()
        }

        _ => panic!("DRC25: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [1u8; 32];
    const VALIDATOR: Address = [2u8; 32];
    const ALICE: Address = [3u8; 32];
    const BOB: Address = [4u8; 32];

    fn init_pool() -> Option<StakingPoolState> {
        let mut state = None;
        let args = serde_json::to_vec(&InitArgs {
            validator: VALIDATOR,
        })
        .unwrap();
        dispatch(&mut state, "init", &args, OWNER);
        state
    }

    #[test]
    fn test_stake_and_balance() {
        let mut state = init_pool();
        let args = serde_json::to_vec(&StakeArgs { amount: 1000 }).unwrap();
        let result = dispatch(&mut state, "stake", &args, ALICE);
        let shares: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(shares, 1000); // first staker: 1:1

        let bal_args = serde_json::to_vec(&AccountArgs { account: ALICE }).unwrap();
        let bal_result = dispatch(&mut state, "balance_of", &bal_args, ALICE);
        let balance: u64 = serde_json::from_slice(&bal_result).unwrap();
        assert_eq!(balance, 1000);
    }

    #[test]
    fn test_unstake_returns_correct_amount() {
        let mut state = init_pool();
        let stake = serde_json::to_vec(&StakeArgs { amount: 1000 }).unwrap();
        dispatch(&mut state, "stake", &stake, ALICE);

        let unstake = serde_json::to_vec(&UnstakeArgs { shares: 500 }).unwrap();
        let result = dispatch(&mut state, "unstake", &unstake, ALICE);
        let amount: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(amount, 500);

        assert_eq!(state.as_ref().unwrap().total_staked, 500);
        assert_eq!(state.as_ref().unwrap().total_shares, 500);
    }

    #[test]
    fn test_distribute_rewards_increases_balance() {
        let mut state = init_pool();

        // Alice and Bob each stake 1000
        let stake = serde_json::to_vec(&StakeArgs { amount: 1000 }).unwrap();
        dispatch(&mut state, "stake", &stake, ALICE);
        dispatch(&mut state, "stake", &stake, BOB);

        // Distribute 200 rewards
        let dist = serde_json::to_vec(&DistributeArgs { reward_amount: 200 }).unwrap();
        dispatch(&mut state, "distribute_rewards", &dist, OWNER);

        // Each should have balance of ~1100 (1000 + 100 reward)
        let bal_args = serde_json::to_vec(&AccountArgs { account: ALICE }).unwrap();
        let result = dispatch(&mut state, "balance_of", &bal_args, ALICE);
        let balance: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(balance, 1100);
    }

    #[test]
    fn test_pending_rewards_and_claim() {
        let mut state = init_pool();
        let stake = serde_json::to_vec(&StakeArgs { amount: 1000 }).unwrap();
        dispatch(&mut state, "stake", &stake, ALICE);

        let dist = serde_json::to_vec(&DistributeArgs { reward_amount: 500 }).unwrap();
        dispatch(&mut state, "distribute_rewards", &dist, OWNER);

        // Check pending
        let pr_args = serde_json::to_vec(&AccountArgs { account: ALICE }).unwrap();
        let pr_result = dispatch(&mut state, "pending_rewards", &pr_args, ALICE);
        let pending: u64 = serde_json::from_slice(&pr_result).unwrap();
        assert_eq!(pending, 500);

        // Claim
        let claim_result = dispatch(&mut state, "claim_rewards", b"", ALICE);
        let claimed: u64 = serde_json::from_slice(&claim_result).unwrap();
        assert_eq!(claimed, 500);

        // Pending should be 0 after claim
        let pr_result2 = dispatch(&mut state, "pending_rewards", &pr_args, ALICE);
        let pending2: u64 = serde_json::from_slice(&pr_result2).unwrap();
        assert_eq!(pending2, 0);
    }

    #[test]
    #[should_panic(expected = "insufficient shares")]
    fn test_unstake_more_than_owned() {
        let mut state = init_pool();
        let stake = serde_json::to_vec(&StakeArgs { amount: 100 }).unwrap();
        dispatch(&mut state, "stake", &stake, ALICE);

        let unstake = serde_json::to_vec(&UnstakeArgs { shares: 200 }).unwrap();
        dispatch(&mut state, "unstake", &unstake, ALICE);
    }

    #[test]
    fn test_proportional_shares_for_second_staker() {
        let mut state = init_pool();

        // Alice stakes 1000 (gets 1000 shares)
        let stake1 = serde_json::to_vec(&StakeArgs { amount: 1000 }).unwrap();
        dispatch(&mut state, "stake", &stake1, ALICE);

        // Distribute 1000 rewards (total_staked becomes 2000, shares still 1000)
        let dist = serde_json::to_vec(&DistributeArgs {
            reward_amount: 1000,
        })
        .unwrap();
        dispatch(&mut state, "distribute_rewards", &dist, OWNER);

        // Bob stakes 2000 — should get 1000 shares (2000 * 1000 / 2000)
        let stake2 = serde_json::to_vec(&StakeArgs { amount: 2000 }).unwrap();
        let result = dispatch(&mut state, "stake", &stake2, BOB);
        let bob_shares: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(bob_shares, 1000);
    }
}
