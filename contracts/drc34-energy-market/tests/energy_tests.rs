use drc34_energy_market::{dispatch, DeliveryStatus, EnergyMarketState};

fn addr(seed: u8) -> [u8; 32] {
    [seed; 32]
}

fn init_market(admin: [u8; 32]) -> Option<EnergyMarketState> {
    let mut state: Option<EnergyMarketState> = None;
    dispatch(&mut state, "init", b"{}", admin);
    state
}

fn list_test_offer(state: &mut Option<EnergyMarketState>, seller: [u8; 32]) -> u64 {
    let args = serde_json::to_vec(&serde_json::json!({
        "kwh_available": 100u64,
        "price_per_kwh": 10u64,
        "location": "Toronto",
        "valid_until": 9999u64
    }))
    .unwrap();
    let result = dispatch(state, "list_energy", &args, seller);
    serde_json::from_slice(&result).unwrap()
}

#[test]
fn list_and_retrieve_offer() {
    let seller = addr(1);
    let mut state = init_market(seller);
    let id = list_test_offer(&mut state, seller);

    let s = state.as_ref().unwrap();
    let offer = s.get_offer(id).unwrap();
    assert_eq!(offer.kwh_available, 100);
    assert_eq!(offer.price_per_kwh, 10);
    assert!(offer.active);
}

#[test]
fn buy_energy_reduces_available() {
    let seller = addr(1);
    let buyer = addr(2);
    let mut state = init_market(seller);
    let offer_id = list_test_offer(&mut state, seller);

    let args = serde_json::to_vec(&serde_json::json!({
        "offer_id": offer_id,
        "kwh": 30u64
    }))
    .unwrap();
    dispatch(&mut state, "buy_energy", &args, buyer);

    let s = state.as_ref().unwrap();
    assert_eq!(s.get_offer(offer_id).unwrap().kwh_available, 70);
    assert_eq!(s.my_purchases(&buyer).len(), 1);
    assert_eq!(s.my_purchases(&buyer)[0].total_price, 300);
}

#[test]
fn buy_all_deactivates_offer() {
    let seller = addr(1);
    let buyer = addr(2);
    let mut state = init_market(seller);
    let offer_id = list_test_offer(&mut state, seller);

    let args = serde_json::to_vec(&serde_json::json!({
        "offer_id": offer_id,
        "kwh": 100u64
    }))
    .unwrap();
    dispatch(&mut state, "buy_energy", &args, buyer);

    let s = state.as_ref().unwrap();
    assert!(!s.get_offer(offer_id).unwrap().active);
    assert_eq!(s.available_energy().len(), 0);
}

#[test]
fn confirm_delivery_updates_status() {
    let seller = addr(1);
    let buyer = addr(2);
    let mut state = init_market(seller);
    let offer_id = list_test_offer(&mut state, seller);

    let buy_args = serde_json::to_vec(&serde_json::json!({
        "offer_id": offer_id,
        "kwh": 50u64
    }))
    .unwrap();
    let result = dispatch(&mut state, "buy_energy", &buy_args, buyer);
    let purchase_id: u64 = serde_json::from_slice(&result).unwrap();

    let confirm_args =
        serde_json::to_vec(&serde_json::json!({ "purchase_id": purchase_id })).unwrap();
    dispatch(&mut state, "confirm_delivery", &confirm_args, seller);

    let s = state.as_ref().unwrap();
    assert_eq!(s.my_purchases(&buyer)[0].status, DeliveryStatus::Delivered);
}

#[test]
#[should_panic(expected = "cannot buy own energy")]
fn cannot_buy_own_energy() {
    let seller = addr(1);
    let mut state = init_market(seller);
    let offer_id = list_test_offer(&mut state, seller);

    let args = serde_json::to_vec(&serde_json::json!({
        "offer_id": offer_id,
        "kwh": 10u64
    }))
    .unwrap();
    dispatch(&mut state, "buy_energy", &args, seller);
}

#[test]
#[should_panic(expected = "insufficient energy")]
fn cannot_buy_more_than_available() {
    let seller = addr(1);
    let buyer = addr(2);
    let mut state = init_market(seller);
    let offer_id = list_test_offer(&mut state, seller);

    let args = serde_json::to_vec(&serde_json::json!({
        "offer_id": offer_id,
        "kwh": 200u64
    }))
    .unwrap();
    dispatch(&mut state, "buy_energy", &args, buyer);
}
