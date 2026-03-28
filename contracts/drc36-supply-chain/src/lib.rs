use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-36  Supply Chain Tracking
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Checkpoint {
    pub holder: Address,
    pub location: String,
    pub timestamp: u64,
    pub action: String,
    pub signature: String,
    pub metadata: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Item {
    pub id: u64,
    pub name: String,
    pub current_holder: Address,
    pub origin: Address,
    pub created_at: u64,
    pub checkpoints: Vec<Checkpoint>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SupplyChainState {
    pub admin: Address,
    pub items: BTreeMap<u64, Item>,
    pub next_id: u64,
}

impl SupplyChainState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            items: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn create_item(
        &mut self,
        caller: Address,
        name: String,
        created_at: u64,
        location: String,
    ) -> u64 {
        assert!(!name.is_empty(), "DRC36: name cannot be empty");
        let id = self.next_id;
        self.next_id += 1;
        let origin_checkpoint = Checkpoint {
            holder: caller,
            location,
            timestamp: created_at,
            action: "created".to_string(),
            signature: String::new(),
            metadata: String::new(),
        };
        let item = Item {
            id,
            name,
            current_holder: caller,
            origin: caller,
            created_at,
            checkpoints: vec![origin_checkpoint],
        };
        self.items.insert(id, item);
        id
    }

    pub fn transfer_item(
        &mut self,
        caller: Address,
        item_id: u64,
        new_holder: Address,
        location: String,
        timestamp: u64,
    ) {
        let item = self.items.get_mut(&item_id).expect("DRC36: item not found");
        assert!(
            item.current_holder == caller,
            "DRC36: only current holder can transfer"
        );
        item.checkpoints.push(Checkpoint {
            holder: new_holder,
            location,
            timestamp,
            action: "transferred".to_string(),
            signature: String::new(),
            metadata: String::new(),
        });
        item.current_holder = new_holder;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_checkpoint(
        &mut self,
        caller: Address,
        item_id: u64,
        location: String,
        timestamp: u64,
        action: String,
        signature: String,
        metadata: String,
    ) {
        let item = self.items.get_mut(&item_id).expect("DRC36: item not found");
        assert!(
            item.current_holder == caller,
            "DRC36: only current holder can add checkpoints"
        );
        item.checkpoints.push(Checkpoint {
            holder: caller,
            location,
            timestamp,
            action,
            signature,
            metadata,
        });
    }

    pub fn get_provenance(&self, item_id: u64) -> Option<&Vec<Checkpoint>> {
        self.items.get(&item_id).map(|i| &i.checkpoints)
    }

    pub fn verify_authenticity(&self, item_id: u64, claimed_origin: &Address) -> bool {
        self.items
            .get(&item_id)
            .is_some_and(|i| i.origin == *claimed_origin)
    }

    pub fn items_held_by(&self, holder: &Address) -> Vec<&Item> {
        self.items
            .values()
            .filter(|i| i.current_holder == *holder)
            .collect()
    }

    pub fn get_item(&self, id: u64) -> Option<&Item> {
        self.items.get(&id)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateItemArgs {
    name: String,
    created_at: u64,
    location: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferItemArgs {
    item_id: u64,
    new_holder: Address,
    location: String,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddCheckpointArgs {
    item_id: u64,
    location: String,
    timestamp: u64,
    action: String,
    signature: String,
    metadata: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetProvenanceArgs {
    item_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct VerifyAuthenticityArgs {
    item_id: u64,
    claimed_origin: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct ItemsHeldByArgs {
    holder: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetItemArgs {
    id: u64,
}

pub fn dispatch(
    state: &mut Option<SupplyChainState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC36: already initialised");
            *state = Some(SupplyChainState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "create_item" => {
            let s = state.as_mut().expect("DRC36: not initialised");
            let a: CreateItemArgs =
                serde_json::from_slice(args).expect("DRC36: bad create_item args");
            let id = s.create_item(caller, a.name, a.created_at, a.location);
            serde_json::to_vec(&id).unwrap()
        }
        "transfer_item" => {
            let s = state.as_mut().expect("DRC36: not initialised");
            let a: TransferItemArgs =
                serde_json::from_slice(args).expect("DRC36: bad transfer_item args");
            s.transfer_item(caller, a.item_id, a.new_holder, a.location, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }
        "add_checkpoint" => {
            let s = state.as_mut().expect("DRC36: not initialised");
            let a: AddCheckpointArgs =
                serde_json::from_slice(args).expect("DRC36: bad add_checkpoint args");
            s.add_checkpoint(
                caller,
                a.item_id,
                a.location,
                a.timestamp,
                a.action,
                a.signature,
                a.metadata,
            );
            serde_json::to_vec("ok").unwrap()
        }
        "get_provenance" => {
            let s = state.as_ref().expect("DRC36: not initialised");
            let a: GetProvenanceArgs =
                serde_json::from_slice(args).expect("DRC36: bad get_provenance args");
            serde_json::to_vec(&s.get_provenance(a.item_id)).unwrap()
        }
        "verify_authenticity" => {
            let s = state.as_ref().expect("DRC36: not initialised");
            let a: VerifyAuthenticityArgs =
                serde_json::from_slice(args).expect("DRC36: bad verify_authenticity args");
            serde_json::to_vec(&s.verify_authenticity(a.item_id, &a.claimed_origin)).unwrap()
        }
        "items_held_by" => {
            let s = state.as_ref().expect("DRC36: not initialised");
            let a: ItemsHeldByArgs =
                serde_json::from_slice(args).expect("DRC36: bad items_held_by args");
            serde_json::to_vec(&s.items_held_by(&a.holder)).unwrap()
        }
        "get_item" => {
            let s = state.as_ref().expect("DRC36: not initialised");
            let a: GetItemArgs = serde_json::from_slice(args).expect("DRC36: bad get_item args");
            serde_json::to_vec(&s.get_item(a.id)).unwrap()
        }
        _ => panic!("DRC36: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ADMIN: Address = [1u8; 32];
    const ORIGIN: Address = [2u8; 32];
    const WAREHOUSE: Address = [3u8; 32];
    const RETAILER: Address = [4u8; 32];

    fn init_state() -> Option<SupplyChainState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", ADMIN);
        state
    }

    fn create_item_via_dispatch(state: &mut Option<SupplyChainState>) -> u64 {
        let args = serde_json::to_vec(&serde_json::json!({
            "name": "Organic Coffee Beans",
            "created_at": 1700000000u64,
            "location": "Farm-Colombia"
        }))
        .unwrap();
        let result = dispatch(state, "create_item", &args, ORIGIN);
        serde_json::from_slice(&result).unwrap()
    }

    #[test]
    fn test_create_item_with_origin_checkpoint() {
        let mut state = init_state();
        let id = create_item_via_dispatch(&mut state);
        assert_eq!(id, 1);

        let s = state.as_ref().unwrap();
        let item = s.get_item(1).unwrap();
        assert_eq!(item.name, "Organic Coffee Beans");
        assert_eq!(item.origin, ORIGIN);
        assert_eq!(item.current_holder, ORIGIN);
        assert_eq!(item.checkpoints.len(), 1);
        assert_eq!(item.checkpoints[0].action, "created");
    }

    #[test]
    fn test_transfer_item() {
        let mut state = init_state();
        create_item_via_dispatch(&mut state);

        let args = serde_json::to_vec(&serde_json::json!({
            "item_id": 1,
            "new_holder": WAREHOUSE,
            "location": "Port-Cartagena",
            "timestamp": 1700100000u64
        }))
        .unwrap();
        dispatch(&mut state, "transfer_item", &args, ORIGIN);

        let s = state.as_ref().unwrap();
        let item = s.get_item(1).unwrap();
        assert_eq!(item.current_holder, WAREHOUSE);
        assert_eq!(item.checkpoints.len(), 2);
        assert_eq!(item.checkpoints[1].action, "transferred");
    }

    #[test]
    fn test_add_checkpoint() {
        let mut state = init_state();
        create_item_via_dispatch(&mut state);

        let args = serde_json::to_vec(&serde_json::json!({
            "item_id": 1,
            "location": "Quality-Lab",
            "timestamp": 1700050000u64,
            "action": "inspected",
            "signature": "sig123",
            "metadata": "{\"grade\":\"A\"}"
        }))
        .unwrap();
        dispatch(&mut state, "add_checkpoint", &args, ORIGIN);

        let prov_args = serde_json::to_vec(&serde_json::json!({"item_id": 1})).unwrap();
        let result = dispatch(&mut state, "get_provenance", &prov_args, ADMIN);
        let checkpoints: Vec<Checkpoint> = serde_json::from_slice(&result).unwrap();
        assert_eq!(checkpoints.len(), 2);
        assert_eq!(checkpoints[1].action, "inspected");
        assert_eq!(checkpoints[1].metadata, "{\"grade\":\"A\"}");
    }

    #[test]
    fn test_full_provenance_chain() {
        let mut state = init_state();
        create_item_via_dispatch(&mut state);

        // Transfer origin -> warehouse
        let t1 = serde_json::to_vec(&serde_json::json!({
            "item_id": 1, "new_holder": WAREHOUSE,
            "location": "Port", "timestamp": 1700100000u64
        }))
        .unwrap();
        dispatch(&mut state, "transfer_item", &t1, ORIGIN);

        // Transfer warehouse -> retailer
        let t2 = serde_json::to_vec(&serde_json::json!({
            "item_id": 1, "new_holder": RETAILER,
            "location": "Store-Toronto", "timestamp": 1700200000u64
        }))
        .unwrap();
        dispatch(&mut state, "transfer_item", &t2, WAREHOUSE);

        let s = state.as_ref().unwrap();
        let item = s.get_item(1).unwrap();
        assert_eq!(item.current_holder, RETAILER);
        assert_eq!(item.checkpoints.len(), 3);
    }

    #[test]
    fn test_items_held_by() {
        let mut state = init_state();
        create_item_via_dispatch(&mut state);

        // Create second item
        let args2 = serde_json::to_vec(&serde_json::json!({
            "name": "Tea Leaves",
            "created_at": 1700000000u64,
            "location": "Farm-India"
        }))
        .unwrap();
        dispatch(&mut state, "create_item", &args2, ORIGIN);

        // Transfer first item away
        let t = serde_json::to_vec(&serde_json::json!({
            "item_id": 1, "new_holder": WAREHOUSE,
            "location": "Port", "timestamp": 1700100000u64
        }))
        .unwrap();
        dispatch(&mut state, "transfer_item", &t, ORIGIN);

        let held_args = serde_json::to_vec(&serde_json::json!({"holder": ORIGIN})).unwrap();
        let result = dispatch(&mut state, "items_held_by", &held_args, ADMIN);
        let items: Vec<Item> = serde_json::from_slice(&result).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Tea Leaves");
    }

    #[test]
    fn test_verify_authenticity() {
        let mut state = init_state();
        create_item_via_dispatch(&mut state);

        let args_ok = serde_json::to_vec(&serde_json::json!({
            "item_id": 1, "claimed_origin": ORIGIN
        }))
        .unwrap();
        let result = dispatch(&mut state, "verify_authenticity", &args_ok, ADMIN);
        let verified: bool = serde_json::from_slice(&result).unwrap();
        assert!(verified);

        let args_bad = serde_json::to_vec(&serde_json::json!({
            "item_id": 1, "claimed_origin": WAREHOUSE
        }))
        .unwrap();
        let result2 = dispatch(&mut state, "verify_authenticity", &args_bad, ADMIN);
        let verified2: bool = serde_json::from_slice(&result2).unwrap();
        assert!(!verified2);
    }

    #[test]
    #[should_panic(expected = "DRC36: only current holder can transfer")]
    fn test_non_holder_cannot_transfer() {
        let mut state = init_state();
        create_item_via_dispatch(&mut state);

        let args = serde_json::to_vec(&serde_json::json!({
            "item_id": 1, "new_holder": RETAILER,
            "location": "Nowhere", "timestamp": 999u64
        }))
        .unwrap();
        dispatch(&mut state, "transfer_item", &args, WAREHOUSE);
    }
}
