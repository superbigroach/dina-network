use std::collections::HashMap;

use dina_core::types::Address;
use dina_core::{Account, Block};

/// Configuration for the state read cache.
#[derive(Clone, Debug)]
pub struct CacheConfig {
    /// Maximum number of account entries in the cache.
    pub max_account_entries: usize,
    /// Maximum number of block entries in the cache.
    pub max_block_entries: usize,
    /// Time-to-live for cache entries in seconds.
    pub ttl_seconds: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_account_entries: 10_000,
            max_block_entries: 100,
            ttl_seconds: 300,
        }
    }
}

/// A cache entry wrapping a value with access and creation timestamps.
#[derive(Clone, Debug)]
pub struct CacheEntry<T> {
    /// The cached value.
    pub value: T,
    /// Timestamp (seconds) when the entry was last accessed.
    pub accessed_at: u64,
    /// Timestamp (seconds) when the entry was created.
    pub created_at: u64,
}

/// Aggregate statistics about cache usage.
#[derive(Clone, Debug)]
pub struct CacheStats {
    /// Number of account entries currently cached.
    pub account_entries: usize,
    /// Number of block entries currently cached.
    pub block_entries: usize,
    /// Hit rate as a fraction (0.0 to 1.0).
    pub hit_rate: f64,
    /// Total number of cache hits.
    pub total_hits: u64,
    /// Total number of cache misses.
    pub total_misses: u64,
}

/// An in-memory read cache for accounts and blocks with LRU-style eviction
/// and TTL-based expiry.
pub struct StateCache {
    accounts: HashMap<Address, CacheEntry<Account>>,
    blocks: HashMap<u64, CacheEntry<Block>>,
    config: CacheConfig,
    total_hits: u64,
    total_misses: u64,
    /// Current time used for access tracking. Call `set_time` or use a real
    /// clock externally. For production use, callers update this before each
    /// operation.
    current_time: u64,
}

impl StateCache {
    /// Create a new `StateCache` with the given configuration.
    pub fn new(config: CacheConfig) -> Self {
        Self {
            accounts: HashMap::new(),
            blocks: HashMap::new(),
            config,
            total_hits: 0,
            total_misses: 0,
            current_time: 0,
        }
    }

    /// Set the current timestamp (seconds since epoch) used for TTL and access tracking.
    pub fn set_time(&mut self, time: u64) {
        self.current_time = time;
    }

    /// Look up an account in the cache. Returns `None` on a miss.
    /// Updates the `accessed_at` timestamp on a hit.
    pub fn get_account(&mut self, address: &Address) -> Option<&Account> {
        let now = self.current_time;
        let ttl = self.config.ttl_seconds;

        // Check if entry exists and whether it is expired.
        let expired = match self.accounts.get(address) {
            Some(entry) => now.saturating_sub(entry.created_at) >= ttl,
            None => {
                self.total_misses += 1;
                return None;
            }
        };

        if expired {
            self.accounts.remove(address);
            self.total_misses += 1;
            return None;
        }

        self.total_hits += 1;
        let entry = self.accounts.get_mut(address).unwrap();
        entry.accessed_at = now;
        Some(&entry.value)
    }

    /// Insert or update an account in the cache.
    /// If the cache exceeds `max_account_entries`, the least-recently-accessed
    /// entry is evicted.
    pub fn set_account(&mut self, address: Address, account: Account) {
        if self.accounts.len() >= self.config.max_account_entries
            && !self.accounts.contains_key(&address)
        {
            self.evict_lru_account();
        }
        self.accounts.insert(
            address,
            CacheEntry {
                value: account,
                accessed_at: self.current_time,
                created_at: self.current_time,
            },
        );
    }

    /// Look up a block in the cache by height. Returns `None` on a miss.
    pub fn get_block(&mut self, height: u64) -> Option<&Block> {
        let now = self.current_time;
        let ttl = self.config.ttl_seconds;

        let expired = match self.blocks.get(&height) {
            Some(entry) => now.saturating_sub(entry.created_at) >= ttl,
            None => {
                self.total_misses += 1;
                return None;
            }
        };

        if expired {
            self.blocks.remove(&height);
            self.total_misses += 1;
            return None;
        }

        self.total_hits += 1;
        let entry = self.blocks.get_mut(&height).unwrap();
        entry.accessed_at = now;
        Some(&entry.value)
    }

    /// Insert or update a block in the cache.
    pub fn set_block(&mut self, height: u64, block: Block) {
        if self.blocks.len() >= self.config.max_block_entries && !self.blocks.contains_key(&height)
        {
            self.evict_lru_block();
        }
        self.blocks.insert(
            height,
            CacheEntry {
                value: block,
                accessed_at: self.current_time,
                created_at: self.current_time,
            },
        );
    }

