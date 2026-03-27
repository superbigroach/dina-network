//! Integration tests for dina-privacy crate.
//!
//! These tests exercise cross-module flows and validate that the public API
//! works correctly when modules are composed together.

use dina_privacy::encrypted_memo::{decrypt_memo, encrypt_memo, EncryptedMemo};
use dina_privacy::permissions::{Action, KeyPermission, PermissionSet};
use dina_privacy::stealth::{
    derive_stealth_address, derive_stealth_spending_key, detect_stealth, generate_meta_address,
    StealthAddress, StealthMetaAddress,
};

use dina_core::Address;
use x25519_dalek::{PublicKey, StaticSecret};

// ---------------------------------------------------------------------------
// Encrypted Memo integration tests
// ---------------------------------------------------------------------------

#[test]
fn encrypt_memo_for_stealth_recipient() {
    // Generate a stealth meta-address
    let (meta, scan_secret, spend_secret) = generate_meta_address();

    // Derive a stealth address (simulating a sender paying the recipient)
    let stealth = derive_stealth_address(&meta);

    // The sender encrypts a memo for the recipient using the spend pubkey
    // (in practice you'd use the stealth pubkey, but here we show the
    // encrypted memo can be attached to a stealth transaction)
    let memo = encrypt_memo(&meta.spend_pubkey, b"Payment for invoice #42");

    // Recipient can detect the stealth address
    assert!(detect_stealth(
        &scan_secret,
        &meta.spend_pubkey,
        &stealth.ephemeral_pubkey,
        &stealth.address,
    ));

    // Recipient can also decrypt the memo using spend_secret
    // (since we encrypted to spend_pubkey)
    let decrypted = decrypt_memo(&spend_secret, &memo).unwrap();
    assert_eq!(decrypted, b"Payment for invoice #42");
}

#[test]
fn encrypted_memo_binary_data_roundtrip() {
    let secret_bytes: [u8; 32] = rand::random();
    let secret = StaticSecret::from(secret_bytes);
    let pubkey = PublicKey::from(&secret);

    // Binary data with all byte values
    let binary_data: Vec<u8> = (0..=255).collect();
    let memo = encrypt_memo(pubkey.as_bytes(), &binary_data);
    let decrypted = decrypt_memo(&secret_bytes, &memo).unwrap();
    assert_eq!(decrypted, binary_data);
}

#[test]
fn encrypted_memo_json_serialization_preserves_all_fields() {
    let secret_bytes: [u8; 32] = rand::random();
    let secret = StaticSecret::from(secret_bytes);
    let pubkey = PublicKey::from(&secret);

    let memo = encrypt_memo(pubkey.as_bytes(), b"test data");
    let json = serde_json::to_string(&memo).unwrap();

    // Verify the JSON contains the expected fields
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("ephemeral_pubkey").is_some());
    assert!(parsed.get("ciphertext").is_some());
    assert!(parsed.get("nonce").is_some());

    // Roundtrip
    let deserialized: EncryptedMemo = serde_json::from_str(&json).unwrap();
    let decrypted = decrypt_memo(&secret_bytes, &deserialized).unwrap();
    assert_eq!(decrypted, b"test data");
}

// ---------------------------------------------------------------------------
// Stealth Address integration tests
// ---------------------------------------------------------------------------

#[test]
fn multiple_senders_same_recipient_all_detected() {
    let (meta, scan_secret, _spend_secret) = generate_meta_address();

    // Simulate 10 different senders each deriving a stealth address
    let stealth_addresses: Vec<StealthAddress> =
        (0..10).map(|_| derive_stealth_address(&meta)).collect();

    // All addresses should be unique
    for i in 0..stealth_addresses.len() {
        for j in (i + 1)..stealth_addresses.len() {
            assert_ne!(stealth_addresses[i].address, stealth_addresses[j].address);
        }
    }

    // Recipient should detect all of them
    for stealth in &stealth_addresses {
        assert!(detect_stealth(
            &scan_secret,
            &meta.spend_pubkey,
            &stealth.ephemeral_pubkey,
            &stealth.address,
        ));
    }
}

