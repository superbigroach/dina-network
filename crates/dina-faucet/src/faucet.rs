use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use dina_core::Address;

/// Errors that can occur during faucet operations.
#[derive(Debug, thiserror::Error)]
pub enum FaucetError {
    #[error("cooldown active: {remaining_seconds}s remaining")]
    CooldownActive { remaining_seconds: u64 },
    #[error("daily limit exceeded: {dispensed} of {limit} USDC already dispensed today")]
    DailyLimitExceeded { dispensed: u64, limit: u64 },
    #[error("faucet is empty")]
    FaucetEmpty,
    #[error("invalid address: {0}")]
    InvalidAddress(String),
}

/// Record of a single faucet drip.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FaucetRequest {
    /// Recipient address.
    pub address: Address,
    /// Amount dispensed in USDC micro-units.
    pub amount: u64,
    /// Unix timestamp of the request.
    pub timestamp: u64,
    /// Transaction hash on Dina Network, once submitted.
    pub tx_hash: Option<[u8; 32]>,
}

/// Aggregate faucet statistics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FaucetStats {
    /// Total USDC (micro-units) dispensed since the faucet started.
    pub total_dispensed: u64,
    /// Number of unique addresses that have received funds.
    pub unique_addresses: usize,
    /// Total number of drip requests served.
    pub total_requests: usize,
    /// Amount dispensed per request (micro-units).
    pub drip_amount: u64,
    /// Maximum USDC per address per day (micro-units).
    pub max_per_address_per_day: u64,
    /// Cooldown between requests in seconds.
    pub cooldown_seconds: u64,
}

/// Testnet faucet that dispenses test USDC to requesting addresses.
///
/// Enforces per-address daily limits and cooldown periods to prevent abuse.
pub struct Faucet {
    /// The faucet's own address (source of funds).
    faucet_address: Address,
    /// Amount dispensed per successful request (USDC micro-units).
    drip_amount: u64,
    /// Maximum USDC (micro-units) any single address can receive per day.
    max_per_address_per_day: u64,
    /// Minimum seconds between requests from the same address.
    cooldown_seconds: u64,
    /// Per-address request history.
    requests: BTreeMap<Address, Vec<FaucetRequest>>,
    /// Cumulative USDC dispensed.
    total_dispensed: u64,
}

/// One day in seconds, used for daily limit calculations.
const SECONDS_PER_DAY: u64 = 86_400;

impl Faucet {
    /// Create a new faucet with the specified parameters.
    ///
    /// - `faucet_address`: The address that funds are sent from.
    /// - `drip_amount`: USDC micro-units per drip (e.g., 100_000_000 = 100 USDC).
    pub fn new(faucet_address: Address, drip_amount: u64) -> Self {
        Self {
            faucet_address,
            drip_amount,
            max_per_address_per_day: 500_000_000, // 500 USDC
            cooldown_seconds: 60,
            requests: BTreeMap::new(),
            total_dispensed: 0,
        }
    }

    /// Create a faucet with custom rate limits.
    pub fn with_limits(
        faucet_address: Address,
        drip_amount: u64,
        max_per_address_per_day: u64,
        cooldown_seconds: u64,
    ) -> Self {
        Self {
            faucet_address,
            drip_amount,
            max_per_address_per_day,
            cooldown_seconds,
            requests: BTreeMap::new(),
            total_dispensed: 0,
        }
    }

    /// The address that this faucet sends funds from.
    pub fn faucet_address(&self) -> &Address {
        &self.faucet_address
    }

    /// Amount dispensed per request in USDC micro-units.
    pub fn drip_amount(&self) -> u64 {
        self.drip_amount
    }

    /// Request testnet USDC for the given address.
    ///
    /// Returns a `FaucetRequest` on success. The caller is responsible for
    /// actually submitting the transfer transaction to Dina Network and
    /// filling in the `tx_hash` field.
    pub fn request_funds(
        &mut self,
        recipient: Address,
        current_time: u64,
    ) -> Result<FaucetRequest, FaucetError> {
        // Check cooldown.
        if let Some(remaining) = self.cooldown_remaining(&recipient, current_time) {
            if remaining > 0 {
                warn!(
                    address = %recipient,
                    remaining_seconds = remaining,
                    "faucet request rejected: cooldown active"
                );
                return Err(FaucetError::CooldownActive {
                    remaining_seconds: remaining,
                });
            }
        }

        // Check daily limit.
        let dispensed_today = self.dispensed_today(&recipient, current_time);
        if dispensed_today + self.drip_amount > self.max_per_address_per_day {
            warn!(
                address = %recipient,
                dispensed_today = dispensed_today,
                limit = self.max_per_address_per_day,
                "faucet request rejected: daily limit exceeded"
            );
            return Err(FaucetError::DailyLimitExceeded {
                dispensed: dispensed_today,
                limit: self.max_per_address_per_day,
            });
        }

        let request = FaucetRequest {
            address: recipient,
            amount: self.drip_amount,
            timestamp: current_time,
            tx_hash: None,
        };

        self.requests
            .entry(recipient)
            .or_default()
            .push(request.clone());
        self.total_dispensed += self.drip_amount;

        info!(
            address = %recipient,
            amount = self.drip_amount,
            total_dispensed = self.total_dispensed,
            "faucet dispensed funds"
        );

        Ok(request)
    }

