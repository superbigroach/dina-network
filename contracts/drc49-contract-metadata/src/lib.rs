use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-49  Contract Metadata  (ERC-7572 equivalent)
// Standard metadata for any contract (name, description, icon, links).
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractMetadataState {
    pub admin: Address,
    /// contract address -> metadata
    pub metadata: BTreeMap<Address, ContractMeta>,
    /// contract address -> owner who registered it
    pub owners: BTreeMap<Address, Address>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractMeta {
    pub name: String,
    pub description: String,
    pub version: String,
    pub icon_url: String,
    pub website: String,
    pub repository: String,
    pub license: String,
    pub tags: Vec<String>,
    pub custom: BTreeMap<String, String>,
}

impl ContractMetadataState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            metadata: BTreeMap::new(),
            owners: BTreeMap::new(),
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn get_metadata(&self, contract: &Address) -> Option<&ContractMeta> {
        self.metadata.get(contract)
    }

    pub fn search_by_tag(&self, tag: &str) -> Vec<(&Address, &ContractMeta)> {
        self.metadata
            .iter()
            .filter(|(_, m)| m.tags.iter().any(|t| t == tag))
            .collect()
    }

    pub fn contracts_with_metadata(&self) -> Vec<&Address> {
        self.metadata.keys().collect()
    }

    // -- Mutations -----------------------------------------------------------

    /// Set metadata for a contract. First setter becomes owner; only owner can update.
    pub fn set_metadata(
        &mut self,
        caller: Address,
        contract: Address,
        meta: ContractMeta,
    ) {
        if let Some(owner) = self.owners.get(&contract) {
            assert!(
                *owner == caller,
                "DRC49: only the original registrant can update metadata"
            );
        } else {
            self.owners.insert(contract, caller);
        }
        self.metadata.insert(contract, meta);
    }

    /// Admin can remove metadata for any contract.
    pub fn remove_metadata(&mut self, caller: Address, contract: Address) {
        assert!(
            caller == self.admin || self.owners.get(&contract) == Some(&caller),
            "DRC49: only admin or owner can remove"
        );
        self.metadata.remove(&contract);
        self.owners.remove(&contract);
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct SetMetadataArgs { contract: Address, meta: ContractMeta }
#[derive(Serialize, Deserialize, Debug)]
struct AddrArg { contract: Address }
#[derive(Serialize, Deserialize, Debug)]
struct TagArg { tag: String }

pub fn dispatch(
    state: &mut Option<ContractMetadataState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC49: already initialised");
            *state = Some(ContractMetadataState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "set_metadata" => {
            let s = state.as_mut().expect("DRC49: not initialised");
            let a: SetMetadataArgs = serde_json::from_slice(args).expect("DRC49: bad args");
            s.set_metadata(caller, a.contract, a.meta);
            serde_json::to_vec("ok").unwrap()
        }
        "get_metadata" => {
            let s = state.as_ref().expect("DRC49: not initialised");
            let a: AddrArg = serde_json::from_slice(args).expect("DRC49: bad args");
            serde_json::to_vec(&s.get_metadata(&a.contract)).unwrap()
        }
        "search_by_tag" => {
            let s = state.as_ref().expect("DRC49: not initialised");
            let a: TagArg = serde_json::from_slice(args).expect("DRC49: bad args");
            let results: Vec<_> = s.search_by_tag(&a.tag).into_iter().map(|(addr, meta)| {
                serde_json::json!({ "address": addr, "meta": meta })
            }).collect();
            serde_json::to_vec(&results).unwrap()
        }
        "contracts_with_metadata" => {
            let s = state.as_ref().expect("DRC49: not initialised");
            serde_json::to_vec(&s.contracts_with_metadata()).unwrap()
        }
        "remove_metadata" => {
            let s = state.as_mut().expect("DRC49: not initialised");
            let a: AddrArg = serde_json::from_slice(args).expect("DRC49: bad args");
            s.remove_metadata(caller, a.contract);
            serde_json::to_vec("ok").unwrap()
        }
        _ => panic!("DRC49: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(n: u8) -> Address { [n; 32] }

    fn sample_meta(name: &str, tags: Vec<&str>) -> ContractMeta {
        ContractMeta {
            name: name.into(),
            description: format!("{name} contract"),
            version: "1.0.0".into(),
            icon_url: "https://example.com/icon.png".into(),
            website: "https://example.com".into(),
            repository: "https://github.com/example".into(),
            license: "MIT".into(),
            tags: tags.into_iter().map(String::from).collect(),
            custom: BTreeMap::new(),
        }
    }

    fn setup() -> Option<ContractMetadataState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", addr(1));
        state
    }

    #[test]
    fn test_set_and_get_metadata() {
        let mut state = setup();
        let args = serde_json::to_vec(&SetMetadataArgs {
            contract: addr(10),
            meta: sample_meta("TokenV1", vec!["defi", "token"]),
        }).unwrap();
        dispatch(&mut state, "set_metadata", &args, addr(2));
        let s = state.as_ref().unwrap();
        let meta = s.get_metadata(&addr(10)).unwrap();
        assert_eq!(meta.name, "TokenV1");
        assert_eq!(meta.tags.len(), 2);
    }

    #[test]
    fn test_search_by_tag() {
        let mut state = setup();
        let a1 = serde_json::to_vec(&SetMetadataArgs {
            contract: addr(10), meta: sample_meta("A", vec!["defi"]),
        }).unwrap();
        let a2 = serde_json::to_vec(&SetMetadataArgs {
            contract: addr(11), meta: sample_meta("B", vec!["nft"]),
        }).unwrap();
        let a3 = serde_json::to_vec(&SetMetadataArgs {
            contract: addr(12), meta: sample_meta("C", vec!["defi", "nft"]),
        }).unwrap();
        dispatch(&mut state, "set_metadata", &a1, addr(2));
        dispatch(&mut state, "set_metadata", &a2, addr(3));
        dispatch(&mut state, "set_metadata", &a3, addr(4));
        let s = state.as_ref().unwrap();
        assert_eq!(s.search_by_tag("defi").len(), 2);
        assert_eq!(s.search_by_tag("nft").len(), 2);
        assert_eq!(s.search_by_tag("gaming").len(), 0);
    }

    #[test]
    fn test_owner_can_update() {
        let mut state = setup();
        let a1 = serde_json::to_vec(&SetMetadataArgs {
            contract: addr(10), meta: sample_meta("V1", vec![]),
        }).unwrap();
        dispatch(&mut state, "set_metadata", &a1, addr(5));
        // Same owner updates
        let a2 = serde_json::to_vec(&SetMetadataArgs {
            contract: addr(10), meta: sample_meta("V2", vec![]),
        }).unwrap();
        dispatch(&mut state, "set_metadata", &a2, addr(5));
        let s = state.as_ref().unwrap();
        assert_eq!(s.get_metadata(&addr(10)).unwrap().name, "V2");
    }

    #[test]
    #[should_panic(expected = "only the original registrant")]
    fn test_non_owner_cannot_update() {
        let mut state = setup();
        let a1 = serde_json::to_vec(&SetMetadataArgs {
            contract: addr(10), meta: sample_meta("V1", vec![]),
        }).unwrap();
        dispatch(&mut state, "set_metadata", &a1, addr(5));
        // Different caller tries to update
        let a2 = serde_json::to_vec(&SetMetadataArgs {
            contract: addr(10), meta: sample_meta("Hacked", vec![]),
        }).unwrap();
        dispatch(&mut state, "set_metadata", &a2, addr(99));
    }

    #[test]
    fn test_contracts_with_metadata() {
        let mut state = setup();
        let a1 = serde_json::to_vec(&SetMetadataArgs {
            contract: addr(10), meta: sample_meta("A", vec![]),
        }).unwrap();
        let a2 = serde_json::to_vec(&SetMetadataArgs {
            contract: addr(11), meta: sample_meta("B", vec![]),
        }).unwrap();
        dispatch(&mut state, "set_metadata", &a1, addr(2));
        dispatch(&mut state, "set_metadata", &a2, addr(3));
        let s = state.as_ref().unwrap();
        assert_eq!(s.contracts_with_metadata().len(), 2);
    }
}
