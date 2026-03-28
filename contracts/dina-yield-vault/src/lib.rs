use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Dina Yield Vault — Core USDC backing for all currency stablecoins
// ---------------------------------------------------------------------------

type Address = [u8; 32];

const SECONDS_PER_YEAR: u64 = 31_536_000;
const BPS_DENOMINATOR: u64 = 10_000;
const MAX_RATE_HISTORY: usize = 720; // 30 days x 24 hours
#[allow(dead_code)]
const OVERCOLLATERAL_BPS: u64 = 1500; // 15% buffer
/// Default yield rate: 4.5% APY
pub const DEFAULT_YIELD_RATE_BPS: u64 = 450;
const SNAPSHOT_INTERVAL: u64 = 3600; // 1 hour in seconds

/// A single user's deposit into the vault
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultDeposit {
    /// USDC amount locked as backing (micro-USDC)
    pub usdc_locked: u64,
    /// Currency code (e.g., "CAD", "EUR", "GBP")
    pub currency: String,
    /// Amount of stablecoin minted to user (micro-units)
    pub stablecoin_minted: u64,
    /// Oracle rate at deposit time (micro-units of currency per 1 USDC)
    pub deposit_rate: u64,
    /// Timestamp of deposit
    pub deposit_time: u64,
    /// Timestamp of last yield claim
    pub last_yield_claim: u64,
    /// Total USDC yield claimed so far
    pub total_usdc_yield_claimed: u64,
    /// Total stablecoin yield claimed so far (in the user's currency)
    pub total_currency_yield_claimed: u64,
}

/// Hourly rate snapshot for a currency
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateSnapshot {
    pub rate: u64,      // micro-units per USDC
    pub timestamp: u64, // unix timestamp
}

/// Per-currency tracking
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurrencyInfo {
    /// Current oracle rate (micro-units of this currency per 1 USDC)
    pub current_rate: u64,
    /// Last oracle update timestamp
    pub last_rate_update: u64,
    /// Hourly rate history (newest first, max 720 entries = 30 days)
    pub rate_history: Vec<RateSnapshot>,
    /// Total USDC backing this currency across all users
    pub total_usdc_backing: u64,
    /// Total stablecoin supply for this currency
    pub total_stablecoin_supply: u64,
    /// Total yield distributed in USDC for this currency
    pub total_yield_distributed_usdc: u64,
}

/// Rate statistics over multiple time windows
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateStats {
    pub current: u64,
    pub high_24h: u64,
    pub low_24h: u64,
    pub avg_24h: u64,
    pub high_7d: u64,
    pub low_7d: u64,
    pub avg_7d: u64,
    pub high_30d: u64,
    pub low_30d: u64,
    pub avg_30d: u64,
    pub change_24h_bps: i64,
    pub change_7d_bps: i64,
    pub change_30d_bps: i64,
}

/// Proof of reserves for a single currency
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurrencyReserveProof {
    pub currency: String,
    pub stablecoin_supply: u64,
    pub usdc_backing: u64,
    pub current_rate: u64,
    /// supply / rate * 1_000_000
    pub supply_value_in_usdc: u64,
    pub is_backed: bool,
    /// 10000 = exactly 1:1
    pub collateral_ratio_bps: u64,
}

/// Full proof of reserves across all currencies
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofOfReserves {
    pub total_usdc_in_vault: u64,
    pub per_currency: Vec<CurrencyReserveProof>,
    pub is_fully_backed: bool,
    /// How much over-backed in bps (e.g., 1500 = 15% over)
    pub overcollateral_bps: u64,
    pub timestamp: u64,
}

/// The main vault state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct YieldVault {
    pub admin: Address,
    /// Yield rate in basis points (450 = 4.5%)
    pub yield_rate_bps: u64,
    /// Per-user deposits: address -> currency -> deposit
    pub deposits: BTreeMap<Address, BTreeMap<String, VaultDeposit>>,
    /// Per-currency info and rate history
    pub currencies: BTreeMap<String, CurrencyInfo>,
    /// Oracle updater addresses
    pub oracle_updaters: Vec<Address>,
    /// Total USDC locked in the entire vault
    pub total_usdc_locked: u64,
    /// Total USDC yield distributed all-time
    pub total_yield_distributed: u64,
    /// Is the vault paused
    pub paused: bool,
}

impl YieldVault {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    pub fn new(admin: Address, yield_rate_bps: u64) -> Self {
        Self {
            admin,
            yield_rate_bps,
            deposits: BTreeMap::new(),
            currencies: BTreeMap::new(),
            oracle_updaters: Vec::new(),
            total_usdc_locked: 0,
            total_yield_distributed: 0,
            paused: false,
        }
    }

    // -----------------------------------------------------------------------
    // Core Methods
    // -----------------------------------------------------------------------

    /// Deposit USDC as backing and calculate stablecoin amount to mint.
    /// Returns the stablecoin amount minted (caller/runtime mints actual tokens).
    pub fn deposit(
        &mut self,
        caller: Address,
        usdc_amount: u64,
        currency: String,
        oracle_rate: u64,
        timestamp: u64,
    ) -> u64 {
        assert!(!self.paused, "vault paused");
        assert!(usdc_amount > 0, "deposit amount must be positive");
        assert!(oracle_rate > 0, "oracle rate must be positive");

        // Calculate stablecoin amount: usdc_amount * oracle_rate / 1_000_000
        let stablecoin_amount =
            (usdc_amount as u128 * oracle_rate as u128 / 1_000_000) as u64;
        assert!(stablecoin_amount > 0, "stablecoin amount too small");

        // Upsert deposit
        let user_deposits = self.deposits.entry(caller).or_default();
        let deposit = user_deposits.entry(currency.clone()).or_insert(VaultDeposit {
            usdc_locked: 0,
            currency: currency.clone(),
            stablecoin_minted: 0,
            deposit_rate: oracle_rate,
            deposit_time: timestamp,
            last_yield_claim: timestamp,
            total_usdc_yield_claimed: 0,
            total_currency_yield_claimed: 0,
        });

        deposit.usdc_locked = deposit.usdc_locked.checked_add(usdc_amount).expect("overflow");
        deposit.stablecoin_minted = deposit
            .stablecoin_minted
            .checked_add(stablecoin_amount)
            .expect("overflow");

        // Update currency totals
        let info = self.currencies.entry(currency).or_insert(CurrencyInfo {
            current_rate: oracle_rate,
            last_rate_update: timestamp,
            rate_history: Vec::new(),
            total_usdc_backing: 0,
            total_stablecoin_supply: 0,
            total_yield_distributed_usdc: 0,
        });
        info.total_usdc_backing = info
            .total_usdc_backing
            .checked_add(usdc_amount)
            .expect("overflow");
        info.total_stablecoin_supply = info
            .total_stablecoin_supply
            .checked_add(stablecoin_amount)
            .expect("overflow");

        // Update vault total
        self.total_usdc_locked = self
            .total_usdc_locked
            .checked_add(usdc_amount)
            .expect("overflow");

        stablecoin_amount
    }

