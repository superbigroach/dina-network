//! End-to-end integration tests spanning multiple crates in the Dina Network.

use dina_core::account::AccountState;
use dina_core::block::{Block, BlockHeader};
use dina_core::crypto;
use dina_core::executor::BlockExecutor;
use dina_core::transaction::{Sig64, Transaction};
use dina_core::types::{Address, Hash};

use dina_channels::channel::PaymentChannel;
use dina_channels::relay;
use dina_channels::state::{self as channel_state, SignedState};

use dina_privacy::encrypted_memo::{decrypt_memo, encrypt_memo};
use dina_privacy::stealth::{
    derive_stealth_address, detect_stealth, generate_meta_address,
};

use drc1_token::TokenState;
use drc101_agent_wallet::AgentWalletState;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_address(i: u64) -> Address {
    let mut bytes = [0u8; 32];
    bytes[0..8].copy_from_slice(&i.to_le_bytes());
    Address(bytes)
}

fn make_signed_transfer(
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

    if let Transaction::Transfer {
        ref mut signature, ..
    } = tx
    {
        *signature = Sig64(sig);
    }

    tx
}

fn make_block_with(
    proposer: Address,
    txs: Vec<Transaction>,
    block_number: u64,
    parent_hash: Hash,
) -> Block {
    Block {
        header: BlockHeader {
            block_number,
            parent_hash,
            state_root: Hash::ZERO,
            transactions_root: Hash::ZERO,
            timestamp: 1_700_000_000 + block_number,
            proposer,
            signature: [0u8; 64],
        },
        transactions: txs,
    }
}

// ===========================================================================
// Test 1: Full block lifecycle
// ===========================================================================

#[test]
fn test_full_block_lifecycle() {
    // Generate 3 validator keypairs
    let (sk_v1, vk_v1) = crypto::generate_keypair();
    let (sk_v2, vk_v2) = crypto::generate_keypair();
    let (sk_v3, vk_v3) = crypto::generate_keypair();

    let addr_v1 = Address::from_pubkey(&vk_v1);
    let addr_v2 = Address::from_pubkey(&vk_v2);
    let addr_v3 = Address::from_pubkey(&vk_v3);

    // Create genesis block (signed by validator 1)
    let genesis = Block::signed_genesis(&sk_v1, 1_700_000_000);
    assert_eq!(genesis.header.block_number, 0);
    assert!(genesis.verify(&vk_v1));

    // Set up account state with initial balances
    let mut state = AccountState::new();
    state.credit(&addr_v1, 100_000);
    state.credit(&addr_v2, 100_000);
    state.credit(&addr_v3, 100_000);

    // Create 5 transfer transactions
    let tx1 = make_signed_transfer(&sk_v1, addr_v2, 1_000, 0, 10);
    let tx2 = make_signed_transfer(&sk_v1, addr_v3, 2_000, 1, 10);
    let tx3 = make_signed_transfer(&sk_v2, addr_v1, 500, 0, 10);
    let tx4 = make_signed_transfer(&sk_v2, addr_v3, 1_500, 1, 10);
    let tx5 = make_signed_transfer(&sk_v3, addr_v1, 3_000, 0, 10);

    // Verify each transaction signature
    assert!(tx1.verify_signature(&vk_v1));
    assert!(tx2.verify_signature(&vk_v1));
    assert!(tx3.verify_signature(&vk_v2));
    assert!(tx4.verify_signature(&vk_v2));
    assert!(tx5.verify_signature(&vk_v3));

    // Build block with transactions
    let proposer = addr_v1;
    let block = make_block_with(
        proposer,
        vec![tx1, tx2, tx3, tx4, tx5],
        1,
        genesis.hash(),
    );

    // Execute block against account state
    let mut executor = BlockExecutor::new(state);
    let result = executor.execute_block(&block).unwrap();

    // Verify all 5 receipts are successful
    assert_eq!(result.receipts.len(), 5);
    for (i, receipt) in result.receipts.iter().enumerate() {
        assert!(
            receipt.success,
            "transaction {i} failed: {:?}",
            receipt.error
        );
    }

    // Verify total fees collected
    assert_eq!(result.total_fees, 50); // 5 txs * 10 fee each

    // Verify final balances:
    // v1: 100_000 - 10(fee) - 1_000 - 10(fee) - 2_000 + 500 + 3_000 + 50(proposer fees) = 100_530
    // v2: 100_000 + 1_000 - 10(fee) - 500 - 10(fee) - 1_500 = 98_980
    // v3: 100_000 + 2_000 + 1_500 - 10(fee) - 3_000 = 100_490
    let final_state = executor.state();
    assert_eq!(
        final_state.get_account(&addr_v1).unwrap().balance,
        100_530
    );
    assert_eq!(
        final_state.get_account(&addr_v2).unwrap().balance,
        98_980
    );
    assert_eq!(
        final_state.get_account(&addr_v3).unwrap().balance,
        100_490
    );

    // Verify nonces incremented
    assert_eq!(final_state.get_account(&addr_v1).unwrap().nonce, 2);
    assert_eq!(final_state.get_account(&addr_v2).unwrap().nonce, 2);
    assert_eq!(final_state.get_account(&addr_v3).unwrap().nonce, 1);

    // Verify state root is non-zero
    assert_ne!(result.state_root, Hash::ZERO);
}

