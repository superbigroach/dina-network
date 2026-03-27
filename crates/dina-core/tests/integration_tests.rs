//! Comprehensive integration tests for dina-core.
//!
//! These tests exercise the public API across module boundaries, verifying
//! that crypto, transactions, accounts, blocks, merkle trees, types, and
//! devices all work together correctly.

use std::str::FromStr;

use dina_core::account::{Account, AccountState};
use dina_core::block::{Block, BlockHeader};
use dina_core::crypto;
use dina_core::device::{DeviceIdentity, DeviceType};
use dina_core::error::DinaError;
use dina_core::merkle::{compute_merkle_root, MerkleTree};
use dina_core::transaction::{DeviceAttestation, Sig64, Transaction, WitnessProof};
use dina_core::types::{Address, Hash};

// ============================================================================
// Crypto tests
// ============================================================================

mod crypto_tests {
    use super::*;

    #[test]
    fn generate_keypair_produces_valid_keys() {
        let (sk, vk) = crypto::generate_keypair();
        // The verifying key derived from the signing key should match
        assert_eq!(sk.verifying_key(), vk);
    }

    #[test]
    fn generate_keypair_produces_unique_keys() {
        let (sk1, _) = crypto::generate_keypair();
        let (sk2, _) = crypto::generate_keypair();
        assert_ne!(sk1.to_bytes(), sk2.to_bytes());
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let (sk, vk) = crypto::generate_keypair();
        let message = b"hello dina network";
        let sig = crypto::sign(&sk, message);
        assert!(crypto::verify(&vk, message, &sig));
    }

    #[test]
    fn sign_and_verify_empty_message() {
        let (sk, vk) = crypto::generate_keypair();
        let sig = crypto::sign(&sk, b"");
        assert!(crypto::verify(&vk, b"", &sig));
    }

    #[test]
    fn sign_and_verify_large_message() {
        let (sk, vk) = crypto::generate_keypair();
        let large_msg = vec![0xAB_u8; 100_000];
        let sig = crypto::sign(&sk, &large_msg);
        assert!(crypto::verify(&vk, &large_msg, &sig));
    }

    #[test]
    fn verify_fails_with_wrong_key() {
        let (sk, _vk) = crypto::generate_keypair();
        let (_, wrong_vk) = crypto::generate_keypair();
        let message = b"secret message";
        let sig = crypto::sign(&sk, message);
        assert!(!crypto::verify(&wrong_vk, message, &sig));
    }

    #[test]
    fn verify_fails_with_tampered_message() {
        let (sk, vk) = crypto::generate_keypair();
        let sig = crypto::sign(&sk, b"original message");
        assert!(!crypto::verify(&vk, b"tampered message", &sig));
    }

    #[test]
    fn verify_fails_with_tampered_signature() {
        let (sk, vk) = crypto::generate_keypair();
        let message = b"test message";
        let mut sig = crypto::sign(&sk, message);
        // Flip a byte in the signature
        sig[0] ^= 0xFF;
        assert!(!crypto::verify(&vk, message, &sig));
    }

    #[test]
    fn address_from_pubkey_is_deterministic() {
        let (_, vk) = crypto::generate_keypair();
        let addr1 = crypto::address_from_pubkey(&vk);
        let addr2 = crypto::address_from_pubkey(&vk);
        assert_eq!(addr1, addr2);
    }

    #[test]
    fn address_from_pubkey_differs_for_different_keys() {
        let (_, vk1) = crypto::generate_keypair();
        let (_, vk2) = crypto::generate_keypair();
        let addr1 = crypto::address_from_pubkey(&vk1);
        let addr2 = crypto::address_from_pubkey(&vk2);
        assert_ne!(addr1, addr2);
    }

