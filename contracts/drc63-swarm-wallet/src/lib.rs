use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-63  Parallel Wallet — Auto-Scaling Parallel Wallet System
// ---------------------------------------------------------------------------
// One authority controls N sub-wallets that each have independent nonces,
// enabling truly parallel on-chain transactions from a single user.
// Features auto-scaling, auto-rebalancing, emergency pause, and consolidation.

// ---------------------------------------------------------------------------
// Core Types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SubWallet {
    pub address: String,
    pub public_key: String,
    pub balance: u64,
    pub nonce: u64,
    pub active: bool,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ParallelAuthority {
    pub owner: String,
    pub sub_wallets: Vec<SubWallet>,
    pub max_wallets: u64,
    pub auto_rebalance: bool,
    pub min_balance_per_wallet: u64,
    pub total_distributed: u64,
    pub created_at: u64,
    pub paused: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ParallelStats {
    pub active_wallets: u64,
    pub total_balance: u64,
    pub avg_balance: u64,
    pub total_txs: u64,
}

// ---------------------------------------------------------------------------
// Contract State
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ParallelWalletState {
    pub authorities: BTreeMap<String, ParallelAuthority>,
    pub next_authority_id: u64,
}

impl ParallelWalletState {
    pub fn new() -> Self {
        Self {
            authorities: BTreeMap::new(),
            next_authority_id: 1,
        }
    }

    /// Derive a deterministic sub-wallet address from authority_id + wallet index.
    fn derive_address(authority_id: &str, index: usize) -> String {
        // In production this would be a proper Ed25519 derivation.
        // Here we produce a deterministic hex string for simulation.
        let input = format!("{authority_id}:{index}");
        let mut hash = [0u8; 32];
        let bytes = input.as_bytes();
        for (i, b) in bytes.iter().enumerate() {
            hash[i % 32] ^= b;
        }
        // Mix further to avoid collisions
        for i in 0..32 {
            hash[i] = hash[i].wrapping_add((index as u8).wrapping_mul(17).wrapping_add(i as u8));
        }
        hex::encode(hash)
    }

    /// Derive a deterministic public key hex from authority_id + wallet index.
    fn derive_public_key(authority_id: &str, index: usize) -> String {
        let input = format!("pubkey:{authority_id}:{index}");
        let mut hash = [0u8; 32];
        let bytes = input.as_bytes();
        for (i, b) in bytes.iter().enumerate() {
            hash[i % 32] ^= b;
        }
        for i in 0..32 {
            hash[i] = hash[i].wrapping_add((index as u8).wrapping_mul(31).wrapping_add(i as u8));
        }
        hex::encode(hash)
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Create a new parallel authority. Returns the authority_id.
    pub fn create_authority(
        &mut self,
        caller: &str,
        owner: String,
        max_wallets: Option<u64>,
        timestamp: u64,
    ) -> String {
        // L-5: Authority owner must be the caller — cannot create for someone else.
        assert!(caller == owner, "DRC63: can only create authority for yourself");
        let id = self.next_authority_id;
        self.next_authority_id += 1;
        let authority_id = format!("pa-{id}");

        let max = max_wallets.unwrap_or(1000);
        assert!(max > 0, "DRC63: max_wallets must be > 0");

        let authority = ParallelAuthority {
            owner,
            sub_wallets: Vec::new(),
            max_wallets: max,
            auto_rebalance: true,
            min_balance_per_wallet: 1_000_000, // 1 USDC in micro-units
            total_distributed: 0,
            created_at: timestamp,
            paused: false,
        };

        self.authorities.insert(authority_id.clone(), authority);
        authority_id
    }

    /// Create N sub-wallets for an authority. Owner only.
    pub fn create_wallets(
        &mut self,
        caller: &str,
        authority_id: &str,
        count: u64,
        timestamp: u64,
    ) -> Vec<usize> {
        let auth = self.authorities.get_mut(authority_id)
            .expect("DRC63: authority not found");
        assert!(auth.owner == caller, "DRC63: not owner");
        assert!(!auth.paused, "DRC63: authority is paused");
        assert!(count > 0, "DRC63: count must be > 0");

        let current_len = auth.sub_wallets.len() as u64;
        assert!(
            current_len + count <= auth.max_wallets,
            "DRC63: would exceed max_wallets (current: {}, requested: {}, max: {})",
            current_len, count, auth.max_wallets
        );

        let mut indices = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let index = auth.sub_wallets.len();
            let address = Self::derive_address(authority_id, index);
            let public_key = Self::derive_public_key(authority_id, index);

            auth.sub_wallets.push(SubWallet {
                address,
                public_key,
                balance: 0,
                nonce: 0,
                active: true,
                created_at: timestamp,
            });

            indices.push(index);
        }

        indices
    }

    /// Auto-scale: create wallets up to needed_count if not enough exist.
    /// Only creates the difference. Owner only.
    pub fn auto_scale(
        &mut self,
        caller: &str,
        authority_id: &str,
        needed_count: u64,
        timestamp: u64,
    ) -> Vec<usize> {
        let auth = self.authorities.get(authority_id)
            .expect("DRC63: authority not found");
        assert!(auth.owner == caller, "DRC63: not owner");
        assert!(!auth.paused, "DRC63: authority is paused");

        let active_count = auth.sub_wallets.iter().filter(|w| w.active).count() as u64;

        if active_count >= needed_count {
            return Vec::new(); // Already have enough
        }

        let to_create = needed_count - active_count;
        self.create_wallets(caller, authority_id, to_create, timestamp)
    }

    /// Distribute total_amount evenly across all active sub-wallets.
    pub fn fund_all(
        &mut self,
        caller: &str,
        authority_id: &str,
        total_amount: u64,
    ) {
        let auth = self.authorities.get_mut(authority_id)
            .expect("DRC63: authority not found");
        assert!(auth.owner == caller, "DRC63: not owner");
        assert!(!auth.paused, "DRC63: authority is paused");
        assert!(total_amount > 0, "DRC63: amount must be > 0");

        let active_indices: Vec<usize> = auth.sub_wallets.iter()
            .enumerate()
            .filter(|(_, w)| w.active)
            .map(|(i, _)| i)
            .collect();

        assert!(!active_indices.is_empty(), "DRC63: no active wallets");

        let count = active_indices.len() as u64;
        let per_wallet = total_amount / count;
        let remainder = total_amount % count;

        for (seq, &idx) in active_indices.iter().enumerate() {
            let extra = if (seq as u64) < remainder { 1 } else { 0 };
            auth.sub_wallets[idx].balance += per_wallet + extra;
        }

        auth.total_distributed += total_amount;
    }

    /// Fund a specific sub-wallet by index.
    pub fn fund_wallet(
        &mut self,
        caller: &str,
        authority_id: &str,
        wallet_index: usize,
        amount: u64,
    ) {
        let auth = self.authorities.get_mut(authority_id)
            .expect("DRC63: authority not found");
        assert!(auth.owner == caller, "DRC63: not owner");
        assert!(!auth.paused, "DRC63: authority is paused");
        assert!(amount > 0, "DRC63: amount must be > 0");
        assert!(wallet_index < auth.sub_wallets.len(), "DRC63: wallet index out of bounds");

        let wallet = &mut auth.sub_wallets[wallet_index];
        assert!(wallet.active, "DRC63: wallet is not active");

        wallet.balance += amount;
        auth.total_distributed += amount;
    }

    /// Drain all sub-wallets back to the owner. Returns total drained.
    pub fn consolidate(
        &mut self,
        caller: &str,
        authority_id: &str,
    ) -> u64 {
        let auth = self.authorities.get_mut(authority_id)
            .expect("DRC63: authority not found");
        assert!(auth.owner == caller, "DRC63: not owner");
        assert!(!auth.paused, "DRC63: authority is paused");

        let mut total_drained: u64 = 0;
        for wallet in auth.sub_wallets.iter_mut() {
            total_drained += wallet.balance;
            wallet.balance = 0;
        }

        auth.total_distributed = auth.total_distributed.saturating_sub(total_drained);
        total_drained
    }

    /// Drain a single sub-wallet back to the owner. Returns the amount drained.
    pub fn consolidate_wallet(
        &mut self,
        caller: &str,
        authority_id: &str,
        wallet_index: usize,
    ) -> u64 {
        let auth = self.authorities.get_mut(authority_id)
            .expect("DRC63: authority not found");
        assert!(auth.owner == caller, "DRC63: not owner");
        assert!(!auth.paused, "DRC63: authority is paused");
        assert!(wallet_index < auth.sub_wallets.len(), "DRC63: wallet index out of bounds");

        let wallet = &mut auth.sub_wallets[wallet_index];
        let drained = wallet.balance;
        wallet.balance = 0;

        auth.total_distributed = auth.total_distributed.saturating_sub(drained);
        drained
    }

    /// Change the safety cap for max wallets. Owner only.
    pub fn set_max_wallets(
        &mut self,
        caller: &str,
        authority_id: &str,
        new_max: u64,
    ) {
        let auth = self.authorities.get_mut(authority_id)
            .expect("DRC63: authority not found");
        assert!(auth.owner == caller, "DRC63: not owner");
        assert!(new_max > 0, "DRC63: max must be > 0");
        assert!(
            new_max >= auth.sub_wallets.len() as u64,
            "DRC63: new max ({}) cannot be less than current wallet count ({})",
            new_max, auth.sub_wallets.len()
        );

        auth.max_wallets = new_max;
    }

    /// Read full authority state.
    pub fn get_authority(&self, authority_id: &str) -> Option<&ParallelAuthority> {
        self.authorities.get(authority_id)
    }

    /// Read a specific sub-wallet.
    pub fn get_sub_wallet(&self, authority_id: &str, index: usize) -> Option<&SubWallet> {
        self.authorities.get(authority_id)
            .and_then(|auth| auth.sub_wallets.get(index))
    }

    /// Return stats for an authority.
    pub fn get_stats(&self, authority_id: &str) -> ParallelStats {
        let auth = self.authorities.get(authority_id)
            .expect("DRC63: authority not found");

        let active_wallets = auth.sub_wallets.iter().filter(|w| w.active).count() as u64;
        let total_balance: u64 = auth.sub_wallets.iter().map(|w| w.balance).sum();
        let total_txs: u64 = auth.sub_wallets.iter().map(|w| w.nonce).sum();
        let avg_balance = if active_wallets > 0 {
            total_balance / active_wallets
        } else {
            0
        };

        ParallelStats {
            active_wallets,
            total_balance,
            avg_balance,
            total_txs,
        }
    }

    /// Pause all operations on an authority. Owner only.
    pub fn pause(&mut self, caller: &str, authority_id: &str) {
        let auth = self.authorities.get_mut(authority_id)
            .expect("DRC63: authority not found");
        assert!(auth.owner == caller, "DRC63: not owner");
        auth.paused = true;
    }

    /// Unpause an authority. Owner only.
    pub fn unpause(&mut self, caller: &str, authority_id: &str) {
        let auth = self.authorities.get_mut(authority_id)
            .expect("DRC63: authority not found");
        assert!(auth.owner == caller, "DRC63: not owner");
        auth.paused = false;
    }

    /// Deactivate and drain a single wallet. Owner only.
    pub fn remove_wallet(
        &mut self,
        caller: &str,
        authority_id: &str,
        index: usize,
    ) -> u64 {
        let auth = self.authorities.get_mut(authority_id)
            .expect("DRC63: authority not found");
        assert!(auth.owner == caller, "DRC63: not owner");
        assert!(!auth.paused, "DRC63: authority is paused");
        assert!(index < auth.sub_wallets.len(), "DRC63: wallet index out of bounds");

        let wallet = &mut auth.sub_wallets[index];
        assert!(wallet.active, "DRC63: wallet already inactive");

        let drained = wallet.balance;
        wallet.balance = 0;
        wallet.active = false;

        auth.total_distributed = auth.total_distributed.saturating_sub(drained);
        drained
    }

    /// Simulate a transfer from a specific sub-wallet (increments nonce).
    /// Used for testing/simulation. In production the runtime handles this.
    pub fn simulate_transfer(
        &mut self,
        caller: &str,
        authority_id: &str,
        wallet_index: usize,
        amount: u64,
    ) {
        let auth = self.authorities.get_mut(authority_id)
            .expect("DRC63: authority not found");
        assert!(auth.owner == caller, "DRC63: not owner");
        assert!(!auth.paused, "DRC63: authority is paused");
        assert!(wallet_index < auth.sub_wallets.len(), "DRC63: wallet index out of bounds");

        let wallet = &mut auth.sub_wallets[wallet_index];
        assert!(wallet.active, "DRC63: wallet is not active");
        assert!(wallet.balance >= amount, "DRC63: insufficient balance");

        wallet.balance -= amount;
        wallet.nonce += 1;
    }
}

// ---------------------------------------------------------------------------
// Dispatch — JSON-RPC-style contract entry point
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateAuthorityArgs {
    owner: String,
    max_wallets: Option<u64>,
    #[serde(default)]
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateWalletsArgs {
    authority_id: String,
    count: u64,
    #[serde(default)]
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AutoScaleArgs {
    authority_id: String,
    needed_count: u64,
    #[serde(default)]
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct FundAllArgs {
    authority_id: String,
    total_amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct FundWalletArgs {
    authority_id: String,
    wallet_index: usize,
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AuthorityIdArgs {
    authority_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ConsolidateWalletArgs {
    authority_id: String,
    wallet_index: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetMaxWalletsArgs {
    authority_id: String,
    new_max: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetSubWalletArgs {
    authority_id: String,
    index: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct RemoveWalletArgs {
    authority_id: String,
    index: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct SimulateTransferArgs {
    authority_id: String,
    wallet_index: usize,
    amount: u64,
}

pub fn dispatch(
    state: &mut Option<ParallelWalletState>,
    method: &str,
    args: &[u8],
    caller: &str,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC63: already initialised");
            *state = Some(ParallelWalletState::new());
            serde_json::to_vec("ok").unwrap()
        }

        "create_authority" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: CreateAuthorityArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            // L-5: Pass caller so create_authority can validate owner == caller.
            let id = s.create_authority(caller, a.owner, a.max_wallets, a.timestamp);
            serde_json::to_vec(&id).unwrap()
        }

        "create_wallets" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: CreateWalletsArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            let indices = s.create_wallets(caller, &a.authority_id, a.count, a.timestamp);
            serde_json::to_vec(&indices).unwrap()
        }

        "auto_scale" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: AutoScaleArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            let indices = s.auto_scale(caller, &a.authority_id, a.needed_count, a.timestamp);
            serde_json::to_vec(&indices).unwrap()
        }

        "fund_all" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: FundAllArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            s.fund_all(caller, &a.authority_id, a.total_amount);
            serde_json::to_vec("ok").unwrap()
        }

        "fund_wallet" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: FundWalletArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            s.fund_wallet(caller, &a.authority_id, a.wallet_index, a.amount);
            serde_json::to_vec("ok").unwrap()
        }

        "consolidate" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: AuthorityIdArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            let total = s.consolidate(caller, &a.authority_id);
            serde_json::to_vec(&total).unwrap()
        }

        "consolidate_wallet" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: ConsolidateWalletArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            let amount = s.consolidate_wallet(caller, &a.authority_id, a.wallet_index);
            serde_json::to_vec(&amount).unwrap()
        }

        "set_max_wallets" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: SetMaxWalletsArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            s.set_max_wallets(caller, &a.authority_id, a.new_max);
            serde_json::to_vec("ok").unwrap()
        }

        "get_authority" => {
            let s = state.as_ref().expect("DRC63: not initialised");
            let a: AuthorityIdArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            serde_json::to_vec(&s.get_authority(&a.authority_id)).unwrap()
        }

        "get_sub_wallet" => {
            let s = state.as_ref().expect("DRC63: not initialised");
            let a: GetSubWalletArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            serde_json::to_vec(&s.get_sub_wallet(&a.authority_id, a.index)).unwrap()
        }

        "get_stats" => {
            let s = state.as_ref().expect("DRC63: not initialised");
            let a: AuthorityIdArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            let stats = s.get_stats(&a.authority_id);
            serde_json::to_vec(&stats).unwrap()
        }

        "pause" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: AuthorityIdArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            s.pause(caller, &a.authority_id);
            serde_json::to_vec("ok").unwrap()
        }

        "unpause" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: AuthorityIdArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            s.unpause(caller, &a.authority_id);
            serde_json::to_vec("ok").unwrap()
        }

        "remove_wallet" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: RemoveWalletArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            let drained = s.remove_wallet(caller, &a.authority_id, a.index);
            serde_json::to_vec(&drained).unwrap()
        }

        "simulate_transfer" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: SimulateTransferArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            s.simulate_transfer(caller, &a.authority_id, a.wallet_index, a.amount);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC63: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Hex encoding helper (no external dep needed beyond serde)
