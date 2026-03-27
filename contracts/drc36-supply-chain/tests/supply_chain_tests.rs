use drc36_supply_chain::{dispatch, SupplyChainState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init_chain(admin: [u8; 32]) -> Option<SupplyChainState> {
    let mut state: Option<SupplyChainState> = None;
    dispatch(&mut state, "init", b"{}", admin);
    state
}

fn create_test_item(state: &mut Option<SupplyChainState>, creator: [u8; 32]) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({
        "name": "Organic Coffee",
        "created_at": 1000u64,
        "location": "Colombia"
    }))
    .unwrap();
    let result = dispatch(state, "create_item", &args, creator);
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn create_item_with_origin_checkpoint() {
    let creator = addr(1);
    let mut state = init_chain(creator);
    let id = create_test_item(&mut state, creator);

    let s = state.as_ref().unwrap();
    let item = s.get_item(id).unwrap();
    assert_eq!(item.name, "Organic Coffee");
    assert_eq!(item.origin, creator);
    assert_eq!(item.checkpoints.len(), 1);
    assert_eq!(item.checkpoints[0].action, "created");
}

#[test]
fn transfer_item_updates_holder() {
    let producer = addr(1);
    let distributor = addr(2);
    let mut state = init_chain(producer);
    let id = create_test_item(&mut state, producer);

    let args = serde_json::to_vec(&serde_json::json!({
        "item_id": id,
        "new_holder": distributor,
        "location": "Miami Port",
        "timestamp": 2000u64
    }))
    .unwrap();
    dispatch(&mut state, "transfer_item", &args, producer);

    let s = state.as_ref().unwrap();
    let item = s.get_item(id).unwrap();
    assert_eq!(item.current_holder, distributor);
    assert_eq!(item.checkpoints.len(), 2);
}

#[test]
fn full_provenance_chain() {
    let producer = addr(1);
    let distributor = addr(2);
    let retailer = addr(3);
    let mut state = init_chain(producer);
    let id = create_test_item(&mut state, producer);

    // Transfer to distributor
    let args1 = serde_json::to_vec(&serde_json::json!({
        "item_id": id, "new_holder": distributor,
        "location": "Miami", "timestamp": 2000u64
    }))
    .unwrap();
    dispatch(&mut state, "transfer_item", &args1, producer);

    // Distributor adds checkpoint
    let cp_args = serde_json::to_vec(&serde_json::json!({
        "item_id": id, "location": "Warehouse",
        "timestamp": 2500u64, "action": "quality_check",
        "signature": "sig_abc", "metadata": "passed"
    }))
    .unwrap();
    dispatch(&mut state, "add_checkpoint", &cp_args, distributor);

    // Transfer to retailer
    let args2 = serde_json::to_vec(&serde_json::json!({
        "item_id": id, "new_holder": retailer,
        "location": "Toronto Store", "timestamp": 3000u64
    }))
    .unwrap();
    dispatch(&mut state, "transfer_item", &args2, distributor);

    let s = state.as_ref().unwrap();
    let provenance = s.get_provenance(id).unwrap();
    assert_eq!(provenance.len(), 4); // created + transfer + checkpoint + transfer
}

#[test]
fn verify_authenticity_correct_origin() {
    let producer = addr(1);
    let mut state = init_chain(producer);
    let id = create_test_item(&mut state, producer);

    let s = state.as_ref().unwrap();
    assert!(s.verify_authenticity(id, &producer));
    assert!(!s.verify_authenticity(id, &addr(99)));
}

#[test]
fn items_held_by_returns_correct() {
    let producer = addr(1);
    let mut state = init_chain(producer);
    create_test_item(&mut state, producer);
    create_test_item(&mut state, producer);

    let s = state.as_ref().unwrap();
    assert_eq!(s.items_held_by(&producer).len(), 2);
    assert_eq!(s.items_held_by(&addr(99)).len(), 0);
}

#[test]
#[should_panic(expected = "only current holder can transfer")]
fn non_holder_cannot_transfer() {
    let producer = addr(1);
    let thief = addr(99);
    let mut state = init_chain(producer);
    let id = create_test_item(&mut state, producer);

    let args = serde_json::to_vec(&serde_json::json!({
        "item_id": id, "new_holder": thief,
        "location": "Stolen", "timestamp": 5000u64
    }))
    .unwrap();
    dispatch(&mut state, "transfer_item", &args, thief);
}