// ===========================================================================
// Test 2: Payment channel lifecycle
// ===========================================================================

#[test]
fn test_payment_channel_lifecycle() {
    let key_a = ed25519_dalek::SigningKey::from_bytes(&[1u8; 32]);
    let key_b = ed25519_dalek::SigningKey::from_bytes(&[2u8; 32]);
    let pub_a = key_a.verifying_key().to_bytes();
    let pub_b = key_b.verifying_key().to_bytes();

    // Open channel between Alice and Bob
    let mut channel = PaymentChannel::open(pub_a, pub_b, 1_000_000, 1_000_000);
    assert_eq!(channel.balance_a, 1_000_000);
    assert_eq!(channel.balance_b, 1_000_000);
    assert_eq!(channel.total_locked, 2_000_000);

    // Alice pays Bob 3 times
    let state1 = channel.update(100_000).unwrap(); // A->B 100k
    let _sig_a1 = channel_state::sign(&state1, &key_a);
    let _sig_b1 = channel_state::sign(&state1, &key_b);

    let state2 = channel.update(200_000).unwrap(); // A->B 200k
    let _sig_a2 = channel_state::sign(&state2, &key_a);
    let _sig_b2 = channel_state::sign(&state2, &key_b);

    let state3 = channel.update(50_000).unwrap(); // A->B 50k
    let sig_a3 = channel_state::sign(&state3, &key_a);
    let sig_b3 = channel_state::sign(&state3, &key_b);

    // After 3 payments from A: A=650k, B=1350k
    assert_eq!(channel.balance_a, 650_000);
    assert_eq!(channel.balance_b, 1_350_000);

    // Bob pays Alice once -- need to swap perspective.
    // The channel.update() always transfers from A to B, so to transfer from B to A
    // we create a fresh channel state manually.
    // Instead, we use a separate channel tracking to represent B->A.
    // For the test, we simulate by adjusting balances directly through state updates.

    // Actually, the channel only supports A->B via update(). We test cooperative close
    // with the latest state.

    // Close channel cooperatively with the latest state (state3)
    let signed_final = SignedState {
        state: state3.clone(),
        signature_a: sig_a3,
        signature_b: sig_b3,
    };

    channel.close_cooperative(&signed_final).unwrap();
    assert_eq!(
        channel.status,
        dina_channels::channel::ChannelStatus::Closed
    );
    assert_eq!(channel.balance_a, 650_000);
    assert_eq!(channel.balance_b, 1_350_000);

    // Create relay blob from the final signed state
    let relay_blob = relay::create_relay_blob(signed_final.clone(), 500);

    // Validate relay blob
    assert!(relay::validate_relay_blob(&relay_blob, &pub_a, &pub_b));

    // Round-trip through QR bytes
    let qr_bytes = relay::blob_to_qr_bytes(&relay_blob);
    let recovered_blob = relay::blob_from_qr_bytes(&qr_bytes).unwrap();
    assert_eq!(
        recovered_blob.signed_state.state,
        signed_final.state
    );
    assert_eq!(recovered_blob.relay_fee, 500);

    // Recovered blob should also validate
    assert!(relay::validate_relay_blob(&recovered_blob, &pub_a, &pub_b));

    // Verify final balances match (conservation of funds)
    assert_eq!(
        channel.balance_a + channel.balance_b,
        channel.total_locked
    );
}

