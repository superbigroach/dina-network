//! Host function declarations and safe wrappers.
//!
//! When compiled to `wasm32`, these call into the Dina WASM runtime via imported
//! extern functions. On native targets, they provide stub implementations that
//! panic — allowing the SDK to be used for type-checking and testing structure
//! without a live runtime.

use crate::types::{Address, Hash};
use serde::Serialize;

// ---------------------------------------------------------------------------
// Raw host imports (WASM only)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
extern "C" {
    /// Returns a pointer to the 32-byte caller address in WASM linear memory.
    pub fn __host_caller() -> u32;
    /// Returns the current block timestamp (Unix seconds).
    pub fn __host_block_time() -> u64;
    /// Returns the current block height.
    pub fn __host_block_height() -> u64;
    /// Returns the USDC balance of the current contract (micro-units).
    pub fn __host_self_balance() -> u64;
    /// Transfers `amount` USDC micro-units to the address at `to_ptr`.
    /// Returns 0 on success, nonzero on failure.
    pub fn __host_transfer(to_ptr: u32, amount: u64) -> u32;
    /// Reads a value from contract storage. `key_ptr`/`key_len` identify the key.
    /// Returns a packed u64: high 32 bits = value pointer, low 32 bits = value length.
    /// Returns 0 if the key does not exist.
    pub fn __host_storage_get(key_ptr: u32, key_len: u32) -> u64;
    /// Writes a value to contract storage.
    pub fn __host_storage_set(key_ptr: u32, key_len: u32, val_ptr: u32, val_len: u32);
    /// Emits a named event with arbitrary data.
    pub fn __host_emit_event(name_ptr: u32, name_len: u32, data_ptr: u32, data_len: u32);
    /// Computes SHA-256 of the data. Returns a pointer to the 32-byte digest.
    pub fn __host_sha256(data_ptr: u32, data_len: u32) -> u32;
    /// Verifies an Ed25519 signature. Returns 1 if valid, 0 if invalid.
    pub fn __host_verify_ed25519(pubkey_ptr: u32, msg_ptr: u32, msg_len: u32, sig_ptr: u32) -> u32;
    /// Deletes a key from contract storage.
    pub fn __host_storage_delete(key_ptr: u32, key_len: u32);
}

// ---------------------------------------------------------------------------
// Safe wrappers — WASM implementations
// ---------------------------------------------------------------------------

/// Returns the address of the transaction caller.
#[cfg(target_arch = "wasm32")]
pub fn caller() -> Address {
    unsafe {
        let ptr = __host_caller();
        let slice = core::slice::from_raw_parts(ptr as *const u8, 32);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(slice);
        Address(bytes)
    }
}

/// Returns the current block timestamp in Unix seconds.
#[cfg(target_arch = "wasm32")]
pub fn block_time() -> u64 {
    unsafe { __host_block_time() }
}

/// Returns the current block height.
#[cfg(target_arch = "wasm32")]
pub fn block_height() -> u64 {
    unsafe { __host_block_height() }
}

/// Returns the USDC balance of this contract in micro-units.
#[cfg(target_arch = "wasm32")]
pub fn self_balance() -> u64 {
    unsafe { __host_self_balance() }
}

/// Transfers USDC to the given address.
///
/// # Panics
/// Panics if the transfer fails (e.g., insufficient balance).
#[cfg(target_arch = "wasm32")]
pub fn transfer_usdc(to: &Address, amount: u64) {
    unsafe {
        let result = __host_transfer(to.0.as_ptr() as u32, amount);
        if result != 0 {
            panic!("transfer failed with code {}", result);
        }
    }
}

/// Emits a contract event that can be indexed by the runtime.
#[cfg(target_arch = "wasm32")]
pub fn emit_event(name: &str, data: impl Serialize) {
    let serialized = serde_json::to_vec(&data).expect("failed to serialize event data");
    unsafe {
        __host_emit_event(
            name.as_ptr() as u32,
            name.len() as u32,
            serialized.as_ptr() as u32,
            serialized.len() as u32,
        );
    }
}

/// Computes the SHA-256 hash of the given data.
#[cfg(target_arch = "wasm32")]
pub fn sha256(data: &[u8]) -> Hash {
    unsafe {
        let ptr = __host_sha256(data.as_ptr() as u32, data.len() as u32);
        let slice = core::slice::from_raw_parts(ptr as *const u8, 32);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(slice);
        Hash(bytes)
    }
}

/// Verifies an Ed25519 signature.
///
/// Returns `true` if the signature is valid for the given public key and message.
#[cfg(target_arch = "wasm32")]
pub fn verify_ed25519(pubkey: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    unsafe {
        let result = __host_verify_ed25519(
            pubkey.as_ptr() as u32,
            message.as_ptr() as u32,
            message.len() as u32,
            signature.as_ptr() as u32,
        );
        result == 1
    }
}

// ---------------------------------------------------------------------------
// Raw host storage helpers used by the storage module (WASM)
// ---------------------------------------------------------------------------

/// Reads raw bytes from contract storage. Returns `None` if the key does not exist.
#[cfg(target_arch = "wasm32")]
pub fn storage_get_raw(key: &[u8]) -> Option<Vec<u8>> {
    unsafe {
        let packed = __host_storage_get(key.as_ptr() as u32, key.len() as u32);
        if packed == 0 {
            return None;
        }
        let ptr = (packed >> 32) as u32;
        let len = (packed & 0xFFFFFFFF) as u32;
        let slice = core::slice::from_raw_parts(ptr as *const u8, len as usize);
        Some(slice.to_vec())
    }
}

/// Writes raw bytes to contract storage.
#[cfg(target_arch = "wasm32")]
pub fn storage_set_raw(key: &[u8], value: &[u8]) {
    unsafe {
        __host_storage_set(
            key.as_ptr() as u32,
            key.len() as u32,
            value.as_ptr() as u32,
            value.len() as u32,
        );
    }
}

/// Deletes a key from contract storage.
#[cfg(target_arch = "wasm32")]
pub fn storage_delete_raw(key: &[u8]) {
    unsafe {
        __host_storage_delete(key.as_ptr() as u32, key.len() as u32);
    }
}

// ---------------------------------------------------------------------------
// Safe wrappers — Native stubs (for type-checking and unit tests)
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
pub fn caller() -> Address {
    panic!("caller() is not available outside WASM context")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn block_time() -> u64 {
    panic!("block_time() is not available outside WASM context")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn block_height() -> u64 {
    panic!("block_height() is not available outside WASM context")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn self_balance() -> u64 {
    panic!("self_balance() is not available outside WASM context")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn transfer_usdc(_to: &Address, _amount: u64) {
    panic!("transfer_usdc() is not available outside WASM context")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn emit_event(_name: &str, _data: impl Serialize) {
    panic!("emit_event() is not available outside WASM context")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn sha256(_data: &[u8]) -> Hash {
    panic!("sha256() is not available outside WASM context")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn verify_ed25519(_pubkey: &[u8; 32], _message: &[u8], _signature: &[u8; 64]) -> bool {
    panic!("verify_ed25519() is not available outside WASM context")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn storage_get_raw(_key: &[u8]) -> Option<Vec<u8>> {
    panic!("storage_get_raw() is not available outside WASM context")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn storage_set_raw(_key: &[u8], _value: &[u8]) {
    panic!("storage_set_raw() is not available outside WASM context")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn storage_delete_raw(_key: &[u8]) {
    panic!("storage_delete_raw() is not available outside WASM context")
}
