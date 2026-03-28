use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};
use tracing::{debug, info};
use wasmtime::{Config, Engine, Linker, Module, Store};

use dina_core::error::DinaError;
use dina_core::types::Address;

use crate::host::{self, ContractEvent, PendingTransfer, WasmHostState};
use crate::sandbox::SandboxLimits;

/// Configuration for the WASM runtime.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum WASM linear memory in bytes.
    pub max_memory_bytes: usize,
    /// Default maximum gas (fuel) for a single call.
    pub max_gas: u64,
    /// Maximum nested cross-contract call depth.
    pub max_call_depth: u32,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 16 * 1024 * 1024, // 16 MiB
            max_gas: 10_000_000,
            max_call_depth: 10,
        }
    }
}

/// Stored contract: compiled WASM bytes and deployer.
#[derive(Clone)]
struct StoredContract {
    wasm_bytes: Vec<u8>,
    #[allow(dead_code)]
    deployer: Address,
}

/// The result of executing a WASM contract method.
#[derive(Debug)]
pub struct ExecutionResult {
    /// Return value bytes from the contract method.
    pub return_value: Vec<u8>,
    /// Gas consumed during execution.
    pub gas_used: u64,
    /// Events emitted during execution.
    pub events: Vec<ContractEvent>,
    /// Pending transfers queued during execution.
    pub transfers: Vec<PendingTransfer>,
    /// Updated contract storage overlay.
    pub storage: HashMap<Vec<u8>, Vec<u8>>,
}

/// The Dina WASM runtime engine.
///
/// Manages contract deployment and execution using wasmtime with fuel-based
/// gas metering. Contracts are stored in-memory and identified by their
/// deterministic address (SHA-256 of deployer + nonce).
pub struct WasmRuntime {
    engine: Engine,
    config: RuntimeConfig,
    /// In-memory contract store: contract_address -> StoredContract.
    contracts: Arc<Mutex<HashMap<Address, StoredContract>>>,
    /// In-memory contract storage: contract_address -> key-value map.
    #[allow(clippy::type_complexity)]
    contract_storage: Arc<Mutex<HashMap<Address, HashMap<Vec<u8>, Vec<u8>>>>>,
    /// Per-deployer nonce for deterministic contract address generation.
    nonces: Arc<Mutex<HashMap<Address, u64>>>,
}