// ===========================================================================
// Test 3: Multi-block chain
// ===========================================================================

#[test]
fn test_multi_block_chain() {
    let (sk_alice, vk_alice) = crypto::generate_keypair();
    let (sk_bob, vk_bob) = crypto::generate_keypair();
    let addr_alice = Address::from_pubkey(&vk_alice);
    let addr_bob = Address::from_pubkey(&vk_bob);
    let proposer = make_address(0);

    // Create genesis
    let genesis = Block::signed_genesis(&sk_alice, 1_700_000_000);
    let mut parent_hash = genesis.hash();

    // Initial state
    let mut state = AccountState::new();
    state.credit(&addr_alice, 1_000_000);
    state.credit(&addr_bob, 500_000);

    let mut executor = BlockExecutor::new(state);
    let mut state_roots = Vec::new();

    // Block 1: Alice -> Bob 1000 (nonce 0), Alice -> Bob 2000 (nonce 1)
    {
        let tx1 = make_signed_transfer(&sk_alice, addr_bob, 1_000, 0, 10);
        let tx2 = make_signed_transfer(&sk_alice, addr_bob, 2_000, 1, 10);
        let block = make_block_with(proposer, vec![tx1, tx2], 1, parent_hash);
        parent_hash = block.hash();
        let result = executor.execute_block(&block).unwrap();
        assert!(result.receipts.iter().all(|r| r.success));
        state_roots.push(result.state_root);
    }

    // Block 2: Bob -> Alice 500 (nonce 0), Bob -> Alice 300 (nonce 1), Alice -> Bob 100 (nonce 2)
    {
        let tx1 = make_signed_transfer(&sk_bob, addr_alice, 500, 0, 10);
        let tx2 = make_signed_transfer(&sk_bob, addr_alice, 300, 1, 10);
        let tx3 = make_signed_transfer(&sk_alice, addr_bob, 100, 2, 10);
        let block = make_block_with(proposer, vec![tx1, tx2, tx3], 2, parent_hash);
        parent_hash = block.hash();
        let result = executor.execute_block(&block).unwrap();
        assert!(result.receipts.iter().all(|r| r.success));
        state_roots.push(result.state_root);
    }

    // Block 3: Alice -> Bob 5000 (nonce 3), Bob -> Alice 1000 (nonce 2)
    {
        let tx1 = make_signed_transfer(&sk_alice, addr_bob, 5_000, 3, 10);
        let tx2 = make_signed_transfer(&sk_bob, addr_alice, 1_000, 2, 10);
        let block = make_block_with(proposer, vec![tx1, tx2], 3, parent_hash);
        let result = executor.execute_block(&block).unwrap();
        assert!(result.receipts.iter().all(|r| r.success));
        state_roots.push(result.state_root);
    }

    // Verify state roots all differ (state changed each block)
    assert_ne!(state_roots[0], state_roots[1]);
    assert_ne!(state_roots[1], state_roots[2]);
    assert_ne!(state_roots[0], state_roots[2]);

    // Verify chain height consistency through 3 blocks
    assert_eq!(state_roots.len(), 3);

    // Verify balances after all blocks:
    // Alice: 1_000_000 - 10 - 1000 - 10 - 2000 + 500 + 300 - 10 - 100 - 10 - 5000 - 10 + 1000 = 993_650
    //   plus proposer fees credited to proposer address
    // Bob: 500_000 + 1000 + 2000 - 10 - 500 - 10 - 300 + 100 - 10 - 1000 = 501_270
    // Proposer gets total fees: 7 txs * 10 = 70
    let final_state = executor.state();

    let alice_balance = final_state.get_account(&addr_alice).unwrap().balance;
    let bob_balance = final_state.get_account(&addr_bob).unwrap().balance;
    let proposer_balance = final_state
        .get_account(&proposer)
        .map(|a| a.balance)
        .unwrap_or(0);

    // Total should be conserved: initial 1_500_000 = alice + bob + proposer
    assert_eq!(alice_balance + bob_balance + proposer_balance, 1_500_000);

    // Verify nonces
    assert_eq!(final_state.get_account(&addr_alice).unwrap().nonce, 4);
    assert_eq!(final_state.get_account(&addr_bob).unwrap().nonce, 3);
}

