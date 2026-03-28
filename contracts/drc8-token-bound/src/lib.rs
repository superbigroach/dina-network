use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-8  Token-Bound Accounts  (ERC-6551 equivalent)
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TbaRegistry {
    pub owner: Address,
    /// (nft_contract, token_id) -> derived account address
    pub accounts: BTreeMap<(Address, u64), Address>,
    /// Reverse index: account address -> (nft_contract, token_id)
    pub account_to_token: BTreeMap<Address, (Address, u64)>,
    /// Track NFT ownership for execute authorization: (nft_contract, token_id) -> nft_owner
    pub nft_owners: BTreeMap<(Address, u64), Address>,
    /// Account asset balances: (account_address, asset_key) -> amount
    pub account_assets: BTreeMap<(Address, String), u64>,
}

impl TbaRegistry {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            accounts: BTreeMap::new(),
            account_to_token: BTreeMap::new(),
            nft_owners: BTreeMap::new(),
            account_assets: BTreeMap::new(),
        }
    }

    // -- Helpers -------------------------------------------------------------

    /// Deterministically derive an account address from the NFT contract and
    /// token ID by hashing them together.
    fn derive_address(nft_contract: &Address, token_id: u64) -> Address {
        let mut hasher = Sha256::new();
        hasher.update(b"DRC8-TBA-v1");
        hasher.update(nft_contract);
        hasher.update(token_id.to_le_bytes());
        let result = hasher.finalize();
        let mut addr = [0u8; 32];
        addr.copy_from_slice(&result);
        addr
    }

    // -- Queries -------------------------------------------------------------

    pub fn account_of(&self, nft_contract: &Address, token_id: u64) -> Option<Address> {
        self.accounts.get(&(*nft_contract, token_id)).copied()
    }

    pub fn is_token_bound(&self, account: &Address) -> bool {
        self.account_to_token.contains_key(account)
    }

    pub fn token_of(&self, account: &Address) -> Option<(Address, u64)> {
        self.account_to_token.get(account).copied()
    }

    // -- Mutations -----------------------------------------------------------

    /// Create a token-bound account for an NFT. The caller is recorded as the
    /// NFT owner (in a real system this would be verified against the NFT
    /// contract state).
    pub fn create_account(
        &mut self,
        caller: Address,
        nft_contract: Address,
        token_id: u64,
    ) -> Address {
        let key = (nft_contract, token_id);
        assert!(
            !self.accounts.contains_key(&key),
            "DRC8: account already exists for this NFT"
        );

        let derived = Self::derive_address(&nft_contract, token_id);

        self.accounts.insert(key, derived);
        self.account_to_token.insert(derived, key);
        self.nft_owners.insert(key, caller);

        derived
    }

    /// Execute an operation from the token-bound account.  Only the NFT owner
    /// can call this.  The `target` and `data` describe the operation; here
    /// we model a simple value transfer from the TBA's internal asset ledger.
    pub fn execute(
        &mut self,
        caller: Address,
        nft_contract: Address,
        token_id: u64,
        target: Address,
        asset_key: String,
        amount: u64,
    ) {
        let key = (nft_contract, token_id);
        let account = self
            .accounts
            .get(&key)
            .expect("DRC8: no account for this NFT");
        let account = *account;

        let nft_owner = self
            .nft_owners
            .get(&key)
            .expect("DRC8: NFT owner not registered");
        assert!(
            caller == *nft_owner,
            "DRC8: only NFT owner can execute from TBA"
        );

        // Deduct from TBA's internal balance
        let from_key = (account, asset_key.clone());
        let from_bal = self.account_assets.get(&from_key).copied().unwrap_or(0);
        assert!(
            from_bal >= amount,
            "DRC8: insufficient TBA balance ({from_bal} < {amount})"
        );
        self.account_assets.insert(from_key, from_bal - amount);

        // Credit the target
        let to_key = (target, asset_key);
        let to_bal = self.account_assets.get(&to_key).copied().unwrap_or(0);
        self.account_assets.insert(to_key, to_bal + amount);
    }

    /// Deposit assets into a token-bound account.  Anyone can deposit.
    pub fn deposit(
        &mut self,
        nft_contract: Address,
        token_id: u64,
        asset_key: String,
        amount: u64,
    ) {
        let key = (nft_contract, token_id);
        let account = self
            .accounts
            .get(&key)
            .expect("DRC8: no account for this NFT");
        let account = *account;

        let bal_key = (account, asset_key);
        let bal = self.account_assets.get(&bal_key).copied().unwrap_or(0);
        self.account_assets.insert(bal_key, bal + amount);
    }

    /// Query the balance of an asset in a token-bound account.
    pub fn account_balance(&self, nft_contract: &Address, token_id: u64, asset_key: &str) -> u64 {
        let key = (*nft_contract, token_id);
        match self.accounts.get(&key) {
            Some(account) => self
                .account_assets
                .get(&(*account, asset_key.to_string()))
                .copied()
                .unwrap_or(0),
            None => 0,
        }
    }

    /// Transfer NFT ownership record (to be called when the NFT is
    /// transferred in the DRC-6 contract).
    pub fn update_nft_owner(
        &mut self,
        caller: Address,
        nft_contract: Address,
        token_id: u64,
        new_owner: Address,
    ) {
        let key = (nft_contract, token_id);
        let current_owner = self
            .nft_owners
            .get(&key)
            .expect("DRC8: NFT owner not registered");
        assert!(
            caller == *current_owner || caller == self.owner,
            "DRC8: only current NFT owner or registry owner can update"
        );
        self.nft_owners.insert(key, new_owner);
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateAccountArgs {
    nft_contract: Address,
    token_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AccountOfArgs {
    nft_contract: Address,
    token_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct IsTokenBoundArgs {
    account: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct TokenOfArgs {
    account: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteArgs {
    nft_contract: Address,
    token_id: u64,
    target: Address,
    asset_key: String,
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs {
    nft_contract: Address,
    token_id: u64,
    asset_key: String,
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AccountBalanceArgs {
    nft_contract: Address,
    token_id: u64,
    asset_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateNftOwnerArgs {
    nft_contract: Address,
    token_id: u64,
    new_owner: Address,
}

pub fn dispatch(
    state: &mut Option<TbaRegistry>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        // -- Init ------------------------------------------------------------
        "init" => {
            assert!(state.is_none(), "DRC8: already initialised");
            *state = Some(TbaRegistry::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "account_of" => {
            let s = state.as_ref().expect("DRC8: not initialised");
            let a: AccountOfArgs = serde_json::from_slice(args).expect("DRC8: bad account_of args");
            serde_json::to_vec(&s.account_of(&a.nft_contract, a.token_id)).unwrap()
        }
        "is_token_bound" => {
            let s = state.as_ref().expect("DRC8: not initialised");
            let a: IsTokenBoundArgs =
                serde_json::from_slice(args).expect("DRC8: bad is_token_bound args");
            serde_json::to_vec(&s.is_token_bound(&a.account)).unwrap()
        }
        "token_of" => {
            let s = state.as_ref().expect("DRC8: not initialised");
            let a: TokenOfArgs = serde_json::from_slice(args).expect("DRC8: bad token_of args");
            serde_json::to_vec(&s.token_of(&a.account)).unwrap()
        }
        "account_balance" => {
            let s = state.as_ref().expect("DRC8: not initialised");
            let a: AccountBalanceArgs =
                serde_json::from_slice(args).expect("DRC8: bad account_balance args");
            serde_json::to_vec(&s.account_balance(&a.nft_contract, a.token_id, &a.asset_key))
                .unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "create_account" => {
            let s = state.as_mut().expect("DRC8: not initialised");
            let a: CreateAccountArgs =
                serde_json::from_slice(args).expect("DRC8: bad create_account args");
            let addr = s.create_account(caller, a.nft_contract, a.token_id);
            serde_json::to_vec(&addr).unwrap()
        }
        "execute" => {
            let s = state.as_mut().expect("DRC8: not initialised");
            let a: ExecuteArgs = serde_json::from_slice(args).expect("DRC8: bad execute args");
            s.execute(
                caller,
                a.nft_contract,
                a.token_id,
                a.target,
                a.asset_key,
                a.amount,
            );
            serde_json::to_vec("ok").unwrap()
        }
        "deposit" => {
            let s = state.as_mut().expect("DRC8: not initialised");
            let a: DepositArgs = serde_json::from_slice(args).expect("DRC8: bad deposit args");
            s.deposit(a.nft_contract, a.token_id, a.asset_key, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "update_nft_owner" => {
            let s = state.as_mut().expect("DRC8: not initialised");
            let a: UpdateNftOwnerArgs =
                serde_json::from_slice(args).expect("DRC8: bad update_nft_owner args");
            s.update_nft_owner(caller, a.nft_contract, a.token_id, a.new_owner);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC8: unknown method '{method}'"),
    }
}