#[test]
fn different_recipients_do_not_collide() {
    let (meta1, scan1, _spend1) = generate_meta_address();
    let (meta2, scan2, _spend2) = generate_meta_address();

    let stealth1 = derive_stealth_address(&meta1);
    let stealth2 = derive_stealth_address(&meta2);

    // Recipient 1 detects their own, not recipient 2's
    assert!(detect_stealth(
        &scan1,
        &meta1.spend_pubkey,
        &stealth1.ephemeral_pubkey,
        &stealth1.address,
    ));
    assert!(!detect_stealth(
        &scan1,
        &meta1.spend_pubkey,
        &stealth2.ephemeral_pubkey,
        &stealth2.address,
    ));

    // Recipient 2 detects their own, not recipient 1's
    assert!(detect_stealth(
        &scan2,
        &meta2.spend_pubkey,
        &stealth2.ephemeral_pubkey,
        &stealth2.address,
    ));
    assert!(!detect_stealth(
        &scan2,
        &meta2.spend_pubkey,
        &stealth1.ephemeral_pubkey,
        &stealth1.address,
    ));
}

#[test]
fn stealth_spending_keys_differ_per_transaction() {
    let (meta, scan_secret, spend_secret) = generate_meta_address();

    let stealth1 = derive_stealth_address(&meta);
    let stealth2 = derive_stealth_address(&meta);

    let key1 = derive_stealth_spending_key(&scan_secret, &spend_secret, &stealth1.ephemeral_pubkey);
    let key2 = derive_stealth_spending_key(&scan_secret, &spend_secret, &stealth2.ephemeral_pubkey);

    // Different ephemeral keys -> different spending keys
    assert_ne!(key1, key2);
}

#[test]
fn stealth_meta_address_serde_roundtrip() {
    let (meta, _, _) = generate_meta_address();
    let json = serde_json::to_string(&meta).unwrap();
    let deserialized: StealthMetaAddress = serde_json::from_str(&json).unwrap();
    assert_eq!(meta, deserialized);
}

// ---------------------------------------------------------------------------
// Permission integration tests
// ---------------------------------------------------------------------------

fn test_addr(byte: u8) -> Address {
    Address([byte; 32])
}

#[test]
fn permission_lifecycle_add_use_rotate_remove() {
    let mut pset = PermissionSet::new(test_addr(0x01));

    // Step 1: Add a key
    let key1: [u8; 32] = [0xAA; 32];
    pset.add_key(
        key1,
        "operator".into(),
        KeyPermission::TransferOnly {
            max_amount: Some(1000),
            allowed_recipients: vec![],
        },
        100,
    );

    // Step 2: Use the key
    assert!(pset.is_authorized(
        &key1,
        &Action::Transfer {
            to: test_addr(0x02),
            amount: 500,
        },
        200,
    ));

    // Step 3: Rotate to a new key
    let key2: [u8; 32] = [0xBB; 32];
    pset.rotate_key(&key1, key2).unwrap();

    // Old key no longer works
    assert!(!pset.is_authorized(
        &key1,
        &Action::Transfer {
            to: test_addr(0x02),
            amount: 500,
        },
        300,
    ));

    // New key works with same permissions
    assert!(pset.is_authorized(
        &key2,
        &Action::Transfer {
            to: test_addr(0x02),
            amount: 500,
        },
        300,
    ));

    // Step 4: Remove the key
    pset.remove_key(&key2).unwrap();
    assert!(!pset.is_authorized(
        &key2,
        &Action::Transfer {
            to: test_addr(0x02),
            amount: 500,
        },
        400,
    ));
}

