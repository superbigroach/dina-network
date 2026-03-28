use crate::types::Address;
use std::collections::BTreeMap;

/// Yield rate in basis points (450 = 4.50% APY)
pub const DEFAULT_YIELD_RATE_BPS: u64 = 450;
pub const SECONDS_PER_YEAR: u64 = 31_536_000;
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Per-account yield tracking
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct YieldAccount {
    /// Base balance (excluding accrued yield)
    pub base_balance: u64,
    /// Timestamp when base_balance was last updated (unix seconds)
    pub last_update: u64,
    /// Yield rate in basis points for this account
    pub yield_rate_bps: u64,
}

impl YieldAccount {
    pub fn new(balance: u64, timestamp: u64) -> Self {
        Self {
            base_balance: balance,
            last_update: timestamp,
            yield_rate_bps: DEFAULT_YIELD_RATE_BPS,
        }
    }

    /// Calculate accrued yield since last update
    pub fn accrued_yield(&self, current_time: u64) -> u64 {
        if current_time <= self.last_update {
            return 0;
        }
        let elapsed = current_time - self.last_update;
        // Use u128 to prevent overflow: balance * rate * elapsed / (BPS * SECONDS)
        let yield_amount = (self.base_balance as u128)
            .checked_mul(self.yield_rate_bps as u128)
            .unwrap_or(0)
            .checked_mul(elapsed as u128)
            .unwrap_or(0)
            / (BPS_DENOMINATOR as u128 * SECONDS_PER_YEAR as u128);
        yield_amount as u64
    }

    /// Get effective balance including accrued yield
    pub fn effective_balance(&self, current_time: u64) -> u64 {
        self.base_balance.saturating_add(self.accrued_yield(current_time))
    }

    /// Materialize accrued yield into base balance (call before any balance change)
    pub fn settle_yield(&mut self, current_time: u64) {
        let accrued = self.accrued_yield(current_time);
        self.base_balance = self.base_balance.saturating_add(accrued);
        self.last_update = current_time;
    }

    /// Deposit funds (settles yield first)
    pub fn deposit(&mut self, amount: u64, current_time: u64) {
        self.settle_yield(current_time);
        self.base_balance = self.base_balance.saturating_add(amount);
    }

    /// Withdraw funds (settles yield first, returns error if insufficient)
    pub fn withdraw(&mut self, amount: u64, current_time: u64) -> Result<(), String> {
        self.settle_yield(current_time);
        if self.base_balance < amount {
            return Err(format!(
                "insufficient balance: have {}, need {}",
                self.base_balance, amount
            ));
        }
        self.base_balance -= amount;
        Ok(())
    }
}

/// Network-wide yield manager
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct YieldManager {
    pub accounts: BTreeMap<Address, YieldAccount>,
    pub global_yield_rate_bps: u64,
    pub total_yield_distributed: u64,
}

impl YieldManager {
    pub fn new() -> Self {
        Self {
            accounts: BTreeMap::new(),
            global_yield_rate_bps: DEFAULT_YIELD_RATE_BPS,
            total_yield_distributed: 0,
        }
    }

    pub fn get_or_create(&mut self, address: &Address, timestamp: u64) -> &mut YieldAccount {
        self.accounts.entry(*address).or_insert_with(|| {
            YieldAccount::new(0, timestamp)
        })
    }

    pub fn effective_balance(&self, address: &Address, current_time: u64) -> u64 {
        self.accounts
            .get(address)
            .map(|a| a.effective_balance(current_time))
            .unwrap_or(0)
    }

    /// Update yield rate (governance function)
    pub fn set_yield_rate(&mut self, rate_bps: u64) {
        self.global_yield_rate_bps = rate_bps;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yield_calculation() {
        let account = YieldAccount::new(1_000_000_000, 0); // $1000 USDC (6 decimals)
        // After 1 year at 4.5%
        let yield_1y = account.accrued_yield(SECONDS_PER_YEAR);
        assert_eq!(yield_1y, 45_000_000); // $45
    }

    #[test]
    fn test_yield_zero_elapsed() {
        let account = YieldAccount::new(1_000_000_000, 100);
        assert_eq!(account.accrued_yield(100), 0);
    }

    #[test]
    fn test_settle_and_withdraw() {
        let mut account = YieldAccount::new(1_000_000_000, 0);
        account.settle_yield(SECONDS_PER_YEAR);
        assert_eq!(account.base_balance, 1_045_000_000); // $1000 + $45
        assert!(account.withdraw(1_045_000_000, SECONDS_PER_YEAR).is_ok());
        assert_eq!(account.base_balance, 0);
    }

    #[test]
    fn test_deposit_settles_yield_first() {
        let mut account = YieldAccount::new(1_000_000_000, 0);
        account.deposit(500_000_000, SECONDS_PER_YEAR); // deposit $500 after 1yr
        // Should be $1000 + $45 yield + $500 deposit = $1545
        assert_eq!(account.base_balance, 1_545_000_000);
    }
}
