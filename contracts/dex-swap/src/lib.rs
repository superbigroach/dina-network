use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DinaDEX — Automated Market Maker (Uniswap V2-style constant product AMM)
// ---------------------------------------------------------------------------

/// Individual liquidity pool for a token pair.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Pool {
    pub id: u64,
    pub token_a: String,
    pub token_b: String,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub total_lp_tokens: u64,
    pub lp_balances: BTreeMap<String, u64>,
    pub fee_bps: u64,
    pub cumulative_volume: u64,
    pub paused: bool,
}

/// Quote result returned by `get_quote`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Quote {
    pub input_token: String,
    pub input_amount: u64,
    pub output_token: String,
    pub output_amount: u64,
    pub price_impact_bps: u64,
    pub fee_amount: u64,
}

/// Global DEX state persisted on-chain.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DexState {
    pub pools: BTreeMap<u64, Pool>,
    pub pool_lookup: BTreeMap<(String, String), u64>,
    pub next_pool_id: u64,
    pub owner: String,
    pub protocol_fee_bps: u64,
    pub protocol_fees: BTreeMap<String, u64>,
}

// ---------------------------------------------------------------------------
// Events (serialised to JSON strings by callers / runtime)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "event")]
pub enum DexEvent {
    PoolCreated {
        pool_id: u64,
        token_a: String,
        token_b: String,
    },
    LiquidityAdded {
        pool_id: u64,
        provider: String,
        amount_a: u64,
        amount_b: u64,
        lp_tokens: u64,
    },
    LiquidityRemoved {
        pool_id: u64,
        provider: String,
        amount_a: u64,
        amount_b: u64,
        lp_burned: u64,
    },
    Swap {
        pool_id: u64,
        trader: String,
        input_token: String,
        input_amount: u64,
        output_token: String,
        output_amount: u64,
    },
    FeeCollected {
        token: String,
        amount: u64,
        collector: String,
    },
    PoolPaused {
        pool_id: u64,
    },
    PoolUnpaused {
        pool_id: u64,
    },
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Canonical ordering so (A, B) and (B, A) map to the same pool.
fn ordered_pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

/// Integer square root (Babylonian method).
fn isqrt(n: u128) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x as u64
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl DexState {
    // -- Constructor --------------------------------------------------------

    pub fn new(owner: String) -> Self {
        Self {
            pools: BTreeMap::new(),
            pool_lookup: BTreeMap::new(),
            next_pool_id: 1,
            owner,
            protocol_fee_bps: 0, // 0% (fee-free) — owner can set later if needed
            protocol_fees: BTreeMap::new(),
        }
    }

    // -- Pool creation ------------------------------------------------------

    pub fn create_pool(&mut self, token_a: &str, token_b: &str) -> DexEvent {
        assert!(token_a != token_b, "DEX: identical tokens");
        let key = ordered_pair(token_a, token_b);
        assert!(
            !self.pool_lookup.contains_key(&key),
            "DEX: pool already exists"
        );

        let id = self.next_pool_id;
        self.next_pool_id += 1;

        let pool = Pool {
            id,
            token_a: key.0.clone(),
            token_b: key.1.clone(),
            reserve_a: 0,
            reserve_b: 0,
            total_lp_tokens: 0,
            lp_balances: BTreeMap::new(),
            fee_bps: 0, // 0% (fee-free) — owner can set fees later if needed
            cumulative_volume: 0,
            paused: false,
        };

        self.pools.insert(id, pool);
        self.pool_lookup.insert(key.clone(), id);

        DexEvent::PoolCreated {
            pool_id: id,
            token_a: key.0,
            token_b: key.1,
        }
    }

    // -- Liquidity ----------------------------------------------------------

    pub fn add_liquidity(
        &mut self,
        pool_id: u64,
        provider: &str,
        amount_a: u64,
        amount_b: u64,
        min_lp_tokens: u64,
    ) -> DexEvent {
        let pool = self.pools.get_mut(&pool_id).expect("DEX: pool not found");
        assert!(!pool.paused, "DEX: pool is paused");
        assert!(amount_a > 0 && amount_b > 0, "DEX: amounts must be positive");

        let lp_tokens = if pool.total_lp_tokens == 0 {
            // First deposit — LP tokens = sqrt(amount_a * amount_b)
            let minted = isqrt(amount_a as u128 * amount_b as u128);
            assert!(minted > 0, "DEX: insufficient initial liquidity");
            minted
        } else {
            // Subsequent deposits — mint proportional to the smaller ratio
            let lp_a = (amount_a as u128 * pool.total_lp_tokens as u128)
                / pool.reserve_a as u128;
            let lp_b = (amount_b as u128 * pool.total_lp_tokens as u128)
                / pool.reserve_b as u128;
            std::cmp::min(lp_a, lp_b) as u64
        };

        assert!(
            lp_tokens >= min_lp_tokens,
            "DEX: insufficient LP tokens ({lp_tokens} < {min_lp_tokens})"
        );

        pool.reserve_a += amount_a;
        pool.reserve_b += amount_b;
        pool.total_lp_tokens += lp_tokens;
        *pool
            .lp_balances
            .entry(provider.to_string())
            .or_insert(0) += lp_tokens;

        DexEvent::LiquidityAdded {
            pool_id,
            provider: provider.to_string(),
            amount_a,
            amount_b,
            lp_tokens,
        }
    }

    pub fn remove_liquidity(
        &mut self,
        pool_id: u64,
        provider: &str,
        lp_amount: u64,
        min_a: u64,
        min_b: u64,
    ) -> DexEvent {
        let pool = self.pools.get_mut(&pool_id).expect("DEX: pool not found");
        assert!(!pool.paused, "DEX: pool is paused");
        assert!(lp_amount > 0, "DEX: lp_amount must be positive");

        let provider_balance = pool
            .lp_balances
            .get(provider)
            .copied()
            .unwrap_or(0);
        assert!(
            provider_balance >= lp_amount,
            "DEX: insufficient LP balance ({provider_balance} < {lp_amount})"
        );

        let amount_a =
            (lp_amount as u128 * pool.reserve_a as u128 / pool.total_lp_tokens as u128) as u64;
        let amount_b =
            (lp_amount as u128 * pool.reserve_b as u128 / pool.total_lp_tokens as u128) as u64;

        assert!(
            amount_a >= min_a,
            "DEX: slippage on token_a ({amount_a} < {min_a})"
        );
        assert!(
            amount_b >= min_b,
            "DEX: slippage on token_b ({amount_b} < {min_b})"
        );

        pool.reserve_a -= amount_a;
        pool.reserve_b -= amount_b;
        pool.total_lp_tokens -= lp_amount;
        pool.lp_balances
            .insert(provider.to_string(), provider_balance - lp_amount);

        DexEvent::LiquidityRemoved {
            pool_id,
            provider: provider.to_string(),
            amount_a,
            amount_b,
            lp_burned: lp_amount,
        }
    }

    // -- Swaps --------------------------------------------------------------

    /// Calculate output amount for a given input using constant product formula.
    /// Returns (output_amount, fee_amount, protocol_fee_amount).
    fn compute_swap(
        &self,
        reserve_in: u64,
        reserve_out: u64,
        input_amount: u64,
        fee_bps: u64,
    ) -> (u64, u64, u64) {
        assert!(reserve_in > 0 && reserve_out > 0, "DEX: empty reserves");
        assert!(input_amount > 0, "DEX: input must be positive");

        let total_fee = (input_amount as u128 * fee_bps as u128) / 10_000;
        let protocol_fee = (input_amount as u128 * self.protocol_fee_bps as u128) / 10_000;
        let input_with_fee = input_amount as u128 * (10_000 - fee_bps) as u128;
        let numerator = reserve_out as u128 * input_with_fee;
        let denominator = reserve_in as u128 * 10_000 + input_with_fee;
        let output = (numerator / denominator) as u64;

        (output, total_fee as u64, protocol_fee as u64)
    }

    /// Calculate required input amount for a desired output using constant product formula.
    /// Returns (input_amount, fee_amount, protocol_fee_amount).
    fn compute_swap_exact_out(
        &self,
        reserve_in: u64,
        reserve_out: u64,
        output_amount: u64,
        fee_bps: u64,
    ) -> (u64, u64, u64) {
        assert!(reserve_in > 0 && reserve_out > 0, "DEX: empty reserves");
        assert!(output_amount > 0, "DEX: output must be positive");
        assert!(
            output_amount < reserve_out,
            "DEX: insufficient liquidity for output"
        );

        let numerator = reserve_in as u128 * output_amount as u128 * 10_000;
        let denominator = (reserve_out as u128 - output_amount as u128) * (10_000 - fee_bps) as u128;
        let input = (numerator / denominator) as u64 + 1; // round up

        let total_fee = (input as u128 * fee_bps as u128) / 10_000;
        let protocol_fee = (input as u128 * self.protocol_fee_bps as u128) / 10_000;

        (input, total_fee as u64, protocol_fee as u64)
    }

    /// Resolve which reserves are (in, out) given the input token.
    fn resolve_reserves(pool: &Pool, input_token: &str) -> (u64, u64, bool) {
        if input_token == pool.token_a {
            (pool.reserve_a, pool.reserve_b, true) // a_is_input = true
        } else if input_token == pool.token_b {
            (pool.reserve_b, pool.reserve_a, false)
        } else {
            panic!("DEX: token not in pool");
        }
    }

    pub fn swap_exact_in(
        &mut self,
        pool_id: u64,
        trader: &str,
        input_token: &str,
        input_amount: u64,
        min_output: u64,
    ) -> DexEvent {
        let pool = self.pools.get(&pool_id).expect("DEX: pool not found");
        assert!(!pool.paused, "DEX: pool is paused");

        let (reserve_in, reserve_out, a_is_input) =
            Self::resolve_reserves(pool, input_token);
        let (output_amount, _fee, protocol_fee) =
            self.compute_swap(reserve_in, reserve_out, input_amount, pool.fee_bps);

        assert!(
            output_amount >= min_output,
            "DEX: slippage exceeded ({output_amount} < {min_output})"
        );

        let output_token = if a_is_input {
            pool.token_b.clone()
        } else {
            pool.token_a.clone()
        };

        // Accumulate protocol fees on the input token
        *self
            .protocol_fees
            .entry(input_token.to_string())
            .or_insert(0) += protocol_fee;

        // Update reserves
        let pool = self.pools.get_mut(&pool_id).unwrap();
        if a_is_input {
            pool.reserve_a += input_amount;
            pool.reserve_b -= output_amount;
        } else {
            pool.reserve_b += input_amount;
            pool.reserve_a -= output_amount;
        }
        pool.cumulative_volume += input_amount;

        DexEvent::Swap {
            pool_id,
            trader: trader.to_string(),
            input_token: input_token.to_string(),
            input_amount,
            output_token,
            output_amount,
        }
    }

    pub fn swap_exact_out(
        &mut self,
        pool_id: u64,
        trader: &str,
        output_token: &str,
        output_amount: u64,
        max_input: u64,
    ) -> DexEvent {
        let pool = self.pools.get(&pool_id).expect("DEX: pool not found");
        assert!(!pool.paused, "DEX: pool is paused");

        // Output token determines which side is "out"
        let (reserve_in, reserve_out, a_is_input) = if output_token == pool.token_b {
            (pool.reserve_a, pool.reserve_b, true)
        } else if output_token == pool.token_a {
            (pool.reserve_b, pool.reserve_a, false)
        } else {
            panic!("DEX: token not in pool");
        };

        let (input_amount, _fee, protocol_fee) =
            self.compute_swap_exact_out(reserve_in, reserve_out, output_amount, pool.fee_bps);

        assert!(
            input_amount <= max_input,
            "DEX: input exceeds max ({input_amount} > {max_input})"
        );

        let input_token = if a_is_input {
            pool.token_a.clone()
        } else {
            pool.token_b.clone()
        };

        // Accumulate protocol fees
        *self
            .protocol_fees
            .entry(input_token.clone())
            .or_insert(0) += protocol_fee;

        // Update reserves
        let pool = self.pools.get_mut(&pool_id).unwrap();
        if a_is_input {
            pool.reserve_a += input_amount;
            pool.reserve_b -= output_amount;
        } else {
            pool.reserve_b += input_amount;
            pool.reserve_a -= output_amount;
        }
        pool.cumulative_volume += input_amount;

        DexEvent::Swap {
            pool_id,
            trader: trader.to_string(),
            input_token,
            input_amount,
            output_token: output_token.to_string(),
            output_amount,
        }
    }

    // -- Quotes (read-only) -------------------------------------------------

    pub fn get_quote(
        &self,
        pool_id: u64,
        input_token: &str,
        input_amount: u64,
    ) -> Quote {
        let pool = self.pools.get(&pool_id).expect("DEX: pool not found");
        let (reserve_in, reserve_out, a_is_input) =
            Self::resolve_reserves(pool, input_token);
        let (output_amount, fee_amount, _protocol_fee) =
            self.compute_swap(reserve_in, reserve_out, input_amount, pool.fee_bps);

        let output_token = if a_is_input {
            pool.token_b.clone()
        } else {
            pool.token_a.clone()
        };

        // Price impact = 1 - (output / ideal_output)  in bps
        let ideal_output =
            (input_amount as u128 * reserve_out as u128 / reserve_in as u128) as u64;
        let impact_bps = if ideal_output > 0 {
            ((ideal_output - output_amount) as u128 * 10_000 / ideal_output as u128) as u64
        } else {
            0
        };

        Quote {
            input_token: input_token.to_string(),
            input_amount,
            output_token,
            output_amount,
            price_impact_bps: impact_bps,
            fee_amount,
        }
    }

    // -- Multi-hop routing --------------------------------------------------

    /// Execute a swap through multiple pools sequentially.
    /// `path` contains pool IDs in order; the output of each swap becomes the
    /// input of the next.
    pub fn swap_route(
        &mut self,
        trader: &str,
        path: &[u64],
        input_token: &str,
        input_amount: u64,
        min_output: u64,
    ) -> Vec<DexEvent> {
        assert!(!path.is_empty(), "DEX: empty path");

        let mut current_token = input_token.to_string();
        let mut current_amount = input_amount;
        let mut events = Vec::new();

        for &pool_id in path {
            let evt =
                self.swap_exact_in(pool_id, trader, &current_token, current_amount, 0);
            if let DexEvent::Swap {
                ref output_token,
                output_amount,
                ..
            } = evt
            {
                current_token = output_token.clone();
                current_amount = output_amount;
            }
            events.push(evt);
        }

        assert!(
            current_amount >= min_output,
            "DEX: route slippage exceeded ({current_amount} < {min_output})"
        );

        events
    }

    /// Find the best single or two-hop route between two tokens.
    /// Returns the list of pool IDs forming the route, or empty if none found.
    pub fn get_best_route(
        &self,
        input_token: &str,
        output_token: &str,
        input_amount: u64,
    ) -> (Vec<u64>, u64) {
        // Try direct pool
        let direct_key = ordered_pair(input_token, output_token);
        let mut best_output = 0u64;
        let mut best_path: Vec<u64> = Vec::new();

        if let Some(&pool_id) = self.pool_lookup.get(&direct_key) {
            let pool = self.pools.get(&pool_id).unwrap();
            if !pool.paused && pool.reserve_a > 0 && pool.reserve_b > 0 {
                let (reserve_in, reserve_out, _) =
                    Self::resolve_reserves(pool, input_token);
                let (out, _, _) =
                    self.compute_swap(reserve_in, reserve_out, input_amount, pool.fee_bps);
                if out > best_output {
                    best_output = out;
                    best_path = vec![pool_id];
                }
            }
        }

        // Try two-hop through every intermediate token
        for (key, &pid1) in &self.pool_lookup {
            let pool1 = self.pools.get(&pid1).unwrap();
            if pool1.paused || pool1.reserve_a == 0 || pool1.reserve_b == 0 {
                continue;
            }

            // Determine if input_token is in this pool
            let intermediate = if key.0 == input_token {
                &key.1
            } else if key.1 == input_token {
                &key.0
            } else {
                continue;
            };

            // Skip if intermediate IS the output (that would be the direct pool)
            if intermediate == output_token {
                continue;
            }

            // Check if there is a pool from intermediate → output_token
            let second_key = ordered_pair(intermediate, output_token);
            if let Some(&pid2) = self.pool_lookup.get(&second_key) {
                let pool2 = self.pools.get(&pid2).unwrap();
                if pool2.paused || pool2.reserve_a == 0 || pool2.reserve_b == 0 {
                    continue;
                }

                // Simulate first hop
                let (r_in1, r_out1, _) =
                    Self::resolve_reserves(pool1, input_token);
                let (mid_out, _, _) =
                    self.compute_swap(r_in1, r_out1, input_amount, pool1.fee_bps);

                if mid_out == 0 {
                    continue;
                }

                // Simulate second hop
                let (r_in2, r_out2, _) =
                    Self::resolve_reserves(pool2, intermediate);
                let (final_out, _, _) =
                    self.compute_swap(r_in2, r_out2, mid_out, pool2.fee_bps);

                if final_out > best_output {
                    best_output = final_out;
                    best_path = vec![pid1, pid2];
                }
            }
        }

        (best_path, best_output)
    }

    // -- Admin --------------------------------------------------------------

    pub fn set_fee(&mut self, caller: &str, pool_id: u64, fee_bps: u64) {
        assert!(caller == self.owner, "DEX: not owner");
        assert!(fee_bps <= 1000, "DEX: fee too high (max 10%)");
        let pool = self.pools.get_mut(&pool_id).expect("DEX: pool not found");
        pool.fee_bps = fee_bps;
    }

    pub fn collect_protocol_fees(&mut self, caller: &str, token: &str) -> DexEvent {
        assert!(caller == self.owner, "DEX: not owner");
        let amount = self
            .protocol_fees
            .get(token)
            .copied()
            .unwrap_or(0);
        assert!(amount > 0, "DEX: no fees to collect");
        self.protocol_fees.insert(token.to_string(), 0);

        DexEvent::FeeCollected {
            token: token.to_string(),
            amount,
            collector: caller.to_string(),
        }
    }

    pub fn pause_pool(&mut self, caller: &str, pool_id: u64) -> DexEvent {
        assert!(caller == self.owner, "DEX: not owner");
        let pool = self.pools.get_mut(&pool_id).expect("DEX: pool not found");
        assert!(!pool.paused, "DEX: already paused");
        pool.paused = true;
        DexEvent::PoolPaused { pool_id }
    }

    pub fn unpause_pool(&mut self, caller: &str, pool_id: u64) -> DexEvent {
        assert!(caller == self.owner, "DEX: not owner");
        let pool = self.pools.get_mut(&pool_id).expect("DEX: pool not found");
        assert!(pool.paused, "DEX: not paused");
        pool.paused = false;
        DexEvent::PoolUnpaused { pool_id }
    }

    // -- View helpers -------------------------------------------------------

    pub fn get_pool(&self, pool_id: u64) -> Option<&Pool> {
        self.pools.get(&pool_id)
    }

    pub fn find_pool(&self, token_a: &str, token_b: &str) -> Option<u64> {
        let key = ordered_pair(token_a, token_b);
        self.pool_lookup.get(&key).copied()
    }

    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> DexState {
        DexState::new("owner".to_string())
    }

    // 1. Create pool
    #[test]
    fn test_create_pool() {
        let mut dex = setup();
        let evt = dex.create_pool("USDC", "ETH");
        assert!(matches!(evt, DexEvent::PoolCreated { pool_id: 1, .. }));
        assert_eq!(dex.pool_count(), 1);
        assert_eq!(dex.find_pool("ETH", "USDC"), Some(1));
    }

    // 2. Cannot create duplicate pool
    #[test]
    #[should_panic(expected = "pool already exists")]
    fn test_create_duplicate_pool() {
        let mut dex = setup();
        dex.create_pool("USDC", "ETH");
        dex.create_pool("ETH", "USDC"); // reversed order, same pair
    }

    // 3. Add first liquidity
    #[test]
    fn test_add_first_liquidity() {
        let mut dex = setup();
        dex.create_pool("USDC", "ETH");
        let evt = dex.add_liquidity(1, "alice", 10_000, 5_000, 0);
        if let DexEvent::LiquidityAdded { lp_tokens, .. } = &evt {
            // sqrt(10000 * 5000) = sqrt(50_000_000) ≈ 7071
            assert_eq!(*lp_tokens, isqrt(10_000u128 * 5_000));
        } else {
            panic!("expected LiquidityAdded");
        }
        let pool = dex.get_pool(1).unwrap();
        assert_eq!(pool.reserve_a, 10_000);
        assert_eq!(pool.reserve_b, 5_000);
    }

    // 4. Add subsequent liquidity (proportional)
    #[test]
    fn test_add_subsequent_liquidity() {
        let mut dex = setup();
        dex.create_pool("USDC", "ETH");
        dex.add_liquidity(1, "alice", 10_000, 5_000, 0);

        let evt = dex.add_liquidity(1, "bob", 2_000, 1_000, 0);
        if let DexEvent::LiquidityAdded { lp_tokens, .. } = &evt {
            assert!(*lp_tokens > 0);
        } else {
            panic!("expected LiquidityAdded");
        }
        let pool = dex.get_pool(1).unwrap();
        assert_eq!(pool.reserve_a, 12_000);
        assert_eq!(pool.reserve_b, 6_000);
    }

    // 5. Swap exact in (constant product)
    #[test]
    fn test_swap_exact_in() {
        let mut dex = setup();
        dex.create_pool("USDC", "ETH");
        dex.add_liquidity(1, "alice", 100_000, 50_000, 0);

        let pool_before = dex.get_pool(1).unwrap().clone();
        let k_before = pool_before.reserve_a as u128 * pool_before.reserve_b as u128;

        let evt = dex.swap_exact_in(1, "bob", "USDC", 1_000, 0);
        if let DexEvent::Swap { output_amount, .. } = &evt {
            assert!(*output_amount > 0);
            // Pool is ordered (ETH, USDC) with reserves (100k, 50k).
            // Swapping 1000 USDC in: reserve_in=50k, reserve_out=100k → ~1960 ETH out.
            // Output must be less than ideal 2000 (due to slippage; 0% fee-free).
            assert!(*output_amount < 2_000);
        } else {
            panic!("expected Swap");
        }

        // k must not decrease (fees make it grow)
        let pool_after = dex.get_pool(1).unwrap();
        let k_after = pool_after.reserve_a as u128 * pool_after.reserve_b as u128;
        assert!(k_after >= k_before, "constant product must not decrease");
    }

    // 6. Swap exact out
    #[test]
    fn test_swap_exact_out() {
        let mut dex = setup();
        dex.create_pool("USDC", "ETH");
        dex.add_liquidity(1, "alice", 100_000, 50_000, 0);

        let evt = dex.swap_exact_out(1, "bob", "ETH", 490, 10_000);
        if let DexEvent::Swap {
            input_amount,
            output_amount,
            ..
        } = &evt
        {
            assert_eq!(*output_amount, 490);
            assert!(*input_amount > 0);
            assert!(*input_amount <= 10_000);
        } else {
            panic!("expected Swap");
        }
    }

    // 7. Remove liquidity
    #[test]
    fn test_remove_liquidity() {
        let mut dex = setup();
        dex.create_pool("USDC", "ETH");
        dex.add_liquidity(1, "alice", 10_000, 5_000, 0);

        let pool = dex.get_pool(1).unwrap();
        let alice_lp = pool.lp_balances.get("alice").copied().unwrap();

        // Remove half
        let half = alice_lp / 2;
        let evt = dex.remove_liquidity(1, "alice", half, 0, 0);
        if let DexEvent::LiquidityRemoved {
            amount_a,
            amount_b,
            ..
        } = &evt
        {
            assert!(*amount_a > 0);
            assert!(*amount_b > 0);
        } else {
            panic!("expected LiquidityRemoved");
        }

        let pool = dex.get_pool(1).unwrap();
        assert!(pool.reserve_a > 0);
        assert!(pool.reserve_b > 0);
    }

    // 8. Slippage protection — revert on min_output
    #[test]
    #[should_panic(expected = "slippage exceeded")]
    fn test_slippage_protection() {
        let mut dex = setup();
        dex.create_pool("USDC", "ETH");
        dex.add_liquidity(1, "alice", 100_000, 50_000, 0);

        // Ask for impossibly high min_output
        dex.swap_exact_in(1, "bob", "USDC", 1_000, 999_999);
    }

    // 9. Fee calculation correctness — 0% (fee-free) by default
    #[test]
    fn test_fee_calculation() {
        let mut dex = setup();
        dex.create_pool("USDC", "ETH");
        dex.add_liquidity(1, "alice", 1_000_000, 500_000, 0);

        let quote = dex.get_quote(1, "USDC", 10_000);
        // Fee = 10_000 * 0 / 10_000 = 0 (fee-free)
        assert_eq!(quote.fee_amount, 0);
        assert!(quote.output_amount > 0);
        assert!(quote.price_impact_bps > 0);
    }

    // 10. Multi-hop swap route
    #[test]
    fn test_multi_hop_route() {
        let mut dex = setup();
        // Create USDC/ETH and ETH/SOL pools
        dex.create_pool("ETH", "USDC");
        dex.create_pool("ETH", "SOL");
        dex.add_liquidity(1, "lp", 100_000, 50_000, 0);
        dex.add_liquidity(2, "lp", 50_000, 200_000, 0);

        // Swap USDC → ETH → SOL
        let events = dex.swap_route("bob", &[1, 2], "USDC", 1_000, 0);
        assert_eq!(events.len(), 2);

        // Final output should be SOL
        if let DexEvent::Swap { output_token, output_amount, .. } = &events[1] {
            assert_eq!(output_token, "SOL");
            assert!(*output_amount > 0);
        }
    }

    // 11. Protocol fee accumulation — 0% (fee-free) by default, but test with explicit fee
    #[test]
    fn test_protocol_fee_accumulation() {
        let mut dex = setup();
        dex.create_pool("USDC", "ETH");
        dex.add_liquidity(1, "alice", 1_000_000, 500_000, 0);

        // With default 0% fees, no protocol fees accumulate
        dex.swap_exact_in(1, "bob", "USDC", 10_000, 0);
        dex.swap_exact_in(1, "bob", "USDC", 20_000, 0);

        let fees = dex.protocol_fees.get("USDC").copied().unwrap_or(0);
        // protocol_fee_bps = 0 → no fees collected
        assert_eq!(fees, 0);

        // Now set fees explicitly (owner CAN enable fees later)
        dex.set_fee("owner", 1, 30); // 0.3%
        dex.protocol_fee_bps = 5; // 0.05%
        dex.swap_exact_in(1, "bob", "USDC", 10_000, 0);

        let fees_after = dex.protocol_fees.get("USDC").copied().unwrap_or(0);
        // protocol_fee_bps = 5 → 10000*5/10000 = 5
        assert_eq!(fees_after, 5);

        // Collect
        let evt = dex.collect_protocol_fees("owner", "USDC");
        if let DexEvent::FeeCollected { amount, .. } = &evt {
            assert_eq!(*amount, 5);
        }
        assert_eq!(dex.protocol_fees.get("USDC").copied().unwrap_or(0), 0);
    }

    // 12. Cannot swap on paused pool
    #[test]
    #[should_panic(expected = "pool is paused")]
    fn test_cannot_swap_paused_pool() {
        let mut dex = setup();
        dex.create_pool("USDC", "ETH");
        dex.add_liquidity(1, "alice", 100_000, 50_000, 0);

        dex.pause_pool("owner", 1);
        dex.swap_exact_in(1, "bob", "USDC", 1_000, 0);
    }

    // 13. Unpause and swap succeeds
    #[test]
    fn test_unpause_then_swap() {
        let mut dex = setup();
        dex.create_pool("USDC", "ETH");
        dex.add_liquidity(1, "alice", 100_000, 50_000, 0);

        dex.pause_pool("owner", 1);
        dex.unpause_pool("owner", 1);
        let evt = dex.swap_exact_in(1, "bob", "USDC", 1_000, 0);
        assert!(matches!(evt, DexEvent::Swap { .. }));
    }

    // 14. Set fee (admin only)
    #[test]
    #[should_panic(expected = "not owner")]
    fn test_set_fee_not_owner() {
        let mut dex = setup();
        dex.create_pool("USDC", "ETH");
        dex.set_fee("hacker", 1, 100);
    }

    // 15. get_best_route finds two-hop path
    #[test]
    fn test_get_best_route() {
        let mut dex = setup();
        dex.create_pool("ETH", "USDC");
        dex.create_pool("ETH", "SOL");
        dex.add_liquidity(1, "lp", 100_000, 100_000, 0);
        dex.add_liquidity(2, "lp", 100_000, 100_000, 0);

        let (path, output) = dex.get_best_route("USDC", "SOL", 1_000);
        assert_eq!(path.len(), 2);
        assert!(output > 0);
    }
}