    /// Calculate pending yield without state changes.
    /// Returns (usdc_yield, currency_yield, current_rate).
    pub fn calculate_pending_yield(
        &self,
        address: &Address,
        currency: &str,
        current_time: u64,
    ) -> (u64, u64, u64) {
        let deposit = self
            .deposits
            .get(address)
            .and_then(|m| m.get(currency));

        let deposit = match deposit {
            Some(d) => d,
            None => return (0, 0, 0),
        };

        if deposit.usdc_locked == 0 {
            return (0, 0, 0);
        }

        let current_rate = self
            .currencies
            .get(currency)
            .map(|c| c.current_rate)
            .unwrap_or(0);

        if current_time <= deposit.last_yield_claim {
            return (0, 0, current_rate);
        }

        let elapsed = current_time - deposit.last_yield_claim;

        // usdc_yield = usdc_locked * yield_rate_bps * elapsed / (BPS * SECONDS_PER_YEAR)
        let usdc_yield = (deposit.usdc_locked as u128)
            .checked_mul(self.yield_rate_bps as u128)
            .unwrap_or(0)
            .checked_mul(elapsed as u128)
            .unwrap_or(0)
            / (BPS_DENOMINATOR as u128 * SECONDS_PER_YEAR as u128);
        let usdc_yield = usdc_yield as u64;

        // currency_yield = usdc_yield * current_rate / 1_000_000
        let currency_yield = if current_rate > 0 {
            (usdc_yield as u128 * current_rate as u128 / 1_000_000) as u64
        } else {
            0
        };

        (usdc_yield, currency_yield, current_rate)
    }

    /// Claim pending yield. Returns (usdc_yield, currency_yield, rate_used).
    pub fn claim_yield(
        &mut self,
        caller: Address,
        currency: &str,
        current_time: u64,
    ) -> (u64, u64, u64) {
        assert!(!self.paused, "vault paused");

        let (usdc_yield, currency_yield, rate) =
            self.calculate_pending_yield(&caller, currency, current_time);

        if usdc_yield == 0 {
            return (0, 0, rate);
        }

        // Update deposit
        let deposit = self
            .deposits
            .get_mut(&caller)
            .and_then(|m| m.get_mut(currency))
            .expect("no deposit found");

        deposit.last_yield_claim = current_time;
        deposit.total_usdc_yield_claimed = deposit
            .total_usdc_yield_claimed
            .saturating_add(usdc_yield);
        deposit.total_currency_yield_claimed = deposit
            .total_currency_yield_claimed
            .saturating_add(currency_yield);
        // Yield gets added to backing
        deposit.usdc_locked = deposit.usdc_locked.saturating_add(usdc_yield);

        // Update vault totals
        self.total_usdc_locked = self.total_usdc_locked.saturating_add(usdc_yield);
        self.total_yield_distributed = self.total_yield_distributed.saturating_add(usdc_yield);

        // Update currency totals
        if let Some(info) = self.currencies.get_mut(currency) {
            info.total_usdc_backing = info.total_usdc_backing.saturating_add(usdc_yield);
            info.total_yield_distributed_usdc = info
                .total_yield_distributed_usdc
                .saturating_add(usdc_yield);
        }

        (usdc_yield, currency_yield, rate)
    }

    /// Withdraw stablecoin, releasing USDC at current rate.
    /// Claims pending yield first. Returns USDC released.
    pub fn withdraw(
        &mut self,
        caller: Address,
        currency: &str,
        stablecoin_amount: u64,
        current_time: u64,
    ) -> u64 {
        assert!(!self.paused, "vault paused");
        assert!(stablecoin_amount > 0, "withdraw amount must be positive");

        // Claim pending yield first
        self.claim_yield(caller, currency, current_time);

        let current_rate = self
            .currencies
            .get(currency)
            .map(|c| c.current_rate)
            .expect("unknown currency");
        assert!(current_rate > 0, "rate must be positive");

        // Calculate USDC to release: stablecoin_amount * 1_000_000 / rate
        let usdc_released =
            (stablecoin_amount as u128 * 1_000_000 / current_rate as u128) as u64;
        assert!(usdc_released > 0, "usdc release too small");

        let deposit = self
            .deposits
            .get_mut(&caller)
            .and_then(|m| m.get_mut(currency))
            .expect("no deposit found");

        assert!(
            deposit.stablecoin_minted >= stablecoin_amount,
            "insufficient stablecoin balance"
        );
        assert!(
            deposit.usdc_locked >= usdc_released,
            "insufficient USDC backing"
        );

        deposit.stablecoin_minted -= stablecoin_amount;
        deposit.usdc_locked -= usdc_released;

        // Update currency totals
        if let Some(info) = self.currencies.get_mut(currency) {
            info.total_usdc_backing = info.total_usdc_backing.saturating_sub(usdc_released);
            info.total_stablecoin_supply = info
                .total_stablecoin_supply
                .saturating_sub(stablecoin_amount);
        }

        // Update vault total
        self.total_usdc_locked = self.total_usdc_locked.saturating_sub(usdc_released);

        usdc_released
    }

    // -----------------------------------------------------------------------
    // Oracle Methods
    // -----------------------------------------------------------------------

