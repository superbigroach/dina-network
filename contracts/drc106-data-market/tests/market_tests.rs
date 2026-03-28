use drc106_data_market::{dispatch, DataMarketplace};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn data_hash() -> [u8; 32] {
    [2u8; 32]
}

fn init() -> Option<DataMarketplace> {
    let mut state: Option<DataMarketplace> = None;
    dispatch(&mut state, "init", b"", addr(1));
    state
}

fn list_data(state: &mut Option<DataMarketplace>, seller: [u8; 32], price: u64) -> [u8; 32] {
    let sample_hash: [u8; 32] = [0u8; 32];
    let full_hash: [u8; 32] = [1u8; 32];
    let args = serde_json::to_vec(&serde_json::json!({
        "listing": {
            "seller": seller,
            "data_type": "SensorData",
            "description": "Temperature readings",
            "sample_hash": sample_hash,
            "full_hash": full_hash,
            "price": price,
            "license": "SingleUse",
            "vector_count": null,
            "dimensions": null,
            "active": true
        }
    }))
    .unwrap();
    let result = dispatch(state, "list_data", &args, seller);
    serde_json::from_slice(&result).unwrap()
}

fn purchase_listing(
    state: &mut Option<DataMarketplace>,
    buyer: [u8; 32],
    listing_id: [u8; 32],
) -> [u8; 32] {
    let args = serde_json::to_vec(&serde_json::json!({
        "listing_id": listing_id, "timestamp": 1000u64
    }))
    .unwrap();
    let result = dispatch(state, "purchase", &args, buyer);
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn list_and_search() {
    let mut state = init();
    list_data(&mut state, addr(2), 100);
    let args = serde_json::to_vec(&serde_json::json!({"data_type": "SensorData"})).unwrap();
    let result = dispatch(&mut state, "search_by_type", &args, addr(1));
    let ids: Vec<[u8; 32]> = serde_json::from_slice(&result).unwrap();
    assert_eq!(ids.len(), 1);
}

#[test]
fn purchase_creates_escrow() {
    let mut state = init();
    let listing_id = list_data(&mut state, addr(2), 100);
    let purchase_id = purchase_listing(&mut state, addr(3), listing_id);
    assert_eq!(
        *state.as_ref().unwrap().escrow.get(&purchase_id).unwrap(),
        100
    );
}

#[test]
#[should_panic(expected = "seller cannot buy own listing")]
fn self_purchase_fails() {
    let mut state = init();
    let listing_id = list_data(&mut state, addr(2), 100);
    purchase_listing(&mut state, addr(2), listing_id);
}

#[test]
fn deliver_marks_as_delivered() {
    let mut state = init();
    let listing_id = list_data(&mut state, addr(2), 100);
    let purchase_id = purchase_listing(&mut state, addr(3), listing_id);
    let args = serde_json::to_vec(&serde_json::json!({
        "purchase_id": purchase_id, "encrypted_data_hash": data_hash()
    }))
    .unwrap();
    dispatch(&mut state, "deliver", &args, addr(2));
    assert!(
        state
            .as_ref()
            .unwrap()
            .purchases
            .get(&purchase_id)
            .unwrap()
            .delivered
    );
}

#[test]
fn confirm_receipt_releases_escrow() {
    let mut state = init();
    let listing_id = list_data(&mut state, addr(2), 100);
    let purchase_id = purchase_listing(&mut state, addr(3), listing_id);
    let deliver_args = serde_json::to_vec(&serde_json::json!({
        "purchase_id": purchase_id, "encrypted_data_hash": data_hash()
    }))
    .unwrap();
    dispatch(&mut state, "deliver", &deliver_args, addr(2));
    let confirm_args =
        serde_json::to_vec(&serde_json::json!({"purchase_id": purchase_id})).unwrap();
    dispatch(&mut state, "confirm_receipt", &confirm_args, addr(3));
    assert!(!state.as_ref().unwrap().escrow.contains_key(&purchase_id));
}

#[test]
fn rate_after_confirm() {
    let mut state = init();
    let listing_id = list_data(&mut state, addr(2), 100);
    let purchase_id = purchase_listing(&mut state, addr(3), listing_id);
    let del = serde_json::to_vec(
        &serde_json::json!({"purchase_id": purchase_id, "encrypted_data_hash": data_hash()}),
    )
    .unwrap();
    dispatch(&mut state, "deliver", &del, addr(2));
    let conf = serde_json::to_vec(&serde_json::json!({"purchase_id": purchase_id})).unwrap();
    dispatch(&mut state, "confirm_receipt", &conf, addr(3));
    let rate_args = serde_json::to_vec(&serde_json::json!({
        "purchase_id": purchase_id, "rating": 5u8, "review": "Great data!"
    }))
    .unwrap();
    dispatch(&mut state, "rate", &rate_args, addr(3));
    assert_eq!(
        state
            .as_ref()
            .unwrap()
            .purchases
            .get(&purchase_id)
            .unwrap()
            .rating,
        Some(5)
    );
}

#[test]
#[should_panic(expected = "rating must be 1-5")]
fn rate_out_of_range_fails() {
    let mut state = init();
    let listing_id = list_data(&mut state, addr(2), 100);
    let purchase_id = purchase_listing(&mut state, addr(3), listing_id);
    let del = serde_json::to_vec(
        &serde_json::json!({"purchase_id": purchase_id, "encrypted_data_hash": data_hash()}),
    )
    .unwrap();
    dispatch(&mut state, "deliver", &del, addr(2));
    let conf = serde_json::to_vec(&serde_json::json!({"purchase_id": purchase_id})).unwrap();
    dispatch(&mut state, "confirm_receipt", &conf, addr(3));
    let rate_args = serde_json::to_vec(&serde_json::json!({
        "purchase_id": purchase_id, "rating": 10u8, "review": "x"
    }))
    .unwrap();
    dispatch(&mut state, "rate", &rate_args, addr(3));
}

#[test]
fn delist_deactivates_listing() {
    let mut state = init();
    let listing_id = list_data(&mut state, addr(2), 100);
    let args = serde_json::to_vec(&serde_json::json!({"listing_id": listing_id})).unwrap();
    dispatch(&mut state, "delist", &args, addr(2));
    assert!(
        !state
            .as_ref()
            .unwrap()
            .listings
            .get(&listing_id)
            .unwrap()
            .active
    );
}
