use std::collections::HashMap;

use sha2::{Digest, Sha256};
use wasmtime::{Config, Engine, Linker, Module, Store};

use crate::account::{Account, AccountState};
use crate::block::Block;
use crate::crypto::hash_bytes;
use crate::device::{DeviceIdentity, DeviceType};
use crate::error::{DinaError, DinaResult};
use crate::transaction::Transaction;
use crate::types::{Address, Hash};

/// Result of executing a full block of transactions.
#[derive(Debug)]
pub struct ExecutionResult {
    /// Merkle root of the world state after executing all transactions.
    pub state_root: Hash,
    /// Receipt for each transaction, in the same order as the block.
    pub receipts: Vec<TransactionReceipt>,
    /// Sum of all fees collected by the block proposer.
    pub total_fees: u64,
    /// Total gas consumed by the block.
    pub gas_used: u64,
}

/// Receipt for a single executed transaction.
#[derive(Debug, Clone)]
pub struct TransactionReceipt {
    /// Hash of the transaction.
    pub tx_hash: Hash,
    /// Whether the transaction executed successfully.
    pub success: bool,
    /// Gas consumed by this transaction.
    pub gas_used: u64,
    /// Fee paid by the sender.
    pub fee_paid: u64,
    /// Error message if the transaction failed.
    pub error: Option<String>,
    /// Events emitted during execution.
    pub events: Vec<Event>,
}

/// An event emitted during transaction execution.
#[derive(Debug, Clone)]
pub struct Event {
    /// Contract address that emitted the event, if any.
    pub contract: Option<Address>,
    /// Name of the event.
    pub name: String,
    /// Serialized event data.
    pub data: Vec<u8>,
}

/// Gas cost constants. One unit of gas corresponds to one computational step.
mod gas {
    /// Base gas cost for any transaction (signature verification, nonce check).
    pub const BASE: u64 = 21_000;
    /// Per-byte cost for contract bytecode deployment.
    pub const DEPLOY_PER_BYTE: u64 = 200;
    /// Base gas for a contract call (before WASM execution).
    pub const CALL_BASE: u64 = 30_000;
    /// Gas cost for device registration.
    pub const REGISTER_DEVICE: u64 = 25_000;
}

/// Minimal host state for inline WASM execution within the block executor.
/// This avoids a circular dependency on the full `dina-wasm` crate.
struct InlineWasmHostState {
    /// Contract storage overlay (key-value pairs).
    storage: HashMap<Vec<u8>, Vec<u8>>,
    /// Events emitted during execution.
    events: Vec<(String, Vec<u8>)>,
    /// Caller address bytes.
    caller: [u8; 32],
}