    /// Remove a specific account from the cache.
    pub fn invalidate_account(&mut self, address: &Address) {
        self.accounts.remove(address);
    }

    /// Remove a specific block from the cache.
    pub fn invalidate_block(&mut self, height: u64) {
        self.blocks.remove(&height);
    }

    /// Clear all entries from the cache.
    pub fn clear(&mut self) {
        self.accounts.clear();
        self.blocks.clear();
        self.total_hits = 0;
        self.total_misses = 0;
    }

    /// Return current cache statistics.
    pub fn stats(&self) -> CacheStats {
        let total = self.total_hits + self.total_misses;
        let hit_rate = if total == 0 {
            0.0
        } else {
            self.total_hits as f64 / total as f64
        };
        CacheStats {
            account_entries: self.accounts.len(),
            block_entries: self.blocks.len(),
            hit_rate,
            total_hits: self.total_hits,
            total_misses: self.total_misses,
        }
    }

    /// Remove all entries whose TTL has expired relative to the given time.
    pub fn evict_expired(&mut self, current_time: u64) {
        let ttl = self.config.ttl_seconds;
        self.accounts
            .retain(|_, entry| current_time.saturating_sub(entry.created_at) < ttl);
        self.blocks
            .retain(|_, entry| current_time.saturating_sub(entry.created_at) < ttl);
    }

    /// Evict the least-recently-accessed account entry.
    fn evict_lru_account(&mut self) {
        if let Some((&lru_addr, _)) = self
            .accounts
            .iter()
            .min_by_key(|(_, entry)| entry.accessed_at)
        {
            self.accounts.remove(&lru_addr);
        }
    }

