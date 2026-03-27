use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// DRC-101  Agent Wallet  -- autonomous wallet with spending rules
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpendingLimits {
    pub max_per_transaction: u64,
    pub max_per_day: u64,
    pub max_per_month: u64,
    pub max_transactions_per_day: u32,
    pub min_interval_ms: u64,
}

impl Default for SpendingLimits {
    fn default() -> Self {
        Self {
            max_per_transaction: u64::MAX,
            max_per_day: u64::MAX,
            max_per_month: u64::MAX,
            max_transactions_per_day: u32::MAX,
            min_interval_ms: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WalletRules {
    pub owner: [u8; 32],
    pub guardian: Option<[u8; 32]>,
    pub limits: SpendingLimits,
    pub approved_counterparties: Vec<[u8; 32]>,
    pub approved_interfaces: Vec<[u8; 32]>,
    pub require_witness: bool,
    pub bound_device: Option<[u8; 32]>,
    pub auto_sweep_threshold: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpendingStats {
    pub total_spent: u64,
    pub spent_today: u64,
    pub spent_this_month: u64,
    pub transactions_today: u32,
    pub last_transaction_ms: u64,
    pub current_day: u64,
    pub current_month: u64,
}

impl SpendingStats {
    pub fn new() -> Self {
        Self {
            total_spent: 0,
            spent_today: 0,
            spent_this_month: 0,
            transactions_today: 0,
            last_transaction_ms: 0,
            current_day: 0,
            current_month: 0,
        }
    }

    /// Reset daily/monthly counters if the period rolled over.
    pub fn roll_over(&mut self, day: u64, month: u64) {
        if day != self.current_day {
            self.spent_today = 0;
            self.transactions_today = 0;
            self.current_day = day;
        }
        if month != self.current_month {
            self.spent_this_month = 0;
            self.current_month = month;
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionRecord {
    pub to: [u8; 32],
    pub amount: u64,
    pub timestamp_ms: u64,
    pub memo: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentWalletState {
    pub rules: WalletRules,
    pub stats: SpendingStats,
    pub transaction_log: Vec<TransactionRecord>,
    pub frozen: bool,
    pub balance: u64,
}

impl AgentWalletState {
    pub fn new(owner: [u8; 32]) -> Self {
        Self {
            rules: WalletRules {
                owner,
                guardian: None,
                limits: SpendingLimits::default(),
                approved_counterparties: Vec::new(),
                approved_interfaces: Vec::new(),
                require_witness: false,
                bound_device: None,
                auto_sweep_threshold: None,
            },
            stats: SpendingStats::new(),
            transaction_log: Vec::new(),
            frozen: false,
            balance: 0,
        }
    }

    // -- Core transfer logic ------------------------------------------------

    pub fn execute_transfer(
        &mut self,
        caller: [u8; 32],
        to: [u8; 32],
        amount: u64,
        timestamp_ms: u64,
        day: u64,
        month: u64,
        memo: String,
        witness: Option<[u8; 32]>,
    ) {
        assert!(!self.frozen, "DRC101: wallet is frozen");
        assert!(
            caller == self.rules.owner
                || self
                    .rules
                    .approved_interfaces
                    .contains(&caller),
            "DRC101: caller not authorised"
        );

        // Counterparty check
        if !self.rules.approved_counterparties.is_empty() {
            assert!(
                self.rules.approved_counterparties.contains(&to),
                "DRC101: counterparty not approved"
            );
        }

        // Witness check
        if self.rules.require_witness {
            assert!(witness.is_some(), "DRC101: witness signature required");
        }

        // Bound device check
        if let Some(device) = self.rules.bound_device {
            assert!(
                witness == Some(device),
                "DRC101: transaction must be witnessed by bound device"
            );
        }

        // Roll over daily/monthly counters
        self.stats.roll_over(day, month);

        // Per-transaction limit
        assert!(
            amount <= self.rules.limits.max_per_transaction,
            "DRC101: exceeds per-transaction limit"
        );

        // Daily limit
        assert!(
            self.stats.spent_today + amount <= self.rules.limits.max_per_day,
            "DRC101: exceeds daily spending limit"
        );

        // Monthly limit
        assert!(
            self.stats.spent_this_month + amount <= self.rules.limits.max_per_month,
            "DRC101: exceeds monthly spending limit"
        );

        // Transactions-per-day limit
        assert!(
            self.stats.transactions_today < self.rules.limits.max_transactions_per_day,
            "DRC101: exceeds daily transaction count limit"
        );

        // Min interval
        if self.stats.last_transaction_ms > 0 {
            let elapsed = timestamp_ms.saturating_sub(self.stats.last_transaction_ms);
            assert!(
                elapsed >= self.rules.limits.min_interval_ms,
                "DRC101: minimum interval not met ({elapsed}ms < {}ms)",
                self.rules.limits.min_interval_ms
            );
        }

        // Balance check
        assert!(
            self.balance >= amount,
            "DRC101: insufficient balance ({} < {amount})",
            self.balance
        );

        // Execute
        self.balance -= amount;
        self.stats.total_spent += amount;
        self.stats.spent_today += amount;
        self.stats.spent_this_month += amount;
        self.stats.transactions_today += 1;
        self.stats.last_transaction_ms = timestamp_ms;
        self.transaction_log.push(TransactionRecord {
            to,
            amount,
            timestamp_ms,
            memo,
        });
    }

    pub fn execute_call(
        &mut self,
        caller: [u8; 32],
        target: [u8; 32],
        amount: u64,
        timestamp_ms: u64,
        day: u64,
        month: u64,
        method: String,
        witness: Option<[u8; 32]>,
    ) {
        // A "call" is a transfer that also invokes a method on the target contract.
        // The spending-rule enforcement is identical.
        self.execute_transfer(
            caller,
            target,
            amount,
            timestamp_ms,
            day,
            month,
            format!("call:{method}"),
            witness,
        );
    }

    pub fn emergency_stop(&mut self, caller: [u8; 32]) {
        assert!(
            caller == self.rules.owner || self.rules.guardian == Some(caller),
            "DRC101: only owner or guardian can freeze"
        );
        self.frozen = true;
    }

    pub fn resume(&mut self, caller: [u8; 32]) {
        assert!(
            caller == self.rules.owner,
            "DRC101: only owner can resume"
        );
        self.frozen = false;
    }

    pub fn set_limits(&mut self, caller: [u8; 32], limits: SpendingLimits) {
        assert!(
            caller == self.rules.owner,
            "DRC101: only owner can set limits"
        );
        self.rules.limits = limits;
    }

    pub fn add_approved_counterparty(&mut self, caller: [u8; 32], counterparty: [u8; 32]) {
        assert!(
            caller == self.rules.owner,
            "DRC101: only owner can modify approved counterparties"
        );
        if !self.rules.approved_counterparties.contains(&counterparty) {
            self.rules.approved_counterparties.push(counterparty);
        }
    }

    pub fn remove_approved_counterparty(&mut self, caller: [u8; 32], counterparty: [u8; 32]) {
        assert!(
            caller == self.rules.owner,
            "DRC101: only owner can modify approved counterparties"
        );
        self.rules
            .approved_counterparties
            .retain(|c| c != &counterparty);
    }

    pub fn deposit(&mut self, amount: u64) {
        assert!(amount > 0, "DRC101: deposit must be positive");
        self.balance += amount;
    }

    pub fn spending_stats(&self) -> &SpendingStats {
        &self.stats
    }
}

// ---------------------------------------------------------------------------
// Dispatch arg types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteTransferArgs {
    to: [u8; 32],
    amount: u64,
    timestamp_ms: u64,
    day: u64,
    month: u64,
    memo: String,
    witness: Option<[u8; 32]>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteCallArgs {
    target: [u8; 32],
    amount: u64,
    timestamp_ms: u64,
    day: u64,
    month: u64,
    method: String,
    witness: Option<[u8; 32]>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetLimitsArgs {
    limits: SpendingLimits,
}

#[derive(Serialize, Deserialize, Debug)]
struct CounterpartyArgs {
    counterparty: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs {
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetGuardianArgs {
    guardian: [u8; 32],
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<AgentWalletState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC101: already initialised");
            *state = Some(AgentWalletState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "execute_transfer" => {
            let s = state.as_mut().expect("DRC101: not initialised");
            let a: ExecuteTransferArgs =
                serde_json::from_slice(args).expect("DRC101: bad execute_transfer args");
            s.execute_transfer(
                caller,
                a.to,
                a.amount,
                a.timestamp_ms,
                a.day,
                a.month,
                a.memo,
                a.witness,
            );
            serde_json::to_vec("ok").unwrap()
        }

        "execute_call" => {
            let s = state.as_mut().expect("DRC101: not initialised");
            let a: ExecuteCallArgs =
                serde_json::from_slice(args).expect("DRC101: bad execute_call args");
            s.execute_call(
                caller,
                a.target,
                a.amount,
                a.timestamp_ms,
                a.day,
                a.month,
                a.method,
                a.witness,
            );
            serde_json::to_vec("ok").unwrap()
        }

        "emergency_stop" => {
            let s = state.as_mut().expect("DRC101: not initialised");
            s.emergency_stop(caller);
            serde_json::to_vec("ok").unwrap()
        }

        "resume" => {
            let s = state.as_mut().expect("DRC101: not initialised");
            s.resume(caller);
            serde_json::to_vec("ok").unwrap()
        }

        "set_limits" => {
            let s = state.as_mut().expect("DRC101: not initialised");
            let a: SetLimitsArgs =
                serde_json::from_slice(args).expect("DRC101: bad set_limits args");
            s.set_limits(caller, a.limits);
            serde_json::to_vec("ok").unwrap()
        }

        "add_approved_counterparty" => {
            let s = state.as_mut().expect("DRC101: not initialised");
            let a: CounterpartyArgs =
                serde_json::from_slice(args).expect("DRC101: bad add_approved_counterparty args");
            s.add_approved_counterparty(caller, a.counterparty);
            serde_json::to_vec("ok").unwrap()
        }

        "remove_approved_counterparty" => {
            let s = state.as_mut().expect("DRC101: not initialised");
            let a: CounterpartyArgs =
                serde_json::from_slice(args).expect("DRC101: bad remove_approved_counterparty args");
            s.remove_approved_counterparty(caller, a.counterparty);
            serde_json::to_vec("ok").unwrap()
        }

        "deposit" => {
            let s = state.as_mut().expect("DRC101: not initialised");
            let a: DepositArgs =
                serde_json::from_slice(args).expect("DRC101: bad deposit args");
            s.deposit(a.amount);
            serde_json::to_vec("ok").unwrap()
        }

        "spending_stats" => {
            let s = state.as_ref().expect("DRC101: not initialised");
            serde_json::to_vec(s.spending_stats()).unwrap()
        }

        "balance" => {
            let s = state.as_ref().expect("DRC101: not initialised");
            serde_json::to_vec(&s.balance).unwrap()
        }

        "is_frozen" => {
            let s = state.as_ref().expect("DRC101: not initialised");
            serde_json::to_vec(&s.frozen).unwrap()
        }

        "transaction_log" => {
            let s = state.as_ref().expect("DRC101: not initialised");
            serde_json::to_vec(&s.transaction_log).unwrap()
        }

        "set_guardian" => {
            let s = state.as_mut().expect("DRC101: not initialised");
            assert!(caller == s.rules.owner, "DRC101: only owner can set guardian");
            let a: SetGuardianArgs =
                serde_json::from_slice(args).expect("DRC101: bad set_guardian args");
            s.rules.guardian = Some(a.guardian);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC101: unknown method '{method}'"),
    }
}