#[test]
fn multiple_keys_with_different_permissions() {
    let mut pset = PermissionSet::new(test_addr(0x01));

    let admin_key: [u8; 32] = [0x10; 32];
    let viewer_key: [u8; 32] = [0x20; 32];
    let transfer_key: [u8; 32] = [0x30; 32];

    pset.add_key(admin_key, "admin".into(), KeyPermission::FullAccess, 100);
    pset.add_key(viewer_key, "viewer".into(), KeyPermission::ViewOnly, 100);
    pset.add_key(
        transfer_key,
        "sender".into(),
        KeyPermission::TransferOnly {
            max_amount: Some(500),
            allowed_recipients: vec![],
        },
        100,
    );

    assert_eq!(pset.keys.len(), 3);

    // Admin can do everything
    assert!(pset.is_authorized(&admin_key, &Action::ManageKeys, 200));
    assert!(pset.is_authorized(
        &admin_key,
        &Action::Transfer {
            to: test_addr(0x02),
            amount: 99999,
        },
        200,
    ));

    // Viewer can only view
    assert!(pset.is_authorized(&viewer_key, &Action::ViewState, 200));
    assert!(!pset.is_authorized(
        &viewer_key,
        &Action::Transfer {
            to: test_addr(0x02),
            amount: 1,
        },
        200,
    ));

    // Transfer key respects limits
    assert!(pset.is_authorized(
        &transfer_key,
        &Action::Transfer {
            to: test_addr(0x02),
            amount: 500,
        },
        200,
    ));
    assert!(!pset.is_authorized(
        &transfer_key,
        &Action::Transfer {
            to: test_addr(0x02),
            amount: 501,
        },
        200,
    ));
}

#[test]
fn session_key_wrapping_contract_call_permission() {
    let contract = test_addr(0x10);
    let mut pset = PermissionSet::new(test_addr(0x01));
    let key: [u8; 32] = [0x40; 32];

    pset.add_key(
        key,
        "temp contract".into(),
        KeyPermission::SessionKey {
            expires_at: 5000,
            permissions: Box::new(KeyPermission::ContractCallOnly {
                allowed_contracts: vec![contract],
                allowed_methods: vec!["deposit".into(), "withdraw".into()],
            }),
        },
        1000,
    );

    // Valid: right contract, right method, before expiry
    assert!(pset.is_authorized(
        &key,
        &Action::ContractCall {
            contract,
            method: "deposit".into(),
        },
        3000,
    ));

    // Wrong method
    assert!(!pset.is_authorized(
        &key,
        &Action::ContractCall {
            contract,
            method: "selfDestruct".into(),
        },
        3000,
    ));

    // After expiry
    assert!(!pset.is_authorized(
        &key,
        &Action::ContractCall {
            contract,
            method: "deposit".into(),
        },
        6000,
    ));
}

#[test]
fn custom_permission_wildcard_allows_everything() {
    let mut pset = PermissionSet::new(test_addr(0x01));
    let key: [u8; 32] = [0x50; 32];

    pset.add_key(
        key,
        "wildcard".into(),
        KeyPermission::Custom {
            label: "superuser".into(),
            capabilities: vec!["*".into()],
        },
        1000,
    );

    assert!(pset.is_authorized(&key, &Action::ViewState, 1001));
    assert!(pset.is_authorized(&key, &Action::ManageKeys, 1001));
    assert!(pset.is_authorized(&key, &Action::EmergencyStop, 1001));
    assert!(pset.is_authorized(
        &key,
        &Action::Transfer {
            to: test_addr(0x02),
            amount: 999,
        },
        1001,
    ));
}

#[test]
fn permission_set_serialization_roundtrip() {
    let mut pset = PermissionSet::new(test_addr(0x01));
    let key: [u8; 32] = [0x60; 32];
    pset.add_key(
        key,
        "test".into(),
        KeyPermission::SessionKey {
            expires_at: 9999,
            permissions: Box::new(KeyPermission::TransferOnly {
                max_amount: Some(100),
                allowed_recipients: vec![test_addr(0x02)],
            }),
        },
        1000,
    );

    let json = serde_json::to_string(&pset).unwrap();
    let deserialized: PermissionSet = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.owner, pset.owner);
    assert_eq!(deserialized.keys.len(), 1);
    assert_eq!(deserialized.keys[0].label, "test");
}
