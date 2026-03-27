use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-6  Non-Fungible Token  (ERC-721 equivalent)
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
pub struct NftState {
    pub collection_name: String,
    pub collection_symbol: String,
    pub minter: Address,
    pub next_token_id: u64,
    /// token_id -> owner
    pub owners: BTreeMap<u64, Address>,
    /// owner -> balance count
    pub balances: BTreeMap<Address, u64>,
    /// token_id -> approved address
    pub approvals: BTreeMap<u64, Address>,
    /// (owner, operator) -> approved for all
    pub operator_approvals: BTreeMap<(Address, Address), bool>,
    /// token_id -> metadata
    pub token_metadata: BTreeMap<u64, TokenMetadata>,
}

impl NftState {
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
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn owner_of(&self, token_id: u64) -> Address {
        *self
            .owners
            .get(&token_id)
            .expect("DRC6: token does not exist")
    }

    pub fn balance_of(&self, owner: &Address) -> u64 {
        self.balances.get(owner).copied().unwrap_or(0)
    }

    pub fn get_approved(&self, token_id: u64) -> Address {
        assert!(self.owners.contains_key(&token_id), "DRC6: token does not exist");
        self.approvals.get(&token_id).copied().unwrap_or(ZERO_ADDRESS)
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

    // -- Internal helpers ----------------------------------------------------

    fn is_approved_or_owner(&self, spender: &Address, token_id: u64) -> bool {
        let owner = self.owner_of(token_id);
        *spender == owner
            || self.get_approved(token_id) == *spender
            || self.is_approved_for_all(&owner, spender)
    }

    // -- Mutations -----------------------------------------------------------

    pub fn mint(
        &mut self,
        caller: Address,
        to: Address,
        metadata: TokenMetadata,
    ) -> u64 {
        assert!(caller == self.minter, "DRC6: only minter can mint");
        assert!(to != ZERO_ADDRESS, "DRC6: cannot mint to zero address");

        let token_id = self.next_token_id;
        self.next_token_id += 1;

        self.owners.insert(token_id, to);
        let bal = self.balance_of(&to);
        self.balances.insert(to, bal + 1);
        self.token_metadata.insert(token_id, metadata);

        token_id
    }

    pub fn transfer_from(
        &mut self,
        caller: Address,
        from: Address,
        to: Address,
        token_id: u64,
    ) {
        assert!(
            self.is_approved_or_owner(&caller, token_id),
            "DRC6: caller is not owner nor approved"
        );
        let owner = self.owner_of(token_id);
        assert!(owner == from, "DRC6: transfer from incorrect owner");
        assert!(to != ZERO_ADDRESS, "DRC6: transfer to zero address");

        // Clear approval
        self.approvals.remove(&token_id);

        // Update balances
        let from_bal = self.balance_of(&from);
        self.balances.insert(from, from_bal - 1);
        let to_bal = self.balance_of(&to);
        self.balances.insert(to, to_bal + 1);

        // Transfer ownership
        self.owners.insert(token_id, to);
    }

    pub fn approve(&mut self, caller: Address, to: Address, token_id: u64) {
        let owner = self.owner_of(token_id);
        assert!(
            caller == owner || self.is_approved_for_all(&owner, &caller),
            "DRC6: caller is not owner nor approved for all"
        );
        assert!(to != owner, "DRC6: approval to current owner");
        self.approvals.insert(token_id, to);
    }

    pub fn set_approval_for_all(
        &mut self,
        caller: Address,
        operator: Address,
        approved: bool,
    ) {
        assert!(caller != operator, "DRC6: approve to caller");
        self.operator_approvals
            .insert((caller, operator), approved);
    }

    pub fn burn(&mut self, caller: Address, token_id: u64) {
        assert!(
            self.is_approved_or_owner(&caller, token_id),
            "DRC6: caller is not owner nor approved"
        );
        let owner = self.owner_of(token_id);

        // Clear approval
        self.approvals.remove(&token_id);

        // Update balance
        let bal = self.balance_of(&owner);
        self.balances.insert(owner, bal - 1);

        // Remove token
        self.owners.remove(&token_id);
        self.token_metadata.remove(&token_id);
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

pub fn dispatch(
    state: &mut Option<NftState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        // -- Init ------------------------------------------------------------
        "init" => {
            assert!(state.is_none(), "DRC6: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC6: bad init args");
            *state = Some(NftState::new(a.name, a.symbol, caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "owner_of" => {
            let s = state.as_ref().expect("DRC6: not initialised");
            let a: OwnerOfArgs =
                serde_json::from_slice(args).expect("DRC6: bad owner_of args");
            serde_json::to_vec(&s.owner_of(a.token_id)).unwrap()
        }
        "balance_of" => {
            let s = state.as_ref().expect("DRC6: not initialised");
            let a: BalanceOfArgs =
                serde_json::from_slice(args).expect("DRC6: bad balance_of args");
            serde_json::to_vec(&s.balance_of(&a.owner)).unwrap()
        }
        "get_approved" => {
            let s = state.as_ref().expect("DRC6: not initialised");
            let a: GetApprovedArgs =
                serde_json::from_slice(args).expect("DRC6: bad get_approved args");
            serde_json::to_vec(&s.get_approved(a.token_id)).unwrap()
        }
        "is_approved_for_all" => {
            let s = state.as_ref().expect("DRC6: not initialised");
            let a: IsApprovedForAllArgs =
                serde_json::from_slice(args).expect("DRC6: bad is_approved_for_all args");
            serde_json::to_vec(&s.is_approved_for_all(&a.owner, &a.operator)).unwrap()
        }
        "token_metadata" => {
            let s = state.as_ref().expect("DRC6: not initialised");
            let a: TokenMetadataArgs =
                serde_json::from_slice(args).expect("DRC6: bad token_metadata args");
            serde_json::to_vec(&s.token_metadata(a.token_id)).unwrap()
        }
        "total_supply" => {
            let s = state.as_ref().expect("DRC6: not initialised");
            serde_json::to_vec(&s.total_supply()).unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "mint" => {
            let s = state.as_mut().expect("DRC6: not initialised");
            let a: MintArgs = serde_json::from_slice(args).expect("DRC6: bad mint args");
            let id = s.mint(caller, a.to, a.metadata);
            serde_json::to_vec(&id).unwrap()
        }
        "transfer_from" => {
            let s = state.as_mut().expect("DRC6: not initialised");
            let a: TransferFromArgs =
                serde_json::from_slice(args).expect("DRC6: bad transfer_from args");
            s.transfer_from(caller, a.from, a.to, a.token_id);
            serde_json::to_vec("ok").unwrap()
        }
        "approve" => {
            let s = state.as_mut().expect("DRC6: not initialised");
            let a: ApproveArgs =
                serde_json::from_slice(args).expect("DRC6: bad approve args");
            s.approve(caller, a.to, a.token_id);
            serde_json::to_vec("ok").unwrap()
        }
        "set_approval_for_all" => {
            let s = state.as_mut().expect("DRC6: not initialised");
            let a: SetApprovalForAllArgs =
                serde_json::from_slice(args).expect("DRC6: bad set_approval_for_all args");
            s.set_approval_for_all(caller, a.operator, a.approved);
            serde_json::to_vec("ok").unwrap()
        }
        "burn" => {
            let s = state.as_mut().expect("DRC6: not initialised");
            let a: BurnArgs = serde_json::from_slice(args).expect("DRC6: bad burn args");
            s.burn(caller, a.token_id);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC6: unknown method '{method}'"),
    }
}
