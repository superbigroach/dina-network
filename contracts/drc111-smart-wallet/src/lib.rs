use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-111  Smart Wallet  -- human wallet with passkeys, sessions, guardians
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PasskeyCredential {
    pub credential_id: Vec<u8>,
    pub public_key: Vec<u8>, // P-256 compressed or uncompressed
    pub name: String,
    pub registered_at: u64,
    pub counter: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Permission {
    Transfer,
    Call,
    Sign,
    ManageSessions,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionLimits {
    pub max_per_transaction: u64,
    pub max_total: u64,
    pub allowed_targets: Vec<[u8; 32]>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionConfig {
    pub label: String,
    pub session_key: [u8; 32],
    pub permissions: Vec<Permission>,
    pub expires_at: u64,
    pub limits: SessionLimits,
    pub spent: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Guardian {
    pub address: [u8; 32],
    pub weight: u16,
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecoveryStatus {
    pub recovery_id: u64,
    pub initiated_by: [u8; 32],
    pub new_passkey: PasskeyCredential,
    pub approvals: Vec<[u8; 32]>,
    pub total_weight: u16,
    pub required_weight: u16,
    pub cooldown_until: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WalletLimits {
    pub max_per_transaction: u64,
    pub daily_limit: u64,
    pub timelock_threshold: u64,
    pub timelock_delay_ms: u64,
}

impl Default for WalletLimits {
    fn default() -> Self {
        Self {
            max_per_transaction: u64::MAX,
            daily_limit: u64::MAX,
            timelock_threshold: u64::MAX,
            timelock_delay_ms: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpendingStats {
    pub total_spent: u64,
    pub spent_today: u64,
    pub transactions_today: u32,
    pub current_day: u64,
}

impl Default for SpendingStats {
    fn default() -> Self {
        Self::new()
    }
}

impl SpendingStats {
    pub fn new() -> Self {
        Self {
            total_spent: 0,
            spent_today: 0,
            transactions_today: 0,
            current_day: 0,
        }
    }

    pub fn roll_over(&mut self, day: u64) {
        if day != self.current_day {
            self.spent_today = 0;
            self.transactions_today = 0;
            self.current_day = day;
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimelockEntry {
    pub to: [u8; 32],
    pub amount: u64,
    pub execute_after: u64,
    pub memo: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum WalletAction {
    Transfer {
        to: [u8; 32],
        amount: u64,
        memo: String,
    },
    Call {
        target: [u8; 32],
        amount: u64,
        method: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SmartWalletState {
    pub owner: [u8; 32],
    pub passkeys: BTreeMap<Vec<u8>, PasskeyCredential>, // credential_id -> credential
    pub sessions: BTreeMap<[u8; 32], SessionConfig>,    // session_key -> config
    pub guardians: Vec<Guardian>,
    pub recovery_threshold: u16,
    pub recovery_cooldown_ms: u64,
    pub active_recovery: Option<RecoveryStatus>,
    pub limits: WalletLimits,
    pub stats: SpendingStats,
    pub balance: u64,
    pub linked_devices: Vec<[u8; 32]>,
    pub pending_timelocked: Vec<TimelockEntry>,
    pub next_recovery_id: u64,
}

impl SmartWalletState {
    pub fn new(owner: [u8; 32], initial_passkey: PasskeyCredential) -> Self {
        let mut passkeys = BTreeMap::new();
        passkeys.insert(initial_passkey.credential_id.clone(), initial_passkey);
        Self {
            owner,
            passkeys,
            sessions: BTreeMap::new(),
            guardians: Vec::new(),
            recovery_threshold: 0,
            recovery_cooldown_ms: 86_400_000, // 24 hours default
            active_recovery: None,
            limits: WalletLimits::default(),
            stats: SpendingStats::new(),
            balance: 0,
            linked_devices: Vec::new(),
            pending_timelocked: Vec::new(),
            next_recovery_id: 1,
        }
    }

    // -- Passkey execution --------------------------------------------------

    /// Execute an action after verifying the passkey.
    /// In a real contract, `signature` and `authenticator_data` would be
    /// verified against the P-256 public key stored in the credential.
    /// Here we verify the credential_id exists and increment the counter.
    pub fn execute_with_passkey(
        &mut self,
        credential_id: &[u8],
        counter: u64,
        action: WalletAction,
        timestamp_ms: u64,
        day: u64,
    ) {
        let credential = self
            .passkeys
            .get_mut(credential_id)
            .expect("DRC111: passkey credential not found");

        // Counter must be strictly increasing (replay protection)
        assert!(
            counter > credential.counter,
            "DRC111: passkey counter must be strictly increasing ({} <= {})",
            counter,
            credential.counter
        );
        credential.counter = counter;

        self.execute_action(action, timestamp_ms, day);
    }

    // -- Session execution --------------------------------------------------

    pub fn create_session(&mut self, caller: [u8; 32], config: SessionConfig) {
        assert!(
            caller == self.owner,
            "DRC111: only owner can create sessions"
        );
        assert!(
            !self.sessions.contains_key(&config.session_key),
            "DRC111: session key already exists"
        );
        self.sessions.insert(config.session_key, config);
    }

    pub fn revoke_session(&mut self, caller: [u8; 32], session_key: [u8; 32]) {
        assert!(
            caller == self.owner,
            "DRC111: only owner can revoke sessions"
        );
        assert!(
            self.sessions.remove(&session_key).is_some(),
            "DRC111: session not found"
        );
    }

    pub fn execute_with_session(
        &mut self,
        session_key: [u8; 32],
        action: WalletAction,
        timestamp_ms: u64,
        day: u64,
    ) {
        let session = self
            .sessions
            .get_mut(&session_key)
            .expect("DRC111: session not found");

        assert!(timestamp_ms < session.expires_at, "DRC111: session expired");

        // Check permission
        let required_permission = match &action {
            WalletAction::Transfer { .. } => Permission::Transfer,
            WalletAction::Call { .. } => Permission::Call,
        };
        assert!(
            session.permissions.contains(&required_permission),
            "DRC111: session lacks required permission"
        );

        // Check amount against session limits
        let amount = match &action {
            WalletAction::Transfer { amount, .. } => *amount,
            WalletAction::Call { amount, .. } => *amount,
        };
        assert!(
            amount <= session.limits.max_per_transaction,
            "DRC111: exceeds session per-transaction limit"
        );
        assert!(
            session.spent + amount <= session.limits.max_total,
            "DRC111: exceeds session total spending limit"
        );

        // Check allowed targets
        if !session.limits.allowed_targets.is_empty() {
            let target = match &action {
                WalletAction::Transfer { to, .. } => *to,
                WalletAction::Call { target, .. } => *target,
            };
            assert!(
                session.limits.allowed_targets.contains(&target),
                "DRC111: target not in session's allowed list"
            );
        }

        session.spent += amount;

        self.execute_action(action, timestamp_ms, day);
    }

    // -- Internal action execution ------------------------------------------

    fn execute_action(&mut self, action: WalletAction, timestamp_ms: u64, day: u64) {
        self.stats.roll_over(day);

        let amount = match &action {
            WalletAction::Transfer { amount, .. } => *amount,
            WalletAction::Call { amount, .. } => *amount,
        };

        // Check wallet-level limits
        assert!(
            amount <= self.limits.max_per_transaction,
            "DRC111: exceeds wallet per-transaction limit"
        );
        assert!(
            self.stats.spent_today + amount <= self.limits.daily_limit,
            "DRC111: exceeds daily limit"
        );

        // Timelock check: large transactions may require a delay
        if amount >= self.limits.timelock_threshold && self.limits.timelock_delay_ms > 0 {
            let to = match &action {
                WalletAction::Transfer { to, .. } => *to,
                WalletAction::Call { target, .. } => *target,
            };
            let memo = match &action {
                WalletAction::Transfer { memo, .. } => memo.clone(),
                WalletAction::Call { method, .. } => format!("call:{method}"),
            };
            self.pending_timelocked.push(TimelockEntry {
                to,
                amount,
                execute_after: timestamp_ms + self.limits.timelock_delay_ms,
                memo,
            });
            return; // deferred, not executed now
        }

        // Balance check
        assert!(
            self.balance >= amount,
            "DRC111: insufficient balance ({} < {amount})",
            self.balance
        );

        self.balance -= amount;
        self.stats.total_spent += amount;
        self.stats.spent_today += amount;
        self.stats.transactions_today += 1;
    }

    /// Execute all pending timelocked transfers whose delay has elapsed.
    pub fn flush_timelocked(&mut self, current_timestamp_ms: u64) -> Vec<TimelockEntry> {
        let (ready, pending): (Vec<_>, Vec<_>) = self
            .pending_timelocked
            .drain(..)
            .partition(|e| current_timestamp_ms >= e.execute_after);

        self.pending_timelocked = pending;

        for entry in &ready {
            assert!(
                self.balance >= entry.amount,
                "DRC111: insufficient balance for timelocked transfer"
            );
            self.balance -= entry.amount;
            self.stats.total_spent += entry.amount;
        }

        ready
    }

    // -- Recovery -----------------------------------------------------------

    pub fn initiate_recovery(
        &mut self,
        caller: [u8; 32],
        new_passkey: PasskeyCredential,
        timestamp_ms: u64,
    ) {
        // Caller must be a guardian
        assert!(
            self.guardians.iter().any(|g| g.address == caller),
            "DRC111: caller is not a guardian"
        );
        assert!(
            self.active_recovery.is_none(),
            "DRC111: recovery already in progress"
        );
        assert!(
            self.recovery_threshold > 0,
            "DRC111: recovery threshold not configured"
        );

        let guardian_weight = self
            .guardians
            .iter()
            .find(|g| g.address == caller)
            .unwrap()
            .weight;

        let recovery_id = self.next_recovery_id;
        self.next_recovery_id += 1;

        self.active_recovery = Some(RecoveryStatus {
            recovery_id,
            initiated_by: caller,
            new_passkey,
            approvals: vec![caller],
            total_weight: guardian_weight,
            required_weight: self.recovery_threshold,
            cooldown_until: timestamp_ms + self.recovery_cooldown_ms,
        });
    }

    pub fn approve_recovery(&mut self, caller: [u8; 32]) {
        let recovery = self
            .active_recovery
            .as_mut()
            .expect("DRC111: no active recovery");

        assert!(
            self.guardians.iter().any(|g| g.address == caller),
            "DRC111: caller is not a guardian"
        );
        assert!(
            !recovery.approvals.contains(&caller),
            "DRC111: guardian already approved"
        );

        let weight = self
            .guardians
            .iter()
            .find(|g| g.address == caller)
            .unwrap()
            .weight;

        recovery.approvals.push(caller);
        recovery.total_weight += weight;
    }

    pub fn execute_recovery(&mut self, caller: [u8; 32], timestamp_ms: u64) {
        let recovery = self
            .active_recovery
            .take()
            .expect("DRC111: no active recovery");

        assert!(
            self.guardians.iter().any(|g| g.address == caller),
            "DRC111: caller is not a guardian"
        );
        assert!(
            recovery.total_weight >= recovery.required_weight,
            "DRC111: insufficient guardian weight ({} < {})",
            recovery.total_weight,
            recovery.required_weight
        );
        assert!(
            timestamp_ms >= recovery.cooldown_until,
            "DRC111: recovery cooldown not elapsed"
        );

        // Replace all passkeys with the new one
        self.passkeys.clear();
        self.passkeys.insert(
            recovery.new_passkey.credential_id.clone(),
            recovery.new_passkey,
        );

        // Revoke all sessions for safety
        self.sessions.clear();
    }

    pub fn cancel_recovery(&mut self, caller: [u8; 32]) {
        assert!(self.active_recovery.is_some(), "DRC111: no active recovery");
        // Owner (via passkey) or any guardian can cancel
        assert!(
            caller == self.owner || self.guardians.iter().any(|g| g.address == caller),
            "DRC111: only owner or guardian can cancel recovery"
        );
        self.active_recovery = None;
    }

    // -- Configuration ------------------------------------------------------

    pub fn set_limits(&mut self, caller: [u8; 32], limits: WalletLimits) {
        assert!(caller == self.owner, "DRC111: only owner can set limits");
        self.limits = limits;
    }

    pub fn set_timelock(&mut self, caller: [u8; 32], threshold: u64, delay_ms: u64) {
        assert!(caller == self.owner, "DRC111: only owner can set timelock");
        self.limits.timelock_threshold = threshold;
        self.limits.timelock_delay_ms = delay_ms;
    }

    pub fn add_guardian(&mut self, caller: [u8; 32], guardian: Guardian) {
        assert!(caller == self.owner, "DRC111: only owner can add guardians");
        assert!(
            !self.guardians.iter().any(|g| g.address == guardian.address),
            "DRC111: guardian already exists"
        );
        self.guardians.push(guardian);
    }

    pub fn set_recovery_threshold(&mut self, caller: [u8; 32], threshold: u16) {
        assert!(
            caller == self.owner,
            "DRC111: only owner can set recovery threshold"
        );
        self.recovery_threshold = threshold;
    }

    pub fn link_device(&mut self, caller: [u8; 32], device_id: [u8; 32]) {
        assert!(caller == self.owner, "DRC111: only owner can link devices");
        if !self.linked_devices.contains(&device_id) {
            self.linked_devices.push(device_id);
        }
    }

    pub fn deposit(&mut self, amount: u64) {
        assert!(amount > 0, "DRC111: deposit must be positive");
        self.balance += amount;
    }

    // -- Queries ------------------------------------------------------------

    pub fn get_balance(&self) -> u64 {
        self.balance
    }

    pub fn get_config(&self) -> WalletConfig {
        WalletConfig {
            owner: self.owner,
            passkey_count: self.passkeys.len(),
            session_count: self.sessions.len(),
            guardian_count: self.guardians.len(),
            recovery_threshold: self.recovery_threshold,
            recovery_cooldown_ms: self.recovery_cooldown_ms,
            limits: self.limits.clone(),
            linked_device_count: self.linked_devices.len(),
            has_active_recovery: self.active_recovery.is_some(),
        }
    }

    pub fn get_passkeys(&self) -> Vec<&PasskeyCredential> {
        self.passkeys.values().collect()
    }

    pub fn active_sessions(&self) -> Vec<&SessionConfig> {
        self.sessions.values().collect()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WalletConfig {
    pub owner: [u8; 32],
    pub passkey_count: usize,
    pub session_count: usize,
    pub guardian_count: usize,
    pub recovery_threshold: u16,
    pub recovery_cooldown_ms: u64,
    pub limits: WalletLimits,
    pub linked_device_count: usize,
    pub has_active_recovery: bool,
}

// ---------------------------------------------------------------------------
// Dispatch arg types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    initial_passkey: PasskeyCredential,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteWithPasskeyArgs {
    credential_id: Vec<u8>,
    counter: u64,
    action: WalletAction,
    timestamp_ms: u64,
    day: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateSessionArgs {
    config: SessionConfig,
}

#[derive(Serialize, Deserialize, Debug)]
struct RevokeSessionArgs {
    session_key: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteWithSessionArgs {
    session_key: [u8; 32],
    action: WalletAction,
    timestamp_ms: u64,
    day: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct InitiateRecoveryArgs {
    new_passkey: PasskeyCredential,
    timestamp_ms: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteRecoveryArgs {
    timestamp_ms: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetLimitsArgs {
    limits: WalletLimits,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetTimelockArgs {
    threshold: u64,
    delay_ms: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddGuardianArgs {
    guardian: Guardian,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetRecoveryThresholdArgs {
    threshold: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct LinkDeviceArgs {
    device_id: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs {
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct FlushTimelockedArgs {
    current_timestamp_ms: u64,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<SmartWalletState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC111: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC111: bad init args");
            *state = Some(SmartWalletState::new(caller, a.initial_passkey));
            serde_json::to_vec("ok").unwrap()
        }

        "execute_with_passkey" => {
            let s = state.as_mut().expect("DRC111: not initialised");
            let a: ExecuteWithPasskeyArgs =
                serde_json::from_slice(args).expect("DRC111: bad execute_with_passkey args");
            s.execute_with_passkey(&a.credential_id, a.counter, a.action, a.timestamp_ms, a.day);
            serde_json::to_vec("ok").unwrap()
        }

        "create_session" => {
            let s = state.as_mut().expect("DRC111: not initialised");
            let a: CreateSessionArgs =
                serde_json::from_slice(args).expect("DRC111: bad create_session args");
            s.create_session(caller, a.config);
            serde_json::to_vec("ok").unwrap()
        }

        "revoke_session" => {
            let s = state.as_mut().expect("DRC111: not initialised");
            let a: RevokeSessionArgs =
                serde_json::from_slice(args).expect("DRC111: bad revoke_session args");
            s.revoke_session(caller, a.session_key);
            serde_json::to_vec("ok").unwrap()
        }

        "execute_with_session" => {
            let s = state.as_mut().expect("DRC111: not initialised");
            let a: ExecuteWithSessionArgs =
                serde_json::from_slice(args).expect("DRC111: bad execute_with_session args");
            s.execute_with_session(a.session_key, a.action, a.timestamp_ms, a.day);
            serde_json::to_vec("ok").unwrap()
        }

        "initiate_recovery" => {
            let s = state.as_mut().expect("DRC111: not initialised");
            let a: InitiateRecoveryArgs =
                serde_json::from_slice(args).expect("DRC111: bad initiate_recovery args");
            s.initiate_recovery(caller, a.new_passkey, a.timestamp_ms);
            serde_json::to_vec("ok").unwrap()
        }

        "approve_recovery" => {
            let s = state.as_mut().expect("DRC111: not initialised");
            s.approve_recovery(caller);
            serde_json::to_vec("ok").unwrap()
        }

        "execute_recovery" => {
            let s = state.as_mut().expect("DRC111: not initialised");
            let a: ExecuteRecoveryArgs =
                serde_json::from_slice(args).expect("DRC111: bad execute_recovery args");
            s.execute_recovery(caller, a.timestamp_ms);
            serde_json::to_vec("ok").unwrap()
        }

        "cancel_recovery" => {
            let s = state.as_mut().expect("DRC111: not initialised");
            s.cancel_recovery(caller);
            serde_json::to_vec("ok").unwrap()
        }

        "set_limits" => {
            let s = state.as_mut().expect("DRC111: not initialised");
            let a: SetLimitsArgs =
                serde_json::from_slice(args).expect("DRC111: bad set_limits args");
            s.set_limits(caller, a.limits);
            serde_json::to_vec("ok").unwrap()
        }

        "set_timelock" => {
            let s = state.as_mut().expect("DRC111: not initialised");
            let a: SetTimelockArgs =
                serde_json::from_slice(args).expect("DRC111: bad set_timelock args");
            s.set_timelock(caller, a.threshold, a.delay_ms);
            serde_json::to_vec("ok").unwrap()
        }

        "add_guardian" => {
            let s = state.as_mut().expect("DRC111: not initialised");
            let a: AddGuardianArgs =
                serde_json::from_slice(args).expect("DRC111: bad add_guardian args");
            s.add_guardian(caller, a.guardian);
            serde_json::to_vec("ok").unwrap()
        }

        "set_recovery_threshold" => {
            let s = state.as_mut().expect("DRC111: not initialised");
            let a: SetRecoveryThresholdArgs =
                serde_json::from_slice(args).expect("DRC111: bad set_recovery_threshold args");
            s.set_recovery_threshold(caller, a.threshold);
            serde_json::to_vec("ok").unwrap()
        }

        "link_device" => {
            let s = state.as_mut().expect("DRC111: not initialised");
            let a: LinkDeviceArgs =
                serde_json::from_slice(args).expect("DRC111: bad link_device args");
            s.link_device(caller, a.device_id);
            serde_json::to_vec("ok").unwrap()
        }

        "deposit" => {
            let s = state.as_mut().expect("DRC111: not initialised");
            let a: DepositArgs = serde_json::from_slice(args).expect("DRC111: bad deposit args");
            s.deposit(a.amount);
            serde_json::to_vec("ok").unwrap()
        }

        "flush_timelocked" => {
            let s = state.as_mut().expect("DRC111: not initialised");
            let a: FlushTimelockedArgs =
                serde_json::from_slice(args).expect("DRC111: bad flush_timelocked args");
            let executed = s.flush_timelocked(a.current_timestamp_ms);
            serde_json::to_vec(&executed).unwrap()
        }

        "balance" => {
            let s = state.as_ref().expect("DRC111: not initialised");
            serde_json::to_vec(&s.get_balance()).unwrap()
        }

        "config" => {
            let s = state.as_ref().expect("DRC111: not initialised");
            serde_json::to_vec(&s.get_config()).unwrap()
        }

        "passkeys" => {
            let s = state.as_ref().expect("DRC111: not initialised");
            let pks: Vec<PasskeyCredential> = s.get_passkeys().into_iter().cloned().collect();
            serde_json::to_vec(&pks).unwrap()
        }

        "active_sessions" => {
            let s = state.as_ref().expect("DRC111: not initialised");
            let sessions: Vec<SessionConfig> = s.active_sessions().into_iter().cloned().collect();
            serde_json::to_vec(&sessions).unwrap()
        }

        "active_recovery" => {
            let s = state.as_ref().expect("DRC111: not initialised");
            serde_json::to_vec(&s.active_recovery).unwrap()
        }

        "pending_timelocked" => {
            let s = state.as_ref().expect("DRC111: not initialised");
            serde_json::to_vec(&s.pending_timelocked).unwrap()
        }

        "stats" => {
            let s = state.as_ref().expect("DRC111: not initialised");
            serde_json::to_vec(&s.stats).unwrap()
        }

        _ => panic!("DRC111: unknown method '{method}'"),
    }
}
