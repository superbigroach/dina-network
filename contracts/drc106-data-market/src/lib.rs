use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-106  Data Marketplace
// ---------------------------------------------------------------------------

type Address = [u8; 32];
type ListingId = [u8; 32];
type PurchaseId = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataType {
    VectorEmbeddings,
    SensorData,
    TrainingData,
    KnowledgeBase,
    Custom(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum License {
    SingleUse,
    Unlimited,
    TimeLimited(u64),
    Sublicensable,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataListing {
    pub seller: Address,
    pub data_type: DataType,
    pub description: String,
    pub sample_hash: [u8; 32],
    pub full_hash: [u8; 32],
    pub price: u64,
    pub license: License,
    pub vector_count: Option<u64>,
    pub dimensions: Option<u16>,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Purchase {
    pub listing_id: ListingId,
    pub buyer: Address,
    pub price: u64,
    pub purchased_at: u64,
    pub delivered: bool,
    pub confirmed: bool,
    pub rating: Option<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Rating {
    pub purchase_id: PurchaseId,
    pub score: u8,
    pub review: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataMarketplace {
    pub owner: Address,
    pub listings: BTreeMap<ListingId, DataListing>,
    pub purchases: BTreeMap<PurchaseId, Purchase>,
    pub ratings: BTreeMap<ListingId, Vec<Rating>>,
    pub escrow: BTreeMap<PurchaseId, u64>,
    pub next_listing_nonce: u64,
    pub next_purchase_nonce: u64,
    pub current_time: u64,
}

impl DataMarketplace {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            listings: BTreeMap::new(),
            purchases: BTreeMap::new(),
            ratings: BTreeMap::new(),
            escrow: BTreeMap::new(),
            next_listing_nonce: 0,
            next_purchase_nonce: 0,
            current_time: 0,
        }
    }

    // -- Helpers -------------------------------------------------------------

    fn derive_id(addr: &Address, nonce: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        id.copy_from_slice(addr);
        let nonce_bytes = nonce.to_le_bytes();
        for i in 0..8 {
            id[i] ^= nonce_bytes[i];
        }
        id
    }

    // -- Queries -------------------------------------------------------------

    pub fn search_by_type(&self, data_type: &DataType) -> Vec<ListingId> {
        self.listings
            .iter()
            .filter(|(_, l)| l.active && &l.data_type == data_type)
            .map(|(id, _)| *id)
            .collect()
    }

    // -- Mutations -----------------------------------------------------------

    pub fn list_data(&mut self, caller: Address, mut listing: DataListing) -> ListingId {
        listing.seller = caller;
        listing.active = true;
        let listing_id = Self::derive_id(&caller, self.next_listing_nonce);
        self.next_listing_nonce += 1;
        self.listings.insert(listing_id, listing);
        listing_id
    }

    pub fn purchase(
        &mut self,
        caller: Address,
        listing_id: ListingId,
        timestamp: u64,
    ) -> PurchaseId {
        let listing = self
            .listings
            .get(&listing_id)
            .expect("DRC106: listing does not exist");
        assert!(listing.active, "DRC106: listing is not active");
        assert!(
            caller != listing.seller,
            "DRC106: seller cannot buy own listing"
        );

        let purchase_id = Self::derive_id(&caller, self.next_purchase_nonce);
        self.next_purchase_nonce += 1;

        let purchase = Purchase {
            listing_id,
            buyer: caller,
            price: listing.price,
            purchased_at: timestamp,
            delivered: false,
            confirmed: false,
            rating: None,
        };

        // Lock payment in escrow
        self.escrow.insert(purchase_id, listing.price);
        self.purchases.insert(purchase_id, purchase);
        purchase_id
    }

    pub fn deliver(
        &mut self,
        caller: Address,
        purchase_id: PurchaseId,
        _encrypted_data_hash: [u8; 32],
    ) {
        let purchase = self
            .purchases
            .get_mut(&purchase_id)
            .expect("DRC106: purchase does not exist");
        let listing = self
            .listings
            .get(&purchase.listing_id)
            .expect("DRC106: listing does not exist");
        assert!(caller == listing.seller, "DRC106: only seller can deliver");
        assert!(!purchase.delivered, "DRC106: already delivered");
        purchase.delivered = true;
    }

    pub fn confirm_receipt(&mut self, caller: Address, purchase_id: PurchaseId) {
        let purchase = self
            .purchases
            .get_mut(&purchase_id)
            .expect("DRC106: purchase does not exist");
        assert!(
            caller == purchase.buyer,
            "DRC106: only buyer can confirm receipt"
        );
        assert!(purchase.delivered, "DRC106: not yet delivered");
        assert!(!purchase.confirmed, "DRC106: already confirmed");
        purchase.confirmed = true;

        // Release escrow (in production, transfer to seller)
        self.escrow.remove(&purchase_id);
    }

    pub fn rate(&mut self, caller: Address, purchase_id: PurchaseId, score: u8, review: String) {
        assert!((1..=5).contains(&score), "DRC106: rating must be 1-5");
        let purchase = self
            .purchases
            .get_mut(&purchase_id)
            .expect("DRC106: purchase does not exist");
        assert!(caller == purchase.buyer, "DRC106: only buyer can rate");
        assert!(purchase.confirmed, "DRC106: must confirm receipt first");
        assert!(purchase.rating.is_none(), "DRC106: already rated");
        purchase.rating = Some(score);

        let rating = Rating {
            purchase_id,
            score,
            review,
        };
        self.ratings
            .entry(purchase.listing_id)
            .or_default()
            .push(rating);
    }

    pub fn delist(&mut self, caller: Address, listing_id: ListingId) {
        let listing = self
            .listings
            .get_mut(&listing_id)
            .expect("DRC106: listing does not exist");
        assert!(caller == listing.seller, "DRC106: only seller can delist");
        listing.active = false;
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct ListDataArgs {
    listing: DataListing,
}

#[derive(Serialize, Deserialize, Debug)]
struct PurchaseArgs {
    listing_id: ListingId,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeliverArgs {
    purchase_id: PurchaseId,
    encrypted_data_hash: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct ConfirmReceiptArgs {
    purchase_id: PurchaseId,
}

#[derive(Serialize, Deserialize, Debug)]
struct RateArgs {
    purchase_id: PurchaseId,
    rating: u8,
    review: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SearchByTypeArgs {
    data_type: DataType,
}

#[derive(Serialize, Deserialize, Debug)]
struct DelistArgs {
    listing_id: ListingId,
}

pub fn dispatch(
    state: &mut Option<DataMarketplace>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC106: already initialised");
            *state = Some(DataMarketplace::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "search_by_type" => {
            let s = state.as_ref().expect("DRC106: not initialised");
            let a: SearchByTypeArgs =
                serde_json::from_slice(args).expect("DRC106: bad search_by_type args");
            serde_json::to_vec(&s.search_by_type(&a.data_type)).unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "list_data" => {
            let s = state.as_mut().expect("DRC106: not initialised");
            let a: ListDataArgs = serde_json::from_slice(args).expect("DRC106: bad list_data args");
            let id = s.list_data(caller, a.listing);
            serde_json::to_vec(&id).unwrap()
        }
        "purchase" => {
            let s = state.as_mut().expect("DRC106: not initialised");
            let a: PurchaseArgs = serde_json::from_slice(args).expect("DRC106: bad purchase args");
            let id = s.purchase(caller, a.listing_id, a.timestamp);
            serde_json::to_vec(&id).unwrap()
        }
        "deliver" => {
            let s = state.as_mut().expect("DRC106: not initialised");
            let a: DeliverArgs = serde_json::from_slice(args).expect("DRC106: bad deliver args");
            s.deliver(caller, a.purchase_id, a.encrypted_data_hash);
            serde_json::to_vec("ok").unwrap()
        }
        "confirm_receipt" => {
            let s = state.as_mut().expect("DRC106: not initialised");
            let a: ConfirmReceiptArgs =
                serde_json::from_slice(args).expect("DRC106: bad confirm_receipt args");
            s.confirm_receipt(caller, a.purchase_id);
            serde_json::to_vec("ok").unwrap()
        }
        "rate" => {
            let s = state.as_mut().expect("DRC106: not initialised");
            let a: RateArgs = serde_json::from_slice(args).expect("DRC106: bad rate args");
            s.rate(caller, a.purchase_id, a.rating, a.review);
            serde_json::to_vec("ok").unwrap()
        }
        "delist" => {
            let s = state.as_mut().expect("DRC106: not initialised");
            let a: DelistArgs = serde_json::from_slice(args).expect("DRC106: bad delist args");
            s.delist(caller, a.listing_id);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC106: unknown method '{method}'"),
    }
}