    #[test]
    fn hash_bytes_is_deterministic() {
        let data = b"deterministic hashing test";
        let h1 = crypto::hash_bytes(data);
        let h2 = crypto::hash_bytes(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_bytes_different_inputs_produce_different_hashes() {
        let h1 = crypto::hash_bytes(b"input one");
        let h2 = crypto::hash_bytes(b"input two");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_bytes_empty_input() {
        let h = crypto::hash_bytes(b"");
        assert_ne!(h, Hash::ZERO);
        // SHA-256 of empty string is a well-known value
        let h2 = crypto::hash_bytes(b"");
        assert_eq!(h, h2);
    }
}

// ============================================================================
// Transaction tests
// ============================================================================

mod transaction_tests {
    use super::*;

    /// Helper: build and sign a Transfer transaction.
    fn make_transfer(
        sk: &ed25519_dalek::SigningKey,
        to: Address,
        amount: u64,
        nonce: u64,
        fee: u64,
    ) -> Transaction {
        let vk = sk.verifying_key();
        let from = Address::from_pubkey(&vk);

        let mut tx = Transaction::Transfer {
            from,
            to,
            amount,
            memo: None,
            device_witness: None,
            nonce,
            fee,
            signature: Sig64([0u8; 64]),
        };

        let msg = tx.signing_bytes();
        let sig = crypto::sign(sk, &msg);
        if let Transaction::Transfer { ref mut signature, .. } = tx {
            *signature = Sig64(sig);
        }
        tx
    }

    /// Helper: build and sign a DeployContract transaction.
    fn make_deploy(
        sk: &ed25519_dalek::SigningKey,
        wasm: Vec<u8>,
        nonce: u64,
        fee: u64,
    ) -> Transaction {
        let vk = sk.verifying_key();
        let from = Address::from_pubkey(&vk);

        let mut tx = Transaction::DeployContract {
            from,
            wasm_bytecode: wasm,
            init_args: vec![],
            nonce,
            fee,
            signature: Sig64([0u8; 64]),
        };

        let msg = tx.signing_bytes();
        let sig = crypto::sign(sk, &msg);
        if let Transaction::DeployContract { ref mut signature, .. } = tx {
            *signature = Sig64(sig);
        }
        tx
    }

    /// Helper: build and sign a CallContract transaction.
    fn make_call(
        sk: &ed25519_dalek::SigningKey,
        contract: Address,
        method: &str,
        nonce: u64,
        fee: u64,
    ) -> Transaction {
        let vk = sk.verifying_key();
        let from = Address::from_pubkey(&vk);

        let mut tx = Transaction::CallContract {
            from,
            contract,
            method: method.to_string(),
            args: vec![1, 2, 3],
            usdc_attached: 500,
            nonce,
            fee,
            signature: Sig64([0u8; 64]),
        };

        let msg = tx.signing_bytes();
        let sig = crypto::sign(sk, &msg);
        if let Transaction::CallContract { ref mut signature, .. } = tx {
            *signature = Sig64(sig);
        }
        tx
    }

    /// Helper: build and sign a RegisterDevice transaction.
    fn make_register_device(
        sk: &ed25519_dalek::SigningKey,
        device_pubkey: [u8; 32],
        nonce: u64,
        fee: u64,
    ) -> Transaction {
        let vk = sk.verifying_key();
        let owner = Address::from_pubkey(&vk);

        let attestation = DeviceAttestation {
            pubkey: device_pubkey,
            firmware_hash: Hash([0xAA; 32]),
            witness_root: Hash::ZERO,
            timestamp: 1_700_000_000,
            signature: Sig64([0u8; 64]),
        };

        let mut tx = Transaction::RegisterDevice {
            device_pubkey,
            owner,
            attestation,
            nonce,
            fee,
            signature: Sig64([0u8; 64]),
        };

        let msg = tx.signing_bytes();
        let sig = crypto::sign(sk, &msg);
        if let Transaction::RegisterDevice { ref mut signature, .. } = tx {
            *signature = Sig64(sig);
        }
        tx
    }

    // --- Transfer variant tests ---

    #[test]
    fn transfer_hash_is_deterministic() {
        let (sk, _) = crypto::generate_keypair();
        let tx = make_transfer(&sk, Address([0xBB; 32]), 1000, 0, 10);
        let h1 = tx.hash();
        let h2 = tx.hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn transfer_signing_bytes_consistent() {
        let (sk, _) = crypto::generate_keypair();
        let tx = make_transfer(&sk, Address([0xBB; 32]), 1000, 0, 10);
        let b1 = tx.signing_bytes();
        let b2 = tx.signing_bytes();
        assert_eq!(b1, b2);
    }

    #[test]
    fn transfer_verify_signature_correct_signer() {
        let (sk, vk) = crypto::generate_keypair();
        let tx = make_transfer(&sk, Address([0xBB; 32]), 1000, 0, 10);
        assert!(tx.verify_signature(&vk));
    }

    #[test]
    fn transfer_verify_signature_wrong_signer() {
        let (sk, _) = crypto::generate_keypair();
        let (_, wrong_vk) = crypto::generate_keypair();
        let tx = make_transfer(&sk, Address([0xBB; 32]), 1000, 0, 10);
        assert!(!tx.verify_signature(&wrong_vk));
    }

    #[test]
    fn transfer_sender_returns_correct_address() {
        let (sk, vk) = crypto::generate_keypair();
        let expected_addr = Address::from_pubkey(&vk);
        let tx = make_transfer(&sk, Address([0xBB; 32]), 1000, 0, 10);
        assert_eq!(tx.sender(), expected_addr);
    }

    #[test]
    fn transfer_nonce_and_fee() {
        let (sk, _) = crypto::generate_keypair();
        let tx = make_transfer(&sk, Address([0xBB; 32]), 5000, 42, 100);
        assert_eq!(tx.nonce(), 42);
        assert_eq!(tx.fee(), 100);
    }

    // --- DeployContract variant tests ---

    #[test]
    fn deploy_contract_roundtrip() {
        let (sk, vk) = crypto::generate_keypair();
        let tx = make_deploy(&sk, vec![0x00, 0x61, 0x73, 0x6D], 1, 50);
        assert!(tx.verify_signature(&vk));
        assert_eq!(tx.sender(), Address::from_pubkey(&vk));
        assert_eq!(tx.nonce(), 1);
        assert_eq!(tx.fee(), 50);
    }

    #[test]
    fn deploy_hash_deterministic() {
        let (sk, _) = crypto::generate_keypair();
        let tx = make_deploy(&sk, vec![0x00, 0x61], 0, 10);
        assert_eq!(tx.hash(), tx.hash());
    }

    #[test]
    fn deploy_wrong_key_rejects() {
        let (sk, _) = crypto::generate_keypair();
        let (_, wrong_vk) = crypto::generate_keypair();
        let tx = make_deploy(&sk, vec![0xFF], 0, 10);
        assert!(!tx.verify_signature(&wrong_vk));
    }

    // --- CallContract variant tests ---

    #[test]
    fn call_contract_roundtrip() {
        let (sk, vk) = crypto::generate_keypair();
        let contract_addr = Address([0xCC; 32]);
        let tx = make_call(&sk, contract_addr, "transfer", 5, 25);
        assert!(tx.verify_signature(&vk));
        assert_eq!(tx.sender(), Address::from_pubkey(&vk));
        assert_eq!(tx.nonce(), 5);
        assert_eq!(tx.fee(), 25);
    }

    #[test]
    fn call_hash_deterministic() {
        let (sk, _) = crypto::generate_keypair();
        let tx = make_call(&sk, Address([0xDD; 32]), "mint", 0, 10);
        assert_eq!(tx.hash(), tx.hash());
    }

    #[test]
    fn call_wrong_key_rejects() {
        let (sk, _) = crypto::generate_keypair();
        let (_, wrong_vk) = crypto::generate_keypair();
        let tx = make_call(&sk, Address([0xDD; 32]), "burn", 0, 10);
        assert!(!tx.verify_signature(&wrong_vk));
    }

    // --- RegisterDevice variant tests ---

    #[test]
    fn register_device_roundtrip() {
        let (sk, vk) = crypto::generate_keypair();
        let device_pk = [0x42; 32];
        let tx = make_register_device(&sk, device_pk, 3, 15);
        assert!(tx.verify_signature(&vk));
        // For RegisterDevice, sender() returns `owner`
        assert_eq!(tx.sender(), Address::from_pubkey(&vk));
        assert_eq!(tx.nonce(), 3);
        assert_eq!(tx.fee(), 15);
    }

    #[test]
    fn register_device_hash_deterministic() {
        let (sk, _) = crypto::generate_keypair();
        let tx = make_register_device(&sk, [0x99; 32], 0, 5);
        assert_eq!(tx.hash(), tx.hash());
    }

    #[test]
    fn register_device_wrong_key_rejects() {
        let (sk, _) = crypto::generate_keypair();
        let (_, wrong_vk) = crypto::generate_keypair();
        let tx = make_register_device(&sk, [0x11; 32], 0, 5);
        assert!(!tx.verify_signature(&wrong_vk));
    }

    // --- Transfer with memo and witness ---

    #[test]
    fn transfer_with_memo() {
        let (sk, vk) = crypto::generate_keypair();
        let from = Address::from_pubkey(&vk);
        let to = Address([0xBB; 32]);

        let mut tx = Transaction::Transfer {
            from,
            to,
            amount: 500,
            memo: Some(b"payment for services".to_vec()),
            device_witness: None,
            nonce: 0,
            fee: 10,
            signature: Sig64([0u8; 64]),
        };

        let msg = tx.signing_bytes();
        let sig = crypto::sign(&sk, &msg);
        if let Transaction::Transfer { ref mut signature, .. } = tx {
            *signature = Sig64(sig);
        }

        assert!(tx.verify_signature(&vk));
    }

    #[test]
    fn transfer_with_witness_proof() {
        let (sk, vk) = crypto::generate_keypair();
        let from = Address::from_pubkey(&vk);
        let to = Address([0xBB; 32]);

        let witness = WitnessProof {
            witness_hash: Hash([0xEE; 32]),
            device_signature: Sig64([0x11; 64]),
        };

        let mut tx = Transaction::Transfer {
            from,
            to,
            amount: 250,
            memo: None,
            device_witness: Some(witness),
            nonce: 7,
            fee: 20,
            signature: Sig64([0u8; 64]),
        };

        let msg = tx.signing_bytes();
        let sig = crypto::sign(&sk, &msg);
        if let Transaction::Transfer { ref mut signature, .. } = tx {
            *signature = Sig64(sig);
        }

        assert!(tx.verify_signature(&vk));
        assert_eq!(tx.nonce(), 7);
        assert_eq!(tx.fee(), 20);
    }

    // --- Different transactions produce different hashes ---

    #[test]
    fn different_transactions_different_hashes() {
        let (sk, _) = crypto::generate_keypair();
        let tx1 = make_transfer(&sk, Address([0xAA; 32]), 100, 0, 10);
        let tx2 = make_transfer(&sk, Address([0xBB; 32]), 200, 1, 20);
        assert_ne!(tx1.hash(), tx2.hash());
    }

    // --- Sig64 conversion ---

    #[test]
    fn sig64_from_and_into_array() {
        let arr = [0x42u8; 64];
        let sig = Sig64::from(arr);
        let back: [u8; 64] = sig.into();
        assert_eq!(arr, back);
    }
}

// ============================================================================
// Account tests
// ============================================================================

mod account_tests {
    use super::*;

    fn addr(byte: u8) -> Address {
        Address([byte; 32])
    }

    #[test]
    fn create_account_with_balance() {
        let a = addr(1);
        let account = Account::with_balance(a, 5000);
        assert_eq!(account.balance, 5000);
        assert_eq!(account.nonce, 0);
        assert_eq!(account.address, a);
        assert!(account.code_hash.is_none());
        assert!(account.storage_root.is_none());
    }

    #[test]
    fn create_account_zero_balance() {
        let a = addr(2);
        let account = Account::new(a);
        assert_eq!(account.balance, 0);
        assert_eq!(account.nonce, 0);
    }

    #[test]
    fn transfer_succeeds_with_sufficient_balance() {
        let mut state = AccountState::new();
        let a = addr(1);
        let b = addr(2);
        state.credit(&a, 1000);
        assert!(state.transfer(&a, &b, 500).is_ok());
        assert_eq!(state.get_account(&a).unwrap().balance, 500);
        assert_eq!(state.get_account(&b).unwrap().balance, 500);
    }

    #[test]
    fn transfer_fails_with_insufficient_balance() {
        let mut state = AccountState::new();
        let a = addr(1);
        let b = addr(2);
        state.credit(&a, 100);
        let result = state.transfer(&a, &b, 200);
        assert!(result.is_err());
        match result.unwrap_err() {
            DinaError::InsufficientBalance { have, need } => {
                assert_eq!(have, 100);
                assert_eq!(need, 200);
            }
            other => panic!("Expected InsufficientBalance, got: {other}"),
        }
    }

    #[test]
    fn transfer_exact_balance() {
        let mut state = AccountState::new();
        let a = addr(1);
        let b = addr(2);
        state.credit(&a, 1000);
        assert!(state.transfer(&a, &b, 1000).is_ok());
        assert_eq!(state.get_account(&a).unwrap().balance, 0);
        assert_eq!(state.get_account(&b).unwrap().balance, 1000);
    }

    #[test]
    fn transfer_zero_amount() {
        let mut state = AccountState::new();
        let a = addr(1);
        let b = addr(2);
        state.credit(&a, 1000);
        assert!(state.transfer(&a, &b, 0).is_ok());
        assert_eq!(state.get_account(&a).unwrap().balance, 1000);
        assert_eq!(state.get_account(&b).unwrap().balance, 0);
    }

    #[test]
    fn transfer_from_nonexistent_account() {
        let mut state = AccountState::new();
        let a = addr(1);
        let b = addr(2);
        let result = state.transfer(&a, &b, 100);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DinaError::AccountNotFound(_)));
    }

    #[test]
    fn transfer_creates_receiver_account() {
        let mut state = AccountState::new();
        let a = addr(1);
        let b = addr(2);
        state.credit(&a, 1000);
        assert!(state.get_account(&b).is_none());
        state.transfer(&a, &b, 300).unwrap();
        assert!(state.get_account(&b).is_some());
        assert_eq!(state.get_account(&b).unwrap().balance, 300);
    }

    #[test]
    fn deduct_fee_works_correctly() {
        let mut state = AccountState::new();
        let a = addr(1);
        state.credit(&a, 1000);
        state.deduct_fee(&a, 50).unwrap();
        assert_eq!(state.get_account(&a).unwrap().balance, 950);
    }

    #[test]
    fn deduct_fee_fails_insufficient_balance() {
        let mut state = AccountState::new();
        let a = addr(1);
        state.credit(&a, 30);
        let result = state.deduct_fee(&a, 50);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DinaError::InsufficientBalance { have: 30, need: 50 }
        ));
    }

