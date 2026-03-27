use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::thread;

use crate::account::{Account, AccountState};
use crate::block::Block;
use crate::crypto::hash_bytes;
use crate::device::{DeviceIdentity, DeviceType};
use crate::error::{DinaError, DinaResult};
use crate::executor::{Event, TransactionReceipt};
use crate::transaction::Transaction;
use crate::types::{Address, Hash};

// Re-use the gas constants from the sequential executor (same values).
mod gas {
    pub const BASE: u64 = 21_000;
    pub const DEPLOY_PER_BYTE: u64 = 200;
    pub const CALL_BASE: u64 = 30_000;
    pub const REGISTER_DEVICE: u64 = 25_000;
}

/// Result of executing a block with the parallel executor.
#[derive(Debug)]
pub struct ParallelExecutionResult {
    /// Merkle root of the world state after executing all transactions.
    pub state_root: Hash,
    /// Receipt for each transaction, in the ORIGINAL transaction order.
    pub receipts: Vec<TransactionReceipt>,
    /// Sum of all fees collected by the block proposer.
    pub total_fees: u64,
    /// Total gas consumed by the block.
    pub gas_used: u64,
    /// Number of parallel lanes that were used.
    pub lanes_used: usize,
    /// Fraction of transactions that ran in parallel (0.0 to 1.0).
    pub parallelism_ratio: f64,
}

/// A single execution lane that processes a subset of transactions sequentially.
struct ExecutionLane {
    lane_id: usize,
    /// (original_index, transaction) pairs assigned to this lane.
    transactions: Vec<(usize, Transaction)>,
}

impl ExecutionLane {
    fn new(lane_id: usize) -> Self {
        Self {
            lane_id,
            transactions: Vec::new(),
        }
    }
}

/// A parallel block execution engine using Block-STM style lane-based
/// parallelism. Independent transactions (no shared accounts) execute
/// concurrently across multiple CPU lanes, while transactions that share
/// accounts are grouped into the same lane for sequential safety.
pub struct ParallelBlockExecutor {
    state: AccountState,
    devices: HashMap<[u8; 32], DeviceIdentity>,
    /// Number of execution lanes (0 = auto-detect from CPU cores).
    num_lanes: usize,
    /// Minimum number of transactions before engaging parallel execution.
    min_txs_for_parallel: usize,
}

impl ParallelBlockExecutor {
    /// Create a new parallel executor with auto-detected lane count.
    pub fn new(state: AccountState) -> Self {
        Self {
            state,
            devices: HashMap::new(),
            num_lanes: 0, // auto-detect
            min_txs_for_parallel: 4,
        }
    }

    /// Create a new parallel executor with explicit configuration.
    pub fn with_config(state: AccountState, max_lanes: usize, min_txs: usize) -> Self {
        Self {
            state,
            devices: HashMap::new(),
            num_lanes: max_lanes,
            min_txs_for_parallel: min_txs,
        }
    }

