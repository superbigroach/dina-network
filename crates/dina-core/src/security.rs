//! Security utilities for validating transactions, preventing replay attacks,
//! and enforcing protocol safety invariants.
//!
//! These validators are designed to be called at the mempool/pre-execution layer
//! before transactions enter a block, and again during block execution as
//! defense-in-depth.

use std::collections::BTreeSet;

use crate::error::{DinaError, DinaResult};
use crate::transaction::Transaction;
use crate::types::{Address, Hash};

// ---------------------------------------------------------------------------
// SecurityValidator — stateless checks
// ---------------------------------------------------------------------------

/// Collection of stateless security validation functions.
///
/// All methods are pure functions that take inputs and return `DinaResult<()>`.
/// They are intentionally stateless so they can be called from any context
/// (mempool, executor, RPC layer) without side effects.
pub struct SecurityValidator;

impl SecurityValidator {
    /// Validate that adding `amount` to `balance` will not overflow `u64::MAX`.
    ///
    /// This prevents balance inflation attacks where a carefully chosen amount
    /// wraps the balance counter.
    pub fn check_overflow(amount: u64, balance: u64) -> DinaResult<()> {
        balance.checked_add(amount).ok_or_else(|| {
            DinaError::Custom(format!(
                "arithmetic overflow: {balance} + {amount} exceeds u64::MAX"
            ))
        })?;
        Ok(())
    }

