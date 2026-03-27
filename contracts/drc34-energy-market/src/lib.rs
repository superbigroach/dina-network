use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-34  Energy Trading for IoT
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EnergyOffer {
    pub id: u64,
    pub seller: Address,
    pub kwh_available: u64, // milliwatt-hours for precision
    pub price_per_kwh: u64,
    pub location: String,
    pub valid_until: u64,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    Disputed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EnergyPurchase {
    pub id: u64,
    pub buyer: Address,
    pub seller: Address,
    pub offer_id: u64,
    pub kwh: u64,
    pub total_price: u64,
    pub status: DeliveryStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EnergyMarketState {
    pub admin: Address,
    pub offers: BTreeMap<u64, EnergyOffer>,
    pub purchases: BTreeMap<u64, EnergyPurchase>,
    pub next_offer_id: u64,
    pub next_purchase_id: u64,
}

impl EnergyMarketState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            offers: BTreeMap::new(),
            purchases: BTreeMap::new(),
            next_offer_id: 1,
            next_purchase_id: 1,
        }
    }

    pub fn list_energy(
        &mut self,
        caller: Address,
        kwh_available: u64,
        price_per_kwh: u64,
        location: String,
        valid_until: u64,
    ) -> u64 {
        assert!(kwh_available > 0, "DRC34: kwh must be positive");
        assert!(price_per_kwh > 0, "DRC34: price must be positive");
        let id = self.next_offer_id;
        self.next_offer_id += 1;
        let offer = EnergyOffer {
            id,
            seller: caller,
            kwh_available,
            price_per_kwh,
            location,
            valid_until,
            active: true,
        };
        self.offers.insert(id, offer);
        id
    }

    pub fn buy_energy(&mut self, caller: Address, offer_id: u64, kwh: u64) -> u64 {
        let offer = self
            .offers
            .get_mut(&offer_id)
            .expect("DRC34: offer not found");
        assert!(offer.active, "DRC34: offer not active");
        assert!(offer.seller != caller, "DRC34: cannot buy own energy");
        assert!(
            kwh <= offer.kwh_available,
            "DRC34: insufficient energy available"
        );
        let total_price = kwh * offer.price_per_kwh;
        offer.kwh_available -= kwh;
        if offer.kwh_available == 0 {
            offer.active = false;
        }
        let purchase_id = self.next_purchase_id;
        self.next_purchase_id += 1;
        let purchase = EnergyPurchase {
            id: purchase_id,
            buyer: caller,
            seller: offer.seller,
            offer_id,
            kwh,
            total_price,
            status: DeliveryStatus::Pending,
        };
        self.purchases.insert(purchase_id, purchase);
        purchase_id
    }

    pub fn confirm_delivery(&mut self, caller: Address, purchase_id: u64) {
        let purchase = self
            .purchases
            .get_mut(&purchase_id)
            .expect("DRC34: purchase not found");
        assert!(
            purchase.seller == caller,
            "DRC34: only seller can confirm delivery"
        );
        assert!(
            purchase.status == DeliveryStatus::Pending,
            "DRC34: not pending"
        );
        purchase.status = DeliveryStatus::Delivered;
    }

    pub fn available_energy(&self) -> Vec<&EnergyOffer> {
        self.offers.values().filter(|o| o.active).collect()
    }

    pub fn my_purchases(&self, buyer: &Address) -> Vec<&EnergyPurchase> {
        self.purchases
            .values()
            .filter(|p| p.buyer == *buyer)
            .collect()
    }

    pub fn get_offer(&self, id: u64) -> Option<&EnergyOffer> {
        self.offers.get(&id)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct ListEnergyArgs {
    kwh_available: u64,
    price_per_kwh: u64,
    location: String,
    valid_until: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BuyEnergyArgs {
    offer_id: u64,
    kwh: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ConfirmDeliveryArgs {
    purchase_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct MyPurchasesArgs {
    buyer: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetOfferArgs {
    id: u64,
}

pub fn dispatch(
    state: &mut Option<EnergyMarketState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC34: already initialised");
            *state = Some(EnergyMarketState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "list_energy" => {
            let s = state.as_mut().expect("DRC34: not initialised");
            let a: ListEnergyArgs =
                serde_json::from_slice(args).expect("DRC34: bad list_energy args");
            let id = s.list_energy(caller, a.kwh_available, a.price_per_kwh, a.location, a.valid_until);
            serde_json::to_vec(&id).unwrap()
        }
        "buy_energy" => {
            let s = state.as_mut().expect("DRC34: not initialised");
            let a: BuyEnergyArgs =
                serde_json::from_slice(args).expect("DRC34: bad buy_energy args");
            let id = s.buy_energy(caller, a.offer_id, a.kwh);
            serde_json::to_vec(&id).unwrap()
        }
        "confirm_delivery" => {
            let s = state.as_mut().expect("DRC34: not initialised");
            let a: ConfirmDeliveryArgs =
                serde_json::from_slice(args).expect("DRC34: bad confirm_delivery args");
            s.confirm_delivery(caller, a.purchase_id);
            serde_json::to_vec("ok").unwrap()
        }
        "available_energy" => {
            let s = state.as_ref().expect("DRC34: not initialised");
            serde_json::to_vec(&s.available_energy()).unwrap()
        }
        "my_purchases" => {
            let s = state.as_ref().expect("DRC34: not initialised");
            let a: MyPurchasesArgs =
                serde_json::from_slice(args).expect("DRC34: bad my_purchases args");
            serde_json::to_vec(&s.my_purchases(&a.buyer)).unwrap()
        }
        "get_offer" => {
            let s = state.as_ref().expect("DRC34: not initialised");
            let a: GetOfferArgs =
                serde_json::from_slice(args).expect("DRC34: bad get_offer args");
            serde_json::to_vec(&s.get_offer(a.id)).unwrap()
        }
        _ => panic!("DRC34: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ADMIN: Address = [1u8; 32];
    const SELLER: Address = [2u8; 32];
    const BUYER: Address = [3u8; 32];

    fn init_state() -> Option<EnergyMarketState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", ADMIN);
        state
    }

    fn list_via_dispatch(state: &mut Option<EnergyMarketState>, caller: Address) -> u64 {
        let args = serde_json::to_vec(&serde_json::json!({
            "kwh_available": 500,
            "price_per_kwh": 10,
            "location": "Toronto-Grid-7",
            "valid_until": 1700000000u64
        }))
        .unwrap();
        let result = dispatch(state, "list_energy", &args, caller);
        serde_json::from_slice(&result).unwrap()
    }

    #[test]
    fn test_init_and_list_energy() {
        let mut state = init_state();
        let offer_id = list_via_dispatch(&mut state, SELLER);
        assert_eq!(offer_id, 1);

        let s = state.as_ref().unwrap();
        let offer = s.get_offer(1).unwrap();
        assert_eq!(offer.seller, SELLER);
        assert_eq!(offer.kwh_available, 500);
        assert_eq!(offer.price_per_kwh, 10);
        assert_eq!(offer.location, "Toronto-Grid-7");
        assert!(offer.active);
    }

    #[test]
    fn test_buy_energy_partial() {
        let mut state = init_state();
        list_via_dispatch(&mut state, SELLER);

        let args = serde_json::to_vec(&serde_json::json!({
            "offer_id": 1,
            "kwh": 200
        }))
        .unwrap();
        let result = dispatch(&mut state, "buy_energy", &args, BUYER);
        let purchase_id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(purchase_id, 1);

        let s = state.as_ref().unwrap();
        let offer = s.get_offer(1).unwrap();
        assert_eq!(offer.kwh_available, 300);
        assert!(offer.active);

        let purchases = s.my_purchases(&BUYER);
        assert_eq!(purchases.len(), 1);
        assert_eq!(purchases[0].kwh, 200);
        assert_eq!(purchases[0].total_price, 2000);
        assert_eq!(purchases[0].status, DeliveryStatus::Pending);
    }

    #[test]
    fn test_buy_energy_exhausts_offer() {
        let mut state = init_state();
        list_via_dispatch(&mut state, SELLER);

        let args = serde_json::to_vec(&serde_json::json!({
            "offer_id": 1,
            "kwh": 500
        }))
        .unwrap();
        dispatch(&mut state, "buy_energy", &args, BUYER);

        let s = state.as_ref().unwrap();
        let offer = s.get_offer(1).unwrap();
        assert_eq!(offer.kwh_available, 0);
        assert!(!offer.active);
    }

    #[test]
    fn test_confirm_delivery() {
        let mut state = init_state();
        list_via_dispatch(&mut state, SELLER);

        let buy_args = serde_json::to_vec(&serde_json::json!({
            "offer_id": 1,
            "kwh": 100
        }))
        .unwrap();
        dispatch(&mut state, "buy_energy", &buy_args, BUYER);

        let confirm_args = serde_json::to_vec(&serde_json::json!({
            "purchase_id": 1
        }))
        .unwrap();
        dispatch(&mut state, "confirm_delivery", &confirm_args, SELLER);

        let s = state.as_ref().unwrap();
        let purchases = s.my_purchases(&BUYER);
        assert_eq!(purchases[0].status, DeliveryStatus::Delivered);
    }

    #[test]
    fn test_available_energy_filters_inactive() {
        let mut state = init_state();
        list_via_dispatch(&mut state, SELLER);

        // Create a second offer
        let args2 = serde_json::to_vec(&serde_json::json!({
            "kwh_available": 100,
            "price_per_kwh": 20,
            "location": "Vancouver-Grid-3",
            "valid_until": 1700000000u64
        }))
        .unwrap();
        dispatch(&mut state, "list_energy", &args2, SELLER);

        // Exhaust offer 1
        let buy_args = serde_json::to_vec(&serde_json::json!({
            "offer_id": 1,
            "kwh": 500
        }))
        .unwrap();
        dispatch(&mut state, "buy_energy", &buy_args, BUYER);

        let result = dispatch(&mut state, "available_energy", b"", ADMIN);
        let available: Vec<EnergyOffer> = serde_json::from_slice(&result).unwrap();
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].id, 2);
    }

    #[test]
    #[should_panic(expected = "DRC34: cannot buy own energy")]
    fn test_cannot_buy_own_energy() {
        let mut state = init_state();
        list_via_dispatch(&mut state, SELLER);

        let args = serde_json::to_vec(&serde_json::json!({
            "offer_id": 1,
            "kwh": 100
        }))
        .unwrap();
        dispatch(&mut state, "buy_energy", &args, SELLER);
    }

    #[test]
    #[should_panic(expected = "DRC34: insufficient energy available")]
    fn test_cannot_buy_more_than_available() {
        let mut state = init_state();
        list_via_dispatch(&mut state, SELLER);

        let args = serde_json::to_vec(&serde_json::json!({
            "offer_id": 1,
            "kwh": 999
        }))
        .unwrap();
        dispatch(&mut state, "buy_energy", &args, BUYER);
    }

    #[test]
    #[should_panic(expected = "DRC34: only seller can confirm delivery")]
    fn test_only_seller_confirms_delivery() {
        let mut state = init_state();
        list_via_dispatch(&mut state, SELLER);

        let buy_args = serde_json::to_vec(&serde_json::json!({
            "offer_id": 1,
            "kwh": 100
        }))
        .unwrap();
        dispatch(&mut state, "buy_energy", &buy_args, BUYER);

        let confirm_args = serde_json::to_vec(&serde_json::json!({
            "purchase_id": 1
        }))
        .unwrap();
        dispatch(&mut state, "confirm_delivery", &confirm_args, BUYER);
    }
}