    #[test]
    fn deduct_fee_nonexistent_account() {
        let mut state = AccountState::new();
        let a = addr(99);
        assert!(matches!(
            state.deduct_fee(&a, 10).unwrap_err(),
            DinaError::AccountNotFound(_)
        ));
    }

    #[test]
    fn increment_nonce_increments() {
        let mut state = AccountState::new();
        let a = addr(1);
        state.credit(&a, 100);
        assert_eq!(state.get_account(&a).unwrap().nonce, 0);
        state.increment_nonce(&a).unwrap();
        assert_eq!(state.get_account(&a).unwrap().nonce, 1);
        state.increment_nonce(&a).unwrap();
        assert_eq!(state.get_account(&a).unwrap().nonce, 2);
        state.increment_nonce(&a).unwrap();
        assert_eq!(state.get_account(&a).unwrap().nonce, 3);
    }

    #[test]
    fn increment_nonce_nonexistent_account() {
        let mut state = AccountState::new();
        let a = addr(99);
        assert!(state.increment_nonce(&a).is_err());
    }

    #[test]
    fn credit_adds_to_balance() {
        let mut state = AccountState::new();
        let a = addr(1);
        state.credit(&a, 100);
        state.credit(&a, 200);
        state.credit(&a, 300);
        assert_eq!(state.get_account(&a).unwrap().balance, 600);
    }

