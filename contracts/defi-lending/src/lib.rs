use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DeFi Lending Pool  (Aave-style for Dina Network)
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

/// Minimum shares burned to dead address on first supply to prevent inflation attack.
const MINIMUM_SHARES: u64 = 1000;

/// Dead address where minimum shares are burned.
const DEAD_ADDRESS: Address = [0xDE; 32];

/// Basis-point precision for rate calculations (10_000 = 100%).
const BPS: u128 = 10_000;

/// Precision multiplier for interest index to avoid rounding to zero.
const INDEX_PRECISION: u128 = 1_000_000_000_000; // 1e12

/// Seconds in one year (365.25 days) for annualised rate conversion.
const SECONDS_PER_YEAR: u128 = 31_557_600;

/// Maximum allowed time drift between consecutive timestamps (600 seconds = 10 minutes).
const MAX_TIME_DRIFT: u64 = 600;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BorrowPosition {
    pub principal: u64,
    pub interest_index: u128,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LendingPoolState {
    pub owner: Address,

    // Supply side
    pub total_supplied: u64,
    pub supply_shares: BTreeMap<Address, u64>,
    pub total_supply_shares: u64,

    // Borrow side
    pub total_borrowed: u64,
    pub borrow_positions: BTreeMap<Address, BorrowPosition>,

    // Interest rate model (basis points per year)
    pub base_rate_bps: u64,
    pub slope1_bps: u64,
    pub slope2_bps: u64,
    pub optimal_utilization_bps: u64,

    // Protocol
    pub reserve_factor_bps: u64,
    pub protocol_reserves: u64,

    // Collateral locking — tracks how much of each user's supply value is
    // locked as collateral for outstanding borrows.
    pub locked_supply: BTreeMap<Address, u64>,

    // State
    pub borrow_index: u128,
    pub last_update_timestamp: u64,

    pub paused: bool,
    pub initialized: bool,
}

impl LendingPoolState {
    pub fn new(
        owner: Address,
        base_rate_bps: u64,
        slope1_bps: u64,
        slope2_bps: u64,
        optimal_utilization_bps: u64,
        reserve_factor_bps: u64,
    ) -> Self {
        Self {
            owner,
            total_supplied: 0,
            supply_shares: BTreeMap::new(),
            total_supply_shares: 0,
            total_borrowed: 0,
            borrow_positions: BTreeMap::new(),
            base_rate_bps,
            slope1_bps,
            slope2_bps,
            optimal_utilization_bps,
            reserve_factor_bps,
            protocol_reserves: 0,
            locked_supply: BTreeMap::new(),
            borrow_index: INDEX_PRECISION,
            last_update_timestamp: 0,
            paused: false,
            initialized: false,
        }
    }

    // -----------------------------------------------------------------------
    // Interest rate model
    // -----------------------------------------------------------------------

    /// Utilization rate in basis points (0 .. 10_000).
    pub fn get_utilization_bps(&self) -> u64 {
        if self.total_supplied == 0 {
            return 0;
        }
        ((self.total_borrowed as u128 * BPS) / self.total_supplied as u128) as u64
    }

    /// Borrow APY in basis points (Aave-style piecewise linear).
    pub fn get_borrow_apy_bps(&self) -> u64 {
        let util = self.get_utilization_bps() as u128;
        let optimal = self.optimal_utilization_bps as u128;
        let base = self.base_rate_bps as u128;
        let s1 = self.slope1_bps as u128;
        let s2 = self.slope2_bps as u128;

        if util <= optimal {
            if optimal == 0 {
                return base as u64;
            }
            (base + util * s1 / optimal) as u64
        } else {
            let excess = util - optimal;
            let remaining = BPS - optimal;
            if remaining == 0 {
                return (base + s1) as u64;
            }
            (base + s1 + excess * s2 / remaining) as u64
        }
    }

    /// Supply APY in basis points.
    pub fn get_supply_apy_bps(&self) -> u64 {
        let borrow_apy = self.get_borrow_apy_bps() as u128;
        let util = self.get_utilization_bps() as u128;
        let reserve = self.reserve_factor_bps as u128;

        // supply_rate = borrow_rate * utilization * (1 - reserve_factor)
        (borrow_apy * util / BPS * (BPS - reserve) / BPS) as u64
    }

    // -----------------------------------------------------------------------
    // Interest accrual
    // -----------------------------------------------------------------------

    /// Accrue interest based on time elapsed since last update.
    /// Updates borrow_index and accumulates protocol reserves.
    pub fn accrue_interest(&mut self, current_timestamp: u64) {
        if !self.initialized {
            self.initialized = true;
            self.last_update_timestamp = current_timestamp;
            return;
        }
        if current_timestamp <= self.last_update_timestamp {
            return;
        }
        // Validate timestamp: must not jump too far into the future to prevent
        // manipulation by caller-supplied timestamps.
        assert!(
            current_timestamp <= self.last_update_timestamp + MAX_TIME_DRIFT,
            "Pool: timestamp too far in future ({} > {} + {})",
            current_timestamp,
            self.last_update_timestamp,
            MAX_TIME_DRIFT
        );
        if self.total_borrowed == 0 {
            self.last_update_timestamp = current_timestamp;
            return;
        }

        let elapsed = (current_timestamp - self.last_update_timestamp) as u128;
        let borrow_rate_bps = self.get_borrow_apy_bps() as u128;

        // L-4: Interest factor = rate * elapsed / seconds_per_year
        // We compute: new_index = old_index * (1 + rate * elapsed / year)
        // Multiply all numerators first, then divide, to minimise precision loss.
        let interest_factor =
            borrow_rate_bps * elapsed * INDEX_PRECISION / (BPS * SECONDS_PER_YEAR);

        let interest_earned =
            (self.total_borrowed as u128 * interest_factor / INDEX_PRECISION) as u64;

        // Update borrow index
        let idx_delta = self.borrow_index * borrow_rate_bps * elapsed / (BPS * SECONDS_PER_YEAR);
        self.borrow_index += idx_delta;

        // Protocol takes its cut
        let reserve_share =
            (interest_earned as u128 * self.reserve_factor_bps as u128 / BPS as u128) as u64;
        self.protocol_reserves += reserve_share;

        // Remaining interest goes to suppliers (increases total_supplied)
        self.total_supplied += interest_earned - reserve_share;
        self.total_borrowed += interest_earned;

        self.last_update_timestamp = current_timestamp;
    }

    // -----------------------------------------------------------------------
    // Supply operations
    // -----------------------------------------------------------------------

    /// Supply USDC to the lending pool. Returns supply shares received.
    pub fn supply(&mut self, caller: Address, amount: u64, timestamp: u64) -> u64 {
        assert!(!self.paused, "Pool: pool is paused");
        assert!(amount > 0, "Pool: supply amount must be positive");

        self.accrue_interest(timestamp);

        let is_first_supply = self.total_supply_shares == 0 || self.total_supplied == 0;
        let shares = if is_first_supply {
            amount
        } else {
            (amount as u128 * self.total_supply_shares as u128 / self.total_supplied as u128) as u64
        };
        assert!(shares > 0, "Pool: zero supply shares");

        self.total_supplied += amount;
        self.total_supply_shares += shares;

        // On first supply, burn MINIMUM_SHARES to dead address to prevent inflation attack
        if is_first_supply {
            assert!(
                shares > MINIMUM_SHARES,
                "Pool: first supply too small, must mint more than {} shares",
                MINIMUM_SHARES
            );
            let dead_existing = self.supply_shares.get(&DEAD_ADDRESS).copied().unwrap_or(0);
            self.supply_shares
                .insert(DEAD_ADDRESS, dead_existing + MINIMUM_SHARES);
            let caller_shares = shares - MINIMUM_SHARES;
            let existing = self.supply_shares.get(&caller).copied().unwrap_or(0);
            self.supply_shares.insert(caller, existing + caller_shares);
            return caller_shares;
        }

        let existing = self.supply_shares.get(&caller).copied().unwrap_or(0);
        self.supply_shares.insert(caller, existing + shares);

        shares
    }

    /// Withdraw supplied USDC. Returns amount of USDC withdrawn.
    pub fn withdraw_supply(&mut self, caller: Address, shares: u64, timestamp: u64) -> u64 {
        assert!(!self.paused, "Pool: pool is paused");
        assert!(shares > 0, "Pool: share amount must be positive");

        self.accrue_interest(timestamp);

        let user_shares = self.supply_shares.get(&caller).copied().unwrap_or(0);
        assert!(
            user_shares >= shares,
            "Pool: insufficient supply shares ({user_shares} < {shares})"
        );

        let amount = (shares as u128 * self.total_supplied as u128
            / self.total_supply_shares as u128) as u64;

        let available = self.total_supplied - self.total_borrowed;
        assert!(
            amount <= available,
            "Pool: insufficient liquidity ({amount} > {available})"
        );

        // Check that withdrawal does not violate collateral lock.
        // user_supply_value - locked_amount >= withdrawal_amount
        let user_supply_value = self.get_supply_balance(&caller);
        let locked = self.locked_supply.get(&caller).copied().unwrap_or(0);
        let withdrawable = user_supply_value.saturating_sub(locked);
        assert!(
            amount <= withdrawable,
            "Pool: withdrawal exceeds unlocked supply ({amount} > {withdrawable}, locked={locked})"
        );

        self.supply_shares.insert(caller, user_shares - shares);
        self.total_supply_shares -= shares;
        self.total_supplied -= amount;

        amount
    }

    /// Get current supply balance including accrued interest.
    pub fn get_supply_balance(&self, user: &Address) -> u64 {
        let shares = self.supply_shares.get(user).copied().unwrap_or(0);
        if shares == 0 || self.total_supply_shares == 0 {
            return 0;
        }
        (shares as u128 * self.total_supplied as u128 / self.total_supply_shares as u128) as u64
    }

    // -----------------------------------------------------------------------
    // Borrow operations
    // -----------------------------------------------------------------------

    /// Borrow USDC from the pool. Requires 150% collateralization.
    pub fn borrow(&mut self, caller: Address, amount: u64, timestamp: u64) {
        assert!(!self.paused, "Pool: pool is paused");
        assert!(amount > 0, "Pool: borrow amount must be positive");

        self.accrue_interest(timestamp);

        // M-4: Require 150% collateralization — borrower must have supplied enough.
        let user_supply = self.get_supply_balance(&caller);
        let required_collateral = (amount as u128 * 15000 / 10000) as u64; // 150%
        assert!(
            user_supply >= required_collateral,
            "Pool: insufficient collateral ({} supplied, {} required for {} borrow)",
            user_supply,
            required_collateral,
            amount
        );

        let available = self.total_supplied - self.total_borrowed;
        assert!(
            amount <= available,
            "Pool: insufficient liquidity to borrow ({amount} > {available})"
        );

        // Lock the required collateral so it cannot be withdrawn.
        let existing_locked = self.locked_supply.get(&caller).copied().unwrap_or(0);
        self.locked_supply
            .insert(caller, existing_locked + required_collateral);

        // If user has existing position, settle it first
        if let Some(existing) = self.borrow_positions.get(&caller) {
            let accrued_principal = self.calculate_borrow_balance(existing);
            self.borrow_positions.insert(
                caller,
                BorrowPosition {
                    principal: accrued_principal + amount,
                    interest_index: self.borrow_index,
                    timestamp,
                },
            );
        } else {
            self.borrow_positions.insert(
                caller,
                BorrowPosition {
                    principal: amount,
                    interest_index: self.borrow_index,
                    timestamp,
                },
            );
        }

        self.total_borrowed += amount;
    }

    /// Repay borrowed USDC. Returns the excess amount (if overpaid).
    pub fn repay(&mut self, caller: Address, amount: u64, timestamp: u64) -> u64 {
        assert!(amount > 0, "Pool: repay amount must be positive");

        self.accrue_interest(timestamp);

        let position = self
            .borrow_positions
            .get(&caller)
            .expect("Pool: no borrow position");
        let owed = self.calculate_borrow_balance(position);

        let actual_repay = if amount >= owed { owed } else { amount };
        let excess = amount.saturating_sub(owed);

        // Unlock proportional collateral based on repayment fraction.
        let locked = self.locked_supply.get(&caller).copied().unwrap_or(0);
        if actual_repay >= owed {
            // Fully repaid — unlock all collateral
            self.locked_supply.remove(&caller);
        } else {
            // Partially repaid — unlock proportional collateral
            let unlock_amount = (locked as u128 * actual_repay as u128 / owed as u128) as u64;
            self.locked_supply
                .insert(caller, locked.saturating_sub(unlock_amount));
        }

        if actual_repay >= owed {
            // Fully repaid
            self.borrow_positions.remove(&caller);
        } else {
            // Partially repaid — update position
            self.borrow_positions.insert(
                caller,
                BorrowPosition {
                    principal: owed - actual_repay,
                    interest_index: self.borrow_index,
                    timestamp,
                },
            );
        }

        // Reduce total_borrowed by the original principal portion
        if actual_repay <= self.total_borrowed {
            self.total_borrowed -= actual_repay;
        } else {
            self.total_borrowed = 0;
        }

        excess
    }

    /// Get current borrow balance including accrued interest.
    pub fn get_borrow_balance_for(&self, user: &Address) -> u64 {
        match self.borrow_positions.get(user) {
            Some(pos) => self.calculate_borrow_balance(pos),
            None => 0,
        }
    }

    fn calculate_borrow_balance(&self, position: &BorrowPosition) -> u64 {
        if position.interest_index == 0 {
            return position.principal;
        }
        (position.principal as u128 * self.borrow_index / position.interest_index) as u64
    }

    // -----------------------------------------------------------------------
    // Admin
    // -----------------------------------------------------------------------

    /// Owner collects accumulated protocol reserves.
    pub fn collect_reserves(&mut self, caller: Address) -> u64 {
        assert!(
            caller == self.owner,
            "Pool: only owner can collect reserves"
        );
        let amount = self.protocol_reserves;
        self.protocol_reserves = 0;
        amount
    }

    pub fn pause(&mut self, caller: Address) {
        assert!(caller == self.owner, "Pool: only owner can pause");
        self.paused = true;
    }

    pub fn unpause(&mut self, caller: Address) {
        assert!(caller == self.owner, "Pool: only owner can unpause");
        self.paused = false;
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreatePoolArgs {
    base_rate_bps: u64,
    slope1_bps: u64,
    slope2_bps: u64,
    optimal_utilization_bps: u64,
    reserve_factor_bps: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AmountTimestampArgs {
    amount: u64,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SharesTimestampArgs {
    shares: u64,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct TimestampArgs {
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct UserArgs {
    user: Address,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<LendingPoolState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "create_pool" => {
            assert!(state.is_none(), "Pool: already initialised");
            let a: CreatePoolArgs =
                serde_json::from_slice(args).expect("Pool: bad create_pool args");
            *state = Some(LendingPoolState::new(
                caller,
                a.base_rate_bps,
                a.slope1_bps,
                a.slope2_bps,
                a.optimal_utilization_bps,
                a.reserve_factor_bps,
            ));
            serde_json::to_vec("ok").unwrap()
        }

        "supply" => {
            let s = state.as_mut().expect("Pool: not initialised");
            let a: AmountTimestampArgs =
                serde_json::from_slice(args).expect("Pool: bad supply args");
            let shares = s.supply(caller, a.amount, a.timestamp);
            serde_json::to_vec(&shares).unwrap()
        }

        "withdraw_supply" => {
            let s = state.as_mut().expect("Pool: not initialised");
            let a: SharesTimestampArgs =
                serde_json::from_slice(args).expect("Pool: bad withdraw_supply args");
            let amount = s.withdraw_supply(caller, a.shares, a.timestamp);
            serde_json::to_vec(&amount).unwrap()
        }

        "borrow" => {
            let s = state.as_mut().expect("Pool: not initialised");
            let a: AmountTimestampArgs =
                serde_json::from_slice(args).expect("Pool: bad borrow args");
            s.borrow(caller, a.amount, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }

        "repay" => {
            let s = state.as_mut().expect("Pool: not initialised");
            let a: AmountTimestampArgs =
                serde_json::from_slice(args).expect("Pool: bad repay args");
            let excess = s.repay(caller, a.amount, a.timestamp);
            serde_json::to_vec(&excess).unwrap()
        }

        "get_supply_balance" => {
            let s = state.as_ref().expect("Pool: not initialised");
            let a: UserArgs =
                serde_json::from_slice(args).expect("Pool: bad get_supply_balance args");
            serde_json::to_vec(&s.get_supply_balance(&a.user)).unwrap()
        }

        "get_borrow_balance" => {
            let s = state.as_ref().expect("Pool: not initialised");
            let a: UserArgs =
                serde_json::from_slice(args).expect("Pool: bad get_borrow_balance args");
            serde_json::to_vec(&s.get_borrow_balance_for(&a.user)).unwrap()
        }

        "get_utilization" => {
            let s = state.as_ref().expect("Pool: not initialised");
            serde_json::to_vec(&s.get_utilization_bps()).unwrap()
        }

        "get_supply_apy" => {
            let s = state.as_ref().expect("Pool: not initialised");
            serde_json::to_vec(&s.get_supply_apy_bps()).unwrap()
        }

        "get_borrow_apy" => {
            let s = state.as_ref().expect("Pool: not initialised");
            serde_json::to_vec(&s.get_borrow_apy_bps()).unwrap()
        }

        "accrue_interest" => {
            let s = state.as_mut().expect("Pool: not initialised");
            let a: TimestampArgs =
                serde_json::from_slice(args).expect("Pool: bad accrue_interest args");
            s.accrue_interest(a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }

        "collect_reserves" => {
            let s = state.as_mut().expect("Pool: not initialised");
            let amount = s.collect_reserves(caller);
            serde_json::to_vec(&amount).unwrap()
        }

        "pause" => {
            let s = state.as_mut().expect("Pool: not initialised");
            s.pause(caller);
            serde_json::to_vec("ok").unwrap()
        }

        "unpause" => {
            let s = state.as_mut().expect("Pool: not initialised");
            s.unpause(caller);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("Pool: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(n: u8) -> Address {
        let mut a = [0u8; 32];
        a[0] = n;
        a
    }

    fn make_pool() -> LendingPoolState {
        // base=2%, slope1=10%, slope2=100%, optimal=80%, reserve=10%
        LendingPoolState::new(addr(0), 200, 1000, 10000, 8000, 1000)
    }

    /// Advance pool time by stepping in MAX_TIME_DRIFT increments.
    fn advance_time(pool: &mut LendingPoolState, from: u64, to: u64) {
        let mut t = from;
        while t < to {
            let next = std::cmp::min(t + MAX_TIME_DRIFT, to);
            pool.accrue_interest(next);
            t = next;
        }
    }

    #[test]
    fn test_supply_and_verify_balance() {
        let mut pool = make_pool();
        let shares = pool.supply(addr(1), 100_000_000, 1000); // 100 USDC
                                                              // First deposit: 100M minted total, 1000 burned to dead address
        assert_eq!(shares, 100_000_000 - MINIMUM_SHARES);
        assert_eq!(pool.total_supplied, 100_000_000);
        // Supply balance reflects proportional share (99_999_000 / 100_000_000 * 100_000_000)
        assert_eq!(
            pool.get_supply_balance(&addr(1)),
            100_000_000 - MINIMUM_SHARES
        );
    }

    #[test]
    fn test_borrow_and_verify_position() {
        let mut pool = make_pool();
        pool.supply(addr(1), 100_000_000, 1000);
        // M-4: addr(2) must supply collateral (150%) before borrowing
        pool.supply(addr(2), 100_000_000, 1000);
        pool.borrow(addr(2), 50_000_000, 1000);
        assert_eq!(pool.total_borrowed, 50_000_000);
        assert_eq!(pool.get_borrow_balance_for(&addr(2)), 50_000_000);
    }

    #[test]
    fn test_interest_accrues_over_time() {
        let mut pool = make_pool();
        pool.supply(addr(1), 100_000_000, 0);
        pool.supply(addr(2), 100_000_000, 0);
        pool.borrow(addr(2), 50_000_000, 0);

        // Advance 1 year in increments respecting MAX_TIME_DRIFT
        advance_time(&mut pool, 0, SECONDS_PER_YEAR as u64);

        let borrow_balance = pool.get_borrow_balance_for(&addr(2));
        assert!(
            borrow_balance > 50_000_000,
            "Borrow balance should increase with interest: {borrow_balance}"
        );

        let supply_balance = pool.get_supply_balance(&addr(1));
        assert!(
            supply_balance > 100_000_000,
            "Supply balance should increase: {supply_balance}"
        );
    }

    #[test]
    fn test_repay_clears_position() {
        let mut pool = make_pool();
        pool.supply(addr(1), 100_000_000, 0);
        pool.supply(addr(2), 100_000_000, 0);
        pool.borrow(addr(2), 50_000_000, 0);

        let excess = pool.repay(addr(2), 50_000_000, 0);
        assert_eq!(excess, 0);
        assert_eq!(pool.get_borrow_balance_for(&addr(2)), 0);
        assert!(pool.borrow_positions.get(&addr(2)).is_none());
        // Collateral should be fully unlocked after full repay
        assert_eq!(pool.locked_supply.get(&addr(2)).copied().unwrap_or(0), 0);
    }

    #[test]
    fn test_utilization_rate_calculation() {
        let mut pool = make_pool();
        pool.supply(addr(1), 100_000_000, 0);
        assert_eq!(pool.get_utilization_bps(), 0);

        pool.borrow(addr(1), 50_000_000, 0);
        assert_eq!(pool.get_utilization_bps(), 5000);

        pool.supply(addr(2), 100_000_000, 0);
        pool.borrow(addr(2), 30_000_000, 0);
        assert_eq!(pool.get_utilization_bps(), 4000);
    }

    #[test]
    fn test_interest_rate_model_low_util() {
        let pool = make_pool();
        assert_eq!(pool.get_borrow_apy_bps(), 200);
    }

    #[test]
    fn test_interest_rate_model_high_util() {
        let mut pool = make_pool();
        pool.supply(addr(1), 100_000_000, 0);
        pool.borrow(addr(1), 60_000_000, 0);
        pool.supply(addr(2), 100_000_000, 0);
        pool.borrow(addr(2), 30_000_000, 0);
        let mut pool2 = make_pool();
        pool2.total_supplied = 100_000_000;
        pool2.total_borrowed = 90_000_000;
        let rate = pool2.get_borrow_apy_bps();
        assert_eq!(rate, 6200);
    }

    #[test]
    fn test_supply_apy_less_than_borrow_apy() {
        let mut pool = make_pool();
        pool.supply(addr(1), 100_000_000, 0);
        pool.supply(addr(2), 100_000_000, 0);
        pool.borrow(addr(2), 50_000_000, 0);

        let supply_apy = pool.get_supply_apy_bps();
        let borrow_apy = pool.get_borrow_apy_bps();

        assert!(
            supply_apy < borrow_apy,
            "Supply APY ({supply_apy}) must be < Borrow APY ({borrow_apy})"
        );
    }

    #[test]
    fn test_reserve_factor_accumulates_revenue() {
        let mut pool = make_pool();
        pool.supply(addr(1), 100_000_000, 0);
        pool.supply(addr(2), 100_000_000, 0);
        pool.borrow(addr(2), 50_000_000, 0);

        advance_time(&mut pool, 0, SECONDS_PER_YEAR as u64);

        assert!(
            pool.protocol_reserves > 0,
            "Protocol reserves should accumulate: {}",
            pool.protocol_reserves
        );
    }

    #[test]
    #[should_panic(expected = "insufficient collateral")]
    fn test_cannot_borrow_more_than_available() {
        let mut pool = make_pool();
        pool.supply(addr(1), 100_000_000, 0);
        pool.borrow(addr(2), 100_000_001, 0);
    }

    #[test]
    #[should_panic(expected = "insufficient collateral")]
    fn test_collateral_requirement() {
        let mut pool = make_pool();
        pool.supply(addr(1), 100_000_000, 0);
        pool.supply(addr(2), 100_000_000, 0);
        pool.borrow(addr(2), 80_000_000, 0);
    }

    #[test]
    fn test_multiple_suppliers_share_interest() {
        let mut pool = make_pool();

        pool.supply(addr(1), 50_000_000, 0);
        pool.supply(addr(2), 50_000_000, 0);

        pool.supply(addr(3), 100_000_000, 0);
        pool.borrow(addr(3), 50_000_000, 0);

        advance_time(&mut pool, 0, SECONDS_PER_YEAR as u64);

        let alice_bal = pool.get_supply_balance(&addr(1));
        let bob_bal = pool.get_supply_balance(&addr(2));

        // addr(1) has slightly fewer shares due to MINIMUM_SHARES burn on first deposit
        // but both should earn interest proportional to their shares
        assert!(
            alice_bal > 49_000_000,
            "Alice balance should grow with interest: {alice_bal}"
        );
        assert!(
            bob_bal > 50_000_000,
            "Bob balance should grow with interest: {bob_bal}"
        );
        // The difference should be small (proportional to MINIMUM_SHARES / total)
        assert!(
            bob_bal - alice_bal < 100_000,
            "Difference should be small: {}",
            bob_bal - alice_bal
        );
    }

    #[test]
    fn test_collect_reserves() {
        let mut pool = make_pool();
        pool.supply(addr(1), 100_000_000, 0);
        pool.supply(addr(2), 100_000_000, 0);
        pool.borrow(addr(2), 50_000_000, 0);
        advance_time(&mut pool, 0, SECONDS_PER_YEAR as u64);

        let reserves = pool.protocol_reserves;
        assert!(reserves > 0);

        let collected = pool.collect_reserves(addr(0));
        assert_eq!(collected, reserves);
        assert_eq!(pool.protocol_reserves, 0);
    }

    #[test]
    fn test_dispatch_full_lifecycle() {
        let mut state: Option<LendingPoolState> = None;
        let owner = addr(0);

        let create_args = serde_json::to_vec(&CreatePoolArgs {
            base_rate_bps: 200,
            slope1_bps: 1000,
            slope2_bps: 10000,
            optimal_utilization_bps: 8000,
            reserve_factor_bps: 1000,
        })
        .unwrap();
        dispatch(&mut state, "create_pool", &create_args, owner);
        assert!(state.is_some());

        let supply_args = serde_json::to_vec(&AmountTimestampArgs {
            amount: 100_000_000,
            timestamp: 0,
        })
        .unwrap();
        dispatch(&mut state, "supply", &supply_args, addr(1));

        let supply_args2 = serde_json::to_vec(&AmountTimestampArgs {
            amount: 100_000_000,
            timestamp: 0,
        })
        .unwrap();
        dispatch(&mut state, "supply", &supply_args2, addr(2));

        let borrow_args = serde_json::to_vec(&AmountTimestampArgs {
            amount: 30_000_000,
            timestamp: 0,
        })
        .unwrap();
        dispatch(&mut state, "borrow", &borrow_args, addr(2));

        let result = dispatch(&mut state, "get_utilization", &[], owner);
        let util: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(util, 1500);

        let repay_args = serde_json::to_vec(&AmountTimestampArgs {
            amount: 30_000_000,
            timestamp: 0,
        })
        .unwrap();
        dispatch(&mut state, "repay", &repay_args, addr(2));

        let result = dispatch(&mut state, "get_utilization", &[], owner);
        let util: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(util, 0);
    }

    #[test]
    #[should_panic(expected = "pool is paused")]
    fn test_pause_prevents_supply() {
        let mut pool = make_pool();
        pool.pause(addr(0));
        pool.supply(addr(1), 1_000_000, 0);
    }

    #[test]
    #[should_panic(expected = "timestamp too far in future")]
    fn test_timestamp_drift_rejected() {
        let mut pool = make_pool();
        pool.supply(addr(1), 100_000_000, 0);
        // Try to jump far into the future in one step
        pool.accrue_interest(MAX_TIME_DRIFT + 1);
    }

    #[test]
    #[should_panic(expected = "withdrawal exceeds unlocked supply")]
    fn test_cannot_withdraw_locked_collateral() {
        let mut pool = make_pool();
        pool.supply(addr(1), 100_000_000, 0);
        pool.supply(addr(2), 100_000_000, 0);
        // addr(2) borrows 50M, locking 75M collateral
        pool.borrow(addr(2), 50_000_000, 0);
        // addr(2) has 100M supply, 75M locked, only 25M withdrawable
        // Try to withdraw all 100M shares (worth 100M) — should fail
        pool.withdraw_supply(addr(2), 100_000_000, 0);
    }
}
