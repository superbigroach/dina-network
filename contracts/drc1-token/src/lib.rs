use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-1  Fungible Token  (ERC-20 equivalent)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TokenState {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u64,
    pub owner: [u8; 32],
    pub balances: BTreeMap<[u8; 32], u64>,
    pub allowances: BTreeMap<([u8; 32], [u8; 32]), u64>,
}

impl TokenState {
    pub fn new(name: String, symbol: String, decimals: u8, owner: [u8; 32]) -> Self {
        Self {
            name,
            symbol,
            decimals,
            total_supply: 0,
            owner,
            balances: BTreeMap::new(),
            allowances: BTreeMap::new(),
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn decimals(&self) -> u8 {
        self.decimals
    }

    pub fn total_supply(&self) -> u64 {
        self.total_supply
    }

    pub fn balance_of(&self, account: &[u8; 32]) -> u64 {
        self.balances.get(account).copied().unwrap_or(0)
    }

    pub fn allowance(&self, owner: &[u8; 32], spender: &[u8; 32]) -> u64 {
        self.allowances
            .get(&(*owner, *spender))
            .copied()
            .unwrap_or(0)
    }

    // -- Mutations -----------------------------------------------------------

    pub fn transfer(&mut self, caller: [u8; 32], to: [u8; 32], amount: u64) {
        assert!(amount > 0, "DRC1: transfer amount must be positive");
        let from_balance = self.balance_of(&caller);
        assert!(
            from_balance >= amount,
            "DRC1: insufficient balance ({from_balance} < {amount})"
        );
        self.balances.insert(caller, from_balance - amount);
        let to_balance = self.balance_of(&to);
        assert!(
            to_balance.checked_add(amount).is_some(),
            "DRC1: balance overflow"
        );
        self.balances.insert(to, to_balance + amount);
    }

    pub fn approve(&mut self, caller: [u8; 32], spender: [u8; 32], amount: u64) {
        self.allowances.insert((caller, spender), amount);
    }

    pub fn transfer_from(&mut self, caller: [u8; 32], from: [u8; 32], to: [u8; 32], amount: u64) {
        assert!(amount > 0, "DRC1: transfer amount must be positive");
        let allowed = self.allowance(&from, &caller);
        assert!(
            allowed >= amount,
            "DRC1: allowance exceeded ({allowed} < {amount})"
        );
        let from_balance = self.balance_of(&from);
        assert!(
            from_balance >= amount,
            "DRC1: insufficient balance ({from_balance} < {amount})"
        );
        self.allowances.insert((from, caller), allowed - amount);
        self.balances.insert(from, from_balance - amount);
        let to_balance = self.balance_of(&to);
        assert!(
            to_balance.checked_add(amount).is_some(),
            "DRC1: balance overflow"
        );
        self.balances.insert(to, to_balance + amount);
    }

    pub fn mint(&mut self, caller: [u8; 32], to: [u8; 32], amount: u64) {
        assert!(caller == self.owner, "DRC1: only owner can mint");
        assert!(amount > 0, "DRC1: mint amount must be positive");
        let balance = self.balance_of(&to);
        assert!(
            balance.checked_add(amount).is_some(),
            "DRC1: balance overflow"
        );
        self.balances.insert(to, balance + amount);
        self.total_supply = self
            .total_supply
            .checked_add(amount)
            .expect("DRC1: total_supply overflow");
    }

    pub fn burn(&mut self, caller: [u8; 32], amount: u64) {
        assert!(amount > 0, "DRC1: burn amount must be positive");
        let balance = self.balance_of(&caller);
        assert!(
            balance >= amount,
            "DRC1: insufficient balance to burn ({balance} < {amount})"
        );
        self.balances.insert(caller, balance - amount);
        self.total_supply = self
            .total_supply
            .checked_sub(amount)
            .expect("DRC1: total_supply underflow");
    }

    pub fn increase_allowance(&mut self, caller: [u8; 32], spender: [u8; 32], added: u64) {
        let current = self.allowance(&caller, &spender);
        let new_allowance = current
            .checked_add(added)
            .expect("DRC1: allowance overflow");
        self.allowances.insert((caller, spender), new_allowance);
    }

    pub fn decrease_allowance(&mut self, caller: [u8; 32], spender: [u8; 32], subtracted: u64) {
        let current = self.allowance(&caller, &spender);
        assert!(
            current >= subtracted,
            "DRC1: decreased allowance below zero"
        );
        self.allowances
            .insert((caller, spender), current - subtracted);
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct TransferArgs {
    to: [u8; 32],
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApproveArgs {
    spender: [u8; 32],
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferFromArgs {
    from: [u8; 32],
    to: [u8; 32],
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct MintArgs {
    to: [u8; 32],
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BurnArgs {
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BalanceOfArgs {
    account: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct AllowanceArgs {
    owner: [u8; 32],
    spender: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct IncreaseAllowanceArgs {
    spender: [u8; 32],
    added: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct DecreaseAllowanceArgs {
    spender: [u8; 32],
    subtracted: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    name: String,
    symbol: String,
    decimals: u8,
}

/// Contract-level dispatch.  `state_bytes` is empty on first call (init).
pub fn dispatch(
    state: &mut Option<TokenState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        // -- Init (creates state) -------------------------------------------
        "init" => {
            assert!(state.is_none(), "DRC1: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC1: bad init args");
            *state = Some(TokenState::new(a.name, a.symbol, a.decimals, caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "name" => {
            let s = state.as_ref().expect("DRC1: not initialised");
            serde_json::to_vec(s.name()).unwrap()
        }
        "symbol" => {
            let s = state.as_ref().expect("DRC1: not initialised");
            serde_json::to_vec(s.symbol()).unwrap()
        }
        "decimals" => {
            let s = state.as_ref().expect("DRC1: not initialised");
            serde_json::to_vec(&s.decimals()).unwrap()
        }
        "total_supply" => {
            let s = state.as_ref().expect("DRC1: not initialised");
            serde_json::to_vec(&s.total_supply()).unwrap()
        }
        "balance_of" => {
            let s = state.as_ref().expect("DRC1: not initialised");
            let a: BalanceOfArgs = serde_json::from_slice(args).expect("DRC1: bad balance_of args");
            serde_json::to_vec(&s.balance_of(&a.account)).unwrap()
        }
        "allowance" => {
            let s = state.as_ref().expect("DRC1: not initialised");
            let a: AllowanceArgs = serde_json::from_slice(args).expect("DRC1: bad allowance args");
            serde_json::to_vec(&s.allowance(&a.owner, &a.spender)).unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "transfer" => {
            let s = state.as_mut().expect("DRC1: not initialised");
            let a: TransferArgs = serde_json::from_slice(args).expect("DRC1: bad transfer args");
            s.transfer(caller, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "approve" => {
            let s = state.as_mut().expect("DRC1: not initialised");
            let a: ApproveArgs = serde_json::from_slice(args).expect("DRC1: bad approve args");
            s.approve(caller, a.spender, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer_from" => {
            let s = state.as_mut().expect("DRC1: not initialised");
            let a: TransferFromArgs =
                serde_json::from_slice(args).expect("DRC1: bad transfer_from args");
            s.transfer_from(caller, a.from, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "mint" => {
            let s = state.as_mut().expect("DRC1: not initialised");
            let a: MintArgs = serde_json::from_slice(args).expect("DRC1: bad mint args");
            s.mint(caller, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "burn" => {
            let s = state.as_mut().expect("DRC1: not initialised");
            let a: BurnArgs = serde_json::from_slice(args).expect("DRC1: bad burn args");
            s.burn(caller, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "increase_allowance" => {
            let s = state.as_mut().expect("DRC1: not initialised");
            let a: IncreaseAllowanceArgs =
                serde_json::from_slice(args).expect("DRC1: bad increase_allowance args");
            s.increase_allowance(caller, a.spender, a.added);
            serde_json::to_vec("ok").unwrap()
        }
        "decrease_allowance" => {
            let s = state.as_mut().expect("DRC1: not initialised");
            let a: DecreaseAllowanceArgs =
                serde_json::from_slice(args).expect("DRC1: bad decrease_allowance args");
            s.decrease_allowance(caller, a.spender, a.subtracted);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC1: unknown method '{method}'"),
    }
}