impl WasmRuntime {
    /// Create a new WASM runtime with the given configuration.
    ///
    /// The underlying wasmtime `Engine` is configured with fuel metering
    /// enabled so that every WASM instruction costs fuel (= gas).
    pub fn new(config: RuntimeConfig) -> Self {
        let mut engine_config = Config::new();
        engine_config.consume_fuel(true);
        engine_config.wasm_bulk_memory(true);

        let engine = Engine::new(&engine_config).expect("failed to create wasmtime engine");

        info!("WASM runtime initialized");

        Self {
            engine,
            config,
            contracts: Arc::new(Mutex::new(HashMap::new())),
            contract_storage: Arc::new(Mutex::new(HashMap::new())),
            nonces: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Deploy a new contract.
    ///
    /// 1. Validates the WASM module by compiling it.
    /// 2. Instantiates with host functions linked.
    /// 3. Calls the `__init` entry point with `init_args` (if exported).
    /// 4. Returns the contract address (SHA-256 of deployer + nonce).
    pub fn deploy_contract(
        &self,
        wasm_bytes: &[u8],
        init_args: &[u8],
        deployer: Address,
    ) -> Result<Address, DinaError> {
        // Validate the WASM module by compiling it.
        let module = Module::new(&self.engine, wasm_bytes)
            .map_err(|e| DinaError::WasmExecutionError(format!("invalid WASM module: {e}")))?;

        debug!(
            exports = module.exports().count(),
            "WASM module compiled for deployment"
        );

        // Generate deterministic contract address from deployer + nonce.
        let nonce = {
            let mut nonces = self.nonces.lock().unwrap_or_else(|e| e.into_inner());
            let n = nonces.entry(deployer).or_insert(0);
            let current = *n;
            *n += 1;
            current
        };

        let contract_address = {
            let mut hasher = Sha256::new();
            hasher.update(deployer.as_bytes());
            hasher.update(nonce.to_le_bytes());
            let result = hasher.finalize();
            let mut bytes = [0u8; 32];
            bytes.copy_from_slice(&result);
            Address(bytes)
        };

        // Build sandbox limits from runtime config.
        let sandbox_limits = SandboxLimits {
            max_memory: self.config.max_memory_bytes,
            max_gas: self.config.max_gas,
            max_call_depth: self.config.max_call_depth,
            ..SandboxLimits::default()
        };

        let host_state = WasmHostState::new(
            deployer,
            contract_address,
            0, // no USDC attached on deploy
            self.config.max_gas,
            0, // initial balance is 0
            0, // block_time: set by caller in production
            0, // block_height: set by caller in production
            HashMap::new(),
            sandbox_limits,
        );

        let mut store = Store::new(&self.engine, host_state);
        store
            .set_fuel(self.config.max_gas)
            .map_err(|e| DinaError::WasmExecutionError(format!("failed to set fuel: {e}")))?;

        // Link host functions.
        let mut linker = Linker::new(&self.engine);
        host::register_host_functions(&mut linker).map_err(|e| {
            DinaError::WasmExecutionError(format!("failed to register host functions: {e}"))
        })?;

        // Instantiate the module.
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| DinaError::WasmExecutionError(format!("instantiation failed: {e}")))?;

        // Call __init if it exists.
        if let Some(init_fn) = instance.get_func(&mut store, "__init") {
            // Allocate and write init_args into guest memory.
            let alloc_fn = instance
                .get_typed_func::<i32, i32>(&mut store, "__alloc")
                .map_err(|_| {
                    DinaError::WasmExecutionError(
                        "contract must export __alloc for init args".into(),
                    )
                })?;

            let args_ptr = alloc_fn
                .call(&mut store, init_args.len() as i32)
                .map_err(|e| DinaError::WasmExecutionError(format!("__alloc failed: {e}")))?;

            let memory = instance
                .get_memory(&mut store, "memory")
                .ok_or_else(|| DinaError::WasmExecutionError("no memory export".into()))?;

            memory
                .write(&mut store, args_ptr as usize, init_args)
                .map_err(|e| {
                    DinaError::WasmExecutionError(format!("init args write failed: {e}"))
                })?;

            // Call __init(args_ptr, args_len).
            let init_typed = init_fn
                .typed::<(i32, i32), ()>(&store)
                .map_err(|e| DinaError::WasmExecutionError(format!("__init type mismatch: {e}")))?;

            init_typed
                .call(&mut store, (args_ptr, init_args.len() as i32))
                .map_err(|e| {
                    DinaError::WasmExecutionError(format!("__init execution failed: {e}"))
                })?;
        }

        // Persist the contract and its initial storage state.
        let init_storage = store.data().storage.clone();

        {
            let mut contracts = self.contracts.lock().unwrap_or_else(|e| e.into_inner());
            contracts.insert(
                contract_address,
                StoredContract {
                    wasm_bytes: wasm_bytes.to_vec(),
                    deployer,
                },
            );
        }
        {
            let mut storage = self
                .contract_storage
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            storage.insert(contract_address, init_storage);
        }

        info!(
            address = %contract_address,
            deployer = %deployer,
            "contract deployed"
        );

        Ok(contract_address)
    }

    /// Call a method on a deployed contract.
    ///
    /// 1. Loads the contract WASM from storage.
    /// 2. Creates a fresh `Store` with the given fuel (gas) limit.
    /// 3. Links host functions.
    /// 4. Calls `__dispatch(method_ptr, method_len, args_ptr, args_len)`.
    /// 5. Returns the serialized result along with execution metadata.
    pub fn call_contract(
        &self,
        contract_addr: Address,
        caller: Address,
        method: &str,
        args: &[u8],
        usdc_attached: u64,
        gas_limit: u64,
    ) -> Result<ExecutionResult, DinaError> {
        // Clamp gas to configured maximum.
        let effective_gas = gas_limit.min(self.config.max_gas);

        // Load contract WASM.
        let stored = {
            let contracts = self.contracts.lock().unwrap_or_else(|e| e.into_inner());
            contracts
                .get(&contract_addr)
                .cloned()
                .ok_or_else(|| DinaError::ContractNotFound(contract_addr.to_string()))?
        };

        // Load existing contract storage.
        let existing_storage = {
            let storage = self
                .contract_storage
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            storage.get(&contract_addr).cloned().unwrap_or_default()
        };

        let sandbox_limits = SandboxLimits {
            max_memory: self.config.max_memory_bytes,
            max_gas: effective_gas,
            max_call_depth: self.config.max_call_depth,
            ..SandboxLimits::default()
        };

        let host_state = WasmHostState::new(
            caller,
            contract_addr,
            usdc_attached,
            effective_gas,
            0, // balance loaded from chain state in production
            0, // block_time from block context in production
            0, // block_height from block context in production
            existing_storage,
            sandbox_limits,
        );

        let mut store = Store::new(&self.engine, host_state);
        store
            .set_fuel(effective_gas)
            .map_err(|e| DinaError::WasmExecutionError(format!("failed to set fuel: {e}")))?;

        // Compile and link.
        let module = Module::new(&self.engine, &stored.wasm_bytes).map_err(|e| {
            DinaError::WasmExecutionError(format!("failed to compile contract: {e}"))
        })?;

        let mut linker = Linker::new(&self.engine);
        host::register_host_functions(&mut linker).map_err(|e| {
            DinaError::WasmExecutionError(format!("failed to register host functions: {e}"))
        })?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| DinaError::WasmExecutionError(format!("instantiation failed: {e}")))?;

        // Allocate method name and args in guest memory.
        let alloc_fn = instance
            .get_typed_func::<i32, i32>(&mut store, "__alloc")
            .map_err(|_| DinaError::WasmExecutionError("contract must export __alloc".into()))?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| DinaError::WasmExecutionError("no memory export".into()))?;