// ===========================================================================
// Test 4: Privacy roundtrip
// ===========================================================================

#[test]
fn test_privacy_roundtrip() {
    // Generate stealth meta-address
    let (meta, scan_secret, _spend_secret) = generate_meta_address();

    // Derive stealth address
    let stealth = derive_stealth_address(&meta);

    // Detect stealth address
    let detected = detect_stealth(
        &scan_secret,
        &meta.spend_pubkey,
        &stealth.ephemeral_pubkey,
        &stealth.address,
    );
    assert!(detected, "recipient should detect their stealth address");

    // Wrong recipient should not detect
    let (_, wrong_scan_secret, _) = generate_meta_address();
    let not_detected = detect_stealth(
        &wrong_scan_secret,
        &meta.spend_pubkey,
        &stealth.ephemeral_pubkey,
        &stealth.address,
    );
    assert!(
        !not_detected,
        "wrong recipient should not detect stealth address"
    );

    // Encrypt memo
    let recipient_secret_bytes: [u8; 32] = rand::random();
    let recipient_secret = x25519_dalek::StaticSecret::from(recipient_secret_bytes);
    let recipient_pubkey = x25519_dalek::PublicKey::from(&recipient_secret);

    let original_plaintext = b"Payment of 42.00 USDC for sensor data relay";
    let memo = encrypt_memo(recipient_pubkey.as_bytes(), original_plaintext);

    // Decrypt memo
    let decrypted = decrypt_memo(&recipient_secret_bytes, &memo).unwrap();

    // Verify original plaintext matches
    assert_eq!(
        decrypted, original_plaintext,
        "decrypted memo must match original plaintext"
    );

    // Verify wrong key cannot decrypt
    let wrong_key: [u8; 32] = rand::random();
    let wrong_result = decrypt_memo(&wrong_key, &memo);
    assert!(wrong_result.is_err(), "wrong key must fail to decrypt");
}

// ===========================================================================
// Test 5: DRC contract lifecycle
// ===========================================================================