    #[test]
    fn credit_creates_account_if_not_exists() {
        let mut state = AccountState::new();
        let a = addr(5);
        assert!(state.get_account(&a).is_none());
        state.credit(&a, 42);
        assert!(state.get_account(&a).is_some());
        assert_eq!(state.get_account(&a).unwrap().balance, 42);
    }

    #[test]
    fn account_state_len_and_is_empty() {
        let mut state = AccountState::new();
        assert!(state.is_empty());
        assert_eq!(state.len(), 0);

        state.credit(&addr(1), 100);
        assert!(!state.is_empty());
        assert_eq!(state.len(), 1);

        state.credit(&addr(2), 200);
        assert_eq!(state.len(), 2);
    }

    #[test]
    fn account_state_iter() {
        let mut state = AccountState::new();
        state.credit(&addr(1), 100);
        state.credit(&addr(2), 200);

        let total: u64 = state.iter().map(|(_, acc)| acc.balance).sum();
        assert_eq!(total, 300);
    }

    #[test]
    fn multiple_transfers_chain() {
        let mut state = AccountState::new();
        let a = addr(1);
        let b = addr(2);
        let c = addr(3);

        state.credit(&a, 1000);
        state.transfer(&a, &b, 400).unwrap();
        state.transfer(&b, &c, 200).unwrap();

        assert_eq!(state.get_account(&a).unwrap().balance, 600);
        assert_eq!(state.get_account(&b).unwrap().balance, 200);
        assert_eq!(state.get_account(&c).unwrap().balance, 200);
    }
}

// ============================================================================
// Block tests
// ============================================================================

mod block_tests {
    use super::*;

    #[test]
    fn genesis_block_creation() {
        let proposer = Address([0x01; 32]);
        let genesis = Block::genesis(proposer, 1_700_000_000);
        assert_eq!(genesis.header.block_number, 0);
        assert_eq!(genesis.header.parent_hash, Hash::ZERO);
        assert_eq!(genesis.header.state_root, Hash::ZERO);
        assert_eq!(genesis.header.transactions_root, Hash::ZERO);
        assert_eq!(genesis.header.timestamp, 1_700_000_000);
        assert_eq!(genesis.header.proposer, proposer);
        assert_eq!(genesis.transaction_count(), 0);
        assert!(genesis.transactions.is_empty());
    }