// ---------------------------------------------------------------------------
mod hex {
    pub fn encode(bytes: [u8; 32]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: &str = "dina1_owner_address_0x1111";
    const OTHER: &str = "dina1_other_address_0x2222";

    fn setup() -> ParallelWalletState {
        ParallelWalletState::new()
    }

    fn setup_with_authority() -> (ParallelWalletState, String) {
        let mut state = setup();
        let auth_id = state.create_authority(OWNER, OWNER.to_string(), Some(1000), 1000);
        (state, auth_id)
    }

    // Test 1: Create authority with max wallets
    #[test]
    fn test_create_authority_with_max_wallets() {
        let mut state = setup();
        let auth_id = state.create_authority(OWNER, OWNER.to_string(), Some(500), 1000);

        let auth = state.get_authority(&auth_id).unwrap();
        assert_eq!(auth.owner, OWNER);
        assert_eq!(auth.max_wallets, 500);
        assert_eq!(auth.sub_wallets.len(), 0);
        assert_eq!(auth.total_distributed, 0);
        assert_eq!(auth.created_at, 1000);
        assert!(!auth.paused);
        assert!(auth.auto_rebalance);
    }

    // Test 2: Create sub-wallets
    #[test]
    fn test_create_sub_wallets() {
        let (mut state, auth_id) = setup_with_authority();

        let indices = state.create_wallets(OWNER, &auth_id, 5, 2000);
        assert_eq!(indices.len(), 5);
        assert_eq!(indices, vec![0, 1, 2, 3, 4]);

        let auth = state.get_authority(&auth_id).unwrap();
        assert_eq!(auth.sub_wallets.len(), 5);

        for (i, wallet) in auth.sub_wallets.iter().enumerate() {
            assert!(wallet.active);
            assert_eq!(wallet.balance, 0);
            assert_eq!(wallet.nonce, 0);
            assert_eq!(wallet.created_at, 2000);
            assert!(!wallet.address.is_empty());
            assert!(!wallet.public_key.is_empty());
            // Each wallet should have a unique address
            for (j, other) in auth.sub_wallets.iter().enumerate() {
                if i != j {
                    assert_ne!(wallet.address, other.address);
                }
            }
        }
    }

    // Test 3: Auto-scale creates only what's needed
    #[test]
    fn test_auto_scale_creates_only_needed() {
        let (mut state, auth_id) = setup_with_authority();

        // Create 3 wallets first
        state.create_wallets(OWNER, &auth_id, 3, 1000);
        assert_eq!(state.get_authority(&auth_id).unwrap().sub_wallets.len(), 3);

        // Auto-scale to 3 — should create nothing
        let created = state.auto_scale(OWNER, &auth_id, 3, 2000);
        assert!(created.is_empty());
        assert_eq!(state.get_authority(&auth_id).unwrap().sub_wallets.len(), 3);

        // Auto-scale to 7 — should create 4 more
        let created = state.auto_scale(OWNER, &auth_id, 7, 3000);
        assert_eq!(created.len(), 4);
        assert_eq!(state.get_authority(&auth_id).unwrap().sub_wallets.len(), 7);

        // Auto-scale to 5 — already have 7 active, should create nothing
        let created = state.auto_scale(OWNER, &auth_id, 5, 4000);
        assert!(created.is_empty());
        assert_eq!(state.get_authority(&auth_id).unwrap().sub_wallets.len(), 7);
    }

    // Test 4: Fund all distributes evenly
    #[test]
    fn test_fund_all_distributes_evenly() {
        let (mut state, auth_id) = setup_with_authority();
        state.create_wallets(OWNER, &auth_id, 4, 1000);

        // 1000 / 4 = 250 each
        state.fund_all(OWNER, &auth_id, 1000);

        let auth = state.get_authority(&auth_id).unwrap();
        for wallet in &auth.sub_wallets {
            assert_eq!(wallet.balance, 250);
        }
        assert_eq!(auth.total_distributed, 1000);

        // Test with remainder: 10 / 4 = 2 each + 2 remainder (first 2 get +1)
        state.fund_all(OWNER, &auth_id, 10);
        let auth = state.get_authority(&auth_id).unwrap();
        assert_eq!(auth.sub_wallets[0].balance, 253); // 250 + 2 + 1 (remainder)
        assert_eq!(auth.sub_wallets[1].balance, 253); // 250 + 2 + 1 (remainder)
        assert_eq!(auth.sub_wallets[2].balance, 252); // 250 + 2
        assert_eq!(auth.sub_wallets[3].balance, 252); // 250 + 2
    }

    // Test 5: Fund specific wallet
    #[test]
    fn test_fund_specific_wallet() {
        let (mut state, auth_id) = setup_with_authority();
        state.create_wallets(OWNER, &auth_id, 3, 1000);

        state.fund_wallet(OWNER, &auth_id, 1, 500);

        let auth = state.get_authority(&auth_id).unwrap();
        assert_eq!(auth.sub_wallets[0].balance, 0);
        assert_eq!(auth.sub_wallets[1].balance, 500);
        assert_eq!(auth.sub_wallets[2].balance, 0);
        assert_eq!(auth.total_distributed, 500);
    }

    // Test 6: Consolidate drains all wallets back
    #[test]
    fn test_consolidate_drains_all() {
        let (mut state, auth_id) = setup_with_authority();
        state.create_wallets(OWNER, &auth_id, 3, 1000);

        state.fund_wallet(OWNER, &auth_id, 0, 100);
        state.fund_wallet(OWNER, &auth_id, 1, 200);
        state.fund_wallet(OWNER, &auth_id, 2, 300);

        let drained = state.consolidate(OWNER, &auth_id);
        assert_eq!(drained, 600);

        let auth = state.get_authority(&auth_id).unwrap();
        for wallet in &auth.sub_wallets {
            assert_eq!(wallet.balance, 0);
        }
    }

    // Test 7: Cannot exceed max_wallets
    #[test]
    #[should_panic(expected = "would exceed max_wallets")]
    fn test_cannot_exceed_max_wallets() {
        let mut state = setup();
        let auth_id = state.create_authority(OWNER, OWNER.to_string(), Some(5), 1000);

        state.create_wallets(OWNER, &auth_id, 3, 1000);
        // This should panic: 3 + 3 = 6 > 5
        state.create_wallets(OWNER, &auth_id, 3, 2000);
    }

    // Test 8: Pause prevents operations
    #[test]
    #[should_panic(expected = "authority is paused")]
    fn test_pause_prevents_operations() {
        let (mut state, auth_id) = setup_with_authority();

        state.pause(OWNER, &auth_id);

        let auth = state.get_authority(&auth_id).unwrap();
        assert!(auth.paused);

        // Should panic because paused
        state.create_wallets(OWNER, &auth_id, 1, 2000);
    }

    // Test 9: Only owner can manage
    #[test]
    #[should_panic(expected = "not owner")]
    fn test_only_owner_can_manage() {
        let (mut state, auth_id) = setup_with_authority();

        // OTHER trying to create wallets should panic
        state.create_wallets(OTHER, &auth_id, 1, 2000);
    }

    // Test 10: Stats are accurate
    #[test]
    fn test_stats_are_accurate() {
        let (mut state, auth_id) = setup_with_authority();
        state.create_wallets(OWNER, &auth_id, 4, 1000);

        state.fund_wallet(OWNER, &auth_id, 0, 100);
        state.fund_wallet(OWNER, &auth_id, 1, 200);
        state.fund_wallet(OWNER, &auth_id, 2, 300);
        // wallet 3 has 0 balance

        // Simulate some transactions
        state.simulate_transfer(OWNER, &auth_id, 0, 50);
        state.simulate_transfer(OWNER, &auth_id, 1, 100);
        state.simulate_transfer(OWNER, &auth_id, 1, 50);

        let stats = state.get_stats(&auth_id);
        assert_eq!(stats.active_wallets, 4);
        assert_eq!(stats.total_balance, 400); // 50 + 50 + 300 + 0
        assert_eq!(stats.avg_balance, 100);   // 400 / 4
        assert_eq!(stats.total_txs, 3);       // 1 + 2 + 0 + 0
    }

    // Test 11: Unpause restores operations
    #[test]
    fn test_unpause_restores_operations() {
        let (mut state, auth_id) = setup_with_authority();

        state.pause(OWNER, &auth_id);
        assert!(state.get_authority(&auth_id).unwrap().paused);

        state.unpause(OWNER, &auth_id);
        assert!(!state.get_authority(&auth_id).unwrap().paused);

        // Should work again
        let indices = state.create_wallets(OWNER, &auth_id, 2, 3000);
        assert_eq!(indices.len(), 2);
    }

    // Test 12: Remove wallet deactivates and drains
    #[test]
    fn test_remove_wallet_deactivates_and_drains() {
        let (mut state, auth_id) = setup_with_authority();
        state.create_wallets(OWNER, &auth_id, 3, 1000);
        state.fund_wallet(OWNER, &auth_id, 1, 500);

        let drained = state.remove_wallet(OWNER, &auth_id, 1);
        assert_eq!(drained, 500);

        let wallet = state.get_sub_wallet(&auth_id, 1).unwrap();
        assert!(!wallet.active);
        assert_eq!(wallet.balance, 0);
    }

    // Test 13: Consolidate single wallet
    #[test]
    fn test_consolidate_single_wallet() {
        let (mut state, auth_id) = setup_with_authority();
        state.create_wallets(OWNER, &auth_id, 3, 1000);
        state.fund_wallet(OWNER, &auth_id, 0, 100);
        state.fund_wallet(OWNER, &auth_id, 1, 200);
        state.fund_wallet(OWNER, &auth_id, 2, 300);

        let drained = state.consolidate_wallet(OWNER, &auth_id, 1);
        assert_eq!(drained, 200);

        // Only wallet 1 should be drained
        assert_eq!(state.get_sub_wallet(&auth_id, 0).unwrap().balance, 100);
        assert_eq!(state.get_sub_wallet(&auth_id, 1).unwrap().balance, 0);
        assert_eq!(state.get_sub_wallet(&auth_id, 2).unwrap().balance, 300);
    }

    // Test 14: Set max wallets
    #[test]
    fn test_set_max_wallets() {
        let (mut state, auth_id) = setup_with_authority();
        state.create_wallets(OWNER, &auth_id, 5, 1000);

        // Can increase
        state.set_max_wallets(OWNER, &auth_id, 2000);
        assert_eq!(state.get_authority(&auth_id).unwrap().max_wallets, 2000);
    }

    // Test 15: Cannot set max_wallets below current count
    #[test]
    #[should_panic(expected = "cannot be less than current wallet count")]
    fn test_cannot_set_max_below_current() {
        let (mut state, auth_id) = setup_with_authority();
        state.create_wallets(OWNER, &auth_id, 5, 1000);

        // 3 < 5 (current count) — should panic
        state.set_max_wallets(OWNER, &auth_id, 3);
    }

    // Test 16: Dispatch roundtrip
    #[test]
    fn test_dispatch_roundtrip() {
        let mut contract_state: Option<ParallelWalletState> = None;

        // Init
        dispatch(&mut contract_state, "init", b"{}", OWNER);
        assert!(contract_state.is_some());

        // Create authority
        let args = serde_json::to_vec(&CreateAuthorityArgs {
            owner: OWNER.to_string(),
            max_wallets: Some(100),
            timestamp: 1000,
        }).unwrap();
        let result = dispatch(&mut contract_state, "create_authority", &args, OWNER);
        let auth_id: String = serde_json::from_slice(&result).unwrap();
        assert!(auth_id.starts_with("pa-"));

        // Create wallets
        let args = serde_json::to_vec(&CreateWalletsArgs {
            authority_id: auth_id.clone(),
            count: 3,
            timestamp: 2000,
        }).unwrap();
        let result = dispatch(&mut contract_state, "create_wallets", &args, OWNER);
        let indices: Vec<usize> = serde_json::from_slice(&result).unwrap();
        assert_eq!(indices.len(), 3);

        // Fund all
        let args = serde_json::to_vec(&FundAllArgs {
            authority_id: auth_id.clone(),
            total_amount: 3000,
        }).unwrap();
        dispatch(&mut contract_state, "fund_all", &args, OWNER);

        // Get stats
        let args = serde_json::to_vec(&AuthorityIdArgs {
            authority_id: auth_id.clone(),
        }).unwrap();
        let result = dispatch(&mut contract_state, "get_stats", &args, OWNER);
        let stats: ParallelStats = serde_json::from_slice(&result).unwrap();
        assert_eq!(stats.active_wallets, 3);
        assert_eq!(stats.total_balance, 3000);
        assert_eq!(stats.avg_balance, 1000);

        // Consolidate
        let args = serde_json::to_vec(&AuthorityIdArgs {
            authority_id: auth_id,
        }).unwrap();
        let result = dispatch(&mut contract_state, "consolidate", &args, OWNER);
        let total: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(total, 3000);
    }

    // Test 17: Auto-scale considers deactivated wallets
    #[test]
    fn test_auto_scale_considers_deactivated() {
        let (mut state, auth_id) = setup_with_authority();
        state.create_wallets(OWNER, &auth_id, 5, 1000);

        // Remove 2 wallets
        state.remove_wallet(OWNER, &auth_id, 1);
        state.remove_wallet(OWNER, &auth_id, 3);

        // Now 3 active wallets. Auto-scale to 5 should create 2 more.
        let created = state.auto_scale(OWNER, &auth_id, 5, 2000);
        assert_eq!(created.len(), 2);

        // Total wallets: 7 (5 original + 2 new), 5 active
        let stats = state.get_stats(&auth_id);
        assert_eq!(stats.active_wallets, 5);
    }

    // Test 18: Multiple authorities are independent
    #[test]
    fn test_multiple_authorities_independent() {
        let mut state = setup();

        let auth1 = state.create_authority(OWNER, OWNER.to_string(), Some(10), 1000);
        let auth2 = state.create_authority(OTHER, OTHER.to_string(), Some(20), 1000);

        state.create_wallets(OWNER, &auth1, 3, 1000);
        state.create_wallets(OTHER, &auth2, 5, 1000);

        let stats1 = state.get_stats(&auth1);
        let stats2 = state.get_stats(&auth2);

        assert_eq!(stats1.active_wallets, 3);
        assert_eq!(stats2.active_wallets, 5);
    }
}