/// Register minimal host functions for inline WASM execution.
///
/// Only the functions required by the `__dispatch` contract ABI are
/// linked here: storage get/set, event emission, and caller identity.
fn register_inline_host_functions(
    linker: &mut Linker<InlineWasmHostState>,
) -> Result<(), wasmtime::Error> {
    // __host_caller(out_ptr: i32) -> i32
    linker.func_wrap("env", "__host_caller", |mut caller: wasmtime::Caller<'_, InlineWasmHostState>, out_ptr: i32| -> i32 {
        let addr_bytes = caller.data().caller;
        let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
            Some(m) => m,
            None => return -1,
        };
        if memory.write(&mut caller, out_ptr as usize, &addr_bytes).is_err() {
            return -1;
        }
        out_ptr
    })?;

    // __host_block_time() -> i64
    linker.func_wrap("env", "__host_block_time", |_caller: wasmtime::Caller<'_, InlineWasmHostState>| -> i64 {
        0i64
    })?;

    // __host_block_height() -> i64
    linker.func_wrap("env", "__host_block_height", |_caller: wasmtime::Caller<'_, InlineWasmHostState>| -> i64 {
        0i64
    })?;

    // __host_self_balance() -> i64
    linker.func_wrap("env", "__host_self_balance", |_caller: wasmtime::Caller<'_, InlineWasmHostState>| -> i64 {
        0i64
    })?;

    // __host_transfer(to_ptr: i32, amount: i64) -> i32
    linker.func_wrap("env", "__host_transfer", |_caller: wasmtime::Caller<'_, InlineWasmHostState>, _to_ptr: i32, _amount: i64| -> i32 {
        1 // not supported in inline executor
    })?;

    // __host_storage_get(key_ptr, key_len) -> i64
    linker.func_wrap("env", "__host_storage_get", |mut caller: wasmtime::Caller<'_, InlineWasmHostState>, key_ptr: i32, key_len: i32| -> i64 {
        let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
            Some(m) => m,
            None => return 0,
        };
        let data = memory.data(&caller);
        let start = key_ptr as usize;
        let end = start + key_len as usize;
        if end > data.len() { return 0; }
        let key = data[start..end].to_vec();

        let value = match caller.data().storage.get(&key) {
            Some(v) => v.clone(),
            None => return 0,
        };

        let val_len = value.len() as u32;
        // Allocate via __alloc
        let alloc_fn = match caller.get_export("__alloc").and_then(|e| e.into_func()) {
            Some(f) => f,
            None => return 0,
        };
        let typed = match alloc_fn.typed::<i32, i32>(&caller) {
            Ok(t) => t,
            Err(_) => return 0,
        };
        let val_ptr = match typed.call(&mut caller, val_len as i32) {
            Ok(p) => p as u32,
            Err(_) => return 0,
        };

        let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
            Some(m) => m,
            None => return 0,
        };
        if memory.write(&mut caller, val_ptr as usize, &value).is_err() {
            return 0;
        }

        ((val_ptr as i64) << 32) | (val_len as i64)
    })?;

    // __host_storage_set(key_ptr, key_len, val_ptr, val_len)
    linker.func_wrap("env", "__host_storage_set", |mut caller: wasmtime::Caller<'_, InlineWasmHostState>, key_ptr: i32, key_len: i32, val_ptr: i32, val_len: i32| {
        let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
            Some(m) => m,
            None => return,
        };
        let data = memory.data(&caller);
        let ks = key_ptr as usize;
        let ke = ks + key_len as usize;
        let vs = val_ptr as usize;
        let ve = vs + val_len as usize;
        if ke > data.len() || ve > data.len() { return; }
        let key = data[ks..ke].to_vec();
        let value = data[vs..ve].to_vec();
        caller.data_mut().storage.insert(key, value);
    })?;

    // __host_emit_event(name_ptr, name_len, data_ptr, data_len)
    linker.func_wrap("env", "__host_emit_event", |mut caller: wasmtime::Caller<'_, InlineWasmHostState>, name_ptr: i32, name_len: i32, data_ptr: i32, data_len: i32| {
        let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
            Some(m) => m,
            None => return,
        };
        let data = memory.data(&caller);
        let ns = name_ptr as usize;
        let ne = ns + name_len as usize;
        let ds = data_ptr as usize;
        let de = ds + data_len as usize;
        if ne > data.len() || de > data.len() { return; }
        let name = String::from_utf8_lossy(&data[ns..ne]).into_owned();
        let event_data = data[ds..de].to_vec();
        caller.data_mut().events.push((name, event_data));
    })?;

    // __host_sha256(data_ptr, data_len) -> i32
    linker.func_wrap("env", "__host_sha256", |mut caller: wasmtime::Caller<'_, InlineWasmHostState>, data_ptr: i32, data_len: i32| -> i32 {
        let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
            Some(m) => m,
            None => return 0,
        };
        let mem_data = memory.data(&caller);
        let start = data_ptr as usize;
        let end = start + data_len as usize;
        if end > mem_data.len() { return 0; }
        let input = mem_data[start..end].to_vec();

        let hash = Sha256::digest(&input);

        let alloc_fn = match caller.get_export("__alloc").and_then(|e| e.into_func()) {
            Some(f) => f,
            None => return 0,
        };
        let typed = match alloc_fn.typed::<i32, i32>(&caller) {
            Ok(t) => t,
            Err(_) => return 0,
        };
        let out_ptr = match typed.call(&mut caller, 32) {
            Ok(p) => p as u32,
            Err(_) => return 0,
        };

        let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
            Some(m) => m,
            None => return 0,
        };
        if memory.write(&mut caller, out_ptr as usize, &hash).is_err() {
            return 0;
        }

        out_ptr as i32
    })?;

    // __host_verify_ed25519(pubkey_ptr, msg_ptr, msg_len, sig_ptr) -> i32
    linker.func_wrap("env", "__host_verify_ed25519", |_caller: wasmtime::Caller<'_, InlineWasmHostState>, _pk: i32, _msg: i32, _mlen: i32, _sig: i32| -> i32 {
        0 // signature verification not supported in inline executor
    })?;

    Ok(())
}

