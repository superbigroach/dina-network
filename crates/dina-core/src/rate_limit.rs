use crate::types::Address;
use std::collections::BTreeMap;

/// Minimum bucket size for any account
pub const MIN_BUCKET_SIZE: u64 = 100;
/// Minimum refill rate (transactions per second)
pub const MIN_REFILL_RATE: u64 = 1;
/// Minimum balance to transact (1 USDC = 1_000_000 micro-USDC)
pub const MIN_BALANCE_TO_TRANSACT: u64 = 1_000_000;

/// Balance tier thresholds and their bucket configs (micro-USDC)
const TIERS: &[(u64, u64, u64)] = &[
    // (min_balance, bucket_size, refill_per_sec)
    (1_000_000,         100,      1),      // $1+
    (100_000_000,       500,      5),      // $100+
    (1_000_000_000,     2_000,    20),     // $1,000+
    (10_000_000_000,    10_000,   100),    // $10,000+
    (100_000_000_000,   50_000,   500),    // $100,000+
];

#[derive(Clone, Debug)]
pub struct TokenBucket {
    pub tokens: u64,
    pub max_tokens: u64,
    pub refill_rate: u64, // tokens per second
    pub last_refill: u64, // unix timestamp
}

impl TokenBucket {
    pub fn new(max_tokens: u64, refill_rate: u64, now: u64) -> Self {
        Self {
            tokens: max_tokens, // start full
            max_tokens,
            refill_rate,
            last_refill: now,
        }
    }

    /// Refill tokens based on elapsed time
    pub fn refill(&mut self, now: u64) {
        if now <= self.last_refill {
            return;
        }
        let elapsed = now - self.last_refill;
        let new_tokens = elapsed.saturating_mul(self.refill_rate);
        self.tokens = self.tokens.saturating_add(new_tokens).min(self.max_tokens);
        self.last_refill = now;
    }

    /// Try to consume one token (one transaction). Returns true if allowed.
    pub fn try_consume(&mut self, now: u64) -> bool {
        self.refill(now);
        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }

    /// Try to consume N tokens (batch transaction). Returns true if allowed.
    pub fn try_consume_n(&mut self, n: u64, now: u64) -> bool {
        self.refill(now);
        if self.tokens >= n {
            self.tokens -= n;
            true
        } else {
            false
        }
    }
}

/// Determine bucket config based on account balance
pub fn bucket_config_for_balance(balance_micro_usdc: u64) -> (u64, u64) {
    let mut bucket_size = MIN_BUCKET_SIZE;
    let mut refill_rate = MIN_REFILL_RATE;

    for &(min_bal, bsize, rate) in TIERS {
        if balance_micro_usdc >= min_bal {
            bucket_size = bsize;
            refill_rate = rate;
        }
    }

    (bucket_size, refill_rate)
}

