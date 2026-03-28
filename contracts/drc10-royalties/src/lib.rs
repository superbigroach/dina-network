use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-10  NFT Royalties  (ERC-2981 equivalent)
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RoyaltyInfo {
    pub recipient: Address,
    pub basis_points: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RoyaltyRegistry {
    pub royalties: BTreeMap<u64, RoyaltyInfo>,
    pub creators: BTreeMap<u64, Address>,
    pub admin: Address,
}

impl RoyaltyRegistry {
    pub fn new(admin: Address) -> Self {
        Self {
            royalties: BTreeMap::new(),
            creators: BTreeMap::new(),
            admin,
        }
    }

    /// Register a token's creator (must be done before setting royalty).
    pub fn register_creator(&mut self, caller: Address, token_id: u64) {
        assert!(
            !self.creators.contains_key(&token_id),
            "DRC10: creator already registered for token {token_id}"
        );
        self.creators.insert(token_id, caller);
    }

    /// Set royalty for a token. Only the registered creator may call this.
    pub fn set_royalty(
        &mut self,
        caller: Address,
        token_id: u64,
        recipient: Address,
        basis_points: u16,
    ) {
        assert!(
            basis_points <= 10000,
            "DRC10: basis_points must be <= 10000 (got {basis_points})"
        );
        let creator = self
            .creators
            .get(&token_id)
            .expect("DRC10: no creator registered for token");
        assert!(caller == *creator, "DRC10: only creator can set royalty");
        self.royalties.insert(
            token_id,
            RoyaltyInfo {
                recipient,
                basis_points,
            },
        );
    }

    /// Compute (recipient, royalty_amount) for a given sale price.
    pub fn royalty_info(&self, token_id: u64, sale_price: u64) -> (Address, u64) {
        let info = self
            .royalties
            .get(&token_id)
            .expect("DRC10: no royalty set for token");
        let royalty_amount = (sale_price as u128 * info.basis_points as u128 / 10000) as u64;
        (info.recipient, royalty_amount)
    }

    /// Get the raw royalty info for a token, if any.
    pub fn get_royalty(&self, token_id: u64) -> Option<&RoyaltyInfo> {
        self.royalties.get(&token_id)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct RegisterCreatorArgs {
    token_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetRoyaltyArgs {
    token_id: u64,
    recipient: Address,
    basis_points: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct RoyaltyInfoArgs {
    token_id: u64,
    sale_price: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetRoyaltyArgs {
    token_id: u64,
}

pub fn dispatch(
    state: &mut Option<RoyaltyRegistry>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC10: already initialised");
            *state = Some(RoyaltyRegistry::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "register_creator" => {
            let s = state.as_mut().expect("DRC10: not initialised");
            let a: RegisterCreatorArgs =
                serde_json::from_slice(args).expect("DRC10: bad register_creator args");
            s.register_creator(caller, a.token_id);
            serde_json::to_vec("ok").unwrap()
        }

        "set_royalty" => {
            let s = state.as_mut().expect("DRC10: not initialised");
            let a: SetRoyaltyArgs =
                serde_json::from_slice(args).expect("DRC10: bad set_royalty args");
            s.set_royalty(caller, a.token_id, a.recipient, a.basis_points);
            serde_json::to_vec("ok").unwrap()
        }

        "royalty_info" => {
            let s = state.as_ref().expect("DRC10: not initialised");
            let a: RoyaltyInfoArgs =
                serde_json::from_slice(args).expect("DRC10: bad royalty_info args");
            let (recipient, amount) = s.royalty_info(a.token_id, a.sale_price);
            serde_json::to_vec(&(recipient, amount)).unwrap()
        }

        "get_royalty" => {
            let s = state.as_ref().expect("DRC10: not initialised");
            let a: GetRoyaltyArgs =
                serde_json::from_slice(args).expect("DRC10: bad get_royalty args");
            serde_json::to_vec(&s.get_royalty(a.token_id)).unwrap()
        }

        _ => panic!("DRC10: unknown method '{method}'"),
    }
}