    /// Update oracle rate for a currency. Only oracle_updaters can call.
    pub fn update_rate(
        &mut self,
        caller: Address,
        currency: &str,
        new_rate: u64,
        timestamp: u64,
    ) {
        assert!(
            self.oracle_updaters.contains(&caller) || caller == self.admin,
            "not authorized as oracle updater"
        );
        assert!(new_rate > 0, "rate must be positive");

        let info = self
            .currencies
            .entry(currency.to_string())
            .or_insert(CurrencyInfo {
                current_rate: new_rate,
                last_rate_update: timestamp,
                rate_history: Vec::new(),
                total_usdc_backing: 0,
                total_stablecoin_supply: 0,
                total_yield_distributed_usdc: 0,
            });

        info.current_rate = new_rate;

        // Push snapshot if enough time has passed (1 hour)
        let should_snapshot = info.rate_history.is_empty()
            || timestamp >= info.last_rate_update + SNAPSHOT_INTERVAL;

        if should_snapshot {
            info.rate_history.insert(
                0,
                RateSnapshot {
                    rate: new_rate,
                    timestamp,
                },
            );
            // Trim to MAX_RATE_HISTORY
            if info.rate_history.len() > MAX_RATE_HISTORY {
                info.rate_history.truncate(MAX_RATE_HISTORY);
            }
        }

        info.last_rate_update = timestamp;
    }

    /// Add an oracle updater address. Admin only.
    pub fn add_oracle_updater(&mut self, caller: Address, address: Address) {
        assert!(caller == self.admin, "only admin");
        if !self.oracle_updaters.contains(&address) {
            self.oracle_updaters.push(address);
        }
    }

    /// Remove an oracle updater address. Admin only.
    pub fn remove_oracle_updater(&mut self, caller: Address, address: Address) {
        assert!(caller == self.admin, "only admin");
        self.oracle_updaters.retain(|a| *a != address);
    }

    // -----------------------------------------------------------------------
    // Query Methods (Read-Only)
    // -----------------------------------------------------------------------

    /// Get a user's deposit for a specific currency.
    pub fn get_deposit(&self, address: &Address, currency: &str) -> Option<&VaultDeposit> {
        self.deposits.get(address).and_then(|m| m.get(currency))
    }

    /// Get pending yield for a user + currency.
    pub fn get_pending_yield(
        &self,
        address: &Address,
        currency: &str,
        current_time: u64,
    ) -> (u64, u64, u64) {
        self.calculate_pending_yield(address, currency, current_time)
    }

    /// Get currency info.
    pub fn get_currency_info(&self, currency: &str) -> Option<&CurrencyInfo> {
        self.currencies.get(currency)
    }

    /// Get rate history for the last N hours.
    pub fn get_rate_history(&self, currency: &str, hours: usize) -> Vec<RateSnapshot> {
        let info = match self.currencies.get(currency) {
            Some(i) => i,
            None => return Vec::new(),
        };
        // Each entry is ~1 hour, newest first
        let count = hours.min(info.rate_history.len());
        info.rate_history[..count].to_vec()
    }

    /// Compute rate statistics from rate_history.
    pub fn get_rate_stats(&self, currency: &str) -> Option<RateStats> {
        let info = self.currencies.get(currency)?;
        let current = info.current_rate;
        let history = &info.rate_history;

        if history.is_empty() {
            return Some(RateStats {
                current,
                high_24h: current,
                low_24h: current,
                avg_24h: current,
                high_7d: current,
                low_7d: current,
                avg_7d: current,
                high_30d: current,
                low_30d: current,
                avg_30d: current,
                change_24h_bps: 0,
                change_7d_bps: 0,
                change_30d_bps: 0,
            });
        }

        // Windows in hours: 24, 168 (7d), 720 (30d)
        let (h24, l24, a24, oldest_24) = Self::window_stats(history, 24);
        let (h7d, l7d, a7d, oldest_7d) = Self::window_stats(history, 168);
        let (h30d, l30d, a30d, oldest_30d) = Self::window_stats(history, 720);

        let change_24h_bps = Self::change_bps(current, oldest_24);
        let change_7d_bps = Self::change_bps(current, oldest_7d);
        let change_30d_bps = Self::change_bps(current, oldest_30d);

        Some(RateStats {
            current,
            high_24h: h24,
            low_24h: l24,
            avg_24h: a24,
            high_7d: h7d,
            low_7d: l7d,
            avg_7d: a7d,
            high_30d: h30d,
            low_30d: l30d,
            avg_30d: a30d,
            change_24h_bps,
            change_7d_bps,
            change_30d_bps,
        })
    }

    /// Compute high/low/avg/oldest for the first `count` entries in history.
    fn window_stats(history: &[RateSnapshot], count: usize) -> (u64, u64, u64, u64) {
        let slice = &history[..count.min(history.len())];
        if slice.is_empty() {
            return (0, 0, 0, 0);
        }
        let mut high = 0u64;
        let mut low = u64::MAX;
        let mut sum = 0u128;
        for snap in slice {
            high = high.max(snap.rate);
            low = low.min(snap.rate);
            sum += snap.rate as u128;
        }
        let avg = (sum / slice.len() as u128) as u64;
        let oldest = slice.last().map(|s| s.rate).unwrap_or(0);
        (high, low, avg, oldest)
    }

    /// Calculate basis-point change: positive = currency weakened vs USD (rate went up).
    fn change_bps(current: u64, old: u64) -> i64 {
        if old == 0 {
            return 0;
        }
        // change_bps = (current - old) / old * 10000
        let diff = current as i128 - old as i128;
        (diff * BPS_DENOMINATOR as i128 / old as i128) as i64
    }

    /// Full proof of reserves.
    pub fn proof_of_reserves(&self, timestamp: u64) -> ProofOfReserves {
        let mut per_currency = Vec::new();
        let mut total_supply_value_usdc: u128 = 0;

        for (currency, info) in &self.currencies {
            let supply_value_in_usdc = if info.current_rate > 0 {
                (info.total_stablecoin_supply as u128 * 1_000_000
                    / info.current_rate as u128) as u64
            } else {
                0
            };

            let is_backed = info.total_usdc_backing >= supply_value_in_usdc;
            let collateral_ratio_bps = if supply_value_in_usdc > 0 {
                (info.total_usdc_backing as u128 * BPS_DENOMINATOR as u128
                    / supply_value_in_usdc as u128) as u64
            } else if info.total_usdc_backing > 0 {
                // No supply but has backing — infinite collateral, cap at max
                u64::MAX
            } else {
                BPS_DENOMINATOR // 1:1 when both zero
            };

            total_supply_value_usdc += supply_value_in_usdc as u128;

            per_currency.push(CurrencyReserveProof {
                currency: currency.clone(),
                stablecoin_supply: info.total_stablecoin_supply,
                usdc_backing: info.total_usdc_backing,
                current_rate: info.current_rate,
                supply_value_in_usdc,
                is_backed,
                collateral_ratio_bps,
            });
        }

        let is_fully_backed = self.total_usdc_locked as u128 >= total_supply_value_usdc;
        let overcollateral_bps = if total_supply_value_usdc > 0 {
            let ratio = self.total_usdc_locked as u128 * BPS_DENOMINATOR as u128
                / total_supply_value_usdc;
            // overcollateral = ratio - 10000 (the excess above 1:1)
            if ratio > BPS_DENOMINATOR as u128 {
                (ratio - BPS_DENOMINATOR as u128) as u64
            } else {
                0
            }
        } else {
            0
        };

        ProofOfReserves {
            total_usdc_in_vault: self.total_usdc_locked,
            per_currency,
            is_fully_backed,
            overcollateral_bps,
            timestamp,
        }
    }

