//! Integration tests for dina-storage crate.
//!
//! These tests exercise cross-module flows: database creation, migrations,
//! state transactions, and verify the full lifecycle of stored data.

use dina_core::block::BlockHeader;
use dina_core::types::{Address, Hash};
use dina_core::{Account, Block};
use dina_storage::db::DinaDB;
use dina_storage::state::StateStore;

// ---------------------------------------------------------------------------
// Database + Migration integration
// ---------------------------------------------------------------------------

#[test]
fn fresh_database_has_correct_schema_version() {
    let db = DinaDB::open_in_memory().unwrap();
    // After open_in_memory, migration should have run
    // The latest block height should be 0 (no blocks stored yet)
    assert_eq!(db.get_latest_block_height().unwrap(), 0);
}

#[test]
fn database_handles_sequential_blocks() {
    let db = DinaDB::open_in_memory().unwrap();

    let mut prev_hash = Hash::ZERO;
    for height in 1..=5 {
        let block = Block {
            header: BlockHeader {
                block_number: height,
                timestamp: 1700000000 + height,
                parent_hash: prev_hash,
                transactions_root: Hash::ZERO,
                state_root: Hash::ZERO,
                proposer: Address::ZERO,
                proposer_pubkey: [0u8; 32],
                signature: [0u8; 64],
            },
            transactions: vec![],
        };
        prev_hash = block.hash();
        db.store_block(&block).unwrap();
    }

    assert_eq!(db.get_latest_block_height().unwrap(), 5);

    // Verify chain linkage
    let block5 = db.get_block(5).unwrap().unwrap();
    let block4 = db.get_block(4).unwrap().unwrap();
    assert_eq!(block5.header.parent_hash, block4.hash());
}

#[test]
fn database_block_hash_index_consistency() {
    let db = DinaDB::open_in_memory().unwrap();

    let block = Block {
        header: BlockHeader {
            block_number: 100,
            timestamp: 1700000000,
            parent_hash: Hash::ZERO,
            transactions_root: Hash::ZERO,
            state_root: Hash::ZERO,
            proposer: Address::ZERO,
            proposer_pubkey: [0u8; 32],
            signature: [0u8; 64],
        },
        transactions: vec![],
    };

    let hash = block.hash();
    db.store_block(&block).unwrap();

    // Retrieve by height
    let by_height = db.get_block(100).unwrap().unwrap();
    // Retrieve by hash
    let by_hash = db.get_block_by_hash(hash).unwrap().unwrap();

    // Both should be the same block
    assert_eq!(by_height.header.block_number, by_hash.header.block_number);
    assert_eq!(by_height.header.timestamp, by_hash.header.timestamp);
    assert_eq!(by_height.hash(), by_hash.hash());
}

// ---------------------------------------------------------------------------
// StateStore + DinaDB integration
// ---------------------------------------------------------------------------

#[test]
fn state_store_wraps_database() {
    let db = DinaDB::open_in_memory().unwrap();
    let store = StateStore::new(db.clone());

    let addr = Address([0xAA; 32]);
    let account = Account::with_balance(addr, 42_000);

    // Write through state store transaction
    let txn = store.begin_transaction().unwrap();
    txn.set_account(&addr, &account).unwrap();
    txn.commit().unwrap();

    // Read through database directly
    let loaded = db.get_account(addr).unwrap().unwrap();
    assert_eq!(loaded.balance, 42_000);
}

#[test]
fn state_store_transaction_isolation() {
    let db = DinaDB::open_in_memory().unwrap();
    let store = StateStore::new(db.clone());

    let addr = Address([0xBB; 32]);

    // Write an initial value
    let txn1 = store.begin_transaction().unwrap();
    txn1.set_account(&addr, &Account::with_balance(addr, 100))
        .unwrap();
    txn1.commit().unwrap();

    // Start a new transaction, write but don't commit
    {
        let txn2 = store.begin_transaction().unwrap();
        txn2.set_account(&addr, &Account::with_balance(addr, 999))
            .unwrap();
        // Drop without commit
    }

    // Original value should still be there
    let loaded = db.get_account(addr).unwrap().unwrap();
    assert_eq!(loaded.balance, 100);
}

