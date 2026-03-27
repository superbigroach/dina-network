use drc112_view_keys::{GrantStatus, ViewFilter, ViewKeyRegistry, ViewScope};

// ============================================================
// Helpers
// ============================================================

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn setup_registry() -> ViewKeyRegistry {
    ViewKeyRegistry::new(addr(0)) // admin = addr(0)
}

fn grant_basic(
    registry: &mut ViewKeyRegistry,
    grantor: [u8; 32],
    grantee: [u8; 32],
    scopes: Vec<ViewScope>,
    expires_at: u64,
    current_time: u64,
) -> [u8; 32] {
    registry.grant_view_key(
        grantor,
        grantee,
        scopes,
        ViewFilter::default(),
        expires_at,
        true, // revocable
        "Test Grant".to_string(),
        vec![1, 2, 3],
        current_time,
    )
}

// ============================================================
// Grant and query view key
// ============================================================

#[test]
fn grant_and_query_view_key() {
    let mut registry = setup_registry();
    let grantor = addr(1);
    let grantee = addr(2);

    let grant_id = grant_basic(
        &mut registry,
        grantor,
        grantee,
        vec![ViewScope::Balances, ViewScope::TransactionHistory],
        1000,
        100,
    );

    // Query it
    let grant = registry.get_grant(grantor, grant_id).unwrap();
    assert_eq!(grant.grantor, grantor);
    assert_eq!(grant.grantee, grantee);
    assert_eq!(grant.scopes.len(), 2);
    assert!(grant.scopes.contains(&ViewScope::Balances));
    assert!(grant.scopes.contains(&ViewScope::TransactionHistory));
    assert_eq!(grant.expires_at, 1000);
    assert!(!grant.revoked);

    // Grantee can also query it
    let grant2 = registry.get_grant(grantee, grant_id).unwrap();
    assert_eq!(grant2.grant_id, grant_id);
}

// ============================================================
// Revoke grant works
// ============================================================

#[test]
fn revoke_grant_works() {
    let mut registry = setup_registry();
    let grantor = addr(1);
    let grantee = addr(2);

    let grant_id = grant_basic(
        &mut registry, grantor, grantee,
        vec![ViewScope::FullAccess], 0, 100,
    );

    assert_eq!(registry.grant_status(grant_id, 200), GrantStatus::Active);

    registry.revoke_grant(grantor, grant_id);
    assert_eq!(registry.grant_status(grant_id, 200), GrantStatus::Revoked);
}

#[test]
#[should_panic(expected = "only grantor can revoke")]
fn non_grantor_cannot_revoke() {
    let mut registry = setup_registry();
    let grantor = addr(1);
    let grantee = addr(2);

    let grant_id = grant_basic(
        &mut registry, grantor, grantee,
        vec![ViewScope::Balances], 0, 100,
    );

    // grantee tries to revoke
    registry.revoke_grant(grantee, grant_id);
}

// ============================================================
// Expired grant shows expired status
// ============================================================

#[test]
fn expired_grant_shows_expired_status() {
    let mut registry = setup_registry();
    let grantor = addr(1);
    let grantee = addr(2);

    let grant_id = grant_basic(
        &mut registry, grantor, grantee,
        vec![ViewScope::Balances], 500, 100,
    );

    // Before expiry
    assert_eq!(registry.grant_status(grant_id, 300), GrantStatus::Active);
    // After expiry
    assert_eq!(registry.grant_status(grant_id, 600), GrantStatus::Expired);
}

// ============================================================
// Non-revocable grant requires authority
// ============================================================

#[test]
fn non_revocable_grant_requires_authority() {
    let admin = addr(0);
    let mut registry = setup_registry();
    let grantor = addr(1);
    let regulator = addr(3);

    // Register authority first
    registry.register_authority(admin, regulator, "SEC".to_string());

    // Non-revocable grant to registered authority works
    let grant_id = registry.grant_view_key(
        grantor,
        regulator,
        vec![ViewScope::Balances],
        ViewFilter::default(),
        0,
        false, // non-revocable
        "Audit".to_string(),
        vec![],
        100,
    );

    assert_eq!(registry.grant_status(grant_id, 200), GrantStatus::Active);
}

#[test]
#[should_panic(expected = "non-revocable grants only for registered regulatory authorities")]
fn non_revocable_grant_fails_for_non_authority() {
    let mut registry = setup_registry();
    let grantor = addr(1);
    let random = addr(99);

    registry.grant_view_key(
        grantor,
        random,
        vec![ViewScope::FullAccess],
        ViewFilter::default(),
        0,
        false, // non-revocable
        "Should fail".to_string(),
        vec![],
        100,
    );
}

// ============================================================
// verify_access with valid scope succeeds
// ============================================================

