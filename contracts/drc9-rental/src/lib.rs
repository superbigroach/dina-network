use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-9  Rental / Timed Access  (ERC-4907 equivalent)
// Extends DRC-6 NFT with rental (user/expiry) semantics.
// ---------------------------------------------------------------------------

type Address = [u8; 32];
const ZERO_ADDRESS: Address = [0u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TokenMetadata {
    pub name: String,
    pub description: String,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RentalState {
    // -- DRC-6 NFT fields ----------------------------------------------------
    pub collection_name: String,
    pub collection_symbol: String,
    pub minter: Address,
    pub next_token_id: u64,
    pub owners: BTreeMap<u64, Address>,
    pub balances: BTreeMap<Address, u64>,
    pub approvals: BTreeMap<u64, Address>,
    pub operator_approvals: BTreeMap<(Address, Address), bool>,
    pub token_metadata: BTreeMap<u64, TokenMetadata>,

    // -- DRC-9 Rental fields -------------------------------------------------
    /// token_id -> user (renter) address
    pub token_users: BTreeMap<u64, Address>,
    /// token_id -> rental expiry timestamp
    pub user_expiries: BTreeMap<u64, u64>,
}

impl RentalState {
    pub fn new(name: String, symbol: String, minter: Address) -> Self {
        Self {
            collection_name: name,
            collection_symbol: symbol,
            minter,
            next_token_id: 1,
            owners: BTreeMap::new(),
            balances: BTreeMap::new(),
            approvals: BTreeMap::new(),
            operator_approvals: BTreeMap::new(),
            token_metadata: BTreeMap::new(),
            token_users: BTreeMap::new(),
            user_expiries: BTreeMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // DRC-6 NFT methods (duplicated here since DRC-9 is a standalone crate)
    // -----------------------------------------------------------------------

    pub fn owner_of(&self, token_id: u64) -> Address {
        *self
            .owners
            .get(&token_id)
            .expect("DRC9: token does not exist")
    }

    pub fn balance_of(&self, owner: &Address) -> u64 {
        self.balances.get(owner).copied().unwrap_or(0)
    }

    pub fn get_approved(&self, token_id: u64) -> Address {
        assert!(
            self.owners.contains_key(&token_id),
            "DRC9: token does not exist"
        );
        self.approvals
            .get(&token_id)
            .copied()
            .unwrap_or(ZERO_ADDRESS)
    }

    pub fn is_approved_for_all(&self, owner: &Address, operator: &Address) -> bool {
        self.operator_approvals
            .get(&(*owner, *operator))
            .copied()
            .unwrap_or(false)
    }

    pub fn token_metadata(&self, token_id: u64) -> Option<&TokenMetadata> {
        self.token_metadata.get(&token_id)
    }

    pub fn total_supply(&self) -> u64 {
        self.owners.len() as u64
    }

    fn is_approved_or_owner(&self, spender: &Address, token_id: u64) -> bool {
        let owner = self.owner_of(token_id);
        *spender == owner
            || self.get_approved(token_id) == *spender
            || self.is_approved_for_all(&owner, spender)
    }

    pub fn mint(&mut self, caller: Address, to: Address, metadata: TokenMetadata) -> u64 {
        assert!(caller == self.minter, "DRC9: only minter can mint");
        assert!(to != ZERO_ADDRESS, "DRC9: cannot mint to zero address");

        let token_id = self.next_token_id;
        self.next_token_id += 1;

        self.owners.insert(token_id, to);
        let bal = self.balance_of(&to);
        self.balances.insert(to, bal + 1);
        self.token_metadata.insert(token_id, metadata);

        token_id
    }

    pub fn transfer_from(&mut self, caller: Address, from: Address, to: Address, token_id: u64) {
        assert!(
            self.is_approved_or_owner(&caller, token_id),
            "DRC9: caller is not owner nor approved"
        );
        let owner = self.owner_of(token_id);
        assert!(owner == from, "DRC9: transfer from incorrect owner");
        assert!(to != ZERO_ADDRESS, "DRC9: transfer to zero address");

        // Clear approval and rental on transfer
        self.approvals.remove(&token_id);
        self.token_users.remove(&token_id);
        self.user_expiries.remove(&token_id);

        let from_bal = self.balance_of(&from);
        self.balances.insert(from, from_bal - 1);
        let to_bal = self.balance_of(&to);
        self.balances.insert(to, to_bal + 1);

        self.owners.insert(token_id, to);
    }

    pub fn approve(&mut self, caller: Address, to: Address, token_id: u64) {
        let owner = self.owner_of(token_id);
        assert!(
            caller == owner || self.is_approved_for_all(&owner, &caller),
            "DRC9: caller is not owner nor approved for all"
        );
        assert!(to != owner, "DRC9: approval to current owner");
        self.approvals.insert(token_id, to);
    }

    pub fn set_approval_for_all(&mut self, caller: Address, operator: Address, approved: bool) {
        assert!(caller != operator, "DRC9: approve to caller");
        self.operator_approvals.insert((caller, operator), approved);
    }

    pub fn burn(&mut self, caller: Address, token_id: u64) {
        assert!(
            self.is_approved_or_owner(&caller, token_id),
            "DRC9: caller is not owner nor approved"
        );
        let owner = self.owner_of(token_id);

        self.approvals.remove(&token_id);
        self.token_users.remove(&token_id);
        self.user_expiries.remove(&token_id);

        let bal = self.balance_of(&owner);
        self.balances.insert(owner, bal - 1);

        self.owners.remove(&token_id);
        self.token_metadata.remove(&token_id);
    }

    // -----------------------------------------------------------------------
    // DRC-9 Rental-specific methods
    // -----------------------------------------------------------------------

    /// Set a user (renter) for a token with an expiry timestamp.
    /// Only the token owner or approved operator can call this.
    pub fn set_user(&mut self, caller: Address, token_id: u64, user: Address, expiry: u64) {
        assert!(
            self.is_approved_or_owner(&caller, token_id),
            "DRC9: caller is not owner nor approved"
        );
        self.token_users.insert(token_id, user);
        self.user_expiries.insert(token_id, expiry);
    }

    /// Returns the current user of a token, or None if no user is set or
    /// the rental has expired.
    pub fn user_of(&self, token_id: u64, current_time: u64) -> Option<Address> {
        let user = self.token_users.get(&token_id)?;
        let expiry = self.user_expiries.get(&token_id)?;
        if current_time > *expiry {
            None
        } else {
            Some(*user)
        }
    }

    /// Returns the expiry timestamp for a token's rental, or None if no
    /// rental is set.
    pub fn user_expires(&self, token_id: u64) -> Option<u64> {
        self.user_expiries.get(&token_id).copied()
    }

    /// Check whether the user rental is currently active.
    pub fn is_user_active(&self, token_id: u64, current_time: u64) -> bool {
        self.user_of(token_id, current_time).is_some()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    name: String,
    symbol: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct MintArgs {
    to: Address,
    metadata: TokenMetadata,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferFromArgs {
    from: Address,
    to: Address,
    token_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApproveArgs {
    to: Address,
    token_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetApprovalForAllArgs {
    operator: Address,
    approved: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct OwnerOfArgs {
    token_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BalanceOfArgs {
    owner: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetApprovedArgs {
    token_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct IsApprovedForAllArgs {
    owner: Address,
    operator: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct TokenMetadataArgs {
    token_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BurnArgs {
    token_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetUserArgs {
    token_id: u64,
    user: Address,
    expiry: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct UserOfArgs {
    token_id: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct UserExpiresArgs {
    token_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct IsUserActiveArgs {
    token_id: u64,
    current_time: u64,
}

pub fn dispatch(
    state: &mut Option<RentalState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        // -- Init ------------------------------------------------------------
        "init" => {
            assert!(state.is_none(), "DRC9: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC9: bad init args");
            *state = Some(RentalState::new(a.name, a.symbol, caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- DRC-6 Queries ---------------------------------------------------
        "owner_of" => {
            let s = state.as_ref().expect("DRC9: not initialised");
            let a: OwnerOfArgs = serde_json::from_slice(args).expect("DRC9: bad owner_of args");
            serde_json::to_vec(&s.owner_of(a.token_id)).unwrap()
        }
        "balance_of" => {
            let s = state.as_ref().expect("DRC9: not initialised");
            let a: BalanceOfArgs = serde_json::from_slice(args).expect("DRC9: bad balance_of args");
            serde_json::to_vec(&s.balance_of(&a.owner)).unwrap()
        }
        "get_approved" => {
            let s = state.as_ref().expect("DRC9: not initialised");
            let a: GetApprovedArgs =
                serde_json::from_slice(args).expect("DRC9: bad get_approved args");
            serde_json::to_vec(&s.get_approved(a.token_id)).unwrap()
        }
        "is_approved_for_all" => {
            let s = state.as_ref().expect("DRC9: not initialised");
            let a: IsApprovedForAllArgs =
                serde_json::from_slice(args).expect("DRC9: bad is_approved_for_all args");
            serde_json::to_vec(&s.is_approved_for_all(&a.owner, &a.operator)).unwrap()
        }
        "token_metadata" => {
            let s = state.as_ref().expect("DRC9: not initialised");
            let a: TokenMetadataArgs =
                serde_json::from_slice(args).expect("DRC9: bad token_metadata args");
            serde_json::to_vec(&s.token_metadata(a.token_id)).unwrap()
        }
        "total_supply" => {
            let s = state.as_ref().expect("DRC9: not initialised");
            serde_json::to_vec(&s.total_supply()).unwrap()
        }

        // -- DRC-6 Mutations -------------------------------------------------
        "mint" => {
            let s = state.as_mut().expect("DRC9: not initialised");
            let a: MintArgs = serde_json::from_slice(args).expect("DRC9: bad mint args");
            let id = s.mint(caller, a.to, a.metadata);
            serde_json::to_vec(&id).unwrap()
        }
        "transfer_from" => {
            let s = state.as_mut().expect("DRC9: not initialised");
            let a: TransferFromArgs =
                serde_json::from_slice(args).expect("DRC9: bad transfer_from args");
            s.transfer_from(caller, a.from, a.to, a.token_id);
            serde_json::to_vec("ok").unwrap()
        }
        "approve" => {
            let s = state.as_mut().expect("DRC9: not initialised");
            let a: ApproveArgs = serde_json::from_slice(args).expect("DRC9: bad approve args");
            s.approve(caller, a.to, a.token_id);
            serde_json::to_vec("ok").unwrap()
        }
        "set_approval_for_all" => {
            let s = state.as_mut().expect("DRC9: not initialised");
            let a: SetApprovalForAllArgs =
                serde_json::from_slice(args).expect("DRC9: bad set_approval_for_all args");
            s.set_approval_for_all(caller, a.operator, a.approved);
            serde_json::to_vec("ok").unwrap()
        }
        "burn" => {
            let s = state.as_mut().expect("DRC9: not initialised");
            let a: BurnArgs = serde_json::from_slice(args).expect("DRC9: bad burn args");
            s.burn(caller, a.token_id);
            serde_json::to_vec("ok").unwrap()
        }

        // -- DRC-9 Rental Queries --------------------------------------------
        "user_of" => {
            let s = state.as_ref().expect("DRC9: not initialised");
            let a: UserOfArgs = serde_json::from_slice(args).expect("DRC9: bad user_of args");
            serde_json::to_vec(&s.user_of(a.token_id, a.current_time)).unwrap()
        }
        "user_expires" => {
            let s = state.as_ref().expect("DRC9: not initialised");
            let a: UserExpiresArgs =
                serde_json::from_slice(args).expect("DRC9: bad user_expires args");
            serde_json::to_vec(&s.user_expires(a.token_id)).unwrap()
        }
        "is_user_active" => {
            let s = state.as_ref().expect("DRC9: not initialised");
            let a: IsUserActiveArgs =
                serde_json::from_slice(args).expect("DRC9: bad is_user_active args");
            serde_json::to_vec(&s.is_user_active(a.token_id, a.current_time)).unwrap()
        }

        // -- DRC-9 Rental Mutations ------------------------------------------
        "set_user" => {
            let s = state.as_mut().expect("DRC9: not initialised");
            let a: SetUserArgs = serde_json::from_slice(args).expect("DRC9: bad set_user args");
            s.set_user(caller, a.token_id, a.user, a.expiry);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC9: unknown method '{method}'"),
    }
}