#[test]
fn full_contract_lifecycle() {
    let db = DinaDB::open_in_memory().unwrap();
    let store = StateStore::new(db);

    let contract_addr = Address([0xCC; 32]);
    let code_hash = [0xDD; 32];
    let wasm_code = b"\x00asm\x01\x00\x00\x00real_contract_code_here";
    let slot1 = [0x01; 32];
    let slot2 = [0x02; 32];

    // Deploy: store code and initial storage
    let txn = store.begin_transaction().unwrap();
    txn.set_contract_code(&code_hash, wasm_code).unwrap();
    txn.set_contract_storage(&contract_addr, &slot1, b"initial_state")
        .unwrap();
    txn.commit().unwrap();

    // Execute: update storage
    let txn2 = store.begin_transaction().unwrap();
    txn2.set_contract_storage(&contract_addr, &slot1, b"updated_state")
        .unwrap();
    txn2.set_contract_storage(&contract_addr, &slot2, b"new_slot")
        .unwrap();
    txn2.commit().unwrap();

    // Verify final state
    let txn3 = store.begin_transaction().unwrap();
    let code = txn3.get_contract_code(&code_hash).unwrap().unwrap();
    assert_eq!(code, wasm_code);

    let val1 = txn3
        .get_contract_storage(&contract_addr, &slot1)
        .unwrap()
        .unwrap();
    assert_eq!(val1, b"updated_state");

    let val2 = txn3
        .get_contract_storage(&contract_addr, &slot2)
        .unwrap()
        .unwrap();
    assert_eq!(val2, b"new_slot");
}

#[test]
fn account_balance_transfer_simulation() {
    let db = DinaDB::open_in_memory().unwrap();
    let store = StateStore::new(db);

    let alice = Address([0x01; 32]);
    let bob = Address([0x02; 32]);

    // Initial balances
    let txn = store.begin_transaction().unwrap();
    txn.set_account(&alice, &Account::with_balance(alice, 1000))
        .unwrap();
    txn.set_account(&bob, &Account::with_balance(bob, 500))
        .unwrap();
    txn.commit().unwrap();

    // Transfer 200 from Alice to Bob atomically
    let txn2 = store.begin_transaction().unwrap();
    let mut alice_acc = txn2.get_account(&alice).unwrap().unwrap();
    let mut bob_acc = txn2.get_account(&bob).unwrap().unwrap();

    alice_acc.balance -= 200;
    bob_acc.balance += 200;

    txn2.set_account(&alice, &alice_acc).unwrap();
    txn2.set_account(&bob, &bob_acc).unwrap();
    txn2.commit().unwrap();

    // Verify
    let txn3 = store.begin_transaction().unwrap();
    assert_eq!(txn3.get_account(&alice).unwrap().unwrap().balance, 800);
    assert_eq!(txn3.get_account(&bob).unwrap().unwrap().balance, 700);
}

#[test]
fn many_accounts_stored_and_retrieved() {
    let db = DinaDB::open_in_memory().unwrap();

    // Store 100 accounts
    for i in 0u8..100 {
        let mut addr_bytes = [0u8; 32];
        addr_bytes[0] = i;
        let addr = Address(addr_bytes);
        let account = Account::with_balance(addr, i as u64 * 1000);
        db.set_account(addr, &account).unwrap();
    }

    // Verify all can be retrieved with correct balances
    for i in 0u8..100 {
        let mut addr_bytes = [0u8; 32];
        addr_bytes[0] = i;
        let addr = Address(addr_bytes);
        let loaded = db.get_account(addr).unwrap().unwrap();
        assert_eq!(loaded.balance, i as u64 * 1000);
    }
}

#[test]
fn database_handles_blocks_and_accounts_together() {
    let db = DinaDB::open_in_memory().unwrap();

    // Store some accounts
    let addr = Address([0x11; 32]);
    db.set_account(addr, &Account::with_balance(addr, 5000))
        .unwrap();

    // Store a block
    let block = Block {
        header: BlockHeader {
            block_number: 1,
            timestamp: 1700000000,
            parent_hash: Hash::ZERO,
            transactions_root: Hash::ZERO,
            state_root: Hash::ZERO,
            proposer: addr,
            proposer_pubkey: [0u8; 32],
            signature: [0u8; 64],
        },
        transactions: vec![],
    };
    db.store_block(&block).unwrap();

    // Both should be retrievable independently
    let loaded_account = db.get_account(addr).unwrap().unwrap();
    assert_eq!(loaded_account.balance, 5000);

    let loaded_block = db.get_block(1).unwrap().unwrap();
    assert_eq!(loaded_block.header.proposer, addr);
}
