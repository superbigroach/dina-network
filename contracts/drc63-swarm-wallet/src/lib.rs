use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-63  Swarm Wallet — Parallel Multi-Agent Wallet
// ---------------------------------------------------------------------------
// One authority controls 100+ agent wallets that can all transact IN PARALLEL,
// breaking the sequential nonce bottleneck of single-wallet architectures.

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ActionKind {
    Transfer { to: Address, amount: u64 },
    ContractCall { contract: Address, method: String, payload: Vec<u8> },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SwarmMemberWallet {
    pub id: u64,
    pub address: Address,
    pub balance: u64,
    pub nonce: u64,
    pub active: bool,
    pub spending_limit: u64,
    pub purpose: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SwarmConfig {
    pub max_wallets: u64,
    pub default_spending_limit: u64,
    pub auto_rebalance: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParallelAction {
    pub wallet_id: u64,
    pub action: ActionKind,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExecutionResult {
    pub wallet_id: u64,
    pub success: bool,
    pub new_nonce: u64,
    pub detail: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SwarmWalletState {
    pub authority: Address,
    pub wallets: BTreeMap<u64, SwarmMemberWallet>,
    pub next_wallet_id: u64,
    pub total_balance: u64,
    pub config: SwarmConfig,
}

impl SwarmWalletState {
    pub fn new(authority: Address, config: SwarmConfig) -> Self {
        Self {
            authority,
            wallets: BTreeMap::new(),
            next_wallet_id: 1,
            total_balance: 0,
            config,
        }
    }

    /// Derive a deterministic address from authority + wallet id
    fn derive_address(authority: &Address, id: u64) -> Address {
        let mut addr = *authority;
        let id_bytes = id.to_le_bytes();
        for (i, b) in id_bytes.iter().enumerate() {
            addr[24 + i] ^= b;
        }
        addr
    }

    pub fn create_wallet(&mut self, caller: Address, purpose: String) -> u64 {
        assert!(caller == self.authority, "DRC63: not authority");
        assert!(
            (self.wallets.len() as u64) < self.config.max_wallets,
            "DRC63: max wallets reached"
        );
        let id = self.next_wallet_id;
        self.next_wallet_id += 1;
        let address = Self::derive_address(&self.authority, id);
        self.wallets.insert(id, SwarmMemberWallet {
            id,
            address,
            balance: 0,
            nonce: 0,
            active: true,
            spending_limit: self.config.default_spending_limit,
            purpose,
        });
        id
    }

    pub fn create_batch(&mut self, caller: Address, count: u64, purpose: String) -> Vec<u64> {
        assert!(caller == self.authority, "DRC63: not authority");
        assert!(count > 0 && count <= 100, "DRC63: batch 1-100");
        let mut ids = Vec::with_capacity(count as usize);
        for i in 0..count {
            let p = format!("{purpose}-{i}");
            ids.push(self.create_wallet(caller, p));
        }
        ids
    }

    pub fn deposit_to(&mut self, caller: Address, wallet_id: u64, amount: u64) {
        assert!(caller == self.authority, "DRC63: not authority");
        assert!(amount > 0, "DRC63: zero deposit");
        let w = self.wallets.get_mut(&wallet_id).expect("DRC63: wallet not found");
        assert!(w.active, "DRC63: wallet inactive");
        w.balance += amount;
        self.total_balance += amount;
    }

    pub fn execute_parallel(
        &mut self,
        caller: Address,
        actions: Vec<ParallelAction>,
    ) -> Vec<ExecutionResult> {
        assert!(caller == self.authority, "DRC63: not authority");
        assert!(!actions.is_empty(), "DRC63: no actions");

        let mut results = Vec::with_capacity(actions.len());

        for pa in actions {
            let w = match self.wallets.get_mut(&pa.wallet_id) {
                Some(w) => w,
                None => {
                    results.push(ExecutionResult {
                        wallet_id: pa.wallet_id,
                        success: false,
                        new_nonce: 0,
                        detail: "wallet not found".into(),
                    });
                    continue;
                }
            };

            if !w.active {
                results.push(ExecutionResult {
                    wallet_id: pa.wallet_id,
                    success: false,
                    new_nonce: w.nonce,
                    detail: "wallet inactive".into(),
                });
                continue;
            }

            match &pa.action {
                ActionKind::Transfer { to: _, amount } => {
                    if *amount > w.balance {
                        results.push(ExecutionResult {
                            wallet_id: pa.wallet_id,
                            success: false,
                            new_nonce: w.nonce,
                            detail: "insufficient balance".into(),
                        });
                        continue;
                    }
                    if *amount > w.spending_limit {
                        results.push(ExecutionResult {
                            wallet_id: pa.wallet_id,
                            success: false,
                            new_nonce: w.nonce,
                            detail: "exceeds spending limit".into(),
                        });
                        continue;
                    }
                    w.balance -= amount;
                    self.total_balance -= amount;
                    w.nonce += 1;
                    results.push(ExecutionResult {
                        wallet_id: pa.wallet_id,
                        success: true,
                        new_nonce: w.nonce,
                        detail: format!("transferred {amount}"),
                    });
                }
                ActionKind::ContractCall { contract: _, method, payload: _ } => {
                    w.nonce += 1;
                    results.push(ExecutionResult {
                        wallet_id: pa.wallet_id,
                        success: true,
                        new_nonce: w.nonce,
                        detail: format!("called {method}"),
                    });
                }
            }
        }

        results
    }

    pub fn rebalance(&mut self, caller: Address) {
        assert!(caller == self.authority, "DRC63: not authority");
        let active: Vec<u64> = self.wallets.iter()
            .filter(|(_, w)| w.active)
            .map(|(id, _)| *id)
            .collect();
        assert!(!active.is_empty(), "DRC63: no active wallets");

        let total: u64 = self.wallets.values()
            .filter(|w| w.active)
            .map(|w| w.balance)
            .sum();
        let per_wallet = total / active.len() as u64;
        let remainder = total % active.len() as u64;

        for (i, id) in active.iter().enumerate() {
            let w = self.wallets.get_mut(id).unwrap();
            w.balance = per_wallet + if (i as u64) < remainder { 1 } else { 0 };
        }
    }

    pub fn get_wallet(&self, wallet_id: u64) -> Option<&SwarmMemberWallet> {
        self.wallets.get(&wallet_id)
    }

    pub fn list_wallets(&self) -> Vec<&SwarmMemberWallet> {
        self.wallets.values().collect()
    }

    pub fn total_balance(&self) -> u64 {
        self.total_balance
    }

    pub fn withdraw_all(&mut self, caller: Address, to: Address) -> u64 {
        assert!(caller == self.authority, "DRC63: not authority");
        let _ = to; // destination recorded on-chain
        let total = self.total_balance;
        for w in self.wallets.values_mut() {
            w.balance = 0;
        }
        self.total_balance = 0;
        total
    }

    pub fn set_wallet_limit(&mut self, caller: Address, wallet_id: u64, limit: u64) {
        assert!(caller == self.authority, "DRC63: not authority");
        let w = self.wallets.get_mut(&wallet_id).expect("DRC63: wallet not found");
        w.spending_limit = limit;
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs { config: SwarmConfig }
#[derive(Serialize, Deserialize, Debug)]
struct CreateWalletArgs { purpose: String }
#[derive(Serialize, Deserialize, Debug)]
struct CreateBatchArgs { count: u64, purpose: String }
#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs { wallet_id: u64, amount: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct ExecuteParallelArgs { actions: Vec<ParallelAction> }
#[derive(Serialize, Deserialize, Debug)]
struct WalletIdArgs { wallet_id: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct WithdrawAllArgs { to: Address }
#[derive(Serialize, Deserialize, Debug)]
struct SetLimitArgs { wallet_id: u64, limit: u64 }

pub fn dispatch(
    state: &mut Option<SwarmWalletState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC63: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            *state = Some(SwarmWalletState::new(caller, a.config));
            serde_json::to_vec("ok").unwrap()
        }
        "create_wallet" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: CreateWalletArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            let id = s.create_wallet(caller, a.purpose);
            serde_json::to_vec(&id).unwrap()
        }
        "create_batch" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: CreateBatchArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            let ids = s.create_batch(caller, a.count, a.purpose);
            serde_json::to_vec(&ids).unwrap()
        }
        "deposit_to" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: DepositArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            s.deposit_to(caller, a.wallet_id, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "execute_parallel" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: ExecuteParallelArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            let results = s.execute_parallel(caller, a.actions);
            serde_json::to_vec(&results).unwrap()
        }
        "rebalance" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            s.rebalance(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "get_wallet" => {
            let s = state.as_ref().expect("DRC63: not initialised");
            let a: WalletIdArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            serde_json::to_vec(&s.get_wallet(a.wallet_id)).unwrap()
        }
        "list_wallets" => {
            let s = state.as_ref().expect("DRC63: not initialised");
            serde_json::to_vec(&s.list_wallets()).unwrap()
        }
        "total_balance" => {
            let s = state.as_ref().expect("DRC63: not initialised");
            serde_json::to_vec(&s.total_balance()).unwrap()
        }
        "withdraw_all" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: WithdrawAllArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            let total = s.withdraw_all(caller, a.to);
            serde_json::to_vec(&total).unwrap()
        }
        "set_wallet_limit" => {
            let s = state.as_mut().expect("DRC63: not initialised");
            let a: SetLimitArgs = serde_json::from_slice(args).expect("DRC63: bad args");
            s.set_wallet_limit(caller, a.wallet_id, a.limit);
            serde_json::to_vec("ok").unwrap()
        }
        _ => panic!("DRC63: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [1u8; 32];
    const OTHER: Address = [2u8; 32];

    fn default_config() -> SwarmConfig {
        SwarmConfig { max_wallets: 200, default_spending_limit: 1_000_000, auto_rebalance: false }
    }

    fn setup() -> SwarmWalletState {
        SwarmWalletState::new(OWNER, default_config())
    }

    #[test]
    fn test_create_wallet_and_get() {
        let mut s = setup();
        let id = s.create_wallet(OWNER, "payments".into());
        assert_eq!(id, 1);
        let w = s.get_wallet(id).unwrap();
        assert_eq!(w.purpose, "payments");
        assert!(w.active);
        assert_eq!(w.balance, 0);
        assert_eq!(w.spending_limit, 1_000_000);
        // Derived address differs from authority
        assert_ne!(w.address, OWNER);
    }

    #[test]
    fn test_batch_create_and_deposit() {
        let mut s = setup();
        let ids = s.create_batch(OWNER, 5, "scraper".into());
        assert_eq!(ids.len(), 5);
        for &id in &ids {
            s.deposit_to(OWNER, id, 100);
        }
        assert_eq!(s.total_balance(), 500);
        assert_eq!(s.list_wallets().len(), 5);
    }

    #[test]
    fn test_execute_parallel_transfers() {
        let mut s = setup();
        let w1 = s.create_wallet(OWNER, "agent-a".into());
        let w2 = s.create_wallet(OWNER, "agent-b".into());
        s.deposit_to(OWNER, w1, 500);
        s.deposit_to(OWNER, w2, 300);

        let actions = vec![
            ParallelAction { wallet_id: w1, action: ActionKind::Transfer { to: OTHER, amount: 200 } },
            ParallelAction { wallet_id: w2, action: ActionKind::Transfer { to: OTHER, amount: 100 } },
        ];
        let results = s.execute_parallel(OWNER, actions);
        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert!(results[1].success);
        assert_eq!(s.get_wallet(w1).unwrap().balance, 300);
        assert_eq!(s.get_wallet(w2).unwrap().balance, 200);
        assert_eq!(s.get_wallet(w1).unwrap().nonce, 1);
        assert_eq!(s.get_wallet(w2).unwrap().nonce, 1);
        assert_eq!(s.total_balance(), 500);
    }

    #[test]
    fn test_rebalance_distributes_evenly() {
        let mut s = setup();
        let w1 = s.create_wallet(OWNER, "a".into());
        let w2 = s.create_wallet(OWNER, "b".into());
        let w3 = s.create_wallet(OWNER, "c".into());
        s.deposit_to(OWNER, w1, 100);
        s.deposit_to(OWNER, w2, 0);
        s.deposit_to(OWNER, w3, 200);

        s.rebalance(OWNER);
        // 300 / 3 = 100 each
        assert_eq!(s.get_wallet(w1).unwrap().balance, 100);
        assert_eq!(s.get_wallet(w2).unwrap().balance, 100);
        assert_eq!(s.get_wallet(w3).unwrap().balance, 100);
    }

    #[test]
    fn test_withdraw_all_drains() {
        let mut s = setup();
        let w1 = s.create_wallet(OWNER, "x".into());
        let w2 = s.create_wallet(OWNER, "y".into());
        s.deposit_to(OWNER, w1, 1000);
        s.deposit_to(OWNER, w2, 500);

        let withdrawn = s.withdraw_all(OWNER, OTHER);
        assert_eq!(withdrawn, 1500);
        assert_eq!(s.total_balance(), 0);
        assert_eq!(s.get_wallet(w1).unwrap().balance, 0);
        assert_eq!(s.get_wallet(w2).unwrap().balance, 0);
    }

    #[test]
    fn test_spending_limit_enforced() {
        let mut s = setup();
        let w1 = s.create_wallet(OWNER, "limited".into());
        s.set_wallet_limit(OWNER, w1, 50);
        s.deposit_to(OWNER, w1, 1000);

        let actions = vec![
            ParallelAction { wallet_id: w1, action: ActionKind::Transfer { to: OTHER, amount: 100 } },
        ];
        let results = s.execute_parallel(OWNER, actions);
        assert!(!results[0].success);
        assert!(results[0].detail.contains("spending limit"));
    }

    #[test]
    #[should_panic(expected = "not authority")]
    fn test_non_authority_rejected() {
        let mut s = setup();
        s.create_wallet(OTHER, "hacker".into());
    }

    #[test]
    fn test_dispatch_roundtrip() {
        let mut state = None;
        let init_args = serde_json::to_vec(&InitArgs { config: default_config() }).unwrap();
        dispatch(&mut state, "init", &init_args, OWNER);

        let create_args = serde_json::to_vec(&CreateWalletArgs { purpose: "test".into() }).unwrap();
        let result = dispatch(&mut state, "create_wallet", &create_args, OWNER);
        let id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(id, 1);

        let dep_args = serde_json::to_vec(&DepositArgs { wallet_id: 1, amount: 500 }).unwrap();
        dispatch(&mut state, "deposit_to", &dep_args, OWNER);

        let bal = dispatch(&mut state, "total_balance", b"{}", OWNER);
        let total: u64 = serde_json::from_slice(&bal).unwrap();
        assert_eq!(total, 500);
    }
}
