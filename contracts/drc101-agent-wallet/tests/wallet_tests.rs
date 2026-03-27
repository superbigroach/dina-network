use drc101_agent_wallet::{AgentWalletState, SpendingLimits};

// ============================================================
// Helpers
// ============================================================

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn create_wallet_with_limits(
    owner: [u8; 32],
    balance: u64,
    max_per_tx: u64,
    max_per_day: u64,
) -> AgentWalletState {
    let mut wallet = AgentWalletState::new(owner);
    wallet.deposit(balance);
    wallet.set_limits(
        owner,
        SpendingLimits {
            max_per_transaction: max_per_tx,
            max_per_day,
            max_per_month: u64::MAX,
            max_transactions_per_day: u32::MAX,
            min_interval_ms: 0,
        },
    );
    wallet
}

// ============================================================
// Create wallet with spending limits
// ============================================================

#[test]
fn create_wallet_with_spending_limits() {
    let owner = addr(1);
    let mut wallet = AgentWalletState::new(owner);

    wallet.set_limits(
        owner,
        SpendingLimits {
            max_per_transaction: 1000,
            max_per_day: 5000,
            max_per_month: 20000,
            max_transactions_per_day: 10,
            min_interval_ms: 60_000,
        },
    );

    assert_eq!(wallet.rules.limits.max_per_transaction, 1000);
    assert_eq!(wallet.rules.limits.max_per_day, 5000);
    assert_eq!(wallet.rules.limits.max_per_month, 20000);
    assert_eq!(wallet.rules.limits.max_transactions_per_day, 10);
    assert_eq!(wallet.rules.limits.min_interval_ms, 60_000);
    assert!(!wallet.frozen);
    assert_eq!(wallet.balance, 0);
}

// ============================================================
// execute_transfer within limits succeeds
// ============================================================

#[test]
fn execute_transfer_within_limits_succeeds() {
    let owner = addr(1);
    let recipient = addr(2);
    let mut wallet = create_wallet_with_limits(owner, 10_000, 5000, 50_000);

    wallet.execute_transfer(
        owner,
        recipient,
        1000,
        100_000, // timestamp
        1,       // day
        1,       // month
        "test payment".to_string(),
        None,
    );

    assert_eq!(wallet.balance, 9000);
    assert_eq!(wallet.stats.total_spent, 1000);
    assert_eq!(wallet.stats.spent_today, 1000);
    assert_eq!(wallet.transaction_log.len(), 1);
}

// ============================================================
// execute_transfer exceeding per-transaction limit fails
// ============================================================

#[test]
#[should_panic(expected = "exceeds per-transaction limit")]
fn execute_transfer_exceeding_per_transaction_limit_fails() {
    let owner = addr(1);
    let recipient = addr(2);
    let mut wallet = create_wallet_with_limits(owner, 10_000, 500, 50_000);

    wallet.execute_transfer(
        owner,
        recipient,
        1000, // exceeds max_per_transaction of 500
        100_000,
        1,
        1,
        "too big".to_string(),
        None,
    );
}

// ============================================================
// execute_transfer exceeding daily limit fails
// ============================================================

#[test]
#[should_panic(expected = "exceeds daily spending limit")]
fn execute_transfer_exceeding_daily_limit_fails() {
    let owner = addr(1);
    let recipient = addr(2);
    let mut wallet = create_wallet_with_limits(owner, 10_000, 5000, 1000);

    // First transfer: 800 (under daily limit of 1000)
    wallet.execute_transfer(
        owner, recipient, 800, 100_000, 1, 1,
        "first".to_string(), None,
    );

    // Second transfer: 300, total would be 1100 > 1000 daily limit
    wallet.execute_transfer(
        owner, recipient, 300, 100_001, 1, 1,
        "second".to_string(), None,
    );
}

// ============================================================
// emergency_stop freezes wallet
// ============================================================

#[test]
fn emergency_stop_freezes_wallet() {
    let owner = addr(1);
    let mut wallet = AgentWalletState::new(owner);
    wallet.deposit(1000);

    assert!(!wallet.frozen);
    wallet.emergency_stop(owner);
    assert!(wallet.frozen);
}

#[test]
#[should_panic(expected = "wallet is frozen")]
fn frozen_wallet_rejects_transfer() {
    let owner = addr(1);
    let recipient = addr(2);
    let mut wallet = AgentWalletState::new(owner);
    wallet.deposit(1000);

    wallet.emergency_stop(owner);
    wallet.execute_transfer(
        owner, recipient, 100, 100_000, 1, 1,
        "should fail".to_string(), None,
    );
}

// ============================================================
// resume unfreezes wallet
// ============================================================

#[test]
fn resume_unfreezes_wallet() {
    let owner = addr(1);
    let recipient = addr(2);
    let mut wallet = AgentWalletState::new(owner);
    wallet.deposit(1000);

    wallet.emergency_stop(owner);
    assert!(wallet.frozen);

    wallet.resume(owner);
    assert!(!wallet.frozen);

    // Can transfer again after resume
    wallet.execute_transfer(
        owner, recipient, 100, 100_000, 1, 1,
        "after resume".to_string(), None,
    );
    assert_eq!(wallet.balance, 900);
}

// ============================================================
// Only owner can emergency_stop
// ============================================================

#[test]
fn guardian_can_emergency_stop() {
    let owner = addr(1);
    let guardian = addr(2);
    let mut wallet = AgentWalletState::new(owner);
    wallet.rules.guardian = Some(guardian);

    wallet.emergency_stop(guardian);
    assert!(wallet.frozen);
}

#[test]
#[should_panic(expected = "only owner or guardian can freeze")]
fn non_owner_non_guardian_cannot_emergency_stop() {
    let owner = addr(1);
    let random = addr(99);
    let mut wallet = AgentWalletState::new(owner);

    wallet.emergency_stop(random);
}

// ============================================================
// Approved counterparty enforcement
// ============================================================

#[test]
fn approved_counterparty_enforcement_works() {
    let owner = addr(1);
    let approved = addr(2);
    let mut wallet = AgentWalletState::new(owner);
    wallet.deposit(5000);

    // Add approved counterparty
    wallet.add_approved_counterparty(owner, approved);

    // Transfer to approved counterparty succeeds
    wallet.execute_transfer(
        owner, approved, 100, 100_000, 1, 1,
        "approved".to_string(), None,
    );
    assert_eq!(wallet.balance, 4900);
}

#[test]
#[should_panic(expected = "counterparty not approved")]
fn unapproved_counterparty_rejected() {
    let owner = addr(1);
    let approved = addr(2);
    let unapproved = addr(3);
    let mut wallet = AgentWalletState::new(owner);
    wallet.deposit(5000);

    // Add ONLY approved counterparty
    wallet.add_approved_counterparty(owner, approved);

    // Transfer to unapproved counterparty should fail
    wallet.execute_transfer(
        owner, unapproved, 100, 100_000, 1, 1,
        "should fail".to_string(), None,
    );
}

#[test]
fn empty_counterparty_list_allows_any_recipient() {
    let owner = addr(1);
    let anyone = addr(99);
    let mut wallet = AgentWalletState::new(owner);
    wallet.deposit(5000);

    // No approved counterparties = no restriction
    wallet.execute_transfer(
        owner, anyone, 100, 100_000, 1, 1,
        "open".to_string(), None,
    );
    assert_eq!(wallet.balance, 4900);
}
