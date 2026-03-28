use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Dina FX Swap — Oracle-priced foreign exchange swap contract
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FxSwapState {
    pub admin: Address,
    /// Liquidity pools: currency_symbol -> pool balance in that currency's micro-units
    pub pools: BTreeMap<String, u64>,
    /// USDC reserves backing all pools
    pub usdc_reserves: u64,
    /// Oracle prices: currency_symbol -> micro-units per 1 USDC
    /// e.g., "EURC" -> 930_000 means 0.93 EUR per $1
    pub oracle_rates: BTreeMap<String, u64>,
    /// Oracle updater addresses (can be multiple oracles)
    pub oracle_updaters: Vec<Address>,
    /// Last oracle update timestamp per currency
    pub oracle_timestamps: BTreeMap<String, u64>,
    /// Max single swap as fraction of pool (basis points, e.g., 100 = 1%)
    pub max_swap_bps: u64,
    /// Oracle staleness limit (seconds)
    pub max_oracle_age: u64,
    /// Total volume swapped (in USDC micro-units, for stats)
    pub total_volume_usdc: u64,
    pub paused: bool,
}

impl FxSwapState {
    pub fn new(admin: Address, max_swap_bps: u64, max_oracle_age: u64) -> Self {
        Self {
            admin,
            pools: BTreeMap::new(),
            usdc_reserves: 0,
            oracle_rates: BTreeMap::new(),
            oracle_updaters: vec![admin],
            oracle_timestamps: BTreeMap::new(),
            max_swap_bps,
            max_oracle_age,
            total_volume_usdc: 0,
            paused: false,
        }
    }

    // -- Oracle --------------------------------------------------------------

    fn is_oracle_updater(&self, addr: &Address) -> bool {
        self.oracle_updaters.contains(addr)
    }

    /// Update oracle rate for a currency. Only authorized oracle updaters.
    pub fn update_oracle_rate(
        &mut self,
        caller: Address,
        symbol: String,
        rate: u64,
        timestamp: u64,
    ) {
        assert!(
            self.is_oracle_updater(&caller),
            "only oracle updaters can set rates"
        );
        assert!(rate > 0, "rate must be positive");
        // Timestamp must be newer than existing (no stale replays)
        if let Some(&existing_ts) = self.oracle_timestamps.get(&symbol) {
            assert!(
                timestamp >= existing_ts,
                "oracle timestamp must be newer or equal"
            );
        }
        self.oracle_rates.insert(symbol.clone(), rate);
        self.oracle_timestamps.insert(symbol, timestamp);
    }

    /// Add an oracle updater (admin only).
    pub fn add_oracle_updater(&mut self, caller: Address, updater: Address) {
        assert!(caller == self.admin, "only admin");
        if !self.oracle_updaters.contains(&updater) {
            self.oracle_updaters.push(updater);
        }
    }

    /// Remove an oracle updater (admin only).
    pub fn remove_oracle_updater(&mut self, caller: Address, updater: Address) {
        assert!(caller == self.admin, "only admin");
        self.oracle_updaters.retain(|u| u != &updater);
    }

    // -- Liquidity -----------------------------------------------------------

    /// Add liquidity to a pool (admin only).
    pub fn add_liquidity(&mut self, caller: Address, symbol: String, amount: u64) {
        assert!(caller == self.admin, "only admin can add liquidity");
        assert!(amount > 0, "amount must be positive");
        let pool = self.pools.entry(symbol).or_insert(0);
        *pool = pool.checked_add(amount).expect("pool overflow");
    }

    /// Remove liquidity from a pool (admin only).
    pub fn remove_liquidity(&mut self, caller: Address, symbol: String, amount: u64) {
        assert!(caller == self.admin, "only admin can remove liquidity");
        let pool = self.pools.get_mut(&symbol).expect("pool does not exist");
        assert!(*pool >= amount, "insufficient pool liquidity");
        *pool -= amount;
    }

    /// Add USDC reserves (admin only).
    pub fn add_usdc_reserves(&mut self, caller: Address, amount: u64) {
        assert!(caller == self.admin, "only admin");
        self.usdc_reserves = self
            .usdc_reserves
            .checked_add(amount)
            .expect("reserves overflow");
    }

    // -- Swap ----------------------------------------------------------------

