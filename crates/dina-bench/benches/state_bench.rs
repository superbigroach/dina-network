use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dina_core::account::{Account, AccountState};
use dina_core::block::{Block, BlockHeader};
use dina_core::crypto;
use dina_core::executor::BlockExecutor;
use dina_core::transaction::{Sig64, Transaction};
use dina_core::types::{Address, Hash};
use dina_storage::DinaDB;

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
        pub_key: *vk.as_bytes(),
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

fn make_block(proposer: Address, txs: Vec<Transaction>, block_number: u64) -> Block {
    Block {
        header: BlockHeader {
            block_number,
            parent_hash: Hash::ZERO,
            state_root: Hash::ZERO,
            transactions_root: Hash::ZERO,
            timestamp: 1_700_000_000 + block_number,
            proposer,
            proposer_pubkey: [0u8; 32],
            signature: [0u8; 64],
        },
        transactions: txs,
    }
}

// ---------------------------------------------------------------------------
// AccountState benchmarks
// ---------------------------------------------------------------------------

fn bench_account_state_get_set(c: &mut Criterion) {
    let mut group = c.benchmark_group("account_state");

    group.bench_function("set_account", |b| {
        let mut state = AccountState::new();
        let mut i = 0u64;
        b.iter(|| {
            let addr = make_address(i);
            state.set_account(Account::with_balance(addr, 1_000_000));
            i += 1;
        });
    });

    // Pre-populate then benchmark get
    let mut state = AccountState::new();
    for i in 0..10_000u64 {
        let addr = make_address(i);
        state.set_account(Account::with_balance(addr, 1_000_000));
    }

    group.bench_function("get_account", |b| {
        let mut i = 0u64;
        b.iter(|| {
            let addr = make_address(i % 10_000);
            let _ = state.get_account(&addr);
            i += 1;
        });
    });

    group.finish();
}

fn bench_account_state_transfer(c: &mut Criterion) {
    c.bench_function("account_state_transfer", |b| {
        b.iter_batched(
            || {
                let mut state = AccountState::new();
                let a = make_address(0);
                let bb = make_address(1);
                state.set_account(Account::with_balance(a, u64::MAX / 2));
                state.set_account(Account::with_balance(bb, u64::MAX / 2));
                state
            },
            |mut state| {
                let a = make_address(0);
                let bb = make_address(1);
                let _ = state.transfer(&a, &bb, 100);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ---------------------------------------------------------------------------
// Block execution benchmarks
// ---------------------------------------------------------------------------

fn bench_block_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("block_execution");

    for tx_count in [10, 100, 1000] {
        // Pre-generate keypairs and transactions
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let proposer = make_address(999_999);

        let txs: Vec<Transaction> = (0..tx_count)
            .map(|i| {
                let to = make_address(i as u64 + 1_000_000);
                make_signed_transfer(&sk, to, 10, i as u64, 1)
            })
            .collect();

        group.bench_with_input(BenchmarkId::new("txs", tx_count), &txs, |b, txs| {
            b.iter_batched(
                || {
                    let mut state = AccountState::new();
                    // Give sender enough balance for all transactions
                    let total_needed = (tx_count as u64) * (10 + 1) + 1_000_000;
                    state.credit(&sender, total_needed);
                    (BlockExecutor::new(state), txs.clone())
                },
                |(mut executor, txs)| {
                    let block = make_block(proposer, txs, 1);
                    let _ = executor.execute_block(&block);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// redb benchmarks
// ---------------------------------------------------------------------------

fn bench_redb_account(c: &mut Criterion) {
    let mut group = c.benchmark_group("redb");

    group.bench_function("write_account", |b| {
        let db = DinaDB::open_in_memory().expect("failed to open db");
        let mut i = 0u64;
        b.iter(|| {
            let addr = make_address(i);
            let account = Account::with_balance(addr, 1_000_000 + i);
            db.set_account(addr, &account).unwrap();
            i += 1;
        });
    });

    group.bench_function("read_account", |b| {
        let db = DinaDB::open_in_memory().expect("failed to open db");
        // Pre-populate
        for i in 0..1_000u64 {
            let addr = make_address(i);
            let account = Account::with_balance(addr, 1_000_000 + i);
            db.set_account(addr, &account).unwrap();
        }
        let mut i = 0u64;
        b.iter(|| {
            let addr = make_address(i % 1_000);
            let _ = db.get_account(addr);
            i += 1;
        });
    });

    group.finish();
}

fn bench_redb_block(c: &mut Criterion) {
    let mut group = c.benchmark_group("redb_block");

    group.bench_function("write_block", |b| {
        let db = DinaDB::open_in_memory().expect("failed to open db");
        let mut height = 0u64;
        b.iter(|| {
            let block = Block {
                header: BlockHeader {
                    block_number: height,
                    parent_hash: Hash::ZERO,
                    state_root: Hash::ZERO,
                    transactions_root: Hash::ZERO,
                    timestamp: 1_700_000_000 + height,
                    proposer: Address::ZERO,
                    proposer_pubkey: [0u8; 32],
                    signature: [0u8; 64],
                },
                transactions: vec![],
            };
            db.store_block(&block).unwrap();
            height += 1;
        });
    });

    group.bench_function("read_block", |b| {
        let db = DinaDB::open_in_memory().expect("failed to open db");
        // Pre-populate
        for h in 0..1_000u64 {
            let block = Block {
                header: BlockHeader {
                    block_number: h,
                    parent_hash: Hash::ZERO,
                    state_root: Hash::ZERO,
                    transactions_root: Hash::ZERO,
                    timestamp: 1_700_000_000 + h,
                    proposer: Address::ZERO,
                    proposer_pubkey: [0u8; 32],
                    signature: [0u8; 64],
                },
                transactions: vec![],
            };
            db.store_block(&block).unwrap();
        }
        let mut i = 0u64;
        b.iter(|| {
            let _ = db.get_block(i % 1_000);
            i += 1;
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_account_state_get_set,
    bench_account_state_transfer,
    bench_block_execution,
    bench_redb_account,
    bench_redb_block,
);

criterion_main!(benches);