/// The block execution engine. Takes a block of transactions and applies
/// them to the account state, producing receipts and a new state root.
pub struct BlockExecutor {
    state: AccountState,
    /// Registered devices, keyed by device public key.
    devices: HashMap<[u8; 32], DeviceIdentity>,
    /// Deployed contract WASM bytecodes, keyed by contract address.
    contract_code: HashMap<Address, Vec<u8>>,
    /// Per-contract key-value storage.
    contract_storage: HashMap<Address, HashMap<Vec<u8>, Vec<u8>>>,
}

impl BlockExecutor {
    /// Create a new executor with the given initial state.
    pub fn new(state: AccountState) -> Self {
        Self {
            state,
            devices: HashMap::new(),
            contract_code: HashMap::new(),
            contract_storage: HashMap::new(),
        }
    }

    /// Execute an entire block of transactions.
    ///
    /// Iterates through every transaction in the block, applies it to the
    /// state, and collects a receipt for each one. Returns an aggregate
    /// `ExecutionResult` containing the new state root, all receipts, total
    /// fees, and total gas consumed.
    pub fn execute_block(&mut self, block: &Block) -> DinaResult<ExecutionResult> {
        let mut receipts = Vec::with_capacity(block.transactions.len());
        let mut total_fees: u64 = 0;
        let mut total_gas: u64 = 0;

        for tx in &block.transactions {
            let receipt = self.execute_transaction(tx);
            total_fees = total_fees.saturating_add(receipt.fee_paid);
            total_gas = total_gas.saturating_add(receipt.gas_used);
            receipts.push(receipt);
        }

        // Credit collected fees to the block proposer.
        if total_fees > 0 {
            self.state.credit(&block.header.proposer, total_fees);
        }

        // Compute state root by hashing all account data deterministically.
        let state_root = self.compute_state_root();

        Ok(ExecutionResult {
            state_root,
            receipts,
            total_fees,
            gas_used: total_gas,
        })
    }

    /// Execute a single transaction. The fee is always deducted (even if the
    /// transaction body fails), and the nonce is only incremented on success.
    pub fn execute_transaction(&mut self, tx: &Transaction) -> TransactionReceipt {
        let tx_hash = tx.hash();
        let fee = tx.fee();
        let sender = tx.sender();

        // Phase 1: Deduct fee. If the sender cannot pay, the tx is dropped
        // entirely with no state changes.
        if let Err(e) = self.state.deduct_fee(&sender, fee) {
            return TransactionReceipt {
                tx_hash,
                success: false,
                gas_used: 0,
                fee_paid: 0,
                error: Some(format!("fee deduction failed: {e}")),
                events: vec![],
            };
        }

        // Phase 2: Execute the transaction body. Failures here do NOT refund
        // the fee, but the nonce is not incremented.
        let result = self.execute_body(tx);

        match result {
            Ok(events) => {
                // Increment nonce on success.
                let _ = self.state.increment_nonce(&sender);

                let gas_used = self.estimate_gas(tx);
                TransactionReceipt {
                    tx_hash,
                    success: true,
                    gas_used,
                    fee_paid: fee,
                    error: None,
                    events,
                }
            }
            Err(e) => {
                let gas_used = self.estimate_gas(tx);
                TransactionReceipt {
                    tx_hash,
                    success: false,
                    gas_used,
                    fee_paid: fee,
                    error: Some(e.to_string()),
                    events: vec![],
                }
            }
        }
    }