    /// Get a price quote for a swap (read-only, no state changes).
    pub fn get_quote(
        &self,
        from_symbol: &str,
        to_symbol: &str,
        amount: u64,
    ) -> u64 {
        let from_rate = *self
            .oracle_rates
            .get(from_symbol)
            .expect("from currency rate not set");
        let to_rate = *self
            .oracle_rates
            .get(to_symbol)
            .expect("to currency rate not set");

        // output = amount * to_rate / from_rate
        (amount as u128 * to_rate as u128 / from_rate as u128) as u64
    }

    /// Execute a swap between two currencies using oracle rates.
    pub fn swap(
        &mut self,
        _caller: Address,
        from_symbol: String,
        to_symbol: String,
        amount: u64,
        min_output: u64,
        timestamp: u64,
    ) -> u64 {
        assert!(!self.paused, "contract paused");
        assert!(amount > 0, "swap amount must be positive");
        assert!(from_symbol != to_symbol, "cannot swap same currency");

        // Validate oracle freshness
        let from_ts = *self
            .oracle_timestamps
            .get(&from_symbol)
            .expect("from currency oracle not set");
        let to_ts = *self
            .oracle_timestamps
            .get(&to_symbol)
            .expect("to currency oracle not set");
        assert!(
            timestamp.saturating_sub(from_ts) <= self.max_oracle_age,
            "from currency oracle is stale"
        );
        assert!(
            timestamp.saturating_sub(to_ts) <= self.max_oracle_age,
            "to currency oracle is stale"
        );

        // Validate pool has enough liquidity in target currency
        let to_pool = *self.pools.get(&to_symbol).unwrap_or(&0);
        assert!(to_pool > 0, "to currency pool is empty");

        // Validate swap size limit
        if self.max_swap_bps > 0 {
            let max_amount = (to_pool as u128 * self.max_swap_bps as u128 / 10_000) as u64;
            // We check after computing output, but first validate source pool exists
            let from_pool = *self.pools.get(&from_symbol).unwrap_or(&0);
            let from_max = (from_pool as u128 * self.max_swap_bps as u128 / 10_000) as u64;
            // Amount must not exceed max_swap_bps of the source pool
            assert!(
                amount <= from_max || from_max == 0,
                "swap exceeds max size limit for source pool"
            );
            // We'll also check output against target pool below
            let _ = max_amount; // checked after output calculation
        }

        // Calculate output
        let output = self.get_quote(&from_symbol, &to_symbol, amount);
        assert!(output >= min_output, "output below minimum");

        // Check output doesn't exceed target pool max swap
        if self.max_swap_bps > 0 {
            let max_output = (to_pool as u128 * self.max_swap_bps as u128 / 10_000) as u64;
            assert!(
                output <= max_output || max_output == 0,
                "swap exceeds max size limit for target pool"
            );
        }

        // Check sufficient target liquidity
        assert!(to_pool >= output, "insufficient liquidity in target pool");

        // Execute: deduct from source pool, add to target pool
        let from_pool = self.pools.entry(from_symbol.clone()).or_insert(0);
        *from_pool = from_pool.checked_add(amount).expect("pool overflow");

        let to_pool = self.pools.get_mut(&to_symbol).unwrap();
        *to_pool -= output;

        // Track volume in USDC equivalent
        let from_rate = *self.oracle_rates.get(&from_symbol).unwrap();
        let usdc_volume = (amount as u128 * 1_000_000 / from_rate as u128) as u64;
        self.total_volume_usdc = self.total_volume_usdc.saturating_add(usdc_volume);

        output
    }

    // -- Proof of reserves ---------------------------------------------------

    /// Returns (total_usdc_reserves, total_pool_value_in_usdc, is_fully_backed).
    pub fn proof_of_reserves(&self) -> (u64, u64, bool) {
        let mut total_value: u128 = 0;
        for (symbol, &pool_amount) in &self.pools {
            if let Some(&rate) = self.oracle_rates.get(symbol) {
                // pool value in USDC = pool_amount * 1_000_000 / rate
                let value = pool_amount as u128 * 1_000_000 / rate as u128;
                total_value += value;
            }
        }
        let total_value = total_value as u64;
        let backed = self.usdc_reserves >= total_value;
        (self.usdc_reserves, total_value, backed)
    }

    // -- Admin ---------------------------------------------------------------

    pub fn pause(&mut self, caller: Address) {
        assert!(caller == self.admin, "only admin");
        self.paused = true;
    }

