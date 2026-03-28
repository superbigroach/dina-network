use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-51  Balance Snapshots  (ERC-20 Snapshot equivalent)
// Capture token balances at a point in time for governance/dividends.
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SnapshotTokenState {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u64,
    pub admin: Address,
    pub balances: BTreeMap<Address, u64>,
    /// snapshot_id -> Snapshot
    pub snapshots: BTreeMap<u64, Snapshot>,
    pub current_snapshot_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Snapshot {
    pub id: u64,
    pub timestamp: u64,
    pub balances: BTreeMap<Address, u64>,
    pub total_supply: u64,
}

impl SnapshotTokenState {
    pub fn new(name: String, symbol: String, decimals: u8, admin: Address) -> Self {
        Self {
            name,
            symbol,
            decimals,
            total_supply: 0,
            admin,
            balances: BTreeMap::new(),
            snapshots: BTreeMap::new(),
            current_snapshot_id: 0,
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn balance_of(&self, account: &Address) -> u64 {
        self.balances.get(account).copied().unwrap_or(0)
    }

    pub fn balance_of_at(&self, account: &Address, snapshot_id: u64) -> u64 {
        let snap = self
            .snapshots
            .get(&snapshot_id)
            .expect("DRC51: snapshot not found");
        snap.balances.get(account).copied().unwrap_or(0)
    }

    pub fn total_supply_at(&self, snapshot_id: u64) -> u64 {
        let snap = self
            .snapshots
            .get(&snapshot_id)
            .expect("DRC51: snapshot not found");
        snap.total_supply
    }

    pub fn list_snapshots(&self) -> Vec<u64> {
        self.snapshots.keys().copied().collect()
    }

    // -- Mutations -----------------------------------------------------------

    pub fn mint(&mut self, caller: Address, to: Address, amount: u64) {
        assert!(caller == self.admin, "DRC51: only admin can mint");
        assert!(amount > 0, "DRC51: mint amount must be positive");
        let bal = self.balance_of(&to);
        self.balances.insert(to, bal + amount);
        self.total_supply += amount;
    }

    pub fn transfer(&mut self, caller: Address, to: Address, amount: u64) {
        assert!(amount > 0, "DRC51: transfer amount must be positive");
        let from_bal = self.balance_of(&caller);
        assert!(from_bal >= amount, "DRC51: insufficient balance");
        self.balances.insert(caller, from_bal - amount);
        let to_bal = self.balance_of(&to);
        self.balances.insert(to, to_bal + amount);
    }

    /// Take a snapshot of all current balances. Returns the new snapshot id.
    pub fn snapshot(&mut self, caller: Address, timestamp: u64) -> u64 {
        assert!(caller == self.admin, "DRC51: only admin can snapshot");
        self.current_snapshot_id += 1;
        let id = self.current_snapshot_id;
        self.snapshots.insert(
            id,
            Snapshot {
                id,
                timestamp,
                balances: self.balances.clone(),
                total_supply: self.total_supply,
            },
        );
        id
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    name: String,
    symbol: String,
    decimals: u8,
}
#[derive(Serialize, Deserialize, Debug)]
struct TransferArgs {
    to: Address,
    amount: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct MintArgs {
    to: Address,
    amount: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct AddrArg {
    account: Address,
}
#[derive(Serialize, Deserialize, Debug)]
struct BalanceAtArgs {
    account: Address,
    snapshot_id: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct SnapshotIdArg {
    snapshot_id: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct SnapshotArgs {
    timestamp: u64,
}

pub fn dispatch(
    state: &mut Option<SnapshotTokenState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC51: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC51: bad init args");
            *state = Some(SnapshotTokenState::new(
                a.name, a.symbol, a.decimals, caller,
            ));
            serde_json::to_vec("ok").unwrap()
        }
        "balance_of" => {
            let s = state.as_ref().expect("DRC51: not initialised");
            let a: AddrArg = serde_json::from_slice(args).expect("DRC51: bad args");
            serde_json::to_vec(&s.balance_of(&a.account)).unwrap()
        }
        "total_supply" => {
            let s = state.as_ref().expect("DRC51: not initialised");
            serde_json::to_vec(&s.total_supply).unwrap()
        }
        "balance_of_at" => {
            let s = state.as_ref().expect("DRC51: not initialised");
            let a: BalanceAtArgs = serde_json::from_slice(args).expect("DRC51: bad args");
            serde_json::to_vec(&s.balance_of_at(&a.account, a.snapshot_id)).unwrap()
        }
        "total_supply_at" => {
            let s = state.as_ref().expect("DRC51: not initialised");
            let a: SnapshotIdArg = serde_json::from_slice(args).expect("DRC51: bad args");
            serde_json::to_vec(&s.total_supply_at(a.snapshot_id)).unwrap()
        }
        "list_snapshots" => {
            let s = state.as_ref().expect("DRC51: not initialised");
            serde_json::to_vec(&s.list_snapshots()).unwrap()
        }
        "mint" => {
            let s = state.as_mut().expect("DRC51: not initialised");
            let a: MintArgs = serde_json::from_slice(args).expect("DRC51: bad args");
            s.mint(caller, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer" => {
            let s = state.as_mut().expect("DRC51: not initialised");
            let a: TransferArgs = serde_json::from_slice(args).expect("DRC51: bad args");
            s.transfer(caller, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "snapshot" => {
            let s = state.as_mut().expect("DRC51: not initialised");
            let a: SnapshotArgs = serde_json::from_slice(args).expect("DRC51: bad args");
            let id = s.snapshot(caller, a.timestamp);
            serde_json::to_vec(&id).unwrap()
        }
        _ => panic!("DRC51: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(n: u8) -> Address {
        [n; 32]
    }

    fn setup() -> Option<SnapshotTokenState> {
        let mut state = None;
        let args = serde_json::to_vec(&InitArgs {
            name: "SnapToken".into(),
            symbol: "SNP".into(),
            decimals: 6,
        })
        .unwrap();
        dispatch(&mut state, "init", &args, addr(1));
        // Mint to addr(1) and addr(2)
        let m1 = serde_json::to_vec(&MintArgs {
            to: addr(1),
            amount: 1000,
        })
        .unwrap();
        let m2 = serde_json::to_vec(&MintArgs {
            to: addr(2),
            amount: 500,
        })
        .unwrap();
        dispatch(&mut state, "mint", &m1, addr(1));
        dispatch(&mut state, "mint", &m2, addr(1));
        state
    }

    #[test]
    fn test_snapshot_captures_balances() {
        let mut state = setup();
        let snap = serde_json::to_vec(&SnapshotArgs { timestamp: 1000 }).unwrap();
        let result = dispatch(&mut state, "snapshot", &snap, addr(1));
        let snap_id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(snap_id, 1);

        let s = state.as_ref().unwrap();
        assert_eq!(s.balance_of_at(&addr(1), 1), 1000);
        assert_eq!(s.balance_of_at(&addr(2), 1), 500);
        assert_eq!(s.total_supply_at(1), 1500);
    }

    #[test]
    fn test_snapshot_is_immutable_after_transfer() {
        let mut state = setup();
        // Snapshot before transfer
        let snap = serde_json::to_vec(&SnapshotArgs { timestamp: 1000 }).unwrap();
        dispatch(&mut state, "snapshot", &snap, addr(1));

        // Transfer after snapshot
        let xfer = serde_json::to_vec(&TransferArgs {
            to: addr(2),
            amount: 300,
        })
        .unwrap();
        dispatch(&mut state, "transfer", &xfer, addr(1));

        let s = state.as_ref().unwrap();
        // Current balances changed
        assert_eq!(s.balance_of(&addr(1)), 700);
        assert_eq!(s.balance_of(&addr(2)), 800);
        // Snapshot still shows old balances
        assert_eq!(s.balance_of_at(&addr(1), 1), 1000);
        assert_eq!(s.balance_of_at(&addr(2), 1), 500);
    }

    #[test]
    fn test_multiple_snapshots() {
        let mut state = setup();
        let s1 = serde_json::to_vec(&SnapshotArgs { timestamp: 1000 }).unwrap();
        dispatch(&mut state, "snapshot", &s1, addr(1));

        let xfer = serde_json::to_vec(&TransferArgs {
            to: addr(2),
            amount: 200,
        })
        .unwrap();
        dispatch(&mut state, "transfer", &xfer, addr(1));

        let s2 = serde_json::to_vec(&SnapshotArgs { timestamp: 2000 }).unwrap();
        dispatch(&mut state, "snapshot", &s2, addr(1));

        let s = state.as_ref().unwrap();
        assert_eq!(s.balance_of_at(&addr(1), 1), 1000);
        assert_eq!(s.balance_of_at(&addr(1), 2), 800);
        assert_eq!(s.list_snapshots(), vec![1, 2]);
    }

    #[test]
    #[should_panic(expected = "only admin can snapshot")]
    fn test_snapshot_non_admin() {
        let mut state = setup();
        let snap = serde_json::to_vec(&SnapshotArgs { timestamp: 1000 }).unwrap();
        dispatch(&mut state, "snapshot", &snap, addr(99));
    }

    #[test]
    #[should_panic(expected = "snapshot not found")]
    fn test_balance_at_invalid_snapshot() {
        let state = setup();
        let s = state.as_ref().unwrap();
        s.balance_of_at(&addr(1), 999);
    }
}
