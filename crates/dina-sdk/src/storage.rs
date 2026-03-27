//! Contract storage abstraction.
//!
//! Provides a [`Map`] type that persists key-value pairs in contract storage
//! via the host functions. Keys are serialized with Borsh and prefixed with a
//! namespace to avoid collisions between different maps in the same contract.

use borsh::{BorshDeserialize, BorshSerialize};
use core::marker::PhantomData;

use crate::host;

/// A persistent key-value map backed by contract storage.
///
/// Each `Map` has a namespace prefix so that multiple maps within the same
/// contract do not collide. Keys and values are serialized using Borsh.
///
/// # Example
/// ```ignore
/// use dina_sdk::storage::Map;
/// use dina_sdk::types::Address;
///
/// let balances: Map<Address, u64> = Map::new(b"balances");
/// balances.set(&owner, &1000u64);
/// assert_eq!(balances.get(&owner), Some(1000));
/// ```
pub struct Map<K, V> {
    prefix: &'static [u8],
    _marker: PhantomData<(K, V)>,
}

impl<K, V> Map<K, V>
where
    K: BorshSerialize,
    V: BorshSerialize + BorshDeserialize,
{
    /// Creates a new map with the given namespace prefix.
    ///
    /// The prefix must be unique within the contract to avoid key collisions.
    /// This is a `const fn` so it can be used in static/const contexts within
    /// contract structs.
    pub const fn new(prefix: &'static [u8]) -> Self {
        Self {
            prefix,
            _marker: PhantomData,
        }
    }

    /// Builds the full storage key by concatenating the namespace prefix,
    /// a separator byte, and the Borsh-serialized key.
    fn storage_key(&self, key: &K) -> Vec<u8> {
        let key_bytes = borsh::to_vec(key).expect("failed to serialize map key");
        let mut full_key = Vec::with_capacity(self.prefix.len() + 1 + key_bytes.len());
        full_key.extend_from_slice(self.prefix);
        full_key.push(b':'); // separator
        full_key.extend_from_slice(&key_bytes);
        full_key
    }

    /// Retrieves the value associated with `key`, or `None` if it does not exist.
    pub fn get(&self, key: &K) -> Option<V> {
        let storage_key = self.storage_key(key);
        let raw = host::storage_get_raw(&storage_key)?;
        let value = V::try_from_slice(&raw).expect("failed to deserialize map value");
        Some(value)
    }

    /// Stores a value at the given key, overwriting any previous value.
    pub fn set(&self, key: &K, value: &V) {
        let storage_key = self.storage_key(key);
        let value_bytes = borsh::to_vec(value).expect("failed to serialize map value");
        host::storage_set_raw(&storage_key, &value_bytes);
    }

    /// Removes the value at the given key.
    pub fn remove(&self, key: &K) {
        let storage_key = self.storage_key(key);
        host::storage_delete_raw(&storage_key);
    }

    /// Returns `true` if the map contains a value for the given key.
    pub fn contains(&self, key: &K) -> bool {
        let storage_key = self.storage_key(key);
        host::storage_get_raw(&storage_key).is_some()
    }

    /// Gets the value for `key`, or inserts and returns `default` if it does not exist.
    pub fn get_or_insert(&self, key: &K, default: &V) -> V {
        match self.get(key) {
            Some(v) => v,
            None => {
                self.set(key, default);
                // Re-deserialize to return an owned copy consistent with what was stored
                let storage_key = self.storage_key(key);
                let raw = host::storage_get_raw(&storage_key)
                    .expect("value should exist after set");
                V::try_from_slice(&raw).expect("failed to deserialize map value")
            }
        }
    }
}

// Map is Send + Sync since it only holds a prefix and PhantomData.
// The actual storage is managed by the host runtime.
unsafe impl<K, V> Send for Map<K, V> {}
unsafe impl<K, V> Sync for Map<K, V> {}