    pub fn unpause(&mut self, caller: Address) {
        assert!(caller == self.admin, "only admin");
        self.paused = false;
    }
}

// ---------------------------------------------------------------------------
// Dispatch argument structs
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    max_swap_bps: u64,
    max_oracle_age: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateOracleRateArgs {
    symbol: String,
    rate: u64,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct OracleUpdaterArgs {
    updater: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct LiquidityArgs {
    symbol: String,
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddUsdcReservesArgs {
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SwapArgs {
    from_symbol: String,
    to_symbol: String,
    amount: u64,
    min_output: u64,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetQuoteArgs {
    from_symbol: String,
    to_symbol: String,
    amount: u64,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<FxSwapState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "already initialised");
            let a: InitArgs =
                serde_json::from_slice(args).expect("bad init args");
            *state = Some(FxSwapState::new(caller, a.max_swap_bps, a.max_oracle_age));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Oracle ----------------------------------------------------------
        "update_oracle_rate" => {
            let s = state.as_mut().expect("not initialised");
            let a: UpdateOracleRateArgs =
                serde_json::from_slice(args).expect("bad update_oracle_rate args");
            s.update_oracle_rate(caller, a.symbol, a.rate, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }
        "add_oracle_updater" => {
            let s = state.as_mut().expect("not initialised");
            let a: OracleUpdaterArgs =
                serde_json::from_slice(args).expect("bad add_oracle_updater args");
            s.add_oracle_updater(caller, a.updater);
            serde_json::to_vec("ok").unwrap()
        }
        "remove_oracle_updater" => {
            let s = state.as_mut().expect("not initialised");
            let a: OracleUpdaterArgs =
                serde_json::from_slice(args).expect("bad remove_oracle_updater args");
            s.remove_oracle_updater(caller, a.updater);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Liquidity -------------------------------------------------------
        "add_liquidity" => {
            let s = state.as_mut().expect("not initialised");
            let a: LiquidityArgs =
                serde_json::from_slice(args).expect("bad add_liquidity args");
            s.add_liquidity(caller, a.symbol, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "remove_liquidity" => {
            let s = state.as_mut().expect("not initialised");
            let a: LiquidityArgs =
                serde_json::from_slice(args).expect("bad remove_liquidity args");
            s.remove_liquidity(caller, a.symbol, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "add_usdc_reserves" => {
            let s = state.as_mut().expect("not initialised");
            let a: AddUsdcReservesArgs =
                serde_json::from_slice(args).expect("bad add_usdc_reserves args");
            s.add_usdc_reserves(caller, a.amount);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Swap ------------------------------------------------------------
        "swap" => {
            let s = state.as_mut().expect("not initialised");
            let a: SwapArgs =
                serde_json::from_slice(args).expect("bad swap args");
            let output =
                s.swap(caller, a.from_symbol, a.to_symbol, a.amount, a.min_output, a.timestamp);
            serde_json::to_vec(&output).unwrap()
        }
        "get_quote" => {
            let s = state.as_ref().expect("not initialised");
            let a: GetQuoteArgs =
                serde_json::from_slice(args).expect("bad get_quote args");
            let quote = s.get_quote(&a.from_symbol, &a.to_symbol, a.amount);
            serde_json::to_vec(&quote).unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "proof_of_reserves" => {
            let s = state.as_ref().expect("not initialised");
            let (reserves, value, backed) = s.proof_of_reserves();
            serde_json::to_vec(&(reserves, value, backed)).unwrap()
        }

        // -- Admin -----------------------------------------------------------
        "pause" => {
            let s = state.as_mut().expect("not initialised");
            s.pause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "unpause" => {
            let s = state.as_mut().expect("not initialised");
            s.unpause(caller);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ADMIN: Address = [1u8; 32];
    const ORACLE: Address = [2u8; 32];
    const USER: Address = [3u8; 32];
    const T0: u64 = 1_700_000_000;

    fn setup_swap() -> FxSwapState {
        let mut s = FxSwapState::new(ADMIN, 500, 3600); // 5% max swap, 1hr staleness
        s.add_oracle_updater(ADMIN, ORACLE);

        // Set up oracle rates
        s.update_oracle_rate(ADMIN, "USDC".into(), 1_000_000, T0);
        s.update_oracle_rate(ADMIN, "EURC".into(), 930_000, T0);
        s.update_oracle_rate(ADMIN, "GBPC".into(), 790_000, T0);

        // Add liquidity (large pools so 5% limit allows 100+ unit swaps)
        s.add_liquidity(ADMIN, "USDC".into(), 10_000_000_000); // 10,000 USDC
        s.add_liquidity(ADMIN, "EURC".into(), 9_300_000_000);  // 9,300 EURC
        s.add_liquidity(ADMIN, "GBPC".into(), 7_900_000_000);  // 7,900 GBPC

        // Add USDC reserves backing
        s.add_usdc_reserves(ADMIN, 30_000_000_000);

        s
    }

    // -- Basic swap USDC -> EURC ---------------------------------------------

    #[test]
    fn test_swap_usdc_to_eurc() {
        let mut s = setup_swap();
        // Swap 100 USDC to EURC
        // output = 100_000_000 * 930_000 / 1_000_000 = 93_000_000
        let output = s.swap(USER, "USDC".into(), "EURC".into(), 100_000_000, 0, T0);
        assert_eq!(output, 93_000_000); // 93 EURC

        // USDC pool increased, EURC pool decreased
        assert_eq!(*s.pools.get("USDC").unwrap(), 10_100_000_000);
        assert_eq!(*s.pools.get("EURC").unwrap(), 9_207_000_000);
    }

    // -- Reverse swap EURC -> USDC -------------------------------------------

    #[test]
    fn test_swap_eurc_to_usdc() {
        let mut s = setup_swap();
        // Swap 93 EURC to USDC
        // output = 93_000_000 * 1_000_000 / 930_000 = 100_000_000
        let output = s.swap(USER, "EURC".into(), "USDC".into(), 93_000_000, 0, T0);
        assert_eq!(output, 100_000_000); // 100 USDC

        assert_eq!(*s.pools.get("EURC").unwrap(), 9_393_000_000);
        assert_eq!(*s.pools.get("USDC").unwrap(), 9_900_000_000);
    }

    // -- Cross-currency swap EURC -> GBPC (via oracle math) ------------------

    #[test]
    fn test_swap_eurc_to_gbpc() {
        let mut s = setup_swap();
        // Swap 93 EURC to GBPC
        // output = 93_000_000 * 790_000 / 930_000 = 79_000_000
        let output = s.swap(USER, "EURC".into(), "GBPC".into(), 93_000_000, 0, T0);
        assert_eq!(output, 79_000_000); // 79 GBPC

        assert_eq!(*s.pools.get("EURC").unwrap(), 9_393_000_000);
        assert_eq!(*s.pools.get("GBPC").unwrap(), 7_821_000_000);
    }

    // -- Stale oracle rejection ----------------------------------------------

    #[test]
    #[should_panic(expected = "from currency oracle is stale")]
    fn test_swap_stale_oracle() {
        let mut s = setup_swap();
        // Advance time by 2 hours (oracle max age is 1 hour)
        let stale_time = T0 + 7200;
        s.swap(USER, "USDC".into(), "EURC".into(), 1_000_000, 0, stale_time);
    }

    // -- Max swap size rejection ---------------------------------------------

    #[test]
    #[should_panic(expected = "swap exceeds max size limit")]
    fn test_swap_exceeds_max_size() {
        let mut s = setup_swap();
        // max_swap_bps = 500 (5%)
        // USDC pool = 10,000,000,000 => max = 500,000,000
        // Try to swap 600,000,000 (6%) — should fail
        s.swap(USER, "USDC".into(), "EURC".into(), 600_000_000, 0, T0);
    }

    #[test]
    fn test_swap_at_max_size() {
        let mut s = setup_swap();
        // Swap exactly 5% of USDC pool = 500,000,000
        let output = s.swap(USER, "USDC".into(), "EURC".into(), 500_000_000, 0, T0);
        // 500_000_000 * 930_000 / 1_000_000 = 465_000_000
        assert_eq!(output, 465_000_000);
    }

    // -- Min output ----------------------------------------------------------

    #[test]
    #[should_panic(expected = "output below minimum")]
    fn test_swap_min_output_not_met() {
        let mut s = setup_swap();
        // Swap 10 USDC expecting at least 100 EURC (impossible)
        s.swap(
            USER,
            "USDC".into(),
            "EURC".into(),
            10_000_000,
            100_000_000,
            T0,
        );
    }

    // -- Insufficient liquidity ----------------------------------------------

    #[test]
    #[should_panic(expected = "insufficient liquidity")]
    fn test_swap_insufficient_liquidity() {
        let mut s = FxSwapState::new(ADMIN, 0, 3600); // 0 = no swap limit
        s.update_oracle_rate(ADMIN, "USDC".into(), 1_000_000, T0);
        s.update_oracle_rate(ADMIN, "EURC".into(), 930_000, T0);
        s.add_liquidity(ADMIN, "USDC".into(), 1_000_000_000);
        s.add_liquidity(ADMIN, "EURC".into(), 10_000); // tiny pool
        s.swap(USER, "USDC".into(), "EURC".into(), 100_000_000, 0, T0);
    }

    // -- Get quote -----------------------------------------------------------

    #[test]
    fn test_get_quote() {
        let s = setup_swap();
        let quote = s.get_quote("USDC", "EURC", 100_000_000);
        assert_eq!(quote, 93_000_000);

        let quote2 = s.get_quote("EURC", "USDC", 93_000_000);
        assert_eq!(quote2, 100_000_000);

        let quote3 = s.get_quote("EURC", "GBPC", 93_000_000);
        assert_eq!(quote3, 79_000_000);
    }

    // -- Proof of reserves ---------------------------------------------------

    #[test]
    fn test_proof_of_reserves() {
        let s = setup_swap();
        let (reserves, value, backed) = s.proof_of_reserves();
        assert_eq!(reserves, 30_000_000_000);
        // USDC: 10B * 1M / 1M = 10B
        // EURC: 9.3B * 1M / 930K = 10B
        // GBPC: 7.9B * 1M / 790K = 10B
        // Total: 30B
        assert_eq!(value, 30_000_000_000);
        assert!(backed);
    }

    #[test]
    fn test_proof_of_reserves_underbacked() {
        let mut s = setup_swap();
        s.usdc_reserves = 1_000_000_000; // Only 1B out of 30B needed
        let (reserves, value, backed) = s.proof_of_reserves();
        assert_eq!(reserves, 1_000_000_000);
        assert_eq!(value, 30_000_000_000);
        assert!(!backed);
    }

    // -- Oracle updaters -----------------------------------------------------

    #[test]
    fn test_add_remove_oracle_updater() {
        let mut s = FxSwapState::new(ADMIN, 100, 3600);
        s.add_oracle_updater(ADMIN, ORACLE);
        assert!(s.is_oracle_updater(&ORACLE));
        s.remove_oracle_updater(ADMIN, ORACLE);
        assert!(!s.is_oracle_updater(&ORACLE));
    }

    #[test]
    #[should_panic(expected = "only oracle updaters")]
    fn test_oracle_update_unauthorized() {
        let mut s = FxSwapState::new(ADMIN, 100, 3600);
        s.update_oracle_rate(USER, "USDC".into(), 1_000_000, T0);
    }

    // -- Liquidity management ------------------------------------------------

    #[test]
    fn test_add_remove_liquidity() {
        let mut s = FxSwapState::new(ADMIN, 100, 3600);
        s.add_liquidity(ADMIN, "USDC".into(), 500);
        assert_eq!(*s.pools.get("USDC").unwrap(), 500);
        s.remove_liquidity(ADMIN, "USDC".into(), 200);
        assert_eq!(*s.pools.get("USDC").unwrap(), 300);
    }

    #[test]
    #[should_panic(expected = "insufficient pool liquidity")]
    fn test_remove_liquidity_insufficient() {
        let mut s = FxSwapState::new(ADMIN, 100, 3600);
        s.add_liquidity(ADMIN, "USDC".into(), 100);
        s.remove_liquidity(ADMIN, "USDC".into(), 200);
    }

    #[test]
    #[should_panic(expected = "only admin")]
    fn test_add_liquidity_non_admin() {
        let mut s = FxSwapState::new(ADMIN, 100, 3600);
        s.add_liquidity(USER, "USDC".into(), 100);
    }

    // -- Pause ---------------------------------------------------------------

    #[test]
    #[should_panic(expected = "contract paused")]
    fn test_swap_while_paused() {
        let mut s = setup_swap();
        s.pause(ADMIN);
        s.swap(USER, "USDC".into(), "EURC".into(), 1_000_000, 0, T0);
    }

    #[test]
    fn test_pause_unpause() {
        let mut s = setup_swap();
        s.pause(ADMIN);
        assert!(s.paused);
        s.unpause(ADMIN);
        assert!(!s.paused);
    }

    // -- Same currency -------------------------------------------------------

    #[test]
    #[should_panic(expected = "cannot swap same currency")]
    fn test_swap_same_currency() {
        let mut s = setup_swap();
        s.swap(USER, "USDC".into(), "USDC".into(), 1_000_000, 0, T0);
    }

    // -- Volume tracking -----------------------------------------------------

    #[test]
    fn test_volume_tracking() {
        let mut s = setup_swap();
        s.swap(USER, "USDC".into(), "EURC".into(), 50_000_000, 0, T0);
        // Volume = 50_000_000 * 1_000_000 / 1_000_000 = 50_000_000 USDC
        assert_eq!(s.total_volume_usdc, 50_000_000);
    }

    // -- Dispatch tests ------------------------------------------------------

    #[test]
    fn test_dispatch_init_and_swap() {
        let mut state: Option<FxSwapState> = None;
        let init_args = serde_json::to_vec(&serde_json::json!({
            "max_swap_bps": 0,
            "max_oracle_age": 3600
        }))
        .unwrap();
        dispatch(&mut state, "init", &init_args, ADMIN);
        assert!(state.is_some());

        // Set oracle rates
        let rate_args = serde_json::to_vec(&serde_json::json!({
            "symbol": "USDC",
            "rate": 1_000_000u64,
            "timestamp": T0,
        }))
        .unwrap();
        dispatch(&mut state, "update_oracle_rate", &rate_args, ADMIN);

        let rate_args2 = serde_json::to_vec(&serde_json::json!({
            "symbol": "EURC",
            "rate": 930_000u64,
            "timestamp": T0,
        }))
        .unwrap();
        dispatch(&mut state, "update_oracle_rate", &rate_args2, ADMIN);

        // Add liquidity
        let liq_args = serde_json::to_vec(&serde_json::json!({
            "symbol": "USDC",
            "amount": 1_000_000_000u64,
        }))
        .unwrap();
        dispatch(&mut state, "add_liquidity", &liq_args, ADMIN);

        let liq_args2 = serde_json::to_vec(&serde_json::json!({
            "symbol": "EURC",
            "amount": 930_000_000u64,
        }))
        .unwrap();
        dispatch(&mut state, "add_liquidity", &liq_args2, ADMIN);

        // Swap
        let swap_args = serde_json::to_vec(&serde_json::json!({
            "from_symbol": "USDC",
            "to_symbol": "EURC",
            "amount": 100_000_000u64,
            "min_output": 90_000_000u64,
            "timestamp": T0,
        }))
        .unwrap();
        let result = dispatch(&mut state, "swap", &swap_args, USER);
        let output: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(output, 93_000_000);
    }

    #[test]
    fn test_dispatch_get_quote() {
        let mut state: Option<FxSwapState> = None;
        let init_args = serde_json::to_vec(&serde_json::json!({
            "max_swap_bps": 0,
            "max_oracle_age": 3600
        }))
        .unwrap();
        dispatch(&mut state, "init", &init_args, ADMIN);

        let rate_args = serde_json::to_vec(&serde_json::json!({
            "symbol": "USDC", "rate": 1_000_000u64, "timestamp": T0,
        }))
        .unwrap();
        dispatch(&mut state, "update_oracle_rate", &rate_args, ADMIN);

        let rate_args2 = serde_json::to_vec(&serde_json::json!({
            "symbol": "EURC", "rate": 930_000u64, "timestamp": T0,
        }))
        .unwrap();
        dispatch(&mut state, "update_oracle_rate", &rate_args2, ADMIN);

        let quote_args = serde_json::to_vec(&serde_json::json!({
            "from_symbol": "USDC",
            "to_symbol": "EURC",
            "amount": 100_000_000u64,
        }))
        .unwrap();
        let result = dispatch(&mut state, "get_quote", &quote_args, USER);
        let quote: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(quote, 93_000_000);
    }

    #[test]
    #[should_panic(expected = "already initialised")]
    fn test_dispatch_double_init() {
        let mut state: Option<FxSwapState> = None;
        let args = serde_json::to_vec(&serde_json::json!({
            "max_swap_bps": 0, "max_oracle_age": 3600
        }))
        .unwrap();
        dispatch(&mut state, "init", &args, ADMIN);
        dispatch(&mut state, "init", &args, ADMIN);
    }
}
