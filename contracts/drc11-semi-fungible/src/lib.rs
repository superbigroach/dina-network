use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-11  Semi-Fungible Token  (ERC-3525 equivalent)
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SftToken {
    pub id: u64,
    pub owner: Address,
    pub slot: u64,
    pub value: u64,
    pub metadata: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SftState {
    pub tokens: BTreeMap<u64, SftToken>,
    pub owner_tokens: BTreeMap<Address, Vec<u64>>,
    pub next_id: u64,
    pub admin: Address,
}

impl SftState {
    pub fn new(admin: Address) -> Self {
        Self {
            tokens: BTreeMap::new(),
            owner_tokens: BTreeMap::new(),
            next_id: 1,
            admin,
        }
    }

    pub fn mint(
        &mut self,
        _caller: Address,
        to: Address,
        slot: u64,
        value: u64,
        metadata: String,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let token = SftToken {
            id,
            owner: to,
            slot,
            value,
            metadata,
        };
        self.tokens.insert(id, token);
        self.owner_tokens.entry(to).or_default().push(id);
        id
    }

    pub fn value_of(&self, token_id: u64) -> u64 {
        self.tokens
            .get(&token_id)
            .expect("DRC11: token does not exist")
            .value
    }

    pub fn slot_of(&self, token_id: u64) -> u64 {
        self.tokens
            .get(&token_id)
            .expect("DRC11: token does not exist")
            .slot
    }

    pub fn owner_of(&self, token_id: u64) -> Address {
        self.tokens
            .get(&token_id)
            .expect("DRC11: token does not exist")
            .owner
    }

    /// Transfer value from one token to another (same slot required).
    pub fn transfer_value(&mut self, caller: Address, from_token: u64, to_token: u64, amount: u64) {
        assert!(amount > 0, "DRC11: amount must be positive");
        let from = self
            .tokens
            .get(&from_token)
            .expect("DRC11: from_token does not exist");
        assert!(
            from.owner == caller,
            "DRC11: caller is not owner of from_token"
        );
        assert!(
            from.value >= amount,
            "DRC11: insufficient value in from_token"
        );
        let from_slot = from.slot;

        let to = self
            .tokens
            .get(&to_token)
            .expect("DRC11: to_token does not exist");
        assert!(
            to.slot == from_slot,
            "DRC11: tokens must be in the same slot"
        );

        self.tokens.get_mut(&from_token).unwrap().value -= amount;
        self.tokens.get_mut(&to_token).unwrap().value += amount;
    }

    /// Transfer value from a token to an address, creating a new token.
    pub fn transfer_value_to(
        &mut self,
        caller: Address,
        from_token: u64,
        to_address: Address,
        amount: u64,
    ) -> u64 {
        assert!(amount > 0, "DRC11: amount must be positive");
        let from = self
            .tokens
            .get(&from_token)
            .expect("DRC11: from_token does not exist");
        assert!(
            from.owner == caller,
            "DRC11: caller is not owner of from_token"
        );
        assert!(
            from.value >= amount,
            "DRC11: insufficient value in from_token"
        );
        let slot = from.slot;
        let metadata = from.metadata.clone();

        self.tokens.get_mut(&from_token).unwrap().value -= amount;

        // Create a new token for the recipient
        let new_id = self.next_id;
        self.next_id += 1;
        let new_token = SftToken {
            id: new_id,
            owner: to_address,
            slot,
            value: amount,
            metadata,
        };
        self.tokens.insert(new_id, new_token);
        self.owner_tokens
            .entry(to_address)
            .or_default()
            .push(new_id);
        new_id
    }

    pub fn burn(&mut self, caller: Address, token_id: u64) {
        let token = self
            .tokens
            .get(&token_id)
            .expect("DRC11: token does not exist");
        assert!(token.owner == caller, "DRC11: caller is not owner of token");
        let owner = token.owner;
        self.tokens.remove(&token_id);
        if let Some(ids) = self.owner_tokens.get_mut(&owner) {
            ids.retain(|&id| id != token_id);
        }
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct MintArgs {
    to: Address,
    slot: u64,
    value: u64,
    metadata: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TokenIdArgs {
    token_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferValueArgs {
    from_token: u64,
    to_token: u64,
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferValueToArgs {
    from_token: u64,
    to_address: Address,
    amount: u64,
}

pub fn dispatch(
    state: &mut Option<SftState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC11: already initialised");
            *state = Some(SftState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "mint" => {
            let s = state.as_mut().expect("DRC11: not initialised");
            let a: MintArgs = serde_json::from_slice(args).expect("DRC11: bad mint args");
            let id = s.mint(caller, a.to, a.slot, a.value, a.metadata);
            serde_json::to_vec(&id).unwrap()
        }

        "value_of" => {
            let s = state.as_ref().expect("DRC11: not initialised");
            let a: TokenIdArgs = serde_json::from_slice(args).expect("DRC11: bad value_of args");
            serde_json::to_vec(&s.value_of(a.token_id)).unwrap()
        }

        "slot_of" => {
            let s = state.as_ref().expect("DRC11: not initialised");
            let a: TokenIdArgs = serde_json::from_slice(args).expect("DRC11: bad slot_of args");
            serde_json::to_vec(&s.slot_of(a.token_id)).unwrap()
        }

        "owner_of" => {
            let s = state.as_ref().expect("DRC11: not initialised");
            let a: TokenIdArgs = serde_json::from_slice(args).expect("DRC11: bad owner_of args");
            serde_json::to_vec(&s.owner_of(a.token_id)).unwrap()
        }

        "transfer_value" => {
            let s = state.as_mut().expect("DRC11: not initialised");
            let a: TransferValueArgs =
                serde_json::from_slice(args).expect("DRC11: bad transfer_value args");
            s.transfer_value(caller, a.from_token, a.to_token, a.amount);
            serde_json::to_vec("ok").unwrap()
        }

        "transfer_value_to" => {
            let s = state.as_mut().expect("DRC11: not initialised");
            let a: TransferValueToArgs =
                serde_json::from_slice(args).expect("DRC11: bad transfer_value_to args");
            let new_id = s.transfer_value_to(caller, a.from_token, a.to_address, a.amount);
            serde_json::to_vec(&new_id).unwrap()
        }

        "burn" => {
            let s = state.as_mut().expect("DRC11: not initialised");
            let a: TokenIdArgs = serde_json::from_slice(args).expect("DRC11: bad burn args");
            s.burn(caller, a.token_id);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC11: unknown method '{method}'"),
    }
}