    #[test]
    fn block_hash_is_deterministic() {
        let genesis = Block::genesis(Address::ZERO, 1_700_000_000);
        let h1 = genesis.hash();
        let h2 = genesis.hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_blocks_different_hashes() {
        let b1 = Block::genesis(Address([0x01; 32]), 1_000);
        let b2 = Block::genesis(Address([0x02; 32]), 2_000);
        assert_ne!(b1.hash(), b2.hash());
    }

    #[test]
    fn block_contains_correct_transaction_count() {
        let genesis = Block::genesis(Address::ZERO, 0);
        assert_eq!(genesis.transaction_count(), 0);
    }

    #[test]
    fn block_with_transactions_count() {
        let (sk, vk) = crypto::generate_keypair();
        let from = Address::from_pubkey(&vk);

        // Create some transactions
        let mut txs = Vec::new();
        for i in 0..5 {
            let mut tx = Transaction::Transfer {
                from,
                to: Address([0xBB; 32]),
                amount: 100 * (i + 1),
                memo: None,
                device_witness: None,
                nonce: i,
                fee: 10,
                signature: Sig64([0u8; 64]),
            };
            let msg = tx.signing_bytes();
            let sig = crypto::sign(&sk, &msg);
            if let Transaction::Transfer { ref mut signature, .. } = tx {
                *signature = Sig64(sig);
            }
            txs.push(tx);
        }

        let block = Block {
            header: BlockHeader {
                block_number: 1,
                parent_hash: Hash::ZERO,
                state_root: Hash::ZERO,
                transactions_root: Hash::ZERO,
                timestamp: 1_700_000_000,
                proposer: from,
                signature: [0u8; 64],
            },
            transactions: txs,
        };

        assert_eq!(block.transaction_count(), 5);
    }

    #[test]
    fn compute_transactions_root_empty_block() {
        let genesis = Block::genesis(Address::ZERO, 0);
        assert_eq!(genesis.compute_transactions_root(), Hash::ZERO);
    }

    #[test]
    fn compute_transactions_root_consistent() {
        let (sk, vk) = crypto::generate_keypair();
        let from = Address::from_pubkey(&vk);

        let mut tx = Transaction::Transfer {
            from,
            to: Address([0xBB; 32]),
            amount: 100,
            memo: None,
            device_witness: None,
            nonce: 0,
            fee: 10,
            signature: Sig64([0u8; 64]),
        };
        let msg = tx.signing_bytes();
        let sig = crypto::sign(&sk, &msg);
        if let Transaction::Transfer { ref mut signature, .. } = tx {
            *signature = Sig64(sig);
        }

        let block = Block {
            header: BlockHeader {
                block_number: 1,
                parent_hash: Hash::ZERO,
                state_root: Hash::ZERO,
                transactions_root: Hash::ZERO,
                timestamp: 1_700_000_000,
                proposer: from,
                signature: [0u8; 64],
            },
            transactions: vec![tx],
        };

        let root1 = block.compute_transactions_root();
        let root2 = block.compute_transactions_root();
        assert_eq!(root1, root2);
        assert_ne!(root1, Hash::ZERO);
    }

    #[test]
    fn signed_genesis_verifies() {
        let (sk, vk) = crypto::generate_keypair();
        let genesis = Block::signed_genesis(&sk, 1_700_000_000);
        assert!(genesis.verify(&vk));
    }

    #[test]
    fn signed_genesis_wrong_key_fails() {
        let (sk, _) = crypto::generate_keypair();
        let (_, wrong_vk) = crypto::generate_keypair();
        let genesis = Block::signed_genesis(&sk, 1_700_000_000);
        assert!(!genesis.verify(&wrong_vk));
    }

    #[test]
    fn signed_genesis_has_correct_proposer() {
        let (sk, vk) = crypto::generate_keypair();
        let genesis = Block::signed_genesis(&sk, 1_700_000_000);
        assert_eq!(genesis.header.proposer, Address::from_pubkey(&vk));
    }

    #[test]
    fn block_header_hash_deterministic() {
        let header = BlockHeader {
            block_number: 42,
            parent_hash: Hash([0x11; 32]),
            state_root: Hash([0x22; 32]),
            transactions_root: Hash([0x33; 32]),
            timestamp: 9999,
            proposer: Address([0x44; 32]),
            signature: [0u8; 64],
        };
        assert_eq!(header.hash(), header.hash());
    }
}

// ============================================================================
// Merkle tree tests
// ============================================================================

mod merkle_tests {
    use super::*;

    #[test]
    fn empty_tree_returns_zero_root() {
        let tree = MerkleTree::new();
        assert_eq!(tree.root(), Hash::ZERO);
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
    }

    #[test]
    fn single_item_tree() {
        let mut tree = MerkleTree::new();
        tree.insert(&[0xAA; 32]);
        let root = tree.root();
        assert_ne!(root, Hash::ZERO);
        assert_eq!(tree.len(), 1);
        assert!(!tree.is_empty());
    }

    #[test]
    fn multiple_items_deterministic_root() {
        let mut t1 = MerkleTree::new();
        let mut t2 = MerkleTree::new();

        let items: Vec<[u8; 32]> = (0..10).map(|i| [i as u8; 32]).collect();
        for item in &items {
            t1.insert(item);
            t2.insert(item);
        }

        assert_eq!(t1.root(), t2.root());
    }

    #[test]
    fn same_items_same_order_same_root() {
        let mut t1 = MerkleTree::new();
        let mut t2 = MerkleTree::new();

        t1.insert(&[1; 32]);
        t1.insert(&[2; 32]);
        t1.insert(&[3; 32]);

        t2.insert(&[1; 32]);
        t2.insert(&[2; 32]);
        t2.insert(&[3; 32]);

        assert_eq!(t1.root(), t2.root());
    }

    #[test]
    fn different_order_different_root() {
        let mut t1 = MerkleTree::new();
        let mut t2 = MerkleTree::new();

        t1.insert(&[1; 32]);
        t1.insert(&[2; 32]);

        t2.insert(&[2; 32]);
        t2.insert(&[1; 32]);

        assert_ne!(t1.root(), t2.root());
    }

    #[test]
    fn different_items_different_root() {
        let mut t1 = MerkleTree::new();
        let mut t2 = MerkleTree::new();

        t1.insert(&[0xAA; 32]);
        t2.insert(&[0xBB; 32]);

        assert_ne!(t1.root(), t2.root());
    }

    #[test]
    fn proof_and_verify_roundtrip() {
        let mut tree = MerkleTree::new();
        let leaves: Vec<[u8; 32]> = (0..5).map(|i| [i as u8; 32]).collect();
        for leaf in &leaves {
            tree.insert(leaf);
        }

        let root = tree.root();

        // Verify proof for each leaf
        for (i, leaf) in leaves.iter().enumerate() {
            let proof_bytes = tree.proof(i).expect("proof should exist");
            assert!(MerkleTree::verify_proof(&root, leaf, i, 5, &proof_bytes));
        }
    }

