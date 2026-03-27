use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-7  Multi-Token  (ERC-1155 equivalent)
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MultiTokenState {
    pub owner: Address,
    /// (address, token_id) -> balance
    pub balances: BTreeMap<(Address, u64), u64>,
    /// (owner, operator) -> approved
    pub operator_approvals: BTreeMap<(Address, Address), bool>,
    /// token_id -> URI string
    pub uris: BTreeMap<u64, String>,
    /// token_id -> creator address
    pub creators: BTreeMap<u64, Address>,
    /// Next token ID for auto-incrementing mints
    pub next_token_id: u64,
}

impl MultiTokenState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            balances: BTreeMap::new(),
            operator_approvals: BTreeMap::new(),
            uris: BTreeMap::new(),
            creators: BTreeMap::new(),
            next_token_id: 1,
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn balance_of(&self, account: &Address, token_id: u64) -> u64 {
        self.balances
            .get(&(*account, token_id))
            .copied()
            .unwrap_or(0)
    }

    pub fn balance_of_batch(
        &self,
        accounts: &[Address],
        token_ids: &[u64],
    ) -> Vec<u64> {
        assert!(
            accounts.len() == token_ids.len(),
            "DRC7: accounts and token_ids length mismatch"
        );
        accounts
            .iter()
            .zip(token_ids.iter())
            .map(|(account, &token_id)| self.balance_of(account, token_id))
            .collect()
    }

    pub fn is_approved_for_all(&self, owner: &Address, operator: &Address) -> bool {
        self.operator_approvals
            .get(&(*owner, *operator))
            .copied()
            .unwrap_or(false)
    }

    pub fn uri(&self, token_id: u64) -> Option<&str> {
        self.uris.get(&token_id).map(|s| s.as_str())
    }

    // -- Internal helpers ----------------------------------------------------

    fn is_owner_or_approved(&self, caller: &Address, owner: &Address) -> bool {
        *caller == *owner || self.is_approved_for_all(owner, caller)
    }

    // -- Mutations -----------------------------------------------------------

    pub fn mint(
        &mut self,
        caller: Address,
        to: Address,
        token_id: u64,
        amount: u64,
        uri: Option<String>,
    ) {
        // Only the contract owner or the token creator can mint
        let creator = self.creators.get(&token_id);
        match creator {
            Some(c) => {
                assert!(
                    caller == *c || caller == self.owner,
                    "DRC7: only creator or owner can mint existing token"
                );
            }
            None => {
                // New token: register creator
                self.creators.insert(token_id, caller);
                if token_id >= self.next_token_id {
                    self.next_token_id = token_id + 1;
                }
            }
        }

        if let Some(u) = uri {
            self.uris.insert(token_id, u);
        }

        let bal = self.balance_of(&to, token_id);
        self.balances.insert((to, token_id), bal + amount);
    }

    pub fn mint_batch(
        &mut self,
        caller: Address,
        to: Address,
        token_ids: &[u64],
        amounts: &[u64],
        uris: &[Option<String>],
    ) {
        assert!(
            token_ids.len() == amounts.len(),
            "DRC7: token_ids and amounts length mismatch"
        );
        assert!(
            token_ids.len() == uris.len(),
            "DRC7: token_ids and uris length mismatch"
        );
        for i in 0..token_ids.len() {
            self.mint(caller, to, token_ids[i], amounts[i], uris[i].clone());
        }
    }

    pub fn transfer(
        &mut self,
        caller: Address,
        from: Address,
        to: Address,
        token_id: u64,
        amount: u64,
    ) {
        assert!(
            self.is_owner_or_approved(&caller, &from),
            "DRC7: caller is not owner nor approved"
        );
        let from_bal = self.balance_of(&from, token_id);
        assert!(
            from_bal >= amount,
            "DRC7: insufficient balance ({from_bal} < {amount})"
        );
        self.balances.insert((from, token_id), from_bal - amount);
        let to_bal = self.balance_of(&to, token_id);
        self.balances.insert((to, token_id), to_bal + amount);
    }

    pub fn batch_transfer(
        &mut self,
        caller: Address,
        from: Address,
        to: Address,
        token_ids: &[u64],
        amounts: &[u64],
    ) {
        assert!(
            token_ids.len() == amounts.len(),
            "DRC7: token_ids and amounts length mismatch"
        );
        for i in 0..token_ids.len() {
            self.transfer(caller, from, to, token_ids[i], amounts[i]);
        }
    }

    pub fn set_approval_for_all(
        &mut self,
        caller: Address,
        operator: Address,
        approved: bool,
    ) {
        assert!(caller != operator, "DRC7: setting approval for self");
        self.operator_approvals
            .insert((caller, operator), approved);
    }

    pub fn set_uri(&mut self, caller: Address, token_id: u64, new_uri: String) {
        // Only the creator or owner can update URI
        let creator = self.creators.get(&token_id);
        assert!(
            caller == self.owner || creator.is_some_and(|c| *c == caller),
            "DRC7: only creator or owner can set URI"
        );
        self.uris.insert(token_id, new_uri);
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct MintArgs {
    to: Address,
    token_id: u64,
    amount: u64,
    uri: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MintBatchArgs {
    to: Address,
    token_ids: Vec<u64>,
    amounts: Vec<u64>,
    uris: Vec<Option<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferArgs {
    from: Address,
    to: Address,
    token_id: u64,
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BatchTransferArgs {
    from: Address,
    to: Address,
    token_ids: Vec<u64>,
    amounts: Vec<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct BalanceOfArgs {
    account: Address,
    token_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BalanceOfBatchArgs {
    accounts: Vec<Address>,
    token_ids: Vec<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetApprovalForAllArgs {
    operator: Address,
    approved: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct IsApprovedForAllArgs {
    owner: Address,
    operator: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct UriArgs {
    token_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetUriArgs {
    token_id: u64,
    uri: String,
}

pub fn dispatch(
    state: &mut Option<MultiTokenState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        // -- Init ------------------------------------------------------------
        "init" => {
            assert!(state.is_none(), "DRC7: already initialised");
            *state = Some(MultiTokenState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "balance_of" => {
            let s = state.as_ref().expect("DRC7: not initialised");
            let a: BalanceOfArgs =
                serde_json::from_slice(args).expect("DRC7: bad balance_of args");
            serde_json::to_vec(&s.balance_of(&a.account, a.token_id)).unwrap()
        }
        "balance_of_batch" => {
            let s = state.as_ref().expect("DRC7: not initialised");
            let a: BalanceOfBatchArgs =
                serde_json::from_slice(args).expect("DRC7: bad balance_of_batch args");
            let result = s.balance_of_batch(&a.accounts, &a.token_ids);
            serde_json::to_vec(&result).unwrap()
        }
        "is_approved_for_all" => {
            let s = state.as_ref().expect("DRC7: not initialised");
            let a: IsApprovedForAllArgs =
                serde_json::from_slice(args).expect("DRC7: bad is_approved_for_all args");
            serde_json::to_vec(&s.is_approved_for_all(&a.owner, &a.operator)).unwrap()
        }
        "uri" => {
            let s = state.as_ref().expect("DRC7: not initialised");
            let a: UriArgs = serde_json::from_slice(args).expect("DRC7: bad uri args");
            serde_json::to_vec(&s.uri(a.token_id)).unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "mint" => {
            let s = state.as_mut().expect("DRC7: not initialised");
            let a: MintArgs = serde_json::from_slice(args).expect("DRC7: bad mint args");
            s.mint(caller, a.to, a.token_id, a.amount, a.uri);
            serde_json::to_vec("ok").unwrap()
        }
        "mint_batch" => {
            let s = state.as_mut().expect("DRC7: not initialised");
            let a: MintBatchArgs =
                serde_json::from_slice(args).expect("DRC7: bad mint_batch args");
            s.mint_batch(caller, a.to, &a.token_ids, &a.amounts, &a.uris);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer" => {
            let s = state.as_mut().expect("DRC7: not initialised");
            let a: TransferArgs =
                serde_json::from_slice(args).expect("DRC7: bad transfer args");
            s.transfer(caller, a.from, a.to, a.token_id, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "batch_transfer" => {
            let s = state.as_mut().expect("DRC7: not initialised");
            let a: BatchTransferArgs =
                serde_json::from_slice(args).expect("DRC7: bad batch_transfer args");
            s.batch_transfer(caller, a.from, a.to, &a.token_ids, &a.amounts);
            serde_json::to_vec("ok").unwrap()
        }
        "set_approval_for_all" => {
            let s = state.as_mut().expect("DRC7: not initialised");
            let a: SetApprovalForAllArgs =
                serde_json::from_slice(args).expect("DRC7: bad set_approval_for_all args");
            s.set_approval_for_all(caller, a.operator, a.approved);
            serde_json::to_vec("ok").unwrap()
        }
        "set_uri" => {
            let s = state.as_mut().expect("DRC7: not initialised");
            let a: SetUriArgs =
                serde_json::from_slice(args).expect("DRC7: bad set_uri args");
            s.set_uri(caller, a.token_id, a.uri);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC7: unknown method '{method}'"),
    }
}