    /// Evict the least-recently-accessed block entry.
    fn evict_lru_block(&mut self) {
        if let Some((&lru_height, _)) = self
            .blocks
            .iter()
            .min_by_key(|(_, entry)| entry.accessed_at)
        {
            self.blocks.remove(&lru_height);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dina_core::block::BlockHeader;
    use dina_core::types::Hash;

    fn make_address(id: u8) -> Address {
        Address([id; 32])
    }

    fn make_account(id: u8, balance: u64) -> Account {
        Account::with_balance(make_address(id), balance)
    }

    fn make_block(height: u64) -> Block {
        Block {
            header: BlockHeader {
                block_number: height,
                timestamp: 1_700_000_000 + height,
                parent_hash: Hash::ZERO,
                transactions_root: Hash::ZERO,
                state_root: Hash::ZERO,
                proposer: Address::ZERO,
                proposer_pubkey: [0u8; 32],
                signature: [0u8; 64],
            },
            transactions: vec![],
        }
    }

    fn default_cache() -> StateCache {
        StateCache::new(CacheConfig::default())
    }

    #[test]
    fn default_config_values() {
        let cfg = CacheConfig::default();
        assert_eq!(cfg.max_account_entries, 10_000);
        assert_eq!(cfg.max_block_entries, 100);
        assert_eq!(cfg.ttl_seconds, 300);
    }

    #[test]
    fn account_cache_hit() {
        let mut cache = default_cache();
        let addr = make_address(1);
        cache.set_account(addr, make_account(1, 500));

        let result = cache.get_account(&addr);
        assert!(result.is_some());
        assert_eq!(result.unwrap().balance, 500);
    }

    #[test]
    fn account_cache_miss() {
        let mut cache = default_cache();
        let addr = make_address(99);
        assert!(cache.get_account(&addr).is_none());
    }

    #[test]
    fn block_cache_hit() {
        let mut cache = default_cache();
        cache.set_block(42, make_block(42));

        let result = cache.get_block(42);
        assert!(result.is_some());
        assert_eq!(result.unwrap().header.block_number, 42);
    }

    #[test]
    fn block_cache_miss() {
        let mut cache = default_cache();
        assert!(cache.get_block(999).is_none());
    }

    #[test]
    fn invalidate_account_removes_entry() {
        let mut cache = default_cache();
        let addr = make_address(5);
        cache.set_account(addr, make_account(5, 100));

        cache.invalidate_account(&addr);
        assert!(cache.get_account(&addr).is_none());
    }

    #[test]
    fn invalidate_block_removes_entry() {
        let mut cache = default_cache();
        cache.set_block(10, make_block(10));

        cache.invalidate_block(10);
        assert!(cache.get_block(10).is_none());
    }

    #[test]
    fn clear_removes_all_entries() {
        let mut cache = default_cache();
        cache.set_account(make_address(1), make_account(1, 100));
        cache.set_account(make_address(2), make_account(2, 200));
        cache.set_block(1, make_block(1));

        cache.clear();

        let stats = cache.stats();
        assert_eq!(stats.account_entries, 0);
        assert_eq!(stats.block_entries, 0);
    }

    #[test]
    fn stats_tracks_hits_and_misses() {
        let mut cache = default_cache();
        let addr = make_address(1);
        cache.set_account(addr, make_account(1, 100));

        // 1 hit
        cache.get_account(&addr);
        // 1 miss
        cache.get_account(&make_address(99));

        let stats = cache.stats();
        assert_eq!(stats.total_hits, 1);
        assert_eq!(stats.total_misses, 1);
        assert!((stats.hit_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn ttl_expiry_evicts_stale_entries() {
        let mut cache = StateCache::new(CacheConfig {
            ttl_seconds: 60,
            ..CacheConfig::default()
        });

        cache.set_time(1000);
        cache.set_account(make_address(1), make_account(1, 100));

        // Still fresh.
        cache.set_time(1050);
        assert!(cache.get_account(&make_address(1)).is_some());

        // Expired after 60 seconds.
        cache.set_time(1061);
        assert!(cache.get_account(&make_address(1)).is_none());
    }

    #[test]
    fn evict_expired_removes_old_entries() {
        let mut cache = StateCache::new(CacheConfig {
            ttl_seconds: 10,
            ..CacheConfig::default()
        });

        cache.set_time(100);
        cache.set_account(make_address(1), make_account(1, 100));
        cache.set_block(1, make_block(1));

        cache.set_time(105);
        cache.set_account(make_address(2), make_account(2, 200));

        // Evict at time 111: entry 1 (created at 100) expired, entry 2 (created at 105) still valid.
        cache.evict_expired(111);

        assert_eq!(cache.stats().account_entries, 1);
        assert_eq!(cache.stats().block_entries, 0);
    }

    #[test]
    fn lru_eviction_removes_oldest_accessed_account() {
        let mut cache = StateCache::new(CacheConfig {
            max_account_entries: 3,
            ttl_seconds: 300,
            ..CacheConfig::default()
        });

        cache.set_time(1);
        cache.set_account(make_address(1), make_account(1, 100));
        cache.set_time(2);
        cache.set_account(make_address(2), make_account(2, 200));
        cache.set_time(3);
        cache.set_account(make_address(3), make_account(3, 300));

        // Access account 1 to make it recently used.
        cache.set_time(4);
        cache.get_account(&make_address(1));

        // Insert a 4th account -- should evict account 2 (LRU).
        cache.set_time(5);
        cache.set_account(make_address(4), make_account(4, 400));

        assert_eq!(cache.stats().account_entries, 3);
        assert!(cache.get_account(&make_address(1)).is_some());
        // Account 2 was evicted (accessed_at = 2, the oldest).
        // We need to check without triggering a miss counter issue.
        // Since get_account increments miss counter, just verify we have the right set.
        assert!(cache.get_account(&make_address(3)).is_some());
        assert!(cache.get_account(&make_address(4)).is_some());
    }

    #[test]
    fn lru_eviction_removes_oldest_accessed_block() {
        let mut cache = StateCache::new(CacheConfig {
            max_block_entries: 2,
            ttl_seconds: 300,
            ..CacheConfig::default()
        });

        cache.set_time(1);
        cache.set_block(1, make_block(1));
        cache.set_time(2);
        cache.set_block(2, make_block(2));

        // Insert a 3rd block -- should evict block 1 (LRU).
        cache.set_time(3);
        cache.set_block(3, make_block(3));

        assert_eq!(cache.stats().block_entries, 2);
        assert!(cache.get_block(2).is_some());
        assert!(cache.get_block(3).is_some());
    }

    #[test]
    fn overwrite_existing_account_does_not_evict() {
        let mut cache = StateCache::new(CacheConfig {
            max_account_entries: 2,
            ttl_seconds: 300,
            ..CacheConfig::default()
        });

        cache.set_account(make_address(1), make_account(1, 100));
        cache.set_account(make_address(2), make_account(2, 200));

        // Overwrite account 1 -- should not trigger eviction.
        cache.set_account(make_address(1), make_account(1, 999));

        assert_eq!(cache.stats().account_entries, 2);
        let acc = cache.get_account(&make_address(1)).unwrap();
        assert_eq!(acc.balance, 999);
    }

    #[test]
    fn stats_hit_rate_zero_when_no_accesses() {
        let cache = default_cache();
        let stats = cache.stats();
        assert_eq!(stats.hit_rate, 0.0);
        assert_eq!(stats.total_hits, 0);
        assert_eq!(stats.total_misses, 0);
    }
}