        // Write method name.
        let method_bytes = method.as_bytes();
        let method_ptr = alloc_fn
            .call(&mut store, method_bytes.len() as i32)
            .map_err(|e| DinaError::WasmExecutionError(format!("alloc method failed: {e}")))?;
        memory
            .write(&mut store, method_ptr as usize, method_bytes)
            .map_err(|e| DinaError::WasmExecutionError(format!("method name write failed: {e}")))?;

        // Write args.
        let args_ptr = alloc_fn
            .call(&mut store, args.len() as i32)
            .map_err(|e| DinaError::WasmExecutionError(format!("alloc args failed: {e}")))?;
        if !args.is_empty() {
            memory
                .write(&mut store, args_ptr as usize, args)
                .map_err(|e| DinaError::WasmExecutionError(format!("args write failed: {e}")))?;
        }

        // Call __dispatch(method_ptr, method_len, args_ptr, args_len) -> packed i64.
        let dispatch_fn = instance
            .get_typed_func::<(i32, i32, i32, i32), i64>(&mut store, "__dispatch")
            .map_err(|_| DinaError::WasmExecutionError("contract must export __dispatch".into()))?;

        let packed_result = dispatch_fn
            .call(
                &mut store,
                (
                    method_ptr,
                    method_bytes.len() as i32,
                    args_ptr,
                    args.len() as i32,
                ),
            )
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("fuel") {
                    DinaError::WasmExecutionError("out of gas".into())
                } else {
                    DinaError::WasmExecutionError(format!("__dispatch failed: {e}"))
                }
            })?;

        // Unpack result: high 32 bits = ptr, low 32 bits = len.
        let result_ptr = (packed_result >> 32) as u32;
        let result_len = (packed_result & 0xFFFF_FFFF) as u32;

        let return_value = if result_len > 0 {
            let mut buf = vec![0u8; result_len as usize];
            memory
                .read(&store, result_ptr as usize, &mut buf)
                .map_err(|e| {
                    DinaError::WasmExecutionError(format!("return data read failed: {e}"))
                })?;
            buf
        } else {
            Vec::new()
        };

        // Calculate gas used.
        let fuel_remaining = store.get_fuel().unwrap_or(0);
        let gas_used = effective_gas.saturating_sub(fuel_remaining);

        // Extract execution results from host state.
        let state = store.into_data();

        // Persist updated storage.
        {
            let mut contract_storage = self
                .contract_storage
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            contract_storage.insert(contract_addr, state.storage.clone());
        }

        Ok(ExecutionResult {
            return_value,
            gas_used,
            events: state.events,
            transfers: state.transfers,
            storage: state.storage,
        })
    }

    /// Get a reference to the runtime configuration.
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_creates_with_default_config() {
        let rt = WasmRuntime::new(RuntimeConfig::default());
        assert_eq!(rt.config.max_memory_bytes, 16 * 1024 * 1024);
        assert_eq!(rt.config.max_gas, 10_000_000);
        assert_eq!(rt.config.max_call_depth, 10);
    }

    #[test]
    fn deploy_invalid_wasm_returns_error() {
        let rt = WasmRuntime::new(RuntimeConfig::default());
        let result = rt.deploy_contract(b"not valid wasm", b"", Address::ZERO);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DinaError::WasmExecutionError(_)
        ));
    }

    #[test]
    fn call_nonexistent_contract_returns_error() {
        let rt = WasmRuntime::new(RuntimeConfig::default());
        let result = rt.call_contract(Address::ZERO, Address::ZERO, "transfer", b"", 0, 1_000_000);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DinaError::ContractNotFound(_)
        ));
    }

    #[test]
    fn contract_address_is_deterministic() {
        let deployer = Address([0x01; 32]);

        let addr_nonce0 = {
            let mut hasher = Sha256::new();
            hasher.update(deployer.as_bytes());
            hasher.update(0u64.to_le_bytes());
            let result = hasher.finalize();
            let mut bytes = [0u8; 32];
            bytes.copy_from_slice(&result);
            Address(bytes)
        };

        let addr_nonce1 = {
            let mut hasher = Sha256::new();
            hasher.update(deployer.as_bytes());
            hasher.update(1u64.to_le_bytes());
            let result = hasher.finalize();
            let mut bytes = [0u8; 32];
            bytes.copy_from_slice(&result);
            Address(bytes)
        };

        // Different nonces produce different addresses.
        assert_ne!(addr_nonce0, addr_nonce1);
    }

    #[test]
    fn default_config_values() {
        let config = RuntimeConfig::default();
        assert_eq!(config.max_memory_bytes, 16 * 1024 * 1024);
        assert_eq!(config.max_gas, 10_000_000);
        assert_eq!(config.max_call_depth, 10);
    }
}