    #[test]
    fn proof_out_of_bounds_returns_none() {
        let mut tree = MerkleTree::new();
        tree.insert(&[0xAA; 32]);
        assert!(tree.proof(5).is_none());
    }

    #[test]
    fn verify_proof_wrong_leaf_fails() {
        let mut tree = MerkleTree::new();
        tree.insert(&[0x11; 32]);
        tree.insert(&[0x22; 32]);

        let root = tree.root();
        let proof = tree.proof(0).unwrap();

        // Try to verify with wrong leaf data
        assert!(!MerkleTree::verify_proof(
            &root,
            &[0xFF; 32],
            0,
            2,
            &proof
        ));
    }

    #[test]
    fn compute_merkle_root_empty() {
        assert_eq!(compute_merkle_root(&[]), Hash::ZERO);
    }

    #[test]
    fn compute_merkle_root_deterministic() {
        let data: Vec<&[u8]> = vec![&[1u8; 32], &[2u8; 32], &[3u8; 32]];
        let r1 = compute_merkle_root(&data);
        let r2 = compute_merkle_root(&data);
        assert_eq!(r1, r2);
        assert_ne!(r1, Hash::ZERO);
    }

    #[test]
    fn default_tree_is_empty() {
        let tree = MerkleTree::default();
        assert!(tree.is_empty());
        assert_eq!(tree.root(), Hash::ZERO);
    }
}

// ============================================================================
// Type tests
// ============================================================================

mod type_tests {
    use super::*;