    /// List all deposits for a user across currencies.
    pub fn list_all_deposits(&self, address: &Address) -> Vec<VaultDeposit> {
        match self.deposits.get(address) {
            Some(map) => map.values().cloned().collect(),
            None => Vec::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Admin
    // -----------------------------------------------------------------------

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
    yield_rate_bps: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs {
    usdc_amount: u64,
    currency: String,
    oracle_rate: u64,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ClaimYieldArgs {
    currency: String,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct WithdrawArgs {
    currency: String,
    stablecoin_amount: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateRateArgs {
    currency: String,
    new_rate: u64,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct OracleUpdaterArgs {
    address: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetDepositArgs {
    address: Address,
    currency: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetPendingYieldArgs {
    address: Address,
    currency: String,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct CurrencyArgs {
    currency: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetRateHistoryArgs {
    currency: String,
    hours: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProofOfReservesArgs {
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ListAllDepositsArgs {
    address: Address,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Contract-level dispatch. `state` is None on first call (init).
pub fn dispatch(
    state: &mut Option<YieldVault>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        // -- Init ------------------------------------------------------------
        "init" => {
            assert!(state.is_none(), "already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("bad init args");
            *state = Some(YieldVault::new(caller, a.yield_rate_bps));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "deposit" => {
            let s = state.as_mut().expect("not initialised");
            let a: DepositArgs = serde_json::from_slice(args).expect("bad deposit args");
            let minted = s.deposit(caller, a.usdc_amount, a.currency, a.oracle_rate, a.timestamp);
            serde_json::to_vec(&minted).unwrap()
        }
        "claim_yield" => {
            let s = state.as_mut().expect("not initialised");
            let a: ClaimYieldArgs = serde_json::from_slice(args).expect("bad claim_yield args");
            let (usdc, currency, rate) = s.claim_yield(caller, &a.currency, a.current_time);
            serde_json::to_vec(&(usdc, currency, rate)).unwrap()
        }
        "withdraw" => {
            let s = state.as_mut().expect("not initialised");
            let a: WithdrawArgs = serde_json::from_slice(args).expect("bad withdraw args");
            let released = s.withdraw(caller, &a.currency, a.stablecoin_amount, a.current_time);
            serde_json::to_vec(&released).unwrap()
        }

        // -- Oracle ----------------------------------------------------------
        "update_rate" => {
            let s = state.as_mut().expect("not initialised");
            let a: UpdateRateArgs = serde_json::from_slice(args).expect("bad update_rate args");
            s.update_rate(caller, &a.currency, a.new_rate, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }
        "add_oracle_updater" => {
            let s = state.as_mut().expect("not initialised");
            let a: OracleUpdaterArgs =
                serde_json::from_slice(args).expect("bad add_oracle_updater args");
            s.add_oracle_updater(caller, a.address);
            serde_json::to_vec("ok").unwrap()
        }
        "remove_oracle_updater" => {
            let s = state.as_mut().expect("not initialised");
            let a: OracleUpdaterArgs =
                serde_json::from_slice(args).expect("bad remove_oracle_updater args");
            s.remove_oracle_updater(caller, a.address);
            serde_json::to_vec("ok").unwrap()
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

        // -- Queries ---------------------------------------------------------
        "get_deposit" => {
            let s = state.as_ref().expect("not initialised");
            let a: GetDepositArgs = serde_json::from_slice(args).expect("bad get_deposit args");
            let deposit = s.get_deposit(&a.address, &a.currency);
            serde_json::to_vec(&deposit).unwrap()
        }
        "get_pending_yield" => {
            let s = state.as_ref().expect("not initialised");
            let a: GetPendingYieldArgs =
                serde_json::from_slice(args).expect("bad get_pending_yield args");
            let (usdc, currency, rate) =
                s.get_pending_yield(&a.address, &a.currency, a.current_time);
            serde_json::to_vec(&(usdc, currency, rate)).unwrap()
        }
        "get_currency_info" => {
            let s = state.as_ref().expect("not initialised");
            let a: CurrencyArgs =
                serde_json::from_slice(args).expect("bad get_currency_info args");
            let info = s.get_currency_info(&a.currency);
            serde_json::to_vec(&info).unwrap()
        }
        "get_rate_history" => {
            let s = state.as_ref().expect("not initialised");
            let a: GetRateHistoryArgs =
                serde_json::from_slice(args).expect("bad get_rate_history args");
            let history = s.get_rate_history(&a.currency, a.hours);
            serde_json::to_vec(&history).unwrap()
        }
        "get_rate_stats" => {
            let s = state.as_ref().expect("not initialised");
            let a: CurrencyArgs =
                serde_json::from_slice(args).expect("bad get_rate_stats args");
            let stats = s.get_rate_stats(&a.currency);
            serde_json::to_vec(&stats).unwrap()
        }
        "proof_of_reserves" => {
            let s = state.as_ref().expect("not initialised");
            let a: ProofOfReservesArgs =
                serde_json::from_slice(args).expect("bad proof_of_reserves args");
            let proof = s.proof_of_reserves(a.timestamp);
            serde_json::to_vec(&proof).unwrap()
        }
        "list_all_deposits" => {
            let s = state.as_ref().expect("not initialised");
            let a: ListAllDepositsArgs =
                serde_json::from_slice(args).expect("bad list_all_deposits args");
            let deposits = s.list_all_deposits(&a.address);
            serde_json::to_vec(&deposits).unwrap()
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
    const ALICE: Address = [2u8; 32];
    const BOB: Address = [3u8; 32];
    const ORACLE: Address = [4u8; 32];
    const T0: u64 = 1_700_000_000;

    // CAD rate: 1 USDC = 1.36 CAD => 1_360_000 micro-units per USDC
    const CAD_RATE: u64 = 1_360_000;
    // EUR rate: 1 USDC = 0.93 EUR => 930_000 micro-units per USDC
    const EUR_RATE: u64 = 930_000;

    fn new_vault() -> YieldVault {
        let mut v = YieldVault::new(ADMIN, DEFAULT_YIELD_RATE_BPS);
        v.add_oracle_updater(ADMIN, ORACLE);
        v
    }

    // 1. test_deposit_and_mint
    #[test]
    fn test_deposit_and_mint() {
        let mut v = new_vault();
        // Deposit 100 USDC at CAD rate 1.36
        let minted = v.deposit(ALICE, 100_000_000, "CAD".into(), CAD_RATE, T0);
        // Expected: 100_000_000 * 1_360_000 / 1_000_000 = 136_000_000
        assert_eq!(minted, 136_000_000);

        let deposit = v.get_deposit(&ALICE, "CAD").unwrap();
        assert_eq!(deposit.usdc_locked, 100_000_000);
        assert_eq!(deposit.stablecoin_minted, 136_000_000);
        assert_eq!(deposit.deposit_rate, CAD_RATE);
        assert_eq!(deposit.deposit_time, T0);
        assert_eq!(deposit.last_yield_claim, T0);

        // Vault totals
        assert_eq!(v.total_usdc_locked, 100_000_000);
        let info = v.get_currency_info("CAD").unwrap();
        assert_eq!(info.total_usdc_backing, 100_000_000);
        assert_eq!(info.total_stablecoin_supply, 136_000_000);
    }

    // 2. test_yield_calculation
    #[test]
    fn test_yield_calculation() {
        let mut v = new_vault();
        v.deposit(ALICE, 100_000_000, "CAD".into(), CAD_RATE, T0);
        // Set rate so calculate_pending_yield can look it up
        v.update_rate(ORACLE, "CAD", CAD_RATE, T0);

        // After 1 year at 4.5% APY
        let t1 = T0 + SECONDS_PER_YEAR;
        let (usdc_yield, cad_yield, rate) = v.calculate_pending_yield(&ALICE, "CAD", t1);

        // usdc_yield = 100_000_000 * 450 / 10_000 = 4_500_000
        assert_eq!(usdc_yield, 4_500_000);
        // cad_yield = 4_500_000 * 1_360_000 / 1_000_000 = 6_120_000
        assert_eq!(cad_yield, 6_120_000);
        assert_eq!(rate, CAD_RATE);
    }

    // 3. test_claim_yield
    #[test]
    fn test_claim_yield() {
        let mut v = new_vault();
        v.deposit(ALICE, 100_000_000, "CAD".into(), CAD_RATE, T0);
        v.update_rate(ORACLE, "CAD", CAD_RATE, T0);

        let t1 = T0 + SECONDS_PER_YEAR;
        let (usdc_yield, cad_yield, rate) = v.claim_yield(ALICE, "CAD", t1);

        assert_eq!(usdc_yield, 4_500_000);
        assert_eq!(cad_yield, 6_120_000);
        assert_eq!(rate, CAD_RATE);

        // Yield added to backing
        let deposit = v.get_deposit(&ALICE, "CAD").unwrap();
        assert_eq!(deposit.usdc_locked, 100_000_000 + 4_500_000);
        assert_eq!(deposit.last_yield_claim, t1);
        assert_eq!(deposit.total_usdc_yield_claimed, 4_500_000);
        assert_eq!(deposit.total_currency_yield_claimed, 6_120_000);

        // Vault totals
        assert_eq!(v.total_usdc_locked, 100_000_000 + 4_500_000);
        assert_eq!(v.total_yield_distributed, 4_500_000);
    }

    // 4. test_claim_yield_rate_change
    #[test]
    fn test_claim_yield_rate_change() {
        let mut v = new_vault();
        v.deposit(ALICE, 100_000_000, "CAD".into(), CAD_RATE, T0);
        v.update_rate(ORACLE, "CAD", CAD_RATE, T0);

        // CAD weakened: 1 USDC = 1.40 CAD
        let new_rate: u64 = 1_400_000;
        let t1 = T0 + SECONDS_PER_YEAR;
        v.update_rate(ORACLE, "CAD", new_rate, t1);

        let (usdc_yield, cad_yield, rate) = v.claim_yield(ALICE, "CAD", t1);

        // USDC yield is the same regardless of rate
        assert_eq!(usdc_yield, 4_500_000);
        // But CAD yield uses the NEW rate: 4_500_000 * 1_400_000 / 1_000_000 = 6_300_000
        assert_eq!(cad_yield, 6_300_000);
        assert_eq!(rate, new_rate);
    }

    // 5. test_withdraw
    #[test]
    fn test_withdraw() {
        let mut v = new_vault();
        v.deposit(ALICE, 100_000_000, "CAD".into(), CAD_RATE, T0);
        v.update_rate(ORACLE, "CAD", CAD_RATE, T0);

        // Withdraw all 136 CAD stablecoin at the same rate
        let released = v.withdraw(ALICE, "CAD", 136_000_000, T0);
        // 136_000_000 * 1_000_000 / 1_360_000 = 100_000_000
        assert_eq!(released, 100_000_000);

        let deposit = v.get_deposit(&ALICE, "CAD").unwrap();
        assert_eq!(deposit.usdc_locked, 0);
        assert_eq!(deposit.stablecoin_minted, 0);
        assert_eq!(v.total_usdc_locked, 0);
    }

    // 6. test_withdraw_after_rate_change
    #[test]
    fn test_withdraw_after_rate_change() {
        let mut v = new_vault();
        // Deposit 110 USDC to give headroom for rate movement
        v.deposit(ALICE, 110_000_000, "CAD".into(), CAD_RATE, T0);
        v.update_rate(ORACLE, "CAD", CAD_RATE, T0);
        // Minted: 110_000_000 * 1_360_000 / 1_000_000 = 149_600_000 CAD

        // CAD strengthened: 1 USDC = 1.30 CAD (CAD is worth more)
        let stronger_rate: u64 = 1_300_000;
        v.update_rate(ORACLE, "CAD", stronger_rate, T0 + 1);

        // Withdraw 130 CAD stablecoin at the stronger rate
        // 130_000_000 * 1_000_000 / 1_300_000 = 100_000_000 USDC
        let released = v.withdraw(ALICE, "CAD", 130_000_000, T0 + 1);
        assert_eq!(released, 100_000_000);

        // At deposit rate 1.36, 130 CAD would have been worth ~95.6M USDC
        // But at stronger rate 1.30, 130 CAD is worth 100M USDC
        // User gets MORE USDC per CAD because CAD strengthened
        // Verify: at deposit rate, same CAD would release less
        let at_deposit_rate = (130_000_000u128 * 1_000_000 / CAD_RATE as u128) as u64;
        assert!(released > at_deposit_rate); // 100M > 95.6M

        // Also test CAD weakening: user gets LESS USDC per CAD
        let weaker_rate: u64 = 1_500_000;
        v.update_rate(ORACLE, "CAD", weaker_rate, T0 + 2);

        // Remaining: 19_600_000 CAD stablecoin, 10_000_000 USDC locked
        // Withdraw 15_000_000 CAD at weak rate: 15M * 1M / 1.5M = 10_000_000 USDC
        let released2 = v.withdraw(ALICE, "CAD", 15_000_000, T0 + 2);
        assert_eq!(released2, 10_000_000);
        // At the original deposit rate 1.36, 15M CAD = ~11M USDC
        // But at weaker rate 1.50, 15M CAD = only 10M USDC
        // User gets LESS USDC because CAD weakened
        let at_deposit_rate2 = (15_000_000u128 * 1_000_000 / CAD_RATE as u128) as u64;
        assert!(released2 < at_deposit_rate2);
    }

    // 7. test_multiple_currencies
    #[test]
    fn test_multiple_currencies() {
        let mut v = new_vault();
        v.update_rate(ORACLE, "CAD", CAD_RATE, T0);
        v.update_rate(ORACLE, "EUR", EUR_RATE, T0);

        let cad_minted = v.deposit(ALICE, 50_000_000, "CAD".into(), CAD_RATE, T0);
        let eur_minted = v.deposit(ALICE, 50_000_000, "EUR".into(), EUR_RATE, T0);

        // CAD: 50M * 1.36 = 68M
        assert_eq!(cad_minted, 68_000_000);
        // EUR: 50M * 0.93 = 46.5M
        assert_eq!(eur_minted, 46_500_000);

        assert_eq!(v.total_usdc_locked, 100_000_000);

        let deposits = v.list_all_deposits(&ALICE);
        assert_eq!(deposits.len(), 2);

        let cad_dep = v.get_deposit(&ALICE, "CAD").unwrap();
        assert_eq!(cad_dep.usdc_locked, 50_000_000);
        let eur_dep = v.get_deposit(&ALICE, "EUR").unwrap();
        assert_eq!(eur_dep.usdc_locked, 50_000_000);
    }

    // 8. test_rate_history
    #[test]
    fn test_rate_history() {
        let mut v = new_vault();

        // Update rates hourly for 5 hours
        for i in 0..5 {
            let rate = CAD_RATE + i * 1000;
            let ts = T0 + i * 3600;
            v.update_rate(ORACLE, "CAD", rate, ts);
        }

        let history = v.get_rate_history("CAD", 10);
        assert_eq!(history.len(), 5);
        // Newest first
        assert_eq!(history[0].rate, CAD_RATE + 4 * 1000);
        assert_eq!(history[4].rate, CAD_RATE);
    }

    // 9. test_rate_stats
    #[test]
    fn test_rate_stats() {
        let mut v = new_vault();

        // Populate 48 hours of rate history (every hour)
        let rates = [
            1_360_000u64, 1_362_000, 1_358_000, 1_365_000, 1_355_000, 1_370_000,
            1_350_000, 1_360_000, 1_360_000, 1_360_000, 1_360_000, 1_360_000,
            1_360_000, 1_360_000, 1_360_000, 1_360_000, 1_360_000, 1_360_000,
            1_360_000, 1_360_000, 1_360_000, 1_360_000, 1_360_000, 1_360_000,
            // Second day
            1_380_000, 1_340_000, 1_360_000, 1_360_000, 1_360_000, 1_360_000,
            1_360_000, 1_360_000, 1_360_000, 1_360_000, 1_360_000, 1_360_000,
            1_360_000, 1_360_000, 1_360_000, 1_360_000, 1_360_000, 1_360_000,
            1_360_000, 1_360_000, 1_360_000, 1_360_000, 1_360_000, 1_360_000,
        ];

        for (i, &rate) in rates.iter().enumerate() {
            let ts = T0 + (i as u64) * 3600;
            v.update_rate(ORACLE, "CAD", rate, ts);
        }

        let stats = v.get_rate_stats("CAD").unwrap();
        assert_eq!(stats.current, *rates.last().unwrap());

        // 24h window = first 24 entries (newest first), which covers the second day
        // The most recent 24 entries in history are rates[24..48]
        assert!(stats.high_24h >= 1_360_000);
        assert!(stats.low_24h <= 1_360_000);

        // 30d window covers all entries
        assert_eq!(stats.high_30d, 1_380_000);
        assert_eq!(stats.low_30d, 1_340_000);
    }

    // 10. test_proof_of_reserves
    #[test]
    fn test_proof_of_reserves() {
        let mut v = new_vault();
        v.update_rate(ORACLE, "CAD", CAD_RATE, T0);

        v.deposit(ALICE, 100_000_000, "CAD".into(), CAD_RATE, T0);

        let proof = v.proof_of_reserves(T0);
        assert_eq!(proof.total_usdc_in_vault, 100_000_000);
        assert!(proof.is_fully_backed);
        assert_eq!(proof.per_currency.len(), 1);

        let cad = &proof.per_currency[0];
        assert_eq!(cad.currency, "CAD");
        assert_eq!(cad.stablecoin_supply, 136_000_000);
        assert_eq!(cad.usdc_backing, 100_000_000);
        // supply_value_in_usdc = 136_000_000 * 1_000_000 / 1_360_000 = 100_000_000
        assert_eq!(cad.supply_value_in_usdc, 100_000_000);
        assert!(cad.is_backed);
        // Collateral ratio = 100M / 100M * 10000 = 10000 (exactly 1:1)
        assert_eq!(cad.collateral_ratio_bps, 10_000);
    }

    // 11. test_overcollateralization
    #[test]
    fn test_overcollateralization() {
        let mut v = new_vault();
        v.update_rate(ORACLE, "CAD", CAD_RATE, T0);

        // Deposit 100 USDC -> 136 CAD
        v.deposit(ALICE, 100_000_000, "CAD".into(), CAD_RATE, T0);

        // CAD weakens: 1 USDC = 1.50 CAD
        // Now 136 CAD is worth only 136/1.50 = 90.67 USDC
        // But vault still has 100 USDC backing
        let weaker_rate: u64 = 1_500_000;
        v.update_rate(ORACLE, "CAD", weaker_rate, T0 + 3600);

        let proof = v.proof_of_reserves(T0 + 3600);
        assert!(proof.is_fully_backed);
        // supply_value = 136_000_000 * 1_000_000 / 1_500_000 = 90_666_666
        let cad = &proof.per_currency[0];
        assert_eq!(cad.supply_value_in_usdc, 90_666_666);
        assert!(cad.collateral_ratio_bps > 10_000); // over-collateralized
        assert!(proof.overcollateral_bps > 0);
    }

    // 12. test_oracle_access_control
    #[test]
    fn test_oracle_access_control() {
        let mut v = new_vault();
        // ORACLE is already added in new_vault()
        v.update_rate(ORACLE, "CAD", CAD_RATE, T0); // should succeed

        // Admin can also update
        v.update_rate(ADMIN, "CAD", CAD_RATE + 1000, T0 + 3600);
    }

    #[test]
    #[should_panic(expected = "not authorized as oracle updater")]
    fn test_oracle_access_control_unauthorized() {
        let mut v = new_vault();
        // ALICE is not an oracle updater
        v.update_rate(ALICE, "CAD", CAD_RATE, T0);
    }

    #[test]
    fn test_oracle_add_remove() {
        let mut v = new_vault();
        v.add_oracle_updater(ADMIN, BOB);
        v.update_rate(BOB, "CAD", CAD_RATE, T0); // should succeed
        v.remove_oracle_updater(ADMIN, BOB);
    }

    #[test]
    #[should_panic(expected = "not authorized as oracle updater")]
    fn test_oracle_removed_cannot_update() {
        let mut v = new_vault();
        v.add_oracle_updater(ADMIN, BOB);
        v.remove_oracle_updater(ADMIN, BOB);
        v.update_rate(BOB, "CAD", CAD_RATE, T0); // should fail
    }

    #[test]
    #[should_panic(expected = "only admin")]
    fn test_oracle_add_non_admin() {
        let mut v = new_vault();
        v.add_oracle_updater(ALICE, BOB);
    }

    // 13. test_pause
    #[test]
    fn test_pause() {
        let mut v = new_vault();
        v.update_rate(ORACLE, "CAD", CAD_RATE, T0);
        v.deposit(ALICE, 100_000_000, "CAD".into(), CAD_RATE, T0);

        v.pause(ADMIN);
        assert!(v.paused);
    }

    #[test]
    #[should_panic(expected = "vault paused")]
    fn test_pause_blocks_deposit() {
        let mut v = new_vault();
        v.pause(ADMIN);
        v.deposit(ALICE, 100_000_000, "CAD".into(), CAD_RATE, T0);
    }

    #[test]
    #[should_panic(expected = "vault paused")]
    fn test_pause_blocks_claim() {
        let mut v = new_vault();
        v.update_rate(ORACLE, "CAD", CAD_RATE, T0);
        v.deposit(ALICE, 100_000_000, "CAD".into(), CAD_RATE, T0);
        v.pause(ADMIN);
        v.claim_yield(ALICE, "CAD", T0 + SECONDS_PER_YEAR);
    }

    #[test]
    #[should_panic(expected = "vault paused")]
    fn test_pause_blocks_withdraw() {
        let mut v = new_vault();
        v.update_rate(ORACLE, "CAD", CAD_RATE, T0);
        v.deposit(ALICE, 100_000_000, "CAD".into(), CAD_RATE, T0);
        v.pause(ADMIN);
        v.withdraw(ALICE, "CAD", 136_000_000, T0);
    }

    #[test]
    fn test_unpause_resumes() {
        let mut v = new_vault();
        v.update_rate(ORACLE, "CAD", CAD_RATE, T0);
        v.pause(ADMIN);
        v.unpause(ADMIN);
        // Should work again
        v.deposit(ALICE, 100_000_000, "CAD".into(), CAD_RATE, T0);
    }

    // 14. test_multiple_claims
    #[test]
    fn test_multiple_claims() {
        let mut v = new_vault();
        v.update_rate(ORACLE, "CAD", CAD_RATE, T0);
        v.deposit(ALICE, 100_000_000, "CAD".into(), CAD_RATE, T0);

        // First claim after 6 months
        let t1 = T0 + SECONDS_PER_YEAR / 2;
        let (usdc1, _, _) = v.claim_yield(ALICE, "CAD", t1);
        // 100_000_000 * 450 * (SECONDS_PER_YEAR/2) / (10000 * SECONDS_PER_YEAR) = 2_250_000
        assert_eq!(usdc1, 2_250_000);

        // Second claim after another 6 months (1 year total)
        let t2 = T0 + SECONDS_PER_YEAR;
        let (usdc2, _, _) = v.claim_yield(ALICE, "CAD", t2);
        // Now earning on 102_250_000 (original + first yield)
        // 102_250_000 * 450 * (SECONDS_PER_YEAR/2) / (10000 * SECONDS_PER_YEAR) = 2_300_625
        assert_eq!(usdc2, 2_300_625);

        // Total yield is compounded (more than simple 4_500_000)
        let deposit = v.get_deposit(&ALICE, "CAD").unwrap();
        assert_eq!(deposit.total_usdc_yield_claimed, usdc1 + usdc2);
        assert_eq!(
            deposit.usdc_locked,
            100_000_000 + usdc1 + usdc2
        );
    }

    // -- Dispatch tests -------------------------------------------------------

    #[test]
    fn test_dispatch_init() {
        let mut state: Option<YieldVault> = None;
        let args = serde_json::to_vec(&serde_json::json!({
            "yield_rate_bps": 450
        }))
        .unwrap();
        let result = dispatch(&mut state, "init", &args, ADMIN);
        let ok: String = serde_json::from_slice(&result).unwrap();
        assert_eq!(ok, "ok");
        assert!(state.is_some());
        assert_eq!(state.as_ref().unwrap().yield_rate_bps, 450);
    }

    #[test]
    fn test_dispatch_deposit_and_query() {
        let mut state: Option<YieldVault> = None;
        let init_args = serde_json::to_vec(&serde_json::json!({
            "yield_rate_bps": 450
        }))
        .unwrap();
        dispatch(&mut state, "init", &init_args, ADMIN);

        // Add oracle updater
        let oracle_args = serde_json::to_vec(&serde_json::json!({
            "address": ORACLE
        }))
        .unwrap();
        dispatch(&mut state, "add_oracle_updater", &oracle_args, ADMIN);

        // Update rate
        let rate_args = serde_json::to_vec(&serde_json::json!({
            "currency": "CAD",
            "new_rate": CAD_RATE,
            "timestamp": T0
        }))
        .unwrap();
        dispatch(&mut state, "update_rate", &rate_args, ORACLE);

        // Deposit
        let dep_args = serde_json::to_vec(&serde_json::json!({
            "usdc_amount": 100_000_000u64,
            "currency": "CAD",
            "oracle_rate": CAD_RATE,
            "timestamp": T0
        }))
        .unwrap();
        let result = dispatch(&mut state, "deposit", &dep_args, ALICE);
        let minted: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(minted, 136_000_000);

        // Query deposit
        let query_args = serde_json::to_vec(&serde_json::json!({
            "address": ALICE,
            "currency": "CAD"
        }))
        .unwrap();
        let result = dispatch(&mut state, "get_deposit", &query_args, ALICE);
        let deposit: Option<VaultDeposit> = serde_json::from_slice(&result).unwrap();
        assert!(deposit.is_some());
        assert_eq!(deposit.unwrap().usdc_locked, 100_000_000);
    }

    #[test]
    fn test_dispatch_claim_and_withdraw() {
        let mut state: Option<YieldVault> = None;
        let init_args = serde_json::to_vec(&serde_json::json!({"yield_rate_bps": 450})).unwrap();
        dispatch(&mut state, "init", &init_args, ADMIN);

        let rate_args = serde_json::to_vec(&serde_json::json!({
            "currency": "CAD", "new_rate": CAD_RATE, "timestamp": T0
        }))
        .unwrap();
        dispatch(&mut state, "update_rate", &rate_args, ADMIN);

        let dep_args = serde_json::to_vec(&serde_json::json!({
            "usdc_amount": 100_000_000u64, "currency": "CAD",
            "oracle_rate": CAD_RATE, "timestamp": T0
        }))
        .unwrap();
        dispatch(&mut state, "deposit", &dep_args, ALICE);

        // Claim yield after 1 year
        let claim_args = serde_json::to_vec(&serde_json::json!({
            "currency": "CAD", "current_time": T0 + SECONDS_PER_YEAR
        }))
        .unwrap();
        let result = dispatch(&mut state, "claim_yield", &claim_args, ALICE);
        let (usdc, _cad, rate): (u64, u64, u64) = serde_json::from_slice(&result).unwrap();
        assert_eq!(usdc, 4_500_000);
        assert_eq!(rate, CAD_RATE);

        // Withdraw partial
        let withdraw_args = serde_json::to_vec(&serde_json::json!({
            "currency": "CAD", "stablecoin_amount": 68_000_000u64,
            "current_time": T0 + SECONDS_PER_YEAR
        }))
        .unwrap();
        let result = dispatch(&mut state, "withdraw", &withdraw_args, ALICE);
        let released: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(released, 50_000_000);
    }

    #[test]
    #[should_panic(expected = "already initialised")]
    fn test_dispatch_double_init() {
        let mut state: Option<YieldVault> = None;
        let args = serde_json::to_vec(&serde_json::json!({"yield_rate_bps": 450})).unwrap();
        dispatch(&mut state, "init", &args, ADMIN);
        dispatch(&mut state, "init", &args, ADMIN);
    }

    #[test]
    #[should_panic(expected = "unknown method")]
    fn test_dispatch_unknown_method() {
        let mut state: Option<YieldVault> = None;
        let args = serde_json::to_vec(&serde_json::json!({"yield_rate_bps": 450})).unwrap();
        dispatch(&mut state, "init", &args, ADMIN);
        dispatch(&mut state, "foo", &[], ADMIN);
    }

    #[test]
    fn test_dispatch_list_all_deposits() {
        let mut state: Option<YieldVault> = None;
        let init_args = serde_json::to_vec(&serde_json::json!({"yield_rate_bps": 450})).unwrap();
        dispatch(&mut state, "init", &init_args, ADMIN);

        let dep1 = serde_json::to_vec(&serde_json::json!({
            "usdc_amount": 50_000_000u64, "currency": "CAD",
            "oracle_rate": CAD_RATE, "timestamp": T0
        }))
        .unwrap();
        dispatch(&mut state, "deposit", &dep1, ALICE);

        let dep2 = serde_json::to_vec(&serde_json::json!({
            "usdc_amount": 50_000_000u64, "currency": "EUR",
            "oracle_rate": EUR_RATE, "timestamp": T0
        }))
        .unwrap();
        dispatch(&mut state, "deposit", &dep2, ALICE);

        let list_args = serde_json::to_vec(&serde_json::json!({"address": ALICE})).unwrap();
        let result = dispatch(&mut state, "list_all_deposits", &list_args, ALICE);
        let deposits: Vec<VaultDeposit> = serde_json::from_slice(&result).unwrap();
        assert_eq!(deposits.len(), 2);
    }

    #[test]
    fn test_dispatch_proof_of_reserves() {
        let mut state: Option<YieldVault> = None;
        let init_args = serde_json::to_vec(&serde_json::json!({"yield_rate_bps": 450})).unwrap();
        dispatch(&mut state, "init", &init_args, ADMIN);

        let rate_args = serde_json::to_vec(&serde_json::json!({
            "currency": "CAD", "new_rate": CAD_RATE, "timestamp": T0
        }))
        .unwrap();
        dispatch(&mut state, "update_rate", &rate_args, ADMIN);

        let dep_args = serde_json::to_vec(&serde_json::json!({
            "usdc_amount": 100_000_000u64, "currency": "CAD",
            "oracle_rate": CAD_RATE, "timestamp": T0
        }))
        .unwrap();
        dispatch(&mut state, "deposit", &dep_args, ALICE);

        let proof_args = serde_json::to_vec(&serde_json::json!({"timestamp": T0})).unwrap();
        let result = dispatch(&mut state, "proof_of_reserves", &proof_args, ALICE);
        let proof: ProofOfReserves = serde_json::from_slice(&result).unwrap();
        assert_eq!(proof.total_usdc_in_vault, 100_000_000);
        assert!(proof.is_fully_backed);
    }
}