    /// Resolve the effective number of lanes.
    fn effective_lanes(&self) -> usize {
        if self.num_lanes == 0 {
            let cpus = thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1);
            cpus.max(1)
        } else {
            self.num_lanes.max(1)
        }
    }

    /// Execute a block of transactions, using parallel lanes when beneficial.
    pub fn execute_block(&mut self, block: &Block) -> DinaResult<ParallelExecutionResult> {
        let tx_count = block.transactions.len();

        // For very small blocks, fall back to single-lane (sequential) execution.
        if tx_count < self.min_txs_for_parallel {
            return self.execute_sequential(block);
        }

        let max_lanes = self.effective_lanes().min(tx_count);

        // Phase 1: Analyze transaction dependencies and assign to lanes.
        let lanes = self.assign_lanes(&block.transactions, max_lanes);
        let lanes_used = lanes.iter().filter(|l| !l.transactions.is_empty()).count();

        // If everything ended up in one lane, just run sequentially.
        if lanes_used <= 1 {
            return self.execute_sequential(block);
        }

        // Phase 2: Execute lanes in parallel.
        let lane_results = self.execute_lanes_parallel(lanes)?;

        // Phase 3: Merge lane results deterministically.
        let (receipts, total_fees, total_gas) =
            self.merge_lane_results(lane_results, tx_count)?;

        // Credit fees to the proposer.
        if total_fees > 0 {
            self.state.credit(&block.header.proposer, total_fees);
        }

        let state_root = self.compute_state_root();

        let parallelism_ratio = if tx_count > 0 {
            1.0 - (self.longest_lane_len(&block.transactions, lanes_used) as f64
                / tx_count as f64)
        } else {
            0.0
        };

        Ok(ParallelExecutionResult {
            state_root,
            receipts,
            total_fees,
            gas_used: total_gas,
            lanes_used,
            parallelism_ratio,
        })
    }

    /// Get a reference to the current account state.
    pub fn state(&self) -> &AccountState {
        &self.state
    }

    /// Consume the executor and return the final account state.
    pub fn into_state(self) -> AccountState {
        self.state
    }

    // ── Phase 1: Dependency analysis & lane assignment ───────────────

    /// Extract all addresses touched by a transaction (sender + receiver/contract).
    fn touched_addresses(tx: &Transaction) -> Vec<Address> {
        match tx {
            Transaction::Transfer { from, to, .. } => vec![*from, *to],
            Transaction::DeployContract { from, .. } => vec![*from],
            Transaction::CallContract {
                from, contract, ..
            } => vec![*from, *contract],
            Transaction::RegisterDevice { owner, .. } => vec![*owner],
        }
    }

    /// Assign transactions to lanes using union-find over touched addresses.
    /// Transactions sharing any address end up in the same lane.
    fn assign_lanes(
        &self,
        transactions: &[Transaction],
        max_lanes: usize,
    ) -> Vec<ExecutionLane> {
        let tx_count = transactions.len();

        // Union-find: each transaction starts in its own group.
        let mut parent: Vec<usize> = (0..tx_count).collect();

        fn find(parent: &mut [usize], mut x: usize) -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]]; // path compression
                x = parent[x];
            }
            x
        }

        fn union(parent: &mut [usize], a: usize, b: usize) {
            let ra = find(parent, a);
            let rb = find(parent, b);
            if ra != rb {
                // Always merge into the lower-indexed root for determinism.
                if ra < rb {
                    parent[rb] = ra;
                } else {
                    parent[ra] = rb;
                }
            }
        }

        // Map address -> first transaction index that touches it.
        let mut addr_to_tx: HashMap<Address, usize> = HashMap::new();

        for (i, tx) in transactions.iter().enumerate() {
            let addrs = Self::touched_addresses(tx);
            for addr in addrs {
                if let Some(&first_tx) = addr_to_tx.get(&addr) {
                    union(&mut parent, first_tx, i);
                } else {
                    addr_to_tx.insert(addr, i);
                }
            }
        }

        // Group transactions by their root in the union-find.
        let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
        for i in 0..tx_count {
            let root = find(&mut parent, i);
            groups.entry(root).or_default().push(i);
        }

        // Sort group keys for determinism, then distribute to lanes round-robin.
        let mut sorted_groups: Vec<Vec<usize>> = groups.into_values().collect();
        // Sort by the first transaction index in each group for determinism.
        sorted_groups.sort_by_key(|g| g[0]);

        let mut lanes: Vec<ExecutionLane> = (0..max_lanes)
            .map(|id| ExecutionLane::new(id))
            .collect();

        // Assign groups to lanes using a greedy approach: put each group
        // into the lane with the fewest transactions so far.
        for group in sorted_groups {
            // Find the lane with the minimum load.
            let target = lanes
                .iter()
                .enumerate()
                .min_by_key(|(_, l)| l.transactions.len())
                .map(|(idx, _)| idx)
                .unwrap_or(0);

            for &tx_idx in &group {
                lanes[target]
                    .transactions
                    .push((tx_idx, transactions[tx_idx].clone()));
            }
        }

        lanes
    }

    /// Compute the longest lane length (used for parallelism ratio).
    fn longest_lane_len(&self, transactions: &[Transaction], lanes_used: usize) -> usize {
        if lanes_used <= 1 {
            return transactions.len();
        }
        // Re-run assignment to find the actual longest lane.
        let lanes = self.assign_lanes(transactions, lanes_used);
        lanes.iter().map(|l| l.transactions.len()).max().unwrap_or(0)
    }

    // ── Phase 2: Parallel execution ─────────────────────────────────

    /// Execute all lanes in parallel using std::thread.
    fn execute_lanes_parallel(
        &self,
        lanes: Vec<ExecutionLane>,
    ) -> DinaResult<Vec<LaneResult>> {
        // Snapshot the current state for each lane to read from.
        let base_state = &self.state;
        let base_devices = &self.devices;

        // We wrap results in a Mutex<Vec> to collect from threads.
        let lane_count = lanes.len();
        let results: Mutex<Vec<Option<LaneResult>>> = {
            let mut v = Vec::with_capacity(lane_count);
            for _ in 0..lane_count {
                v.push(None);
            }
            Mutex::new(v)
        };

        thread::scope(|s| {
            for lane in lanes {
                let results_ref = &results;
                let lane_id = lane.lane_id;

                // Clone the base state for this lane.
                let mut lane_state = base_state.clone();
                let mut lane_devices = base_devices.clone();

                s.spawn(move || {
                    let result = Self::execute_lane(
                        &mut lane_state,
                        &mut lane_devices,
                        &lane,
                    );

                    let mut guard = results_ref.lock().unwrap();
                    guard[lane_id] = Some(result);
                });
            }
        });

        let guard = results.into_inner().unwrap();
        guard
            .into_iter()
            .map(|opt| opt.ok_or_else(|| DinaError::Custom("lane thread did not produce result".into())))
            .collect()
    }

    /// Execute a single lane sequentially against a cloned state.
    fn execute_lane(
        state: &mut AccountState,
        devices: &mut HashMap<[u8; 32], DeviceIdentity>,
        lane: &ExecutionLane,
    ) -> LaneResult {
        // Collect which addresses are touched by this lane's transactions.
        let mut touched_addrs: HashSet<Address> = HashSet::new();
        for (_, tx) in &lane.transactions {
            for addr in Self::touched_addresses(tx) {
                touched_addrs.insert(addr);
            }
        }

        let mut receipts: Vec<(usize, TransactionReceipt)> = Vec::new();
        let mut total_fees: u64 = 0;
        let mut total_gas: u64 = 0;

        for (orig_idx, tx) in &lane.transactions {
            let receipt = Self::execute_transaction_on_state(state, devices, tx);
            total_fees = total_fees.saturating_add(receipt.fee_paid);
            total_gas = total_gas.saturating_add(receipt.gas_used);
            receipts.push((*orig_idx, receipt));
        }

        // Only include accounts that were touched by this lane.
        let state_overlay: HashMap<Address, Account> = state
            .iter()
            .filter(|(addr, _)| touched_addrs.contains(addr))
            .map(|(addr, acct)| (*addr, acct.clone()))
            .collect();

        // Only include devices registered by this lane (new ones).
        let device_overlay: HashMap<[u8; 32], DeviceIdentity> = lane
            .transactions
            .iter()
            .filter_map(|(_, tx)| {
                if let Transaction::RegisterDevice { device_pubkey, .. } = tx {
                    devices.get(device_pubkey).map(|d| (*device_pubkey, d.clone()))
                } else {
                    None
                }
            })
            .collect();

        LaneResult {
            lane_id: lane.lane_id,
            receipts,
            total_fees,
            total_gas,
            state_overlay,
            device_overlay,
        }
    }

    /// Execute a single transaction against a mutable AccountState.
    /// This mirrors `BlockExecutor::execute_transaction` exactly.
    fn execute_transaction_on_state(
        state: &mut AccountState,
        devices: &mut HashMap<[u8; 32], DeviceIdentity>,
        tx: &Transaction,
    ) -> TransactionReceipt {
        let tx_hash = tx.hash();
        let fee = tx.fee();
        let sender = tx.sender();

        // Phase 1: Deduct fee.
        if let Err(e) = state.deduct_fee(&sender, fee) {
            return TransactionReceipt {
                tx_hash,
                success: false,
                gas_used: 0,
                fee_paid: 0,
                error: Some(format!("fee deduction failed: {e}")),
                events: vec![],
            };
        }

        // Phase 2: Execute body.
        let result = Self::execute_body_on_state(state, devices, tx);

        match result {
            Ok(events) => {
                let _ = state.increment_nonce(&sender);
                let gas_used = Self::estimate_gas(tx);
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
                let gas_used = Self::estimate_gas(tx);
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

    /// Execute the body of a transaction (mirrors BlockExecutor::execute_body).
    fn execute_body_on_state(
        state: &mut AccountState,
        devices: &mut HashMap<[u8; 32], DeviceIdentity>,
        tx: &Transaction,
    ) -> DinaResult<Vec<Event>> {
        let sender = tx.sender();
        let account = state
            .get_account(&sender)
            .ok_or_else(|| DinaError::AccountNotFound(sender.to_string()))?;

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
                state.transfer(from, to, *amount)?;
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
                if let Some(acct) = state.get_account(from).cloned() {
                    let mut updated = acct;
                    updated.code_hash = Some(code_hash);
                    state.set_account(updated);
                }
                Ok(vec![Event {
                    contract: Some(*from),
                    name: "ContractDeployed".to_string(),
                    data: code_hash.0.to_vec(),
                }])
            }

            Transaction::CallContract {
                contract,
                usdc_attached,
                from,
                method,
                ..
            } => {
                if *usdc_attached > 0 {
                    state.transfer(from, contract, *usdc_attached)?;
                }
                Ok(vec![Event {
                    contract: Some(*contract),
                    name: format!("ContractCalled::{method}"),
                    data: Vec::new(),
                }])
            }

            Transaction::RegisterDevice {
                device_pubkey,
                owner,
                attestation,
                ..
            } => {
                if devices.contains_key(device_pubkey) {
                    return Err(DinaError::Custom(
                        "device already registered".to_string(),
                    ));
                }

                let device = DeviceIdentity::new(
                    *device_pubkey,
                    *owner,
                    DeviceType::CognitumSeed,
                    attestation.firmware_hash,
                    attestation.witness_root,
                    attestation.timestamp,
                );

                devices.insert(*device_pubkey, device);

                Ok(vec![Event {
                    contract: None,
                    name: "DeviceRegistered".to_string(),
                    data: device_pubkey.to_vec(),
                }])
            }
        }
    }

    /// Estimate gas for a transaction (identical to BlockExecutor).
    fn estimate_gas(tx: &Transaction) -> u64 {
        match tx {
            Transaction::Transfer { .. } => gas::BASE,
            Transaction::DeployContract { wasm_bytecode, .. } => {
                gas::BASE + (wasm_bytecode.len() as u64) * gas::DEPLOY_PER_BYTE
            }
            Transaction::CallContract { .. } => gas::CALL_BASE,
            Transaction::RegisterDevice { .. } => gas::REGISTER_DEVICE,
        }
    }

    // ── Phase 3: Merge ──────────────────────────────────────────────

    /// Merge lane results into the executor's canonical state.
    /// Lanes are merged in order of lane_id (deterministic).
    /// Receipts are re-sorted into original transaction order.
    fn merge_lane_results(
        &mut self,
        mut lane_results: Vec<LaneResult>,
        tx_count: usize,
    ) -> DinaResult<(Vec<TransactionReceipt>, u64, u64)> {
        // Sort lanes by lane_id for deterministic merge order.
        lane_results.sort_by_key(|lr| lr.lane_id);

        let mut total_fees: u64 = 0;
        let mut total_gas: u64 = 0;

        // Collect all (orig_idx, receipt) pairs.
        let mut indexed_receipts: Vec<(usize, TransactionReceipt)> =
            Vec::with_capacity(tx_count);

        // Track which addresses were modified by which lanes for conflict detection.
        let mut addr_to_lane: HashMap<Address, usize> = HashMap::new();
        let mut conflicts_detected = false;

        for lr in &lane_results {
            total_fees = total_fees.saturating_add(lr.total_fees);
            total_gas = total_gas.saturating_add(lr.total_gas);
            indexed_receipts.extend(lr.receipts.iter().cloned());

            // Check for conflicts: if an address appears in multiple lane overlays.
            for addr in lr.state_overlay.keys() {
                if let Some(&prev_lane) = addr_to_lane.get(addr) {
                    if prev_lane != lr.lane_id {
                        conflicts_detected = true;
                    }
                } else {
                    addr_to_lane.insert(*addr, lr.lane_id);
                }
            }
        }

        if conflicts_detected {
            // Conflict detected -- this should not happen if the dependency
            // analysis was correct. Fall back to sequential re-execution is
            // the safe option, but since our union-find is sound, we treat
            // this as an internal error.
            return Err(DinaError::Custom(
                "parallel execution conflict detected: overlapping state modifications across lanes".into(),
            ));
        }

        // Merge state overlays into the canonical state (lane order = deterministic).
        for lr in lane_results {
            for (_addr, account) in lr.state_overlay {
                self.state.set_account(account.clone());
                // Also check: if the base state had this account and the overlay
                // differs, the overlay wins (lane executed against a clone).
            }
            for (pubkey, device) in lr.device_overlay {
                self.devices.insert(pubkey, device);
            }
        }

        // Sort receipts back into original transaction order.
        indexed_receipts.sort_by_key(|(idx, _)| *idx);
        let receipts: Vec<TransactionReceipt> =
            indexed_receipts.into_iter().map(|(_, r)| r).collect();

        Ok((receipts, total_fees, total_gas))
    }

    // ── Sequential fallback ─────────────────────────────────────────

    /// Execute a block sequentially (single lane). Used as fallback for
    /// small blocks or when all transactions are dependent.
    fn execute_sequential(
        &mut self,
        block: &Block,
    ) -> DinaResult<ParallelExecutionResult> {
        let mut receipts = Vec::with_capacity(block.transactions.len());
        let mut total_fees: u64 = 0;
        let mut total_gas: u64 = 0;

        for tx in &block.transactions {
            let receipt =
                Self::execute_transaction_on_state(&mut self.state, &mut self.devices, tx);
            total_fees = total_fees.saturating_add(receipt.fee_paid);
            total_gas = total_gas.saturating_add(receipt.gas_used);
            receipts.push(receipt);
        }

        if total_fees > 0 {
            self.state.credit(&block.header.proposer, total_fees);
        }

        let state_root = self.compute_state_root();

        Ok(ParallelExecutionResult {
            state_root,
            receipts,
            total_fees,
            gas_used: total_gas,
            lanes_used: 1,
            parallelism_ratio: 0.0,
        })
    }

    // ── State root ──────────────────────────────────────────────────

    /// Compute a deterministic state root (identical to BlockExecutor).
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

/// Internal: results from executing a single lane.
struct LaneResult {
    lane_id: usize,
    receipts: Vec<(usize, TransactionReceipt)>,
    total_fees: u64,
    total_gas: u64,
    state_overlay: HashMap<Address, Account>,
    device_overlay: HashMap<[u8; 32], DeviceIdentity>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{Block, BlockHeader};
    use crate::crypto;
    use crate::executor::BlockExecutor;
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

    /// Run the same block through both sequential and parallel executors and
    /// assert that receipts, fees, gas, and state root match exactly.
    fn assert_matches_sequential(
        initial_state: AccountState,
        block: &Block,
    ) {
        // Sequential execution.
        let mut seq_executor = BlockExecutor::new(initial_state.clone());
        let seq_result = seq_executor.execute_block(block).unwrap();

        // Parallel execution (force 1 lane -- should match trivially).
        let mut par1 = ParallelBlockExecutor::with_config(initial_state.clone(), 1, 0);
        let par1_result = par1.execute_block(block).unwrap();

        assert_eq!(seq_result.state_root, par1_result.state_root,
            "state root mismatch (1-lane parallel vs sequential)");
        assert_eq!(seq_result.total_fees, par1_result.total_fees);
        assert_eq!(seq_result.gas_used, par1_result.gas_used);
        assert_eq!(seq_result.receipts.len(), par1_result.receipts.len());
        for (i, (s, p)) in seq_result.receipts.iter().zip(par1_result.receipts.iter()).enumerate() {
            assert_eq!(s.tx_hash, p.tx_hash, "receipt {i} tx_hash mismatch");
            assert_eq!(s.success, p.success, "receipt {i} success mismatch");
            assert_eq!(s.fee_paid, p.fee_paid, "receipt {i} fee_paid mismatch");
            assert_eq!(s.gas_used, p.gas_used, "receipt {i} gas_used mismatch");
        }

        // Parallel execution (force many lanes).
        let mut par_multi = ParallelBlockExecutor::with_config(initial_state.clone(), 8, 0);
        let par_multi_result = par_multi.execute_block(block).unwrap();

        assert_eq!(seq_result.state_root, par_multi_result.state_root,
            "state root mismatch (8-lane parallel vs sequential)");
        assert_eq!(seq_result.total_fees, par_multi_result.total_fees);
        assert_eq!(seq_result.gas_used, par_multi_result.gas_used);
        assert_eq!(seq_result.receipts.len(), par_multi_result.receipts.len());
        for (i, (s, p)) in seq_result.receipts.iter().zip(par_multi_result.receipts.iter()).enumerate() {
            assert_eq!(s.tx_hash, p.tx_hash, "receipt {i} tx_hash mismatch (multi)");
            assert_eq!(s.success, p.success, "receipt {i} success mismatch (multi)");
            assert_eq!(s.fee_paid, p.fee_paid, "receipt {i} fee_paid mismatch (multi)");
            assert_eq!(s.gas_used, p.gas_used, "receipt {i} gas_used mismatch (multi)");
        }
    }

    #[test]
    fn empty_block_matches_sequential() {
        let mut state = AccountState::new();
        let proposer = Address([0x01; 32]);
        state.credit(&proposer, 0);

        let block = make_block(proposer, vec![]);
        assert_matches_sequential(state, &block);
    }

    #[test]
    fn single_transfer_matches_sequential() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let recipient = Address([0xbb; 32]);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&sender, 10_000);

        let tx = make_transfer(&sk, recipient, 1_000, 0, 10);
        let block = make_block(proposer, vec![tx]);
        assert_matches_sequential(state, &block);
    }

    #[test]
    fn independent_transfers_produce_same_result() {
        // Create 6 independent sender/receiver pairs -- no shared addresses.
        let mut state = AccountState::new();
        let proposer = Address([0x01; 32]);
        let mut txs = Vec::new();

        for i in 0u8..6 {
            let (sk, vk) = crypto::generate_keypair();
            let sender = Address::from_pubkey(&vk);
            let recipient = Address([0x10 + i; 32]);
            state.credit(&sender, 100_000);
            txs.push(make_transfer(&sk, recipient, 1_000, 0, 10));
        }

        let block = make_block(proposer, txs);
        assert_matches_sequential(state, &block);
    }

    #[test]
    fn shared_accounts_sequenced_correctly() {
        // Two transactions from the same sender -- must be in the same lane.
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let recipient_a = Address([0xaa; 32]);
        let recipient_b = Address([0xbb; 32]);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&sender, 50_000);

        let tx1 = make_transfer(&sk, recipient_a, 1_000, 0, 10);
        let tx2 = make_transfer(&sk, recipient_b, 2_000, 1, 10);

        let block = make_block(proposer, vec![tx1, tx2]);
        assert_matches_sequential(state, &block);
    }

    #[test]
    fn mixed_independent_and_dependent() {
        // Sender A sends to B; Sender C sends to D (independent pair).
        // Sender A also sends to E (depends on first tx).
        let (sk_a, vk_a) = crypto::generate_keypair();
        let (sk_c, vk_c) = crypto::generate_keypair();
        let addr_a = Address::from_pubkey(&vk_a);
        let addr_c = Address::from_pubkey(&vk_c);
        let addr_b = Address([0xbb; 32]);
        let addr_d = Address([0xdd; 32]);
        let addr_e = Address([0xee; 32]);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&addr_a, 100_000);
        state.credit(&addr_c, 100_000);

        let tx1 = make_transfer(&sk_a, addr_b, 1_000, 0, 10);
        let tx2 = make_transfer(&sk_c, addr_d, 2_000, 0, 10);
        let tx3 = make_transfer(&sk_a, addr_e, 3_000, 1, 10);

        let block = make_block(proposer, vec![tx1, tx2, tx3]);
        assert_matches_sequential(state, &block);
    }

    #[test]
    fn state_root_deterministic_across_lane_counts() {
        // Run the same block with 1, 2, 4, 8 lanes and verify state root is identical.
        let mut state = AccountState::new();
        let proposer = Address([0x01; 32]);
        let mut txs = Vec::new();

        let mut keys = Vec::new();
        for i in 0u8..8 {
            let (sk, vk) = crypto::generate_keypair();
            let sender = Address::from_pubkey(&vk);
            let recipient = Address([0x20 + i; 32]);
            state.credit(&sender, 100_000);
            keys.push(sk.clone());
            txs.push(make_transfer(&sk, recipient, 500, 0, 5));
        }

        let block = make_block(proposer, txs);

        let mut roots = Vec::new();
        for lanes in &[1, 2, 4, 8] {
            let mut executor =
                ParallelBlockExecutor::with_config(state.clone(), *lanes, 0);
            let result = executor.execute_block(&block).unwrap();
            roots.push(result.state_root);
        }

        // All state roots must be identical.
        for (i, root) in roots.iter().enumerate().skip(1) {
            assert_eq!(roots[0], *root, "state root differs with lane count variant {i}");
        }
    }

    #[test]
    fn parallelism_ratio_reported_correctly() {
        // All independent transactions should show high parallelism.
        let mut state = AccountState::new();
        let proposer = Address([0x01; 32]);
        let mut txs = Vec::new();

        for i in 0u8..8 {
            let (sk, vk) = crypto::generate_keypair();
            let sender = Address::from_pubkey(&vk);
            let recipient = Address([0x30 + i; 32]);
            state.credit(&sender, 100_000);
            txs.push(make_transfer(&sk, recipient, 100, 0, 5));
        }

        let block = make_block(proposer, txs);

        let mut executor = ParallelBlockExecutor::with_config(state.clone(), 4, 0);
        let result = executor.execute_block(&block).unwrap();

        // With 8 independent txs across 4 lanes, parallelism ratio should be > 0.
        assert!(
            result.parallelism_ratio > 0.0,
            "expected positive parallelism ratio, got {}",
            result.parallelism_ratio
        );
        assert!(result.lanes_used > 1, "expected multiple lanes");
    }

    #[test]
    fn sequential_fallback_for_small_blocks() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&sender, 100_000);

        // Only 2 txs, but min_txs_for_parallel defaults to 4.
        let tx1 = make_transfer(&sk, Address([0xaa; 32]), 100, 0, 5);
        let tx2 = make_transfer(&sk, Address([0xbb; 32]), 200, 1, 5);
        let block = make_block(proposer, vec![tx1, tx2]);

        let mut executor = ParallelBlockExecutor::new(state.clone());
        let result = executor.execute_block(&block).unwrap();

        // Should fall back to sequential (1 lane).
        assert_eq!(result.lanes_used, 1);
        assert_eq!(result.parallelism_ratio, 0.0);
    }

    #[test]
    fn receipts_in_original_order() {
        // Create many independent transactions and verify receipt order.
        let mut state = AccountState::new();
        let proposer = Address([0x01; 32]);
        let mut txs = Vec::new();
        let mut expected_hashes = Vec::new();

        for i in 0u8..10 {
            let (sk, vk) = crypto::generate_keypair();
            let sender = Address::from_pubkey(&vk);
            let recipient = Address([0x40 + i; 32]);
            state.credit(&sender, 100_000);
            let tx = make_transfer(&sk, recipient, 100, 0, 5);
            expected_hashes.push(tx.hash());
            txs.push(tx);
        }

        let block = make_block(proposer, txs);
        let mut executor = ParallelBlockExecutor::with_config(state, 4, 0);
        let result = executor.execute_block(&block).unwrap();

        assert_eq!(result.receipts.len(), expected_hashes.len());
        for (i, receipt) in result.receipts.iter().enumerate() {
            assert_eq!(
                receipt.tx_hash, expected_hashes[i],
                "receipt at index {i} has wrong tx_hash -- order was not preserved"
            );
        }
    }

    #[test]
    fn insufficient_balance_handled_correctly() {
        let (sk, vk) = crypto::generate_keypair();
        let sender = Address::from_pubkey(&vk);
        let proposer = Address([0x01; 32]);

        let mut state = AccountState::new();
        state.credit(&sender, 100);

        let tx = make_transfer(&sk, Address([0xbb; 32]), 500, 0, 10);
        let block = make_block(proposer, vec![tx]);

        assert_matches_sequential(state, &block);
    }
}