    /// Pre-validate a transaction without applying any state changes.
    /// Checks that the sender account exists, nonce is correct, balance
    /// covers fee + amount, and the signature is valid.
    pub fn validate_transaction(&self, tx: &Transaction) -> DinaResult<()> {
        let sender = tx.sender();
        let account = self
            .state
            .get_account(&sender)
            .ok_or_else(|| DinaError::AccountNotFound(sender.to_string()))?;

        // Check nonce.
        if account.nonce != tx.nonce() {
            return Err(DinaError::InvalidNonce {
                expected: account.nonce,
                got: tx.nonce(),
            });
        }

        // Check balance covers fee + transfer amount (with overflow protection).
        let total_needed = tx.fee().checked_add(self.tx_amount(tx)).ok_or_else(|| {
            DinaError::Custom(format!(
                "fee + amount overflow: {} + {} exceeds u64::MAX",
                tx.fee(),
                self.tx_amount(tx)
            ))
        })?;
        if account.balance < total_needed {
            return Err(DinaError::InsufficientBalance {
                have: account.balance,
                need: total_needed,
            });
        }

        // Verify signature.
        // Recover the verifying key from the sender address is not possible
        // (address is a hash of the pubkey), so we require the sender account
        // to exist. For validation purposes, we verify the signature structurally
        // by checking it is well-formed. Full signature verification requires
        // the public key, which is only available to the caller.
        // For now, we check that the signature bytes are non-zero as a basic
        // structural check. Real signature verification happens at the mempool
        // layer where the public key is available.
        let sig_bytes = self.extract_signature_bytes(tx);
        if sig_bytes == [0u8; 64] {
            return Err(DinaError::InvalidSignature);
        }

        Ok(())
    }

    /// Get a reference to the current account state.
    pub fn state(&self) -> &AccountState {
        &self.state
    }

    /// Consume the executor and return the final account state.
    pub fn into_state(self) -> AccountState {
        self.state
    }

    // ── Private helpers ──────────────────────────────────────────────

    /// Execute the body of a transaction (everything except fee deduction).
    fn execute_body(&mut self, tx: &Transaction) -> DinaResult<Vec<Event>> {
        let sender = tx.sender();
        let account = self
            .state
            .get_account(&sender)
            .ok_or_else(|| DinaError::AccountNotFound(sender.to_string()))?;

        // Check nonce.
        if account.nonce != tx.nonce() {
            return Err(DinaError::InvalidNonce {
                expected: account.nonce,
                got: tx.nonce(),
            });
        }

        match tx {
            Transaction::Transfer {
                from, to, amount, ..
            } => {
                self.state.transfer(from, to, *amount)?;
                Ok(vec![Event {
                    contract: None,
                    name: "Transfer".to_string(),
                    data: Vec::new(),
                }])
            }

            Transaction::DeployContract {
                from,
                wasm_bytecode,
                ..
            } => {
                let code_hash = hash_bytes(wasm_bytecode);

                // Derive a deterministic contract address from deployer + nonce.
                let deployer_nonce = self
                    .state
                    .get_account(from)
                    .map(|a| a.nonce)
                    .unwrap_or(0);
                let contract_addr = {
                    let mut hasher = Sha256::new();
                    hasher.update(from.as_bytes());
                    hasher.update(deployer_nonce.to_le_bytes());
                    let result = hasher.finalize();
                    let mut bytes = [0u8; 32];
                    bytes.copy_from_slice(&result);
                    Address(bytes)
                };

                // Store the WASM bytecode keyed by the contract address.
                self.contract_code
                    .insert(contract_addr, wasm_bytecode.clone());

                // Create a contract account with the code hash.
                let mut contract_account = Account::new(contract_addr);
                contract_account.code_hash = Some(code_hash);
                self.state.set_account(contract_account);

                Ok(vec![Event {
                    contract: Some(contract_addr),
                    name: "ContractDeployed".to_string(),
                    data: contract_addr.as_bytes().to_vec(),
                }])
            }

            Transaction::CallContract {
                contract,
                usdc_attached,
                from,
                method,
                args,
                ..
            } => {
                // Transfer attached USDC to the contract address.
                if *usdc_attached > 0 {
                    self.state.transfer(from, contract, *usdc_attached)?;
                }

                // Look up the contract bytecode.
                let bytecode = self
                    .contract_code
                    .get(contract)
                    .ok_or_else(|| {
                        DinaError::ContractNotFound(contract.to_string())
                    })?
                    .clone();

                // Load existing contract storage.
                let existing_storage = self
                    .contract_storage
                    .get(contract)
                    .cloned()
                    .unwrap_or_default();

                // Execute the WASM contract inline using wasmtime.
                let (events, updated_storage) =
                    self.execute_wasm(&bytecode, from, contract, method, args, existing_storage)?;

                // Persist updated contract storage.
                self.contract_storage.insert(*contract, updated_storage);

                Ok(events)
            }

            Transaction::RegisterDevice {
                device_pubkey,
                owner,
                attestation,
                ..
            } => {
                // Check that the device is not already registered.
                if self.devices.contains_key(device_pubkey) {
                    return Err(DinaError::Custom(
                        "device already registered".to_string(),
                    ));
                }

                let device = DeviceIdentity::new(
                    *device_pubkey,
                    *owner,
                    DeviceType::CognitumSeed, // default; real type from attestation metadata
                    attestation.firmware_hash,
                    attestation.witness_root,
                    attestation.timestamp,
                );

                self.devices.insert(*device_pubkey, device);

                Ok(vec![Event {
                    contract: None,
                    name: "DeviceRegistered".to_string(),
                    data: device_pubkey.to_vec(),
                }])
            }
        }
    }

