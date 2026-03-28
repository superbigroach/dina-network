use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-48  Minimal Proxy  (ERC-7511 equivalent)
// Deploy cheap proxy clones of a contract. Factory pattern.
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MinimalProxyState {
    pub admin: Address,
    /// The implementation contract address that clones delegate to.
    pub implementation: Address,
    /// Clone id -> clone info.
    pub clones: BTreeMap<u64, CloneInfo>,
    pub next_clone_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CloneInfo {
    pub id: u64,
    /// Derived deterministic address from (implementation, id).
    pub address: Address,
    pub creator: Address,
    pub created_at: u64,
    pub init_data: Vec<u8>,
}

impl MinimalProxyState {
    pub fn new(admin: Address, implementation: Address) -> Self {
        Self {
            admin,
            implementation,
            clones: BTreeMap::new(),
            next_clone_id: 1,
        }
    }

    /// Derive a deterministic clone address from implementation + clone_id.
    fn derive_clone_address(implementation: &Address, clone_id: u64) -> Address {
        let mut addr = [0u8; 32];
        // Mix implementation bytes with clone_id for determinism
        for (i, &b) in implementation.iter().enumerate() {
            addr[i] = b ^ ((clone_id >> (i % 8 * 8)) as u8);
        }
        // Stamp the last 8 bytes with the clone_id for uniqueness
        let id_bytes = clone_id.to_le_bytes();
        addr[24..32].copy_from_slice(&id_bytes);
        addr
    }

    // -- Queries -------------------------------------------------------------

    pub fn clone_count(&self) -> u64 {
        self.clones.len() as u64
    }

    pub fn get_clone(&self, id: u64) -> Option<&CloneInfo> {
        self.clones.get(&id)
    }

    pub fn clones_by_creator(&self, creator: &Address) -> Vec<&CloneInfo> {
        self.clones
            .values()
            .filter(|c| &c.creator == creator)
            .collect()
    }

    pub fn implementation(&self) -> &Address {
        &self.implementation
    }

    // -- Mutations -----------------------------------------------------------

    pub fn create_clone(&mut self, caller: Address, init_data: Vec<u8>, timestamp: u64) -> Address {
        let id = self.next_clone_id;
        self.next_clone_id += 1;
        let address = Self::derive_clone_address(&self.implementation, id);
        self.clones.insert(
            id,
            CloneInfo {
                id,
                address,
                creator: caller,
                created_at: timestamp,
                init_data,
            },
        );
        address
    }

    pub fn set_implementation(&mut self, caller: Address, new_impl: Address) {
        assert!(
            caller == self.admin,
            "DRC48: only admin can set implementation"
        );
        self.implementation = new_impl;
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    implementation: Address,
}
#[derive(Serialize, Deserialize, Debug)]
struct CreateCloneArgs {
    init_data: Vec<u8>,
    timestamp: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct CloneIdArg {
    id: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct AddrArg {
    account: Address,
}
#[derive(Serialize, Deserialize, Debug)]
struct SetImplArgs {
    implementation: Address,
}

pub fn dispatch(
    state: &mut Option<MinimalProxyState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC48: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC48: bad init args");
            *state = Some(MinimalProxyState::new(caller, a.implementation));
            serde_json::to_vec("ok").unwrap()
        }
        "create_clone" => {
            let s = state.as_mut().expect("DRC48: not initialised");
            let a: CreateCloneArgs = serde_json::from_slice(args).expect("DRC48: bad args");
            let addr = s.create_clone(caller, a.init_data, a.timestamp);
            serde_json::to_vec(&addr).unwrap()
        }
        "clone_count" => {
            let s = state.as_ref().expect("DRC48: not initialised");
            serde_json::to_vec(&s.clone_count()).unwrap()
        }
        "get_clone" => {
            let s = state.as_ref().expect("DRC48: not initialised");
            let a: CloneIdArg = serde_json::from_slice(args).expect("DRC48: bad args");
            serde_json::to_vec(&s.get_clone(a.id)).unwrap()
        }
        "clones_by_creator" => {
            let s = state.as_ref().expect("DRC48: not initialised");
            let a: AddrArg = serde_json::from_slice(args).expect("DRC48: bad args");
            let clones = s.clones_by_creator(&a.account);
            serde_json::to_vec(&clones).unwrap()
        }
        "implementation" => {
            let s = state.as_ref().expect("DRC48: not initialised");
            serde_json::to_vec(s.implementation()).unwrap()
        }
        "set_implementation" => {
            let s = state.as_mut().expect("DRC48: not initialised");
            let a: SetImplArgs = serde_json::from_slice(args).expect("DRC48: bad args");
            s.set_implementation(caller, a.implementation);
            serde_json::to_vec("ok").unwrap()
        }
        _ => panic!("DRC48: unknown method '{method}'"),
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

    fn setup() -> Option<MinimalProxyState> {
        let mut state = None;
        let args = serde_json::to_vec(&InitArgs {
            implementation: addr(99),
        })
        .unwrap();
        dispatch(&mut state, "init", &args, addr(1));
        state
    }

    #[test]
    fn test_create_clone() {
        let mut state = setup();
        let args = serde_json::to_vec(&CreateCloneArgs {
            init_data: vec![1, 2, 3],
            timestamp: 1000,
        })
        .unwrap();
        let result = dispatch(&mut state, "create_clone", &args, addr(2));
        let clone_addr: Address = serde_json::from_slice(&result).unwrap();
        assert_ne!(clone_addr, [0u8; 32]);
        assert_eq!(state.as_ref().unwrap().clone_count(), 1);
    }

    #[test]
    fn test_multiple_clones_unique_addresses() {
        let mut state = setup();
        let args1 = serde_json::to_vec(&CreateCloneArgs {
            init_data: vec![],
            timestamp: 1000,
        })
        .unwrap();
        let args2 = serde_json::to_vec(&CreateCloneArgs {
            init_data: vec![],
            timestamp: 2000,
        })
        .unwrap();
        let r1 = dispatch(&mut state, "create_clone", &args1, addr(2));
        let r2 = dispatch(&mut state, "create_clone", &args2, addr(2));
        let a1: Address = serde_json::from_slice(&r1).unwrap();
        let a2: Address = serde_json::from_slice(&r2).unwrap();
        assert_ne!(a1, a2);
        assert_eq!(state.as_ref().unwrap().clone_count(), 2);
    }

    #[test]
    fn test_clones_by_creator() {
        let mut state = setup();
        let args = serde_json::to_vec(&CreateCloneArgs {
            init_data: vec![],
            timestamp: 1000,
        })
        .unwrap();
        dispatch(&mut state, "create_clone", &args, addr(3));
        dispatch(&mut state, "create_clone", &args, addr(3));
        dispatch(&mut state, "create_clone", &args, addr(4));
        let s = state.as_ref().unwrap();
        assert_eq!(s.clones_by_creator(&addr(3)).len(), 2);
        assert_eq!(s.clones_by_creator(&addr(4)).len(), 1);
    }

    #[test]
    fn test_get_clone_info() {
        let mut state = setup();
        let args = serde_json::to_vec(&CreateCloneArgs {
            init_data: vec![42],
            timestamp: 5000,
        })
        .unwrap();
        dispatch(&mut state, "create_clone", &args, addr(5));
        let s = state.as_ref().unwrap();
        let clone = s.get_clone(1).unwrap();
        assert_eq!(clone.creator, addr(5));
        assert_eq!(clone.init_data, vec![42]);
        assert_eq!(clone.created_at, 5000);
    }

    #[test]
    fn test_implementation() {
        let state = setup();
        assert_eq!(state.as_ref().unwrap().implementation(), &addr(99));
    }

    #[test]
    #[should_panic(expected = "only admin")]
    fn test_set_implementation_non_admin() {
        let mut state = setup();
        let args = serde_json::to_vec(&SetImplArgs {
            implementation: addr(50),
        })
        .unwrap();
        dispatch(&mut state, "set_implementation", &args, addr(99));
    }
}
