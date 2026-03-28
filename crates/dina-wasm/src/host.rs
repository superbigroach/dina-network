use std::collections::HashMap;

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use wasmtime::{AsContext, AsContextMut, Caller, Linker};

use dina_core::types::Address;

use crate::gas::DEFAULT_GAS_COSTS;
use crate::sandbox::SandboxLimits;

/// An event emitted by a contract during execution.
#[derive(Debug, Clone)]
pub struct ContractEvent {
    pub name: String,
    pub data: Vec<u8>,
}

/// A pending USDC transfer queued by a contract during execution.
#[derive(Debug, Clone)]
pub struct PendingTransfer {
    pub to: Address,
    pub amount: u64,
}

/// State accessible to host functions via `Caller<WasmHostState>`.
///
/// This is the bridge between the WASM sandbox and the blockchain runtime.
/// Each contract call gets its own `WasmHostState` instance.
pub struct WasmHostState {
    /// Address of the account that invoked this contract call.
    pub caller: Address,
    /// Current block timestamp (seconds since Unix epoch).
    pub block_time: u64,
    /// Current block height.
    pub block_height: u64,
    /// Address of the contract being executed.
    pub contract_address: Address,
    /// USDC micro-units attached to this call.
    pub usdc_attached: u64,
    /// Gas remaining for this execution (tracked via wasmtime fuel).
    pub gas_remaining: u64,
    /// In-memory contract storage overlay (flushed to persistent storage on success).
    pub storage: HashMap<Vec<u8>, Vec<u8>>,
    /// Events emitted during this call.
    pub events: Vec<ContractEvent>,
    /// Transfers queued during this call.
    pub transfers: Vec<PendingTransfer>,
    /// Counter for storage writes (for sandbox limit enforcement).
    pub storage_write_count: u32,
    /// Contract's USDC balance.
    pub self_balance: u64,
    /// Sandbox limits for this execution.
    pub limits: SandboxLimits,
}

impl WasmHostState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        caller: Address,
        contract_address: Address,
        usdc_attached: u64,
        gas_limit: u64,
        self_balance: u64,
        block_time: u64,
        block_height: u64,
        storage: HashMap<Vec<u8>, Vec<u8>>,
        limits: SandboxLimits,
    ) -> Self {
        Self {
            caller,
            block_time,
            block_height,
            contract_address,
            usdc_attached,
            gas_remaining: gas_limit,
            storage,
            events: Vec::new(),
            transfers: Vec::new(),
            storage_write_count: 0,
            self_balance,
            limits,
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: consume fuel (gas) from the caller's store
// ---------------------------------------------------------------------------
fn consume_fuel(caller: &mut Caller<'_, WasmHostState>, amount: u64) {
    if let Ok(current) = caller.get_fuel() {
        let _ = caller.set_fuel(current.saturating_sub(amount));
    }
}

// ---------------------------------------------------------------------------
// Helper: read a byte slice from WASM linear memory
// ---------------------------------------------------------------------------
fn read_wasm_memory(
    caller: &mut Caller<'_, WasmHostState>,
    ptr: u32,
    len: u32,
) -> Result<Vec<u8>, wasmtime::Error> {
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| wasmtime::Error::msg("contract has no exported memory"))?;
    let data = memory.data(&*caller);
    let start = ptr as usize;
    let end = start
        .checked_add(len as usize)
        .ok_or_else(|| wasmtime::Error::msg("memory read overflow"))?;
    if end > data.len() {
        return Err(wasmtime::Error::msg("memory read out of bounds"));
    }
    Ok(data[start..end].to_vec())
}

// ---------------------------------------------------------------------------
// Helper: write bytes into WASM linear memory starting at `ptr`
// ---------------------------------------------------------------------------
fn write_wasm_memory(
    caller: &mut Caller<'_, WasmHostState>,
    ptr: u32,
    bytes: &[u8],
) -> Result<(), wasmtime::Error> {
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| wasmtime::Error::msg("contract has no exported memory"))?;
    let data = memory.data_mut(caller);
    let start = ptr as usize;
    let end = start
        .checked_add(bytes.len())
        .ok_or_else(|| wasmtime::Error::msg("memory write overflow"))?;
    if end > data.len() {
        return Err(wasmtime::Error::msg("memory write out of bounds"));
    }
    data[start..end].copy_from_slice(bytes);
    Ok(())
}

// ---------------------------------------------------------------------------
// Helper: allocate bytes inside the WASM guest by calling its __alloc export
// ---------------------------------------------------------------------------
fn guest_alloc(caller: &mut Caller<'_, WasmHostState>, size: u32) -> Result<u32, wasmtime::Error> {
    let alloc_fn = caller
        .get_export("__alloc")
        .and_then(|e| e.into_func())
        .ok_or_else(|| wasmtime::Error::msg("contract must export __alloc(size: i32) -> i32"))?;
    let typed = alloc_fn.typed::<i32, i32>(caller.as_context())?;
    let ptr = typed.call(caller.as_context_mut(), size as i32)?;
    Ok(ptr as u32)
}