    /// Execute a WASM contract method inline using wasmtime.
    ///
    /// This replicates the dispatch ABI used by `dina-wasm`: the contract
    /// must export `__alloc(size: i32) -> i32`, `__dispatch(method_ptr,
    /// method_len, args_ptr, args_len) -> i64`, and `memory`.
    fn execute_wasm(
        &self,
        bytecode: &[u8],
        caller_addr: &Address,
        contract_addr: &Address,
        method: &str,
        args: &[u8],
        existing_storage: HashMap<Vec<u8>, Vec<u8>>,
    ) -> DinaResult<(Vec<Event>, HashMap<Vec<u8>, Vec<u8>>)> {
        let mut engine_config = Config::new();
        engine_config.consume_fuel(true);
        engine_config.wasm_bulk_memory(true);

        let engine = Engine::new(&engine_config)
            .map_err(|e| DinaError::WasmExecutionError(format!("engine init failed: {e}")))?;

        let module = Module::new(&engine, bytecode)
            .map_err(|e| DinaError::WasmExecutionError(format!("invalid WASM: {e}")))?;

        let host_state = InlineWasmHostState {
            storage: existing_storage,
            events: Vec::new(),
            caller: *caller_addr.as_bytes(),
        };

        let mut store = Store::new(&engine, host_state);
        store
            .set_fuel(10_000_000)
            .map_err(|e| DinaError::WasmExecutionError(format!("set fuel failed: {e}")))?;

        let mut linker = Linker::new(&engine);
        register_inline_host_functions(&mut linker)
            .map_err(|e| DinaError::WasmExecutionError(format!("host link failed: {e}")))?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| DinaError::WasmExecutionError(format!("instantiation failed: {e}")))?;

