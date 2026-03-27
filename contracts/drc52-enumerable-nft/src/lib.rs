use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-52  Enumerable NFT  (ERC-721 Enumerable equivalent)
// NFT with enumeration — list all tokens, tokens by owner, token at index.
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EnumerableNftState {
    pub name: String,
    pub symbol: String,
    pub minter: Address,
    /// token_id -> owner
    pub owners: BTreeMap<u64, Address>,
    /// owner -> balance count
    pub balances: BTreeMap<Address, u64>,
    /// All token ids in order of minting.
    pub all_tokens: Vec<u64>,
    /// owner -> list of token ids
    pub owner_tokens: BTreeMap<Address, Vec<u64>>,
    /// token_id -> index in all_tokens
    pub token_index: BTreeMap<u64, usize>,
    /// token_id -> index in owner's token list
    pub owner_token_index: BTreeMap<u64, usize>,
    pub next_id: u64,
}

impl EnumerableNftState {
    pub fn new(name: String, symbol: String, minter: Address) -> Self {
        Self {
            name,
            symbol,
            minter,
            owners: BTreeMap::new(),
            balances: BTreeMap::new(),
            all_tokens: Vec::new(),
            owner_tokens: BTreeMap::new(),
            token_index: BTreeMap::new(),
            owner_token_index: BTreeMap::new(),
            next_id: 1,
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn total_supply(&self) -> usize {
        self.all_tokens.len()
    }

    pub fn token_by_index(&self, index: usize) -> u64 {
        assert!(index < self.all_tokens.len(), "DRC52: index out of bounds");
        self.all_tokens[index]
    }

    pub fn token_of_owner_by_index(&self, owner: &Address, index: usize) -> u64 {
        let tokens = self.owner_tokens.get(owner).expect("DRC52: owner has no tokens");
        assert!(index < tokens.len(), "DRC52: owner index out of bounds");
        tokens[index]
    }

    pub fn tokens_of_owner(&self, owner: &Address) -> Vec<u64> {
        self.owner_tokens.get(owner).cloned().unwrap_or_default()
    }

    pub fn owner_of(&self, token_id: u64) -> &Address {
        self.owners.get(&token_id).expect("DRC52: token does not exist")
    }

    pub fn balance_of(&self, owner: &Address) -> u64 {
        self.balances.get(owner).copied().unwrap_or(0)
    }

    // -- Internal helpers ----------------------------------------------------

    fn add_token_to_owner(&mut self, owner: Address, token_id: u64) {
        let tokens = self.owner_tokens.entry(owner).or_default();
        let idx = tokens.len();
        tokens.push(token_id);
        self.owner_token_index.insert(token_id, idx);
        *self.balances.entry(owner).or_insert(0) += 1;
    }

    fn remove_token_from_owner(&mut self, owner: Address, token_id: u64) {
        let tokens = self.owner_tokens.get_mut(&owner).unwrap();
        let idx = *self.owner_token_index.get(&token_id).unwrap();
        let last_token = *tokens.last().unwrap();

        // Swap with last and pop
        tokens[idx] = last_token;
        self.owner_token_index.insert(last_token, idx);
        tokens.pop();
        self.owner_token_index.remove(&token_id);

        *self.balances.get_mut(&owner).unwrap() -= 1;
        if tokens.is_empty() {
            self.owner_tokens.remove(&owner);
        }
    }

    fn add_token_to_all(&mut self, token_id: u64) {
        let idx = self.all_tokens.len();
        self.all_tokens.push(token_id);
        self.token_index.insert(token_id, idx);
    }

    fn remove_token_from_all(&mut self, token_id: u64) {
        let idx = *self.token_index.get(&token_id).unwrap();
        let last_token = *self.all_tokens.last().unwrap();

        self.all_tokens[idx] = last_token;
        self.token_index.insert(last_token, idx);
        self.all_tokens.pop();
        self.token_index.remove(&token_id);
    }

    // -- Mutations -----------------------------------------------------------

    pub fn mint(&mut self, caller: Address, to: Address) -> u64 {
        assert!(caller == self.minter, "DRC52: only minter can mint");
        let id = self.next_id;
        self.next_id += 1;

        self.owners.insert(id, to);
        self.add_token_to_owner(to, id);
        self.add_token_to_all(id);
        id
    }

    pub fn transfer_from(&mut self, caller: Address, from: Address, to: Address, token_id: u64) {
        let owner = *self.owner_of(token_id);
        assert!(owner == from, "DRC52: from is not the owner");
        assert!(
            caller == from || caller == self.minter,
            "DRC52: caller not authorized"
        );

        self.remove_token_from_owner(from, token_id);
        self.add_token_to_owner(to, token_id);
        self.owners.insert(token_id, to);
    }

    pub fn burn(&mut self, caller: Address, token_id: u64) {
        let owner = *self.owner_of(token_id);
        assert!(
            caller == owner || caller == self.minter,
            "DRC52: caller not authorized to burn"
        );
        self.remove_token_from_owner(owner, token_id);
        self.remove_token_from_all(token_id);
        self.owners.remove(&token_id);
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs { name: String, symbol: String }
#[derive(Serialize, Deserialize, Debug)]
struct MintArgs { to: Address }
#[derive(Serialize, Deserialize, Debug)]
struct TransferFromArgs { from: Address, to: Address, token_id: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct BurnArgs { token_id: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct IndexArg { index: usize }
#[derive(Serialize, Deserialize, Debug)]
struct OwnerIndexArg { owner: Address, index: usize }
#[derive(Serialize, Deserialize, Debug)]
struct AddrArg { account: Address }
#[derive(Serialize, Deserialize, Debug)]
struct TokenIdArg { token_id: u64 }

pub fn dispatch(
    state: &mut Option<EnumerableNftState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC52: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC52: bad init args");
            *state = Some(EnumerableNftState::new(a.name, a.symbol, caller));
            serde_json::to_vec("ok").unwrap()
        }
        "total_supply" => {
            let s = state.as_ref().expect("DRC52: not initialised");
            serde_json::to_vec(&s.total_supply()).unwrap()
        }
        "token_by_index" => {
            let s = state.as_ref().expect("DRC52: not initialised");
            let a: IndexArg = serde_json::from_slice(args).expect("DRC52: bad args");
            serde_json::to_vec(&s.token_by_index(a.index)).unwrap()
        }
        "token_of_owner_by_index" => {
            let s = state.as_ref().expect("DRC52: not initialised");
            let a: OwnerIndexArg = serde_json::from_slice(args).expect("DRC52: bad args");
            serde_json::to_vec(&s.token_of_owner_by_index(&a.owner, a.index)).unwrap()
        }
        "tokens_of_owner" => {
            let s = state.as_ref().expect("DRC52: not initialised");
            let a: AddrArg = serde_json::from_slice(args).expect("DRC52: bad args");
            serde_json::to_vec(&s.tokens_of_owner(&a.account)).unwrap()
        }
        "owner_of" => {
            let s = state.as_ref().expect("DRC52: not initialised");
            let a: TokenIdArg = serde_json::from_slice(args).expect("DRC52: bad args");
            serde_json::to_vec(s.owner_of(a.token_id)).unwrap()
        }
        "balance_of" => {
            let s = state.as_ref().expect("DRC52: not initialised");
            let a: AddrArg = serde_json::from_slice(args).expect("DRC52: bad args");
            serde_json::to_vec(&s.balance_of(&a.account)).unwrap()
        }
        "mint" => {
            let s = state.as_mut().expect("DRC52: not initialised");
            let a: MintArgs = serde_json::from_slice(args).expect("DRC52: bad args");
            let id = s.mint(caller, a.to);
            serde_json::to_vec(&id).unwrap()
        }
        "transfer_from" => {
            let s = state.as_mut().expect("DRC52: not initialised");
            let a: TransferFromArgs = serde_json::from_slice(args).expect("DRC52: bad args");
            s.transfer_from(caller, a.from, a.to, a.token_id);
            serde_json::to_vec("ok").unwrap()
        }
        "burn" => {
            let s = state.as_mut().expect("DRC52: not initialised");
            let a: BurnArgs = serde_json::from_slice(args).expect("DRC52: bad args");
            s.burn(caller, a.token_id);
            serde_json::to_vec("ok").unwrap()
        }
        _ => panic!("DRC52: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(n: u8) -> Address { [n; 32] }

    fn setup() -> Option<EnumerableNftState> {
        let mut state = None;
        let args = serde_json::to_vec(&InitArgs {
            name: "EnumNFT".into(), symbol: "ENFT".into(),
        }).unwrap();
        dispatch(&mut state, "init", &args, addr(1));
        state
    }

    #[test]
    fn test_mint_and_enumerate() {
        let mut state = setup();
        let m1 = serde_json::to_vec(&MintArgs { to: addr(2) }).unwrap();
        let m2 = serde_json::to_vec(&MintArgs { to: addr(2) }).unwrap();
        let m3 = serde_json::to_vec(&MintArgs { to: addr(3) }).unwrap();
        dispatch(&mut state, "mint", &m1, addr(1));
        dispatch(&mut state, "mint", &m2, addr(1));
        dispatch(&mut state, "mint", &m3, addr(1));

        let s = state.as_ref().unwrap();
        assert_eq!(s.total_supply(), 3);
        assert_eq!(s.token_by_index(0), 1);
        assert_eq!(s.token_by_index(2), 3);
        assert_eq!(s.balance_of(&addr(2)), 2);
        assert_eq!(s.tokens_of_owner(&addr(2)), vec![1, 2]);
    }

    #[test]
    fn test_transfer_from() {
        let mut state = setup();
        let mint = serde_json::to_vec(&MintArgs { to: addr(2) }).unwrap();
        dispatch(&mut state, "mint", &mint, addr(1));

        let xfer = serde_json::to_vec(&TransferFromArgs {
            from: addr(2), to: addr(3), token_id: 1,
        }).unwrap();
        dispatch(&mut state, "transfer_from", &xfer, addr(2));
        let s = state.as_ref().unwrap();
        assert_eq!(*s.owner_of(1), addr(3));
        assert_eq!(s.balance_of(&addr(2)), 0);
        assert_eq!(s.balance_of(&addr(3)), 1);
    }

    #[test]
    fn test_burn_and_reindex() {
        let mut state = setup();
        for _ in 0..3 {
            let mint = serde_json::to_vec(&MintArgs { to: addr(2) }).unwrap();
            dispatch(&mut state, "mint", &mint, addr(1));
        }
        // Burn token 2 (middle)
        let burn = serde_json::to_vec(&BurnArgs { token_id: 2 }).unwrap();
        dispatch(&mut state, "burn", &burn, addr(2));

        let s = state.as_ref().unwrap();
        assert_eq!(s.total_supply(), 2);
        assert_eq!(s.balance_of(&addr(2)), 2);
        // Token 3 should have been swapped into position of token 2
        assert!(s.all_tokens.contains(&1));
        assert!(s.all_tokens.contains(&3));
        assert!(!s.all_tokens.contains(&2));
    }

    #[test]
    fn test_token_of_owner_by_index() {
        let mut state = setup();
        let m1 = serde_json::to_vec(&MintArgs { to: addr(5) }).unwrap();
        let m2 = serde_json::to_vec(&MintArgs { to: addr(5) }).unwrap();
        dispatch(&mut state, "mint", &m1, addr(1));
        dispatch(&mut state, "mint", &m2, addr(1));
        let s = state.as_ref().unwrap();
        assert_eq!(s.token_of_owner_by_index(&addr(5), 0), 1);
        assert_eq!(s.token_of_owner_by_index(&addr(5), 1), 2);
    }

    #[test]
    #[should_panic(expected = "only minter can mint")]
    fn test_mint_non_minter() {
        let mut state = setup();
        let mint = serde_json::to_vec(&MintArgs { to: addr(2) }).unwrap();
        dispatch(&mut state, "mint", &mint, addr(99));
    }
}