#[test]
fn test_drc_contract_lifecycle() {
    let owner: [u8; 32] = [0x01; 32];
    let alice: [u8; 32] = [0x02; 32];
    let bob: [u8; 32] = [0x03; 32];

    // -----------------------------------------------------------------------
    // DRC-1 Token lifecycle
    // -----------------------------------------------------------------------

    // Create DRC-1 token state via dispatch
    let mut token_state: Option<TokenState> = None;

    let init_args = serde_json::to_vec(&serde_json::json!({
        "name": "Dina Token",
        "symbol": "DINA",
        "decimals": 6
    }))
    .unwrap();

    drc1_token::dispatch(&mut token_state, "init", &init_args, owner);
    assert!(token_state.is_some());

    // Mint tokens
    let mint_args = serde_json::to_vec(&serde_json::json!({
        "to": alice,
        "amount": 1_000_000u64
    }))
    .unwrap();
    drc1_token::dispatch(&mut token_state, "mint", &mint_args, owner);

    // Verify balance via dispatch
    let balance_args = serde_json::to_vec(&serde_json::json!({
        "account": alice
    }))
    .unwrap();
    let balance_result =
        drc1_token::dispatch(&mut token_state, "balance_of", &balance_args, owner);
    let balance: u64 = serde_json::from_slice(&balance_result).unwrap();
    assert_eq!(balance, 1_000_000);

    // Transfer tokens (Alice -> Bob)
    let transfer_args = serde_json::to_vec(&serde_json::json!({
        "to": bob,
        "amount": 250_000u64
    }))
    .unwrap();
    drc1_token::dispatch(&mut token_state, "transfer", &transfer_args, alice);

    // Check balances
    let alice_bal_args = serde_json::to_vec(&serde_json::json!({ "account": alice })).unwrap();
    let alice_bal_result =
        drc1_token::dispatch(&mut token_state, "balance_of", &alice_bal_args, owner);
    let alice_balance: u64 = serde_json::from_slice(&alice_bal_result).unwrap();
    assert_eq!(alice_balance, 750_000);

    let bob_bal_args = serde_json::to_vec(&serde_json::json!({ "account": bob })).unwrap();
    let bob_bal_result =
        drc1_token::dispatch(&mut token_state, "balance_of", &bob_bal_args, owner);
    let bob_balance: u64 = serde_json::from_slice(&bob_bal_result).unwrap();
    assert_eq!(bob_balance, 250_000);

    // Verify total supply
    let supply_result =
        drc1_token::dispatch(&mut token_state, "total_supply", &[], owner);
    let total_supply: u64 = serde_json::from_slice(&supply_result).unwrap();
    assert_eq!(total_supply, 1_000_000);

    // -----------------------------------------------------------------------
    // DRC-101 Agent Wallet lifecycle
    // -----------------------------------------------------------------------

    // Create DRC-101 agent wallet
    let mut wallet_state: Option<AgentWalletState> = None;
    drc101_agent_wallet::dispatch(&mut wallet_state, "init", &[], owner);
    assert!(wallet_state.is_some());

    // Deposit funds
    let deposit_args = serde_json::to_vec(&serde_json::json!({
        "amount": 500_000u64
    }))
    .unwrap();
    drc101_agent_wallet::dispatch(&mut wallet_state, "deposit", &deposit_args, owner);

    // Check balance
    let balance_result =
        drc101_agent_wallet::dispatch(&mut wallet_state, "balance", &[], owner);
    let wallet_balance: u64 = serde_json::from_slice(&balance_result).unwrap();
    assert_eq!(wallet_balance, 500_000);

    // Set spending limits
    let limits_args = serde_json::to_vec(&serde_json::json!({
        "limits": {
            "max_per_transaction": 100_000u64,
            "max_per_day": 200_000u64,
            "max_per_month": 1_000_000u64,
            "max_transactions_per_day": 10u32,
            "min_interval_ms": 0u64
        }
    }))
    .unwrap();
    drc101_agent_wallet::dispatch(&mut wallet_state, "set_limits", &limits_args, owner);

    // Execute transfer within limits
    let exec_args = serde_json::to_vec(&serde_json::json!({
        "to": alice,
        "amount": 50_000u64,
        "timestamp_ms": 1_700_000_000_000u64,
        "day": 1u64,
        "month": 1u64,
        "memo": "payment for relay service",
        "witness": null
    }))
    .unwrap();
    drc101_agent_wallet::dispatch(
        &mut wallet_state,
        "execute_transfer",
        &exec_args,
        owner,
    );

    // Verify spending stats updated
    let stats_result =
        drc101_agent_wallet::dispatch(&mut wallet_state, "spending_stats", &[], owner);
    let stats: drc101_agent_wallet::SpendingStats =
        serde_json::from_slice(&stats_result).unwrap();
    assert_eq!(stats.total_spent, 50_000);
    assert_eq!(stats.spent_today, 50_000);
    assert_eq!(stats.transactions_today, 1);

    // Verify balance decreased
    let balance_result2 =
        drc101_agent_wallet::dispatch(&mut wallet_state, "balance", &[], owner);
    let wallet_balance2: u64 = serde_json::from_slice(&balance_result2).unwrap();
    assert_eq!(wallet_balance2, 450_000);

    // Execute a second transfer
    let exec_args2 = serde_json::to_vec(&serde_json::json!({
        "to": bob,
        "amount": 25_000u64,
        "timestamp_ms": 1_700_000_001_000u64,
        "day": 1u64,
        "month": 1u64,
        "memo": "payment for data attestation",
        "witness": null
    }))
    .unwrap();
    drc101_agent_wallet::dispatch(
        &mut wallet_state,
        "execute_transfer",
        &exec_args2,
        owner,
    );

    // Verify updated stats
    let stats_result2 =
        drc101_agent_wallet::dispatch(&mut wallet_state, "spending_stats", &[], owner);
    let stats2: drc101_agent_wallet::SpendingStats =
        serde_json::from_slice(&stats_result2).unwrap();
    assert_eq!(stats2.total_spent, 75_000);
    assert_eq!(stats2.spent_today, 75_000);
    assert_eq!(stats2.transactions_today, 2);
    assert_eq!(stats2.spent_this_month, 75_000);
}