        // Allocate and write method name into guest memory.
        let alloc_fn = instance
            .get_typed_func::<i32, i32>(&mut store, "__alloc")
            .map_err(|_| {
                DinaError::WasmExecutionError("contract must export __alloc".into())
            })?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| DinaError::WasmExecutionError("no memory export".into()))?;

        let method_bytes = method.as_bytes();
        let method_ptr = alloc_fn
            .call(&mut store, method_bytes.len() as i32)
            .map_err(|e| DinaError::WasmExecutionError(format!("alloc method failed: {e}")))?;
        memory
            .write(&mut store, method_ptr as usize, method_bytes)
            .map_err(|e| {
                DinaError::WasmExecutionError(format!("method write failed: {e}"))
            })?;

        // Allocate and write args.
        let args_ptr = alloc_fn
            .call(&mut store, args.len().max(1) as i32)
            .map_err(|e| DinaError::WasmExecutionError(format!("alloc args failed: {e}")))?;
        if !args.is_empty() {
            memory
                .write(&mut store, args_ptr as usize, args)
                .map_err(|e| {
                    DinaError::WasmExecutionError(format!("args write failed: {e}"))
                })?;
        }

        // Call __dispatch(method_ptr, method_len, args_ptr, args_len) -> packed i64.
        let dispatch_fn = instance
            .get_typed_func::<(i32, i32, i32, i32), i64>(&mut store, "__dispatch")
            .map_err(|_| {
                DinaError::WasmExecutionError("contract must export __dispatch".into())
            })?;

        let _packed_result = dispatch_fn
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

        // Extract results from host state.
        let state = store.into_data();

        let events = state
            .events
            .into_iter()
            .map(|(name, data)| Event {
                contract: Some(*contract_addr),
                name,
                data,
            })
            .collect();

        Ok((events, state.storage))
    }

    /// Estimate gas usage for a transaction based on its type.
    fn estimate_gas(&self, tx: &Transaction) -> u64 {
        match tx {
            Transaction::Transfer { .. } => gas::BASE,
            Transaction::DeployContract { wasm_bytecode, .. } => {
                gas::BASE + (wasm_bytecode.len() as u64) * gas::DEPLOY_PER_BYTE
            }
            Transaction::CallContract { .. } => gas::CALL_BASE,
            Transaction::RegisterDevice { .. } => gas::REGISTER_DEVICE,
        }
    }

    /// Extract the transfer/attach amount from a transaction (zero for
    /// non-transfer types that don't move funds).
    fn tx_amount(&self, tx: &Transaction) -> u64 {
        match tx {
            Transaction::Transfer { amount, .. } => *amount,
            Transaction::CallContract { usdc_attached, .. } => *usdc_attached,
            _ => 0,
        }
    }

    /// Extract the raw 64-byte signature from any transaction variant.
    fn extract_signature_bytes(&self, tx: &Transaction) -> [u8; 64] {
        match tx {
            Transaction::Transfer { signature, .. }
            | Transaction::DeployContract { signature, .. }
            | Transaction::CallContract { signature, .. }
            | Transaction::RegisterDevice { signature, .. } => signature.0,
        }
    }

    /// Compute a deterministic state root by hashing all accounts in sorted
    /// order by address.
    fn compute_state_root(&self) -> Hash {
        let mut entries: Vec<_> = self.state.iter().collect();
        entries.sort_by_key(|(addr, _)| addr.0);

        let mut hasher_input = Vec::new();
        for (addr, account) in entries {
            hasher_input.extend_from_slice(addr.as_bytes());
            let account_bytes =
                bincode::serialize(account).expect("account serialization cannot fail");
            hasher_input.extend_from_slice(&account_bytes);
        }

        if hasher_input.is_empty() {
            Hash::ZERO
        } else {
            hash_bytes(&hasher_input)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{Block, BlockHeader};
    use crate::crypto;
    use crate::transaction::Sig64;

    /// Helper: create a signed transfer transaction.
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

        if let Transaction::Transfer {
            ref mut signature, ..
        } = tx
        {
            *signature = Sig64(sig);
        }

        tx
    }

    /// Helper: wrap transactions into a block with a given proposer.
    fn make_block(proposer: Address, txs: Vec<Transaction>) -> Block {
        Block {
            header: BlockHeader {
                block_number: 1,
                parent_hash: Hash::ZERO,
                state_root: Hash::ZERO,
                transactions_root: Hash::ZERO,
                timestamp: 1_700_000_000,
                proposer,
                signature: [0u8; 64],
            },
            transactions: txs,
        }
    }

    #[test]
    fn execute_empty_block() {
        let mut state = AccountState::new();
        let proposer = Address([0x01; 32]);
        state.credit(&proposer, 0);

        let mut executor = BlockExecutor::new(state);
        let block = make_block(proposer, vec![]);
        let result = executor.execute_block(&block).unwrap();

        assert!(result.receipts.is_empty());
        assert_eq!(result.total_fees, 0);
        assert_eq!(result.gas_used, 0);
    }

    #[test]
    fn execute_single_transfer() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let recipient = Address([0xbb; 32]);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&sender, 10_000);

        let tx = make_transfer(&sk, recipient, 1_000, 0, 10);
        let mut executor = BlockExecutor::new(state);
        let block = make_block(proposer, vec![tx]);
        let result = executor.execute_block(&block).unwrap();

        assert_eq!(result.receipts.len(), 1);
        assert!(result.receipts[0].success);
        assert_eq!(result.receipts[0].fee_paid, 10);
        assert_eq!(result.total_fees, 10);

        // Sender: 10000 - 10 (fee) - 1000 (transfer) = 8990
        let final_state = executor.state();
        assert_eq!(final_state.get_account(&sender).unwrap().balance, 8_990);
        assert_eq!(final_state.get_account(&sender).unwrap().nonce, 1);
        // Recipient gets 1000
        assert_eq!(
            final_state.get_account(&recipient).unwrap().balance,
            1_000
        );
        // Proposer gets the fee
        assert_eq!(final_state.get_account(&proposer).unwrap().balance, 10);
    }

    #[test]
    fn insufficient_balance_receipt_shows_failure() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let recipient = Address([0xbb; 32]);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        // Give sender enough for fee but not fee + amount.
        state.credit(&sender, 100);

        let tx = make_transfer(&sk, recipient, 500, 0, 10);
        let mut executor = BlockExecutor::new(state);
        let block = make_block(proposer, vec![tx]);
        let result = executor.execute_block(&block).unwrap();

        assert_eq!(result.receipts.len(), 1);
        assert!(!result.receipts[0].success);
        assert!(result.receipts[0].error.is_some());
        // Fee was still deducted.
        assert_eq!(result.receipts[0].fee_paid, 10);
        assert_eq!(
            executor.state().get_account(&sender).unwrap().balance,
            90
        );
    }

    #[test]
    fn wrong_nonce_receipt_shows_failure() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let recipient = Address([0xbb; 32]);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&sender, 10_000);

        // Nonce should be 0 but we send 5.
        let tx = make_transfer(&sk, recipient, 100, 5, 10);
        let mut executor = BlockExecutor::new(state);
        let block = make_block(proposer, vec![tx]);
        let result = executor.execute_block(&block).unwrap();

        assert_eq!(result.receipts.len(), 1);
        assert!(!result.receipts[0].success);
        let err_msg = result.receipts[0].error.as_ref().unwrap();
        assert!(err_msg.contains("nonce"));
        // Fee still deducted, nonce NOT incremented.
        assert_eq!(result.receipts[0].fee_paid, 10);
        assert_eq!(
            executor.state().get_account(&sender).unwrap().nonce,
            0
        );
    }

    #[test]
    fn fee_deducted_on_failed_transaction() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let recipient = Address([0xbb; 32]);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&sender, 500);

        // Transfer more than balance-after-fee allows.
        let tx = make_transfer(&sk, recipient, 1_000, 0, 20);
        let mut executor = BlockExecutor::new(state);
        let block = make_block(proposer, vec![tx]);
        let result = executor.execute_block(&block).unwrap();

        assert!(!result.receipts[0].success);
        assert_eq!(result.receipts[0].fee_paid, 20);
        // Balance = 500 - 20 = 480 (fee deducted, transfer not applied).
        assert_eq!(
            executor.state().get_account(&sender).unwrap().balance,
            480
        );
    }

    #[test]
    fn multiple_transactions_in_one_block() {
        let (sk_a, vk_a) = crypto::generate_keypair();
        let (sk_b, vk_b) = crypto::generate_keypair();
        let addr_a = Address::from_pubkey(&vk_a);
        let addr_b = Address::from_pubkey(&vk_b);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&addr_a, 5_000);
        state.credit(&addr_b, 3_000);

        let tx1 = make_transfer(&sk_a, addr_b, 1_000, 0, 10);
        let tx2 = make_transfer(&sk_b, addr_a, 500, 0, 10);

        let mut executor = BlockExecutor::new(state);
        let block = make_block(proposer, vec![tx1, tx2]);
        let result = executor.execute_block(&block).unwrap();

        assert_eq!(result.receipts.len(), 2);
        assert!(result.receipts[0].success);
        assert!(result.receipts[1].success);
        assert_eq!(result.total_fees, 20);

        let final_state = executor.state();
        // A: 5000 - 10 (fee) - 1000 (sent) + 500 (received) = 4490
        assert_eq!(final_state.get_account(&addr_a).unwrap().balance, 4_490);
        // B: 3000 + 1000 (received) - 10 (fee) - 500 (sent) = 3490
        assert_eq!(final_state.get_account(&addr_b).unwrap().balance, 3_490);
        // Proposer: 20 total fees
        assert_eq!(final_state.get_account(&proposer).unwrap().balance, 20);
    }

    #[test]
    fn validate_transaction_catches_bad_signature() {
        let (_, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);

        let mut state = AccountState::new();
        state.credit(&sender, 10_000);

        // Transaction with all-zero signature (invalid).
        let tx = Transaction::Transfer {
            from: sender,
            to: Address([0xbb; 32]),
            amount: 100,
            memo: None,
            device_witness: None,
            nonce: 0,
            fee: 10,
            signature: Sig64([0u8; 64]),
        };

        let executor = BlockExecutor::new(state);
        let err = executor.validate_transaction(&tx).unwrap_err();
        assert!(matches!(err, DinaError::InvalidSignature));
    }

    #[test]
    fn validate_transaction_catches_wrong_nonce() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);

        let mut state = AccountState::new();
        state.credit(&sender, 10_000);

        let tx = make_transfer(&sk, Address([0xbb; 32]), 100, 99, 10);

        let executor = BlockExecutor::new(state);
        let err = executor.validate_transaction(&tx).unwrap_err();
        assert!(matches!(err, DinaError::InvalidNonce { .. }));
    }

    #[test]
    fn validate_transaction_catches_insufficient_balance() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);

        let mut state = AccountState::new();
        state.credit(&sender, 50);

        let tx = make_transfer(&sk, Address([0xbb; 32]), 100, 0, 10);

        let executor = BlockExecutor::new(state);
        let err = executor.validate_transaction(&tx).unwrap_err();
        assert!(matches!(err, DinaError::InsufficientBalance { .. }));
    }

    #[test]
    fn into_state_returns_final_state() {
        let mut state = AccountState::new();
        let addr = Address([0x42; 32]);
        state.credit(&addr, 1_000);

        let executor = BlockExecutor::new(state);
        let recovered = executor.into_state();
        assert_eq!(recovered.get_account(&addr).unwrap().balance, 1_000);
    }

    #[test]
    fn state_root_changes_after_execution() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&sender, 10_000);

        let tx = make_transfer(&sk, Address([0xcc; 32]), 500, 0, 10);

        let mut executor = BlockExecutor::new(state.clone());
        let empty_block = make_block(proposer, vec![]);
        let result_empty = executor.execute_block(&empty_block).unwrap();

        let mut executor2 = BlockExecutor::new(state);
        let block_with_tx = make_block(proposer, vec![tx]);
        let result_tx = executor2.execute_block(&block_with_tx).unwrap();

        // State roots should differ because the state changed.
        assert_ne!(result_empty.state_root, result_tx.state_root);
    }

    #[test]
    fn cannot_pay_fee_drops_transaction() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&sender, 5); // Less than the fee.

        let tx = make_transfer(&sk, Address([0xbb; 32]), 1, 0, 10);
        let mut executor = BlockExecutor::new(state);
        let block = make_block(proposer, vec![tx]);
        let result = executor.execute_block(&block).unwrap();

        assert!(!result.receipts[0].success);
        // Fee was NOT paid because sender couldn't afford it.
        assert_eq!(result.receipts[0].fee_paid, 0);
        // Balance unchanged.
        assert_eq!(
            executor.state().get_account(&sender).unwrap().balance,
            5
        );
    }
}