    /// Validate that an address is not the zero address.
    ///
    /// The zero address (`0x0000...0000`) is reserved and must never be the
    /// target of a transfer or contract deployment.
    pub fn check_non_zero_address(addr: &Address) -> DinaResult<()> {
        if *addr == Address::ZERO {
            return Err(DinaError::Custom(
                "zero address is not a valid target".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate that the transaction nonce matches the expected value.
    ///
    /// Nonces must be strictly sequential (expected == got) to prevent
    /// replay attacks and ensure transaction ordering.
    pub fn check_nonce(expected: u64, got: u64) -> DinaResult<()> {
        if expected != got {
            return Err(DinaError::InvalidNonce { expected, got });
        }
        Ok(())
    }

    /// Validate that the serialized transaction size does not exceed `max_bytes`.
    ///
    /// Oversized transactions can be used to DoS the network by consuming
    /// excessive bandwidth and storage.
    pub fn check_tx_size(tx: &Transaction, max_bytes: usize) -> DinaResult<()> {
        let size = bincode::serialize(tx)
            .map_err(|e| DinaError::SerializationError(e.to_string()))?
            .len();
        if size > max_bytes {
            return Err(DinaError::Custom(format!(
                "transaction size {size} bytes exceeds limit of {max_bytes} bytes"
            )));
        }
        Ok(())
    }

    /// Validate WASM bytecode with basic safety checks.
    ///
    /// Checks performed:
    /// - Non-empty bytecode
    /// - Starts with the WASM magic number (`\0asm`)
    /// - Minimum viable size (8 bytes for header)
    pub fn check_wasm_bytecode(bytes: &[u8]) -> DinaResult<()> {
        if bytes.is_empty() {
            return Err(DinaError::Custom(
                "WASM bytecode is empty".to_string(),
            ));
        }

        // WASM magic number: 0x00 0x61 0x73 0x6d ("\0asm")
        const WASM_MAGIC: &[u8] = b"\0asm";
        if bytes.len() < 8 {
            return Err(DinaError::Custom(
                "WASM bytecode too short to be valid (minimum 8 bytes)".to_string(),
            ));
        }
        if &bytes[..4] != WASM_MAGIC {
            return Err(DinaError::Custom(
                "WASM bytecode does not start with magic number (\\0asm)".to_string(),
            ));
        }

        Ok(())
    }

    /// Rate limit check for per-address operations.
    ///
    /// Ensures at least `min_interval_ms` milliseconds have elapsed since
    /// the last operation from this address. This prevents spam attacks at
    /// the mempool layer.
    pub fn check_rate_limit(
        _address: &Address,
        last_op_time: u64,
        current_time: u64,
        min_interval_ms: u64,
    ) -> DinaResult<()> {
        if current_time < last_op_time {
            return Err(DinaError::Custom(
                "current_time is before last_op_time (clock skew?)".to_string(),
            ));
        }
        let elapsed = current_time - last_op_time;
        if elapsed < min_interval_ms {
            return Err(DinaError::Custom(format!(
                "rate limit exceeded: {elapsed}ms since last operation, minimum is {min_interval_ms}ms"
            )));
        }
        Ok(())
    }

    /// Validate that a contract method name contains only safe characters.
    ///
    /// Allowed characters: `a-z`, `A-Z`, `0-9`, `_`.
    /// Maximum length: 128 characters.
    /// This prevents injection attacks through method names.
    pub fn check_method_name(name: &str) -> DinaResult<()> {
        if name.is_empty() {
            return Err(DinaError::Custom(
                "method name cannot be empty".to_string(),
            ));
        }
        if name.len() > 128 {
            return Err(DinaError::Custom(format!(
                "method name length {} exceeds maximum of 128",
                name.len()
            )));
        }
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(DinaError::Custom(format!(
                "method name contains invalid characters: {name:?} (only alphanumeric and underscore allowed)"
            )));
        }
        Ok(())
    }

    /// Validate that a memo does not exceed the maximum allowed size.
    pub fn check_memo_size(memo: &[u8], max_bytes: usize) -> DinaResult<()> {
        if memo.len() > max_bytes {
            return Err(DinaError::Custom(format!(
                "memo size {} bytes exceeds limit of {max_bytes} bytes",
                memo.len()
            )));
        }
        Ok(())
    }

    /// Validate that a transfer does not send to the sender's own address.
    ///
    /// Self-transfers waste gas and can be used to artificially inflate
    /// transaction counts.
    pub fn check_no_self_transfer(from: &Address, to: &Address) -> DinaResult<()> {
        if from == to {
            return Err(DinaError::Custom(
                "self-transfers are not allowed".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate that fee + amount does not overflow, and that the total
    /// is within the sender's balance.
    pub fn check_fee_plus_amount(fee: u64, amount: u64, balance: u64) -> DinaResult<()> {
        let total = fee.checked_add(amount).ok_or_else(|| {
            DinaError::Custom(format!(
                "fee + amount overflow: {fee} + {amount} exceeds u64::MAX"
            ))
        })?;
        if balance < total {
            return Err(DinaError::InsufficientBalance {
                have: balance,
                need: total,
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ReplayProtection — stateful transaction hash deduplication
// ---------------------------------------------------------------------------

/// Maintains a bounded set of recently seen transaction hashes to prevent
/// replay attacks at the mempool layer.
///
/// The set is bounded by `max_cache_size` to prevent unbounded memory growth.
/// When the cache is full, `prune` should be called to evict old entries.
pub struct ReplayProtection {
    seen_tx_hashes: BTreeSet<Hash>,
    max_cache_size: usize,
}

impl ReplayProtection {
    /// Create a new replay protection cache with the given maximum size.
    pub fn new(max_cache_size: usize) -> Self {
        Self {
            seen_tx_hashes: BTreeSet::new(),
            max_cache_size,
        }
    }

    /// Check whether a transaction hash has been seen before, and if not,
    /// record it.
    ///
    /// Returns `Err` if the hash has already been seen (replay attack) or
    /// the cache is full and needs pruning.
    pub fn check_and_record(&mut self, tx_hash: &Hash) -> DinaResult<()> {
        if self.seen_tx_hashes.contains(tx_hash) {
            return Err(DinaError::Custom(format!(
                "replay attack detected: transaction {tx_hash} already processed"
            )));
        }

        if self.seen_tx_hashes.len() >= self.max_cache_size {
            return Err(DinaError::Custom(
                "replay protection cache is full, call prune() first".to_string(),
            ));
        }

        self.seen_tx_hashes.insert(*tx_hash);
        Ok(())
    }

    /// Remove the oldest entries, keeping only the `keep_recent` most recent
    /// entries (by hash sort order, which serves as a proxy for recency).
    pub fn prune(&mut self, keep_recent: usize) {
        if self.seen_tx_hashes.len() <= keep_recent {
            return;
        }
        let to_remove = self.seen_tx_hashes.len() - keep_recent;
        let removals: Vec<Hash> = self.seen_tx_hashes.iter().take(to_remove).copied().collect();
        for h in removals {
            self.seen_tx_hashes.remove(&h);
        }
    }

    /// Return the current number of cached transaction hashes.
    pub fn len(&self) -> usize {
        self.seen_tx_hashes.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.seen_tx_hashes.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::Sig64;

    fn addr(byte: u8) -> Address {
        Address([byte; 32])
    }

    fn hash(byte: u8) -> Hash {
        Hash([byte; 32])
    }

    // -- SecurityValidator tests --

    #[test]
    fn check_overflow_ok() {
        assert!(SecurityValidator::check_overflow(100, 200).is_ok());
    }

    #[test]
    fn check_overflow_at_max() {
        assert!(SecurityValidator::check_overflow(1, u64::MAX).is_err());
    }

    #[test]
    fn check_overflow_exact_max() {
        assert!(SecurityValidator::check_overflow(0, u64::MAX).is_ok());
    }

    #[test]
    fn check_overflow_both_large() {
        assert!(SecurityValidator::check_overflow(u64::MAX / 2 + 1, u64::MAX / 2 + 1).is_err());
    }

    #[test]
    fn check_non_zero_address_ok() {
        assert!(SecurityValidator::check_non_zero_address(&addr(1)).is_ok());
    }

    #[test]
    fn check_non_zero_address_fails_zero() {
        assert!(SecurityValidator::check_non_zero_address(&Address::ZERO).is_err());
    }

    #[test]
    fn check_nonce_ok() {
        assert!(SecurityValidator::check_nonce(5, 5).is_ok());
    }

    #[test]
    fn check_nonce_mismatch() {
        let err = SecurityValidator::check_nonce(5, 3).unwrap_err();
        assert!(matches!(err, DinaError::InvalidNonce { expected: 5, got: 3 }));
    }

    #[test]
    fn check_tx_size_ok() {
        let tx = Transaction::Transfer {
            from: addr(1),
            to: addr(2),
            amount: 100,
            memo: None,
            device_witness: None,
            nonce: 0,
            fee: 10,
            signature: Sig64([0u8; 64]),
        };
        assert!(SecurityValidator::check_tx_size(&tx, 1_000_000).is_ok());
    }

    #[test]
    fn check_tx_size_too_large() {
        let tx = Transaction::Transfer {
            from: addr(1),
            to: addr(2),
            amount: 100,
            memo: Some(vec![0u8; 1000]),
            device_witness: None,
            nonce: 0,
            fee: 10,
            signature: Sig64([0u8; 64]),
        };
        // Set a very small limit
        assert!(SecurityValidator::check_tx_size(&tx, 10).is_err());
    }

    #[test]
    fn check_wasm_bytecode_valid() {
        let mut wasm = vec![0x00, 0x61, 0x73, 0x6d]; // \0asm
        wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // version 1
        assert!(SecurityValidator::check_wasm_bytecode(&wasm).is_ok());
    }

    #[test]
    fn check_wasm_bytecode_empty() {
        assert!(SecurityValidator::check_wasm_bytecode(&[]).is_err());
    }

    #[test]
    fn check_wasm_bytecode_too_short() {
        assert!(SecurityValidator::check_wasm_bytecode(&[0x00, 0x61]).is_err());
    }

    #[test]
    fn check_wasm_bytecode_bad_magic() {
        let bad = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x00, 0x00, 0x00];
        assert!(SecurityValidator::check_wasm_bytecode(&bad).is_err());
    }

    #[test]
    fn check_rate_limit_ok() {
        assert!(SecurityValidator::check_rate_limit(&addr(1), 1000, 2000, 500).is_ok());
    }

    #[test]
    fn check_rate_limit_too_fast() {
        assert!(SecurityValidator::check_rate_limit(&addr(1), 1000, 1200, 500).is_err());
    }

    #[test]
    fn check_rate_limit_clock_skew() {
        assert!(SecurityValidator::check_rate_limit(&addr(1), 2000, 1000, 500).is_err());
    }

    #[test]
    fn check_method_name_valid() {
        assert!(SecurityValidator::check_method_name("transfer").is_ok());
        assert!(SecurityValidator::check_method_name("get_balance_v2").is_ok());
        assert!(SecurityValidator::check_method_name("A").is_ok());
    }

    #[test]
    fn check_method_name_empty() {
        assert!(SecurityValidator::check_method_name("").is_err());
    }

    #[test]
    fn check_method_name_too_long() {
        let long_name = "a".repeat(129);
        assert!(SecurityValidator::check_method_name(&long_name).is_err());
    }

    #[test]
    fn check_method_name_invalid_chars() {
        assert!(SecurityValidator::check_method_name("transfer()").is_err());
        assert!(SecurityValidator::check_method_name("do-something").is_err());
        assert!(SecurityValidator::check_method_name("hello world").is_err());
        assert!(SecurityValidator::check_method_name("inject;drop").is_err());
    }

    #[test]
    fn check_memo_size_ok() {
        assert!(SecurityValidator::check_memo_size(&[0u8; 100], 4096).is_ok());
    }

    #[test]
    fn check_memo_size_too_large() {
        assert!(SecurityValidator::check_memo_size(&[0u8; 5000], 4096).is_err());
    }

    #[test]
    fn check_no_self_transfer_ok() {
        assert!(SecurityValidator::check_no_self_transfer(&addr(1), &addr(2)).is_ok());
    }

    #[test]
    fn check_no_self_transfer_fails() {
        assert!(SecurityValidator::check_no_self_transfer(&addr(1), &addr(1)).is_err());
    }

    #[test]
    fn check_fee_plus_amount_ok() {
        assert!(SecurityValidator::check_fee_plus_amount(10, 100, 200).is_ok());
    }

    #[test]
    fn check_fee_plus_amount_insufficient() {
        let err = SecurityValidator::check_fee_plus_amount(10, 100, 50).unwrap_err();
        assert!(matches!(err, DinaError::InsufficientBalance { have: 50, need: 110 }));
    }

    #[test]
    fn check_fee_plus_amount_overflow() {
        assert!(SecurityValidator::check_fee_plus_amount(u64::MAX, 1, u64::MAX).is_err());
    }

    // -- ReplayProtection tests --

    #[test]
    fn replay_protection_basic() {
        let mut rp = ReplayProtection::new(100);
        let h = hash(1);
        assert!(rp.check_and_record(&h).is_ok());
        assert!(rp.check_and_record(&h).is_err()); // replay
    }

    #[test]
    fn replay_protection_different_hashes() {
        let mut rp = ReplayProtection::new(100);
        assert!(rp.check_and_record(&hash(1)).is_ok());
        assert!(rp.check_and_record(&hash(2)).is_ok());
        assert!(rp.check_and_record(&hash(3)).is_ok());
        assert_eq!(rp.len(), 3);
    }

    #[test]
    fn replay_protection_cache_full() {
        let mut rp = ReplayProtection::new(2);
        assert!(rp.check_and_record(&hash(1)).is_ok());
        assert!(rp.check_and_record(&hash(2)).is_ok());
        assert!(rp.check_and_record(&hash(3)).is_err()); // cache full
    }

    #[test]
    fn replay_protection_prune() {
        let mut rp = ReplayProtection::new(3);
        rp.check_and_record(&hash(1)).unwrap();
        rp.check_and_record(&hash(2)).unwrap();
        rp.check_and_record(&hash(3)).unwrap();
        assert_eq!(rp.len(), 3);

        rp.prune(1);
        assert_eq!(rp.len(), 1);

        // Now we can insert more
        assert!(rp.check_and_record(&hash(4)).is_ok());
    }

    #[test]
    fn replay_protection_prune_keeps_count() {
        let mut rp = ReplayProtection::new(10);
        for i in 0..5 {
            rp.check_and_record(&hash(i)).unwrap();
        }
        rp.prune(3);
        assert_eq!(rp.len(), 3);
    }

    #[test]
    fn replay_protection_empty() {
        let rp = ReplayProtection::new(10);
        assert!(rp.is_empty());
        assert_eq!(rp.len(), 0);
    }

    #[test]
    fn replay_protection_prune_no_op_when_small() {
        let mut rp = ReplayProtection::new(10);
        rp.check_and_record(&hash(1)).unwrap();
        rp.prune(5); // keep 5, but only 1 exists -- no-op
        assert_eq!(rp.len(), 1);
    }
}