#[test]
fn verify_access_with_valid_scope_succeeds() {
    let mut registry = setup_registry();
    let grantor = addr(1);
    let grantee = addr(2);

    grant_basic(
        &mut registry, grantor, grantee,
        vec![ViewScope::Balances, ViewScope::TransactionAmounts],
        1000, 100,
    );

    let result = registry.verify_access(grantee, &ViewScope::Balances, 500);
    assert!(result.is_some());

    let result = registry.verify_access(grantee, &ViewScope::TransactionAmounts, 500);
    assert!(result.is_some());
}

// ============================================================
// verify_access with wrong scope fails
// ============================================================

#[test]
fn verify_access_with_wrong_scope_fails() {
    let mut registry = setup_registry();
    let grantor = addr(1);
    let grantee = addr(2);

    grant_basic(
        &mut registry, grantor, grantee,
        vec![ViewScope::Balances], 1000, 100,
    );

    // Grantee does NOT have TransactionMemos scope
    let result = registry.verify_access(grantee, &ViewScope::TransactionMemos, 500);
    assert!(result.is_none());
}

#[test]
fn verify_access_fails_after_expiry() {
    let mut registry = setup_registry();
    let grantor = addr(1);
    let grantee = addr(2);

    grant_basic(
        &mut registry, grantor, grantee,
        vec![ViewScope::Balances], 500, 100,
    );

    // After expiry
    let result = registry.verify_access(grantee, &ViewScope::Balances, 600);
    assert!(result.is_none());
}

#[test]
fn verify_access_fails_after_revocation() {
    let mut registry = setup_registry();
    let grantor = addr(1);
    let grantee = addr(2);

    let grant_id = grant_basic(
        &mut registry, grantor, grantee,
        vec![ViewScope::Balances], 0, 100,
    );

    // Active before revocation
    assert!(registry.verify_access(grantee, &ViewScope::Balances, 200).is_some());

    registry.revoke_grant(grantor, grant_id);

    // No access after revocation
    assert!(registry.verify_access(grantee, &ViewScope::Balances, 200).is_none());
}

// ============================================================
// extend_grant works
// ============================================================

#[test]
fn extend_grant_works() {
    let mut registry = setup_registry();
    let grantor = addr(1);
    let grantee = addr(2);

    let grant_id = grant_basic(
        &mut registry, grantor, grantee,
        vec![ViewScope::Balances], 1000, 100,
    );

    // Would have been expired at 1500
    assert_eq!(registry.grant_status(grant_id, 1500), GrantStatus::Expired);

    // Extend to 2000
    registry.extend_grant(grantor, grant_id, 2000);

    // Now active at 1500
    assert_eq!(registry.grant_status(grant_id, 1500), GrantStatus::Active);
    // Still expires after 2000
    assert_eq!(registry.grant_status(grant_id, 2500), GrantStatus::Expired);
}

#[test]
#[should_panic(expected = "can only extend, not shorten")]
fn extend_grant_cannot_shorten() {
    let mut registry = setup_registry();
    let grantor = addr(1);
    let grantee = addr(2);

    let grant_id = grant_basic(
        &mut registry, grantor, grantee,
        vec![ViewScope::Balances], 1000, 100,
    );

    // Try to shorten from 1000 to 500
    registry.extend_grant(grantor, grant_id, 500);
}

// ============================================================
// add_scopes works
// ============================================================

#[test]
fn add_scopes_works() {
    let mut registry = setup_registry();
    let grantor = addr(1);
    let grantee = addr(2);

    let grant_id = grant_basic(
        &mut registry, grantor, grantee,
        vec![ViewScope::Balances], 1000, 100,
    );

    // Initially no TransactionHistory scope
    assert!(registry.verify_access(grantee, &ViewScope::TransactionHistory, 500).is_none());

    // Add new scope
    registry.add_scopes(grantor, grant_id, vec![ViewScope::TransactionHistory]);

    // Now has TransactionHistory
    assert!(registry.verify_access(grantee, &ViewScope::TransactionHistory, 500).is_some());

    // Original scope still works
    assert!(registry.verify_access(grantee, &ViewScope::Balances, 500).is_some());

    // Verify no duplicates when adding existing scope
    registry.add_scopes(grantor, grant_id, vec![ViewScope::Balances]);
    let grant = registry.get_grant(grantor, grant_id).unwrap();
    assert_eq!(grant.scopes.len(), 2); // Still 2, not 3
}

#[test]
fn full_access_grants_all_scopes() {
    let mut registry = setup_registry();
    let grantor = addr(1);
    let grantee = addr(2);

    grant_basic(
        &mut registry, grantor, grantee,
        vec![ViewScope::FullAccess], 1000, 100,
    );

    // FullAccess should give access to any scope
    assert!(registry.verify_access(grantee, &ViewScope::Balances, 500).is_some());
    assert!(registry.verify_access(grantee, &ViewScope::TransactionHistory, 500).is_some());
    assert!(registry.verify_access(grantee, &ViewScope::TransactionMemos, 500).is_some());
    assert!(registry.verify_access(grantee, &ViewScope::ContractCalls, 500).is_some());
}