// ---------------------------------------------------------------------------
// Register all host functions on a wasmtime::Linker
// ---------------------------------------------------------------------------

/// Register Dina host functions into the provided `Linker`.
///
/// These are the functions that contracts compiled with `dina-sdk` can call.
/// The module name is `"env"` to match the SDK's `extern "C"` imports.
pub fn register_host_functions(linker: &mut Linker<WasmHostState>) -> Result<(), wasmtime::Error> {
    // __host_caller(out_ptr: i32)
    // Writes the 32-byte caller address to the given pointer.
    linker.func_wrap(
        "env",
        "__host_caller",
        |mut caller: Caller<'_, WasmHostState>, out_ptr: i32| -> i32 {
            let addr_bytes = *caller.data().caller.as_bytes();
            if let Err(e) = write_wasm_memory(&mut caller, out_ptr as u32, &addr_bytes) {
                tracing::error!("__host_caller: {e}");
                return -1;
            }
            // Consume gas for memory write
            consume_fuel(&mut caller, DEFAULT_GAS_COSTS.memory_write);
            out_ptr
        },
    )?;

    // __host_block_time() -> i64
    linker.func_wrap(
        "env",
        "__host_block_time",
        |caller: Caller<'_, WasmHostState>| -> i64 { caller.data().block_time as i64 },
    )?;

    // __host_block_height() -> i64
    linker.func_wrap(
        "env",
        "__host_block_height",
        |caller: Caller<'_, WasmHostState>| -> i64 { caller.data().block_height as i64 },
    )?;

    // __host_self_balance() -> i64
    linker.func_wrap(
        "env",
        "__host_self_balance",
        |caller: Caller<'_, WasmHostState>| -> i64 { caller.data().self_balance as i64 },
    )?;

    // __host_transfer(to_ptr: i32, amount: i64) -> i32
    // Returns 0 on success, 1 on failure.
    linker.func_wrap(
        "env",
        "__host_transfer",
        |mut caller: Caller<'_, WasmHostState>, to_ptr: i32, amount: i64| -> i32 {
            consume_fuel(&mut caller, DEFAULT_GAS_COSTS.transfer);

            // Reject negative amounts -- a malicious contract could pass a
            // negative i64 which, when cast to u64 via `as`, wraps to a
            // very large positive value, draining the contract balance.
            if amount <= 0 {
                return 1; // zero or negative amount transfer is invalid
            }
            let amount = amount as u64;

            // Read destination address from WASM memory
            let to_bytes = match read_wasm_memory(&mut caller, to_ptr as u32, 32) {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("__host_transfer: failed to read to_ptr: {e}");
                    return 1;
                }
            };

            let balance = caller.data().self_balance;
            if amount > balance {
                return 1; // insufficient balance
            }

            let mut addr = [0u8; 32];
            addr.copy_from_slice(&to_bytes);

            caller.data_mut().self_balance -= amount;
            caller.data_mut().transfers.push(PendingTransfer {
                to: Address(addr),
                amount,
            });

            0
        },
    )?;

    // __host_storage_get(key_ptr: i32, key_len: i32) -> i64
    // Returns a packed (ptr << 32 | len) for the value, or 0 if key not found.
    linker.func_wrap(
        "env",
        "__host_storage_get",
        |mut caller: Caller<'_, WasmHostState>, key_ptr: i32, key_len: i32| -> i64 {
            consume_fuel(&mut caller, DEFAULT_GAS_COSTS.storage_read);

            let key = match read_wasm_memory(&mut caller, key_ptr as u32, key_len as u32) {
                Ok(k) => k,
                Err(e) => {
                    tracing::error!("__host_storage_get: failed to read key: {e}");
                    return 0;
                }
            };

            let value = match caller.data().storage.get(&key) {
                Some(v) => v.clone(),
                None => return 0,
            };

            let val_len = value.len() as u32;
            let val_ptr = match guest_alloc(&mut caller, val_len) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("__host_storage_get: alloc failed: {e}");
                    return 0;
                }
            };

            if let Err(e) = write_wasm_memory(&mut caller, val_ptr, &value) {
                tracing::error!("__host_storage_get: write failed: {e}");
                return 0;
            }

            // Pack pointer (high 32 bits) and length (low 32 bits) into i64
            ((val_ptr as i64) << 32) | (val_len as i64)
        },
    )?;

    // __host_storage_set(key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32)
    linker.func_wrap(
        "env",
        "__host_storage_set",
        |mut caller: Caller<'_, WasmHostState>,
         key_ptr: i32,
         key_len: i32,
         val_ptr: i32,
         val_len: i32| {
            consume_fuel(&mut caller, DEFAULT_GAS_COSTS.storage_write);

            let key = match read_wasm_memory(&mut caller, key_ptr as u32, key_len as u32) {
                Ok(k) => k,
                Err(e) => {
                    tracing::error!("__host_storage_set: key read failed: {e}");
                    return;
                }
            };

            let value = match read_wasm_memory(&mut caller, val_ptr as u32, val_len as u32) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("__host_storage_set: value read failed: {e}");
                    return;
                }
            };

            // Check sandbox limits
            let write_count = caller.data().storage_write_count + 1;
            let limits = caller.data().limits.clone();
            if let Err(violation) = limits.validate_within_limits(write_count, 0, 0) {
                tracing::warn!("__host_storage_set: {violation}");
                return;
            }

            caller.data_mut().storage_write_count = write_count;
            caller.data_mut().storage.insert(key, value);
        },
    )?;

    // __host_emit_event(name_ptr: i32, name_len: i32, data_ptr: i32, data_len: i32)
    linker.func_wrap(
        "env",
        "__host_emit_event",
        |mut caller: Caller<'_, WasmHostState>,
         name_ptr: i32,
         name_len: i32,
         data_ptr: i32,
         data_len: i32| {
            consume_fuel(&mut caller, DEFAULT_GAS_COSTS.emit_event);

            let event_count = caller.data().events.len() as u32 + 1;
            let limits = caller.data().limits.clone();
            if let Err(violation) = limits.validate_within_limits(0, 0, event_count) {
                tracing::warn!("__host_emit_event: {violation}");
                return;
            }

            let name_bytes = match read_wasm_memory(&mut caller, name_ptr as u32, name_len as u32) {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("__host_emit_event: name read failed: {e}");
                    return;
                }
            };

            let data = match read_wasm_memory(&mut caller, data_ptr as u32, data_len as u32) {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("__host_emit_event: data read failed: {e}");
                    return;
                }
            };

            let name = String::from_utf8_lossy(&name_bytes).into_owned();
            caller.data_mut().events.push(ContractEvent { name, data });
        },
    )?;

    // __host_sha256(data_ptr: i32, data_len: i32) -> i32
    // Allocates 32 bytes in guest memory and writes the SHA-256 hash there.
    // Returns pointer to the 32-byte result.
    linker.func_wrap(
        "env",
        "__host_sha256",
        |mut caller: Caller<'_, WasmHostState>, data_ptr: i32, data_len: i32| -> i32 {
            consume_fuel(&mut caller, DEFAULT_GAS_COSTS.sha256);

            let data = match read_wasm_memory(&mut caller, data_ptr as u32, data_len as u32) {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("__host_sha256: data read failed: {e}");
                    return 0;
                }
            };

            let hash = Sha256::digest(&data);

            let out_ptr = match guest_alloc(&mut caller, 32) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("__host_sha256: alloc failed: {e}");
                    return 0;
                }
            };

            if let Err(e) = write_wasm_memory(&mut caller, out_ptr, &hash) {
                tracing::error!("__host_sha256: write failed: {e}");
                return 0;
            }

            out_ptr as i32
        },
    )?;

    // __host_verify_ed25519(pubkey_ptr: i32, msg_ptr: i32, msg_len: i32, sig_ptr: i32) -> i32
    // pubkey is 32 bytes, sig is 64 bytes (both at fixed offsets).
    // Returns 1 if valid, 0 if invalid.
    linker.func_wrap(
        "env",
        "__host_verify_ed25519",
        |mut caller: Caller<'_, WasmHostState>,
         pubkey_ptr: i32,
         msg_ptr: i32,
         msg_len: i32,
         sig_ptr: i32|
         -> i32 {
            consume_fuel(&mut caller, DEFAULT_GAS_COSTS.ed25519_verify);

            let pubkey_bytes = match read_wasm_memory(&mut caller, pubkey_ptr as u32, 32) {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("__host_verify_ed25519: pubkey read failed: {e}");
                    return 0;
                }
            };

            let msg = match read_wasm_memory(&mut caller, msg_ptr as u32, msg_len as u32) {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("__host_verify_ed25519: msg read failed: {e}");
                    return 0;
                }
            };

            let sig_bytes = match read_wasm_memory(&mut caller, sig_ptr as u32, 64) {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("__host_verify_ed25519: sig read failed: {e}");
                    return 0;
                }
            };

            // Parse public key
            let pubkey_arr: [u8; 32] = match pubkey_bytes.try_into() {
                Ok(a) => a,
                Err(_) => return 0,
            };
            let verifying_key = match VerifyingKey::from_bytes(&pubkey_arr) {
                Ok(k) => k,
                Err(_) => return 0,
            };

            // Parse signature
            let sig_arr: [u8; 64] = match sig_bytes.try_into() {
                Ok(a) => a,
                Err(_) => return 0,
            };
            let signature = Signature::from_bytes(&sig_arr);

            // Verify
            match verifying_key.verify(&msg, &signature) {
                Ok(()) => 1,
                Err(_) => 0,
            }
        },
    )?;

    Ok(())
}