    #[test]
    fn address_from_to_hex_roundtrip() {
        let original = Address([0xAB; 32]);
        let hex_str = original.to_string();
        let parsed = Address::from_str(&hex_str).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn address_from_hex_without_prefix() {
        let original = Address([0xCD; 32]);
        let hex_str = hex::encode(original.0);
        let parsed = Address::from_str(&hex_str).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn address_from_hex_with_0x_prefix() {
        let original = Address([0xEF; 32]);
        let hex_str = format!("0x{}", hex::encode(original.0));
        let parsed = Address::from_str(&hex_str).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn address_from_invalid_hex_fails() {
        let result = Address::from_str("0xGGGG");
        assert!(result.is_err());
    }

    #[test]
    fn address_from_wrong_length_fails() {
        // 16 bytes instead of 32
        let short_hex = hex::encode([0xAA; 16]);
        let result = Address::from_str(&short_hex);
        assert!(result.is_err());
    }

    #[test]
    fn hash_from_to_hex_roundtrip() {
        let original = Hash([0x42; 32]);
        let hex_str = original.to_string();
        let parsed = Hash::from_str(&hex_str).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn hash_from_hex_without_prefix() {
        let original = Hash([0x99; 32]);
        let hex_str = hex::encode(original.0);
        let parsed = Hash::from_str(&hex_str).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn hash_from_invalid_hex_fails() {
        let result = Hash::from_str("not_valid_hex");
        assert!(result.is_err());
    }

    #[test]
    fn hash_from_wrong_length_fails() {
        let short_hex = hex::encode([0xBB; 16]);
        let result = Hash::from_str(&short_hex);
        assert!(result.is_err());
    }

    #[test]
    fn address_display_format() {
        let addr = Address([0x00; 32]);
        let display = format!("{}", addr);
        assert!(display.starts_with("0x"));
        assert_eq!(display.len(), 66); // "0x" + 64 hex chars
    }

    #[test]
    fn address_debug_format() {
        let addr = Address([0xFF; 32]);
        let debug = format!("{:?}", addr);
        assert!(debug.starts_with("Address(0x"));
    }

    #[test]
    fn hash_display_format() {
        let hash = Hash([0x00; 32]);
        let display = format!("{}", hash);
        assert!(display.starts_with("0x"));
        assert_eq!(display.len(), 66); // "0x" + 64 hex chars
    }

    #[test]
    fn hash_debug_format() {
        let hash = Hash([0xFF; 32]);
        let debug = format!("{:?}", hash);
        assert!(debug.starts_with("Hash(0x"));
        // Debug format truncates to 16 hex chars
    }

    #[test]
    fn address_zero_constant() {
        assert_eq!(Address::ZERO.0, [0u8; 32]);
    }

    #[test]
    fn hash_zero_constant() {
        assert_eq!(Hash::ZERO.0, [0u8; 32]);
    }

    #[test]
    fn address_as_bytes() {
        let addr = Address([0x42; 32]);
        assert_eq!(addr.as_bytes(), &[0x42; 32]);
    }

    #[test]
    fn hash_as_bytes() {
        let hash = Hash([0xAB; 32]);
        assert_eq!(hash.as_bytes(), &[0xAB; 32]);
    }

    #[test]
    fn address_equality() {
        let a1 = Address([0x11; 32]);
        let a2 = Address([0x11; 32]);
        let a3 = Address([0x22; 32]);
        assert_eq!(a1, a2);
        assert_ne!(a1, a3);
    }

    #[test]
    fn hash_equality() {
        let h1 = Hash([0x11; 32]);
        let h2 = Hash([0x11; 32]);
        let h3 = Hash([0x22; 32]);
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn address_from_pubkey_matches_crypto_module() {
        let (_, vk) = crypto::generate_keypair();
        let addr_from_types = Address::from_pubkey(&vk);
        let addr_from_crypto = crypto::address_from_pubkey(&vk);
        assert_eq!(addr_from_types, addr_from_crypto);
    }
}

// ============================================================================
// Device tests
// ============================================================================

mod device_tests {
    use super::*;

    #[test]
    fn device_identity_creation() {
        let pubkey = [0x42; 32];
        let owner = Address([0x01; 32]);
        let firmware_hash = Hash([0xAA; 32]);
        let witness_root = Hash::ZERO;

        let device = DeviceIdentity::new(
            pubkey,
            owner,
            DeviceType::CognitumSeed,
            firmware_hash,
            witness_root,
            1_700_000_000,
        );

        assert!(device.active);
        assert_eq!(device.pubkey, pubkey);
        assert_eq!(device.owner, owner);
        assert_eq!(device.device_type, DeviceType::CognitumSeed);
        assert_eq!(device.firmware_hash, firmware_hash);
        assert_eq!(device.witness_root, witness_root);
        assert_eq!(device.registered_at, 1_700_000_000);
        // id should be deterministic (hash of pubkey)
        assert_ne!(device.id, Address::ZERO);
    }

    #[test]
    fn device_identity_id_is_deterministic() {
        let pubkey = [0x42; 32];
        let d1 = DeviceIdentity::new(
            pubkey,
            Address::ZERO,
            DeviceType::IoTSensor,
            Hash::ZERO,
            Hash::ZERO,
            0,
        );
        let d2 = DeviceIdentity::new(
            pubkey,
            Address::ZERO,
            DeviceType::IoTSensor,
            Hash::ZERO,
            Hash::ZERO,
            0,
        );
        assert_eq!(d1.id, d2.id);
    }

    #[test]
    fn device_identity_different_pubkeys_different_ids() {
        let d1 = DeviceIdentity::new(
            [0x01; 32],
            Address::ZERO,
            DeviceType::Robot,
            Hash::ZERO,
            Hash::ZERO,
            0,
        );
        let d2 = DeviceIdentity::new(
            [0x02; 32],
            Address::ZERO,
            DeviceType::Robot,
            Hash::ZERO,
            Hash::ZERO,
            0,
        );
        assert_ne!(d1.id, d2.id);
    }

    #[test]
    fn device_deactivate() {
        let mut device = DeviceIdentity::new(
            [0x01; 32],
            Address::ZERO,
            DeviceType::Drone,
            Hash::ZERO,
            Hash::ZERO,
            0,
        );
        assert!(device.active);
        device.deactivate();
        assert!(!device.active);
    }

    #[test]
    fn device_update_firmware() {
        let mut device = DeviceIdentity::new(
            [0x01; 32],
            Address::ZERO,
            DeviceType::CognitumAppliance,
            Hash([0xAA; 32]),
            Hash::ZERO,
            0,
        );
        assert_eq!(device.firmware_hash, Hash([0xAA; 32]));
        let new_hash = Hash([0xBB; 32]);
        device.update_firmware(new_hash);
        assert_eq!(device.firmware_hash, new_hash);
    }

    #[test]
    fn device_type_cognitum_seed_display() {
        assert_eq!(DeviceType::CognitumSeed.to_string(), "CognitumSeed");
    }

    #[test]
    fn device_type_cognitum_appliance_display() {
        assert_eq!(
            DeviceType::CognitumAppliance.to_string(),
            "CognitumAppliance"
        );
    }

    #[test]
    fn device_type_robot_display() {
        assert_eq!(DeviceType::Robot.to_string(), "Robot");
    }

    #[test]
    fn device_type_drone_display() {
        assert_eq!(DeviceType::Drone.to_string(), "Drone");
    }

    #[test]
    fn device_type_iot_sensor_display() {
        assert_eq!(DeviceType::IoTSensor.to_string(), "IoTSensor");
    }

    #[test]
    fn device_type_virtual_agent_display() {
        assert_eq!(DeviceType::VirtualAgent.to_string(), "VirtualAgent");
    }

    #[test]
    fn device_type_custom_display() {
        assert_eq!(
            DeviceType::Custom("MyRobot".to_string()).to_string(),
            "Custom(MyRobot)"
        );
    }

    #[test]
    fn device_type_equality() {
        assert_eq!(DeviceType::Robot, DeviceType::Robot);
        assert_ne!(DeviceType::Robot, DeviceType::Drone);
        assert_eq!(
            DeviceType::Custom("X".to_string()),
            DeviceType::Custom("X".to_string())
        );
        assert_ne!(
            DeviceType::Custom("X".to_string()),
            DeviceType::Custom("Y".to_string())
        );
    }

    #[test]
    fn device_type_serde_roundtrip() {
        let variants = vec![
            DeviceType::CognitumSeed,
            DeviceType::CognitumAppliance,
            DeviceType::Robot,
            DeviceType::Drone,
            DeviceType::IoTSensor,
            DeviceType::VirtualAgent,
            DeviceType::Custom("TestBot".to_string()),
        ];

        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: DeviceType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized);
        }
    }

    #[test]
    fn device_identity_default_metadata() {
        let device = DeviceIdentity::new(
            [0x01; 32],
            Address::ZERO,
            DeviceType::VirtualAgent,
            Hash::ZERO,
            Hash::ZERO,
            0,
        );
        assert!(device.metadata.name.is_none());
        assert!(device.metadata.manufacturer.is_none());
        assert!(device.metadata.model.is_none());
        assert!(device.metadata.firmware_version.is_none());
        assert!(device.metadata.location.is_none());
        assert!(device.metadata.interfaces.is_empty());
        assert!(device.metadata.extra.is_empty());
    }
}

// ============================================================================
// Cross-module integration tests
// ============================================================================

mod cross_module_tests {
    use super::*;

    #[test]
    fn full_transaction_lifecycle() {
        // Generate keypairs for sender and receiver
        let (sender_sk, sender_vk) = crypto::generate_keypair();
        let (_, receiver_vk) = crypto::generate_keypair();

        let sender_addr = Address::from_pubkey(&sender_vk);
        let receiver_addr = Address::from_pubkey(&receiver_vk);

        // Setup accounts
        let mut state = AccountState::new();
        state.credit(&sender_addr, 10_000);

        // Create and sign a transfer
        let mut tx = Transaction::Transfer {
            from: sender_addr,
            to: receiver_addr,
            amount: 500,
            memo: Some(b"test payment".to_vec()),
            device_witness: None,
            nonce: 0,
            fee: 10,
            signature: Sig64([0u8; 64]),
        };

        let msg = tx.signing_bytes();
        let sig = crypto::sign(&sender_sk, &msg);
        if let Transaction::Transfer { ref mut signature, .. } = tx {
            *signature = Sig64(sig);
        }

        // Verify signature
        assert!(tx.verify_signature(&sender_vk));

        // Apply transaction to state
        state.deduct_fee(&sender_addr, tx.fee()).unwrap();
        state.transfer(&sender_addr, &receiver_addr, 500).unwrap();
        state.increment_nonce(&sender_addr).unwrap();

        // Verify final state
        assert_eq!(state.get_account(&sender_addr).unwrap().balance, 9_490);
        assert_eq!(state.get_account(&receiver_addr).unwrap().balance, 500);
        assert_eq!(state.get_account(&sender_addr).unwrap().nonce, 1);
    }

    #[test]
    fn block_with_transactions_and_merkle_root() {
        let (sk, vk) = crypto::generate_keypair();
        let from = Address::from_pubkey(&vk);

        // Create two transactions
        let mut txs = Vec::new();
        for i in 0..3u64 {
            let mut tx = Transaction::Transfer {
                from,
                to: Address([(i as u8) + 0xA0; 32]),
                amount: 100 * (i + 1),
                memo: None,
                device_witness: None,
                nonce: i,
                fee: 10,
                signature: Sig64([0u8; 64]),
            };
            let msg = tx.signing_bytes();
            let sig = crypto::sign(&sk, &msg);
            if let Transaction::Transfer { ref mut signature, .. } = tx {
                *signature = Sig64(sig);
            }
            txs.push(tx);
        }

        let block = Block {
            header: BlockHeader {
                block_number: 1,
                parent_hash: Hash::ZERO,
                state_root: Hash::ZERO,
                transactions_root: Hash::ZERO,
                timestamp: 1_700_000_000,
                proposer: from,
                signature: [0u8; 64],
            },
            transactions: txs,
        };

        // Merkle root should be non-zero and deterministic
        let root = block.compute_transactions_root();
        assert_ne!(root, Hash::ZERO);
        assert_eq!(root, block.compute_transactions_root());
        assert_eq!(block.transaction_count(), 3);
    }

    #[test]
    fn device_registration_and_account_setup() {
        let (owner_sk, owner_vk) = crypto::generate_keypair();
        let owner_addr = Address::from_pubkey(&owner_vk);

        // Create device identity
        let device_pk = [0x42; 32];
        let device = DeviceIdentity::new(
            device_pk,
            owner_addr,
            DeviceType::CognitumSeed,
            Hash([0xFE; 32]),
            Hash::ZERO,
            1_700_000_000,
        );

        // Create RegisterDevice transaction
        let attestation = DeviceAttestation {
            pubkey: device_pk,
            firmware_hash: device.firmware_hash,
            witness_root: device.witness_root,
            timestamp: device.registered_at,
            signature: Sig64([0u8; 64]),
        };

        let mut tx = Transaction::RegisterDevice {
            device_pubkey: device_pk,
            owner: owner_addr,
            attestation,
            nonce: 0,
            fee: 50,
            signature: Sig64([0u8; 64]),
        };

        let msg = tx.signing_bytes();
        let sig = crypto::sign(&owner_sk, &msg);
        if let Transaction::RegisterDevice { ref mut signature, .. } = tx {
            *signature = Sig64(sig);
        }

        // Verify
        assert!(tx.verify_signature(&owner_vk));
        assert_eq!(tx.sender(), owner_addr);

        // Setup account for fee deduction
        let mut state = AccountState::new();
        state.credit(&owner_addr, 1000);
        state.deduct_fee(&owner_addr, tx.fee()).unwrap();
        state.increment_nonce(&owner_addr).unwrap();

        assert_eq!(state.get_account(&owner_addr).unwrap().balance, 950);
        assert_eq!(state.get_account(&owner_addr).unwrap().nonce, 1);
    }

    #[test]
    fn signed_genesis_into_chain() {
        let (sk, vk) = crypto::generate_keypair();
        let genesis = Block::signed_genesis(&sk, 1_700_000_000);

        // Genesis should verify
        assert!(genesis.verify(&vk));
        assert_eq!(genesis.header.block_number, 0);
        assert_eq!(genesis.header.parent_hash, Hash::ZERO);

        // Build a second block referencing genesis
        let genesis_hash = genesis.hash();
        let block1 = Block {
            header: BlockHeader {
                block_number: 1,
                parent_hash: genesis_hash,
                state_root: Hash::ZERO,
                transactions_root: Hash::ZERO,
                timestamp: 1_700_000_001,
                proposer: Address::from_pubkey(&vk),
                signature: [0u8; 64],
            },
            transactions: Vec::new(),
        };

        assert_eq!(block1.header.parent_hash, genesis_hash);
        assert_eq!(block1.header.block_number, 1);
        assert_ne!(block1.hash(), genesis.hash());
    }
}

// ============================================================================
// Error tests
// ============================================================================

mod error_tests {
    use super::*;

    #[test]
    fn insufficient_balance_error_display() {
        let err = DinaError::InsufficientBalance {
            have: 100,
            need: 500,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("100"));
        assert!(msg.contains("500"));
    }

    #[test]
    fn account_not_found_error_display() {
        let err = DinaError::AccountNotFound("0xdeadbeef".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("0xdeadbeef"));
    }

    #[test]
    fn invalid_signature_error_display() {
        let err = DinaError::InvalidSignature;
        let msg = format!("{}", err);
        assert!(msg.contains("invalid signature"));
    }

    #[test]
    fn serialization_error_display() {
        let err = DinaError::SerializationError("bad data".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("bad data"));
    }

    #[test]
    fn error_is_clone() {
        let err = DinaError::InsufficientBalance {
            have: 10,
            need: 20,
        };
        let cloned = err.clone();
        assert_eq!(format!("{}", err), format!("{}", cloned));
    }
}
