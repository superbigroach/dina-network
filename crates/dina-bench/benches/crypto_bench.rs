use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use dina_core::crypto::{address_from_pubkey, generate_keypair, hash_bytes, sign, verify};
use dina_core::merkle::MerkleTree;
use dina_core::transaction::{Sig64, Transaction};
use dina_core::types::Address;

fn bench_ed25519_keygen(c: &mut Criterion) {
    c.bench_function("ed25519_keygen", |b| {
        b.iter(|| {
            let _ = generate_keypair();
        });
    });
}

fn bench_ed25519_sign(c: &mut Criterion) {
    let (sk, _) = generate_keypair();
    let msg = b"benchmark message for signing operations";

    c.bench_function("ed25519_sign", |b| {
        b.iter(|| {
            let _ = sign(&sk, msg);
        });
    });
}

fn bench_ed25519_verify(c: &mut Criterion) {
    let (sk, vk) = generate_keypair();
    let msg = b"benchmark message for verify operations";
    let sig = sign(&sk, msg);

    c.bench_function("ed25519_verify", |b| {
        b.iter(|| {
            let _ = verify(&vk, msg, &sig);
        });
    });
}

fn bench_sha256_hash(c: &mut Criterion) {
    let data_32 = vec![0xABu8; 32];
    let data_1kb = vec![0xCDu8; 1024];
    let data_1mb = vec![0xEFu8; 1024 * 1024];

    let mut group = c.benchmark_group("sha256_hash");

    group.throughput(Throughput::Bytes(32));
    group.bench_with_input(BenchmarkId::new("sha256", "32B"), &data_32, |b, data| {
        b.iter(|| hash_bytes(data));
    });

    group.throughput(Throughput::Bytes(1024));
    group.bench_with_input(BenchmarkId::new("sha256", "1KB"), &data_1kb, |b, data| {
        b.iter(|| hash_bytes(data));
    });

    group.throughput(Throughput::Bytes(1024 * 1024));
    group.bench_with_input(BenchmarkId::new("sha256", "1MB"), &data_1mb, |b, data| {
        b.iter(|| hash_bytes(data));
    });

    group.finish();
}

fn bench_transaction_serialization(c: &mut Criterion) {
    let (sk, vk) = generate_keypair();
    let from = Address::from_pubkey(&vk);
    let to = Address([0xBB; 32]);

    let mut tx = Transaction::Transfer {
        from,
        to,
        amount: 1_000_000,
        memo: Some(b"benchmark transfer memo".to_vec()),
        device_witness: None,
        nonce: 42,
        fee: 100,
        pub_key: *vk.as_bytes(),
        signature: Sig64([0u8; 64]),
    };

    let msg = tx.signing_bytes();
    let sig = sign(&sk, &msg);
    if let Transaction::Transfer {
        ref mut signature, ..
    } = tx
    {
        *signature = Sig64(sig);
    }

    c.bench_function("tx_serialization", |b| {
        b.iter(|| {
            let _ = tx.signing_bytes();
        });
    });
}

fn bench_transaction_hash(c: &mut Criterion) {
    let (sk, vk) = generate_keypair();
    let from = Address::from_pubkey(&vk);
    let to = Address([0xBB; 32]);

    let mut tx = Transaction::Transfer {
        from,
        to,
        amount: 1_000_000,
        memo: None,
        device_witness: None,
        nonce: 0,
        fee: 10,
        pub_key: *vk.as_bytes(),
        signature: Sig64([0u8; 64]),
    };

    let msg = tx.signing_bytes();
    let sig = sign(&sk, &msg);
    if let Transaction::Transfer {
        ref mut signature, ..
    } = tx
    {
        *signature = Sig64(sig);
    }

    c.bench_function("tx_hash", |b| {
        b.iter(|| {
            let _ = tx.hash();
        });
    });
}

fn bench_address_from_pubkey(c: &mut Criterion) {
    let (_, vk) = generate_keypair();

    c.bench_function("address_from_pubkey", |b| {
        b.iter(|| {
            let _ = address_from_pubkey(&vk);
        });
    });
}

fn bench_merkle_root(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_root");

    for count in [10, 100, 1000] {
        let leaves: Vec<[u8; 32]> = (0..count)
            .map(|i| {
                let mut leaf = [0u8; 32];
                leaf[0..8].copy_from_slice(&(i as u64).to_le_bytes());
                leaf
            })
            .collect();

        group.bench_with_input(BenchmarkId::new("items", count), &leaves, |b, leaves| {
            b.iter(|| {
                let mut tree = MerkleTree::new();
                for leaf in leaves {
                    tree.insert(leaf);
                }
                tree.root()
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_ed25519_keygen,
    bench_ed25519_sign,
    bench_ed25519_verify,
    bench_sha256_hash,
    bench_transaction_serialization,
    bench_transaction_hash,
    bench_address_from_pubkey,
    bench_merkle_root,
);

criterion_main!(benches);