    /// Check whether the given address can currently request funds.
    pub fn can_request(&self, recipient: &Address, current_time: u64) -> bool {
        // Check cooldown.
        if let Some(remaining) = self.cooldown_remaining(recipient, current_time) {
            if remaining > 0 {
                return false;
            }
        }

        // Check daily limit.
        let dispensed_today = self.dispensed_today(recipient, current_time);
        dispensed_today + self.drip_amount <= self.max_per_address_per_day
    }

    /// Return the number of seconds until the next request is allowed.
    /// Returns 0 if the address can request immediately.
    pub fn time_until_next(&self, recipient: &Address, current_time: u64) -> u64 {
        self.cooldown_remaining(recipient, current_time)
            .unwrap_or(0)
    }

    /// Aggregate statistics about the faucet's usage.
    pub fn stats(&self) -> FaucetStats {
        let total_requests: usize = self.requests.values().map(|v| v.len()).sum();

        FaucetStats {
            total_dispensed: self.total_dispensed,
            unique_addresses: self.requests.len(),
            total_requests,
            drip_amount: self.drip_amount,
            max_per_address_per_day: self.max_per_address_per_day,
            cooldown_seconds: self.cooldown_seconds,
        }
    }

    /// Return the request history for a specific address.
    pub fn history(&self, address: &Address) -> &[FaucetRequest] {
        self.requests.get(address).map_or(&[], |v| v.as_slice())
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Calculate how many seconds of cooldown remain for the given address.
    /// Returns `None` if the address has never made a request.
    fn cooldown_remaining(&self, address: &Address, current_time: u64) -> Option<u64> {
        let history = self.requests.get(address)?;
        let last_request = history.last()?;

        let elapsed = current_time.saturating_sub(last_request.timestamp);
        if elapsed >= self.cooldown_seconds {
            Some(0)
        } else {
            Some(self.cooldown_seconds - elapsed)
        }
    }

    /// Calculate total USDC dispensed to the given address in the current
    /// 24-hour window (rolling window from `current_time - 86400`).
    fn dispensed_today(&self, address: &Address, current_time: u64) -> u64 {
        let Some(history) = self.requests.get(address) else {
            return 0;
        };

        let day_start = current_time.saturating_sub(SECONDS_PER_DAY);

        history
            .iter()
            .filter(|r| r.timestamp > day_start)
            .map(|r| r.amount)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn faucet_addr() -> Address {
        Address([0x01; 32])
    }

    fn user_addr() -> Address {
        Address([0x02; 32])
    }

    fn make_faucet() -> Faucet {
        // 100 USDC per drip, 500 USDC per day, 60s cooldown.
        Faucet::new(faucet_addr(), 100_000_000)
    }

    #[test]
    fn new_faucet_has_correct_defaults() {
        let f = make_faucet();
        assert_eq!(f.drip_amount(), 100_000_000);
        assert_eq!(f.max_per_address_per_day, 500_000_000);
        assert_eq!(f.cooldown_seconds, 60);
        assert_eq!(*f.faucet_address(), faucet_addr());
    }

    #[test]
    fn first_request_succeeds() {
        let mut f = make_faucet();
        let req = f.request_funds(user_addr(), 1000).unwrap();
        assert_eq!(req.amount, 100_000_000);
        assert_eq!(req.address, user_addr());
        assert_eq!(req.timestamp, 1000);
        assert!(req.tx_hash.is_none());
    }

    #[test]
    fn can_request_before_any_requests() {
        let f = make_faucet();
        assert!(f.can_request(&user_addr(), 1000));
    }

    #[test]
    fn cooldown_prevents_immediate_second_request() {
        let mut f = make_faucet();
        f.request_funds(user_addr(), 1000).unwrap();

        // 30 seconds later — should still be on cooldown.
        let result = f.request_funds(user_addr(), 1030);
        assert!(matches!(result, Err(FaucetError::CooldownActive { .. })));
        assert!(!f.can_request(&user_addr(), 1030));
    }

    #[test]
    fn request_after_cooldown_succeeds() {
        let mut f = make_faucet();
        f.request_funds(user_addr(), 1000).unwrap();

        // 61 seconds later — cooldown expired.
        let req = f.request_funds(user_addr(), 1061).unwrap();
        assert_eq!(req.amount, 100_000_000);
    }

    #[test]
    fn time_until_next_no_history() {
        let f = make_faucet();
        assert_eq!(f.time_until_next(&user_addr(), 1000), 0);
    }

    #[test]
    fn time_until_next_during_cooldown() {
        let mut f = make_faucet();
        f.request_funds(user_addr(), 1000).unwrap();

        // 20 seconds in, 40 seconds remaining.
        assert_eq!(f.time_until_next(&user_addr(), 1020), 40);
    }

    #[test]
    fn time_until_next_after_cooldown() {
        let mut f = make_faucet();
        f.request_funds(user_addr(), 1000).unwrap();

        assert_eq!(f.time_until_next(&user_addr(), 1100), 0);
    }

    #[test]
    fn daily_limit_enforced() {
        // 100 USDC per drip, 500 USDC limit per day -> 5 requests max.
        let mut f = make_faucet();
        let base_time = 100_000u64;

        for i in 0..5 {
            let time = base_time + (i as u64 * 61); // space requests by cooldown.
            f.request_funds(user_addr(), time).unwrap();
        }

        // 6th request should fail — daily limit reached.
        let time = base_time + 5 * 61;
        let result = f.request_funds(user_addr(), time);
        assert!(matches!(result, Err(FaucetError::DailyLimitExceeded { .. })));
    }

    #[test]
    fn daily_limit_resets_after_24h() {
        let mut f = make_faucet();
        let base_time = 100_000u64;

        // Use up the daily limit.
        for i in 0..5 {
            f.request_funds(user_addr(), base_time + i * 61).unwrap();
        }

        // 24 hours + 1 second later, limit should reset.
        let next_day = base_time + SECONDS_PER_DAY + 1;
        let req = f.request_funds(user_addr(), next_day).unwrap();
        assert_eq!(req.amount, 100_000_000);
    }

    #[test]
    fn different_addresses_have_independent_limits() {
        let mut f = make_faucet();
        let addr_a = Address([0x0a; 32]);
        let addr_b = Address([0x0b; 32]);

        f.request_funds(addr_a, 1000).unwrap();

        // addr_b should not be affected by addr_a's cooldown.
        assert!(f.can_request(&addr_b, 1000));
        f.request_funds(addr_b, 1000).unwrap();
    }

    #[test]
    fn stats_are_accurate() {
        let mut f = make_faucet();
        let addr_a = Address([0x0a; 32]);
        let addr_b = Address([0x0b; 32]);

        f.request_funds(addr_a, 1000).unwrap();
        f.request_funds(addr_b, 1000).unwrap();
        f.request_funds(addr_a, 1061).unwrap();

        let stats = f.stats();
        assert_eq!(stats.total_dispensed, 300_000_000);
        assert_eq!(stats.unique_addresses, 2);
        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.drip_amount, 100_000_000);
    }

    #[test]
    fn history_returns_requests_for_address() {
        let mut f = make_faucet();
        f.request_funds(user_addr(), 1000).unwrap();
        f.request_funds(user_addr(), 1061).unwrap();

        let history = f.history(&user_addr());
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].timestamp, 1000);
        assert_eq!(history[1].timestamp, 1061);
    }

    #[test]
    fn history_empty_for_unknown_address() {
        let f = make_faucet();
        let history = f.history(&Address([0xff; 32]));
        assert!(history.is_empty());
    }

    #[test]
    fn custom_limits() {
        let mut f = Faucet::with_limits(
            faucet_addr(),
            50_000_000,  // 50 USDC per drip
            100_000_000, // 100 USDC per day
            30,          // 30s cooldown
        );

        // First request: OK.
        f.request_funds(user_addr(), 1000).unwrap();

        // Second request after 31s: OK (50 + 50 = 100, at the limit).
        f.request_funds(user_addr(), 1031).unwrap();

        // Third request: exceeds 100 USDC daily limit.
        let result = f.request_funds(user_addr(), 1062);
        assert!(matches!(result, Err(FaucetError::DailyLimitExceeded { .. })));
    }

    #[test]
    fn total_dispensed_accumulates() {
        let mut f = make_faucet();
        f.request_funds(user_addr(), 1000).unwrap();
        assert_eq!(f.stats().total_dispensed, 100_000_000);

        f.request_funds(user_addr(), 1061).unwrap();
        assert_eq!(f.stats().total_dispensed, 200_000_000);
    }
}