/// Network-wide rate limiter
#[derive(Clone, Debug, Default)]
pub struct RateLimiter {
    buckets: BTreeMap<Address, TokenBucket>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: BTreeMap::new(),
        }
    }

    /// Check if an account can transact. Updates the bucket.
    pub fn check_rate_limit(
        &mut self,
        address: &Address,
        balance: u64,
        now: u64,
    ) -> bool {
        // Must hold minimum balance
        if balance < MIN_BALANCE_TO_TRANSACT {
            return false;
        }

        let (bucket_size, refill_rate) = bucket_config_for_balance(balance);

        let bucket = self.buckets.entry(*address).or_insert_with(|| {
            TokenBucket::new(bucket_size, refill_rate, now)
        });

        // Update bucket config if balance tier changed
        if bucket.max_tokens != bucket_size {
            bucket.max_tokens = bucket_size;
            bucket.refill_rate = refill_rate;
            // Don't reset tokens — keep existing tokens up to new max
            bucket.tokens = bucket.tokens.min(bucket_size);
        }

        bucket.try_consume(now)
    }

    /// Get remaining tokens for an account
    pub fn remaining_tokens(&mut self, address: &Address, balance: u64, now: u64) -> u64 {
        let (bucket_size, refill_rate) = bucket_config_for_balance(balance);
        let bucket = self.buckets.entry(*address).or_insert_with(|| {
            TokenBucket::new(bucket_size, refill_rate, now)
        });
        bucket.refill(now);
        bucket.tokens
    }

    /// Prune buckets for accounts that haven't been seen in a while
    pub fn prune_stale(&mut self, now: u64, max_age_secs: u64) {
        self.buckets.retain(|_, bucket| {
            now.saturating_sub(bucket.last_refill) < max_age_secs
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_address(byte: u8) -> Address {
        Address([byte; 32])
    }

    // -- Minimum balance rejection ------------------------------------------

    #[test]
    fn reject_below_minimum_balance() {
        let mut limiter = RateLimiter::new();
        let addr = test_address(1);
        // Balance below MIN_BALANCE_TO_TRANSACT should be rejected
        assert!(!limiter.check_rate_limit(&addr, 999_999, 1000));
        assert!(!limiter.check_rate_limit(&addr, 0, 1000));
    }

    #[test]
    fn accept_at_minimum_balance() {
        let mut limiter = RateLimiter::new();
        let addr = test_address(2);
        assert!(limiter.check_rate_limit(&addr, MIN_BALANCE_TO_TRANSACT, 1000));
    }

    // -- Bucket refill over time --------------------------------------------

    #[test]
    fn bucket_refills_over_time() {
        let mut limiter = RateLimiter::new();
        let addr = test_address(3);
        let balance = 1_000_000; // $1 tier: 100 bucket, 1/sec refill

        // Consume all 100 tokens
        for _ in 0..100 {
            assert!(limiter.check_rate_limit(&addr, balance, 1000));
        }
        // 101st should fail
        assert!(!limiter.check_rate_limit(&addr, balance, 1000));

        // Wait 10 seconds — should get 10 tokens back
        for _ in 0..10 {
            assert!(limiter.check_rate_limit(&addr, balance, 1010));
        }
        assert!(!limiter.check_rate_limit(&addr, balance, 1010));
    }

    // -- Burst (consume 100 tokens instantly) --------------------------------

    #[test]
    fn burst_consume_all_tokens() {
        let mut bucket = TokenBucket::new(100, 1, 0);
        // Consume all 100 at time 0
        for i in 0..100 {
            assert!(bucket.try_consume(0), "failed at token {}", i);
        }
        assert!(!bucket.try_consume(0)); // empty
        assert_eq!(bucket.tokens, 0);
    }

    // -- Tier scaling -------------------------------------------------------

    #[test]
    fn tier_scaling_different_balances() {
        // $1 tier
        let (size, rate) = bucket_config_for_balance(1_000_000);
        assert_eq!(size, 100);
        assert_eq!(rate, 1);

        // $100 tier
        let (size, rate) = bucket_config_for_balance(100_000_000);
        assert_eq!(size, 500);
        assert_eq!(rate, 5);

        // $1,000 tier
        let (size, rate) = bucket_config_for_balance(1_000_000_000);
        assert_eq!(size, 2_000);
        assert_eq!(rate, 20);

        // $10,000 tier
        let (size, rate) = bucket_config_for_balance(10_000_000_000);
        assert_eq!(size, 10_000);
        assert_eq!(rate, 100);

        // $100,000 tier
        let (size, rate) = bucket_config_for_balance(100_000_000_000);
        assert_eq!(size, 50_000);
        assert_eq!(rate, 500);
    }

    #[test]
    fn below_all_tiers_gets_minimum() {
        let (size, rate) = bucket_config_for_balance(500_000); // $0.50
        assert_eq!(size, MIN_BUCKET_SIZE);
        assert_eq!(rate, MIN_REFILL_RATE);
    }

    // -- try_consume_n for batch --------------------------------------------

    #[test]
    fn try_consume_n_batch() {
        let mut bucket = TokenBucket::new(100, 1, 0);
        // Consume 50 at once
        assert!(bucket.try_consume_n(50, 0));
        assert_eq!(bucket.tokens, 50);
        // Consume another 50
        assert!(bucket.try_consume_n(50, 0));
        assert_eq!(bucket.tokens, 0);
        // Cannot consume 1 more
        assert!(!bucket.try_consume_n(1, 0));
    }

    #[test]
    fn try_consume_n_insufficient() {
        let mut bucket = TokenBucket::new(100, 1, 0);
        // Try to consume more than available
        assert!(!bucket.try_consume_n(101, 0));
        // Tokens unchanged
        assert_eq!(bucket.tokens, 100);
    }

    // -- Prune stale --------------------------------------------------------

    #[test]
    fn prune_stale_buckets() {
        let mut limiter = RateLimiter::new();
        let addr1 = test_address(10);
        let addr2 = test_address(11);

        // Create buckets at different times
        limiter.check_rate_limit(&addr1, 1_000_000, 100);
        limiter.check_rate_limit(&addr2, 1_000_000, 500);

        // Prune at time 700 with max_age 300 — addr1 (last_refill=100) is stale
        limiter.prune_stale(700, 300);

        // addr1 should be pruned, addr2 should remain
        assert_eq!(limiter.remaining_tokens(&addr2, 1_000_000, 700), 99); // had 1 consumed
    }

    // -- Remaining tokens ---------------------------------------------------

    #[test]
    fn remaining_tokens_after_consumption() {
        let mut limiter = RateLimiter::new();
        let addr = test_address(20);
        let balance = 1_000_000;

        // Full bucket = 100
        assert_eq!(limiter.remaining_tokens(&addr, balance, 0), 100);

        // Consume 5
        for _ in 0..5 {
            limiter.check_rate_limit(&addr, balance, 0);
        }
        assert_eq!(limiter.remaining_tokens(&addr, balance, 0), 95);
    }
}
