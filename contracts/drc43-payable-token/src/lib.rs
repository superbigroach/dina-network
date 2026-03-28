use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-43  Payable Token  (ERC-1363 equivalent)
// Token with callback on transfer — recipient contract gets notified.
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PayableTokenState {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u64,
    pub owner: Address,
    pub balances: BTreeMap<Address, u64>,
    pub allowances: BTreeMap<(Address, Address), u64>,
    /// Addresses registered as receivers capable of handling transfer callbacks.
    pub registered_receivers: BTreeMap<Address, bool>,
    /// Log of callback notifications for auditing.
    pub callback_log: Vec<TransferCallback>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransferCallback {
    pub from: Address,
    pub to: Address,
    pub amount: u64,
    pub data: Vec<u8>,
    pub callback_type: String, // "transfer" or "approval"
}

impl PayableTokenState {
    pub fn new(name: String, symbol: String, decimals: u8, owner: Address) -> Self {
        Self {
            name,
            symbol,
            decimals,
            total_supply: 0,
            owner,
            balances: BTreeMap::new(),
            allowances: BTreeMap::new(),
            registered_receivers: BTreeMap::new(),
            callback_log: Vec::new(),
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn balance_of(&self, account: &Address) -> u64 {
        self.balances.get(account).copied().unwrap_or(0)
    }

    pub fn allowance(&self, owner: &Address, spender: &Address) -> u64 {
        self.allowances
            .get(&(*owner, *spender))
            .copied()
            .unwrap_or(0)
    }

    pub fn is_receiver(&self, addr: &Address) -> bool {
        self.registered_receivers
            .get(addr)
            .copied()
            .unwrap_or(false)
    }

    pub fn callback_count(&self) -> usize {
        self.callback_log.len()
    }

    // -- Mutations -----------------------------------------------------------

    pub fn mint(&mut self, caller: Address, to: Address, amount: u64) {
        assert!(caller == self.owner, "DRC43: only owner can mint");
        assert!(amount > 0, "DRC43: mint amount must be positive");
        let bal = self.balance_of(&to);
        self.balances.insert(to, bal + amount);
        self.total_supply += amount;
    }

    pub fn transfer(&mut self, caller: Address, to: Address, amount: u64) {
        assert!(amount > 0, "DRC43: transfer amount must be positive");
        let from_bal = self.balance_of(&caller);
        assert!(from_bal >= amount, "DRC43: insufficient balance");
        self.balances.insert(caller, from_bal - amount);
        let to_bal = self.balance_of(&to);
        self.balances.insert(to, to_bal + amount);
    }

    pub fn approve(&mut self, caller: Address, spender: Address, amount: u64) {
        self.allowances.insert((caller, spender), amount);
    }

    pub fn transfer_from(&mut self, caller: Address, from: Address, to: Address, amount: u64) {
        assert!(amount > 0, "DRC43: transfer amount must be positive");
        let allowed = self.allowance(&from, &caller);
        assert!(allowed >= amount, "DRC43: allowance exceeded");
        let from_bal = self.balance_of(&from);
        assert!(from_bal >= amount, "DRC43: insufficient balance");
        self.allowances.insert((from, caller), allowed - amount);
        self.balances.insert(from, from_bal - amount);
        let to_bal = self.balance_of(&to);
        self.balances.insert(to, to_bal + amount);
    }

    pub fn register_receiver(&mut self, caller: Address) {
        self.registered_receivers.insert(caller, true);
    }

    /// Transfer tokens and notify recipient via callback.
    /// If recipient is a registered receiver, the callback is logged.
    pub fn transfer_and_call(&mut self, caller: Address, to: Address, amount: u64, data: Vec<u8>) {
        self.transfer(caller, to, amount);
        if self.is_receiver(&to) {
            self.callback_log.push(TransferCallback {
                from: caller,
                to,
                amount,
                data,
                callback_type: "transfer".to_string(),
            });
        }
    }

    /// Approve and notify spender via callback.
    pub fn approve_and_call(
        &mut self,
        caller: Address,
        spender: Address,
        amount: u64,
        data: Vec<u8>,
    ) {
        self.approve(caller, spender, amount);
        if self.is_receiver(&spender) {
            self.callback_log.push(TransferCallback {
                from: caller,
                to: spender,
                amount,
                data,
                callback_type: "approval".to_string(),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    name: String,
    symbol: String,
    decimals: u8,
}
#[derive(Serialize, Deserialize, Debug)]
struct TransferArgs {
    to: Address,
    amount: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct ApproveArgs {
    spender: Address,
    amount: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct TransferFromArgs {
    from: Address,
    to: Address,
    amount: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct MintArgs {
    to: Address,
    amount: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct TransferAndCallArgs {
    to: Address,
    amount: u64,
    data: Vec<u8>,
}
#[derive(Serialize, Deserialize, Debug)]
struct ApproveAndCallArgs {
    spender: Address,
    amount: u64,
    data: Vec<u8>,
}
#[derive(Serialize, Deserialize, Debug)]
struct AddrArg {
    account: Address,
}

pub fn dispatch(
    state: &mut Option<PayableTokenState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC43: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC43: bad init args");
            *state = Some(PayableTokenState::new(a.name, a.symbol, a.decimals, caller));
            serde_json::to_vec("ok").unwrap()
        }
        "balance_of" => {
            let s = state.as_ref().expect("DRC43: not initialised");
            let a: AddrArg = serde_json::from_slice(args).expect("DRC43: bad args");
            serde_json::to_vec(&s.balance_of(&a.account)).unwrap()
        }
        "allowance" => {
            let s = state.as_ref().expect("DRC43: not initialised");
            let a: TransferFromArgs = serde_json::from_slice(args).expect("DRC43: bad args");
            serde_json::to_vec(&s.allowance(&a.from, &a.to)).unwrap()
        }
        "is_receiver" => {
            let s = state.as_ref().expect("DRC43: not initialised");
            let a: AddrArg = serde_json::from_slice(args).expect("DRC43: bad args");
            serde_json::to_vec(&s.is_receiver(&a.account)).unwrap()
        }
        "total_supply" => {
            let s = state.as_ref().expect("DRC43: not initialised");
            serde_json::to_vec(&s.total_supply).unwrap()
        }
        "callback_count" => {
            let s = state.as_ref().expect("DRC43: not initialised");
            serde_json::to_vec(&s.callback_count()).unwrap()
        }
        "mint" => {
            let s = state.as_mut().expect("DRC43: not initialised");
            let a: MintArgs = serde_json::from_slice(args).expect("DRC43: bad args");
            s.mint(caller, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer" => {
            let s = state.as_mut().expect("DRC43: not initialised");
            let a: TransferArgs = serde_json::from_slice(args).expect("DRC43: bad args");
            s.transfer(caller, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "approve" => {
            let s = state.as_mut().expect("DRC43: not initialised");
            let a: ApproveArgs = serde_json::from_slice(args).expect("DRC43: bad args");
            s.approve(caller, a.spender, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer_from" => {
            let s = state.as_mut().expect("DRC43: not initialised");
            let a: TransferFromArgs = serde_json::from_slice(args).expect("DRC43: bad args");
            s.transfer_from(caller, a.from, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "register_receiver" => {
            let s = state.as_mut().expect("DRC43: not initialised");
            s.register_receiver(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer_and_call" => {
            let s = state.as_mut().expect("DRC43: not initialised");
            let a: TransferAndCallArgs = serde_json::from_slice(args).expect("DRC43: bad args");
            s.transfer_and_call(caller, a.to, a.amount, a.data);
            serde_json::to_vec("ok").unwrap()
        }
        "approve_and_call" => {
            let s = state.as_mut().expect("DRC43: not initialised");
            let a: ApproveAndCallArgs = serde_json::from_slice(args).expect("DRC43: bad args");
            s.approve_and_call(caller, a.spender, a.amount, a.data);
            serde_json::to_vec("ok").unwrap()
        }
        _ => panic!("DRC43: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(n: u8) -> Address {
        [n; 32]
    }

    fn setup() -> Option<PayableTokenState> {
        let mut state = None;
        let args = serde_json::to_vec(&InitArgs {
            name: "PayToken".into(),
            symbol: "PAY".into(),
            decimals: 6,
        })
        .unwrap();
        dispatch(&mut state, "init", &args, addr(1));
        // Mint 1000 to addr(1)
        let mint = serde_json::to_vec(&MintArgs {
            to: addr(1),
            amount: 1000,
        })
        .unwrap();
        dispatch(&mut state, "mint", &mint, addr(1));
        state
    }

    #[test]
    fn test_transfer_and_call_with_receiver() {
        let mut state = setup();
        // Register addr(2) as receiver
        dispatch(&mut state, "register_receiver", b"", addr(2));
        let args = serde_json::to_vec(&TransferAndCallArgs {
            to: addr(2),
            amount: 100,
            data: vec![1, 2, 3],
        })
        .unwrap();
        dispatch(&mut state, "transfer_and_call", &args, addr(1));
        let s = state.as_ref().unwrap();
        assert_eq!(s.balance_of(&addr(1)), 900);
        assert_eq!(s.balance_of(&addr(2)), 100);
        assert_eq!(s.callback_count(), 1);
        assert_eq!(s.callback_log[0].callback_type, "transfer");
    }

    #[test]
    fn test_transfer_and_call_without_receiver() {
        let mut state = setup();
        // addr(2) is NOT registered — no callback
        let args = serde_json::to_vec(&TransferAndCallArgs {
            to: addr(2),
            amount: 50,
            data: vec![],
        })
        .unwrap();
        dispatch(&mut state, "transfer_and_call", &args, addr(1));
        let s = state.as_ref().unwrap();
        assert_eq!(s.balance_of(&addr(2)), 50);
        assert_eq!(s.callback_count(), 0);
    }

    #[test]
    fn test_approve_and_call() {
        let mut state = setup();
        dispatch(&mut state, "register_receiver", b"", addr(3));
        let args = serde_json::to_vec(&ApproveAndCallArgs {
            spender: addr(3),
            amount: 500,
            data: vec![9],
        })
        .unwrap();
        dispatch(&mut state, "approve_and_call", &args, addr(1));
        let s = state.as_ref().unwrap();
        assert_eq!(s.allowance(&addr(1), &addr(3)), 500);
        assert_eq!(s.callback_count(), 1);
        assert_eq!(s.callback_log[0].callback_type, "approval");
    }

    #[test]
    fn test_is_receiver() {
        let mut state = setup();
        let s = state.as_ref().unwrap();
        assert!(!s.is_receiver(&addr(5)));
        dispatch(&mut state, "register_receiver", b"", addr(5));
        let s = state.as_ref().unwrap();
        assert!(s.is_receiver(&addr(5)));
    }

    #[test]
    fn test_standard_transfer_and_mint() {
        let mut state = setup();
        let args = serde_json::to_vec(&TransferArgs {
            to: addr(2),
            amount: 200,
        })
        .unwrap();
        dispatch(&mut state, "transfer", &args, addr(1));
        let s = state.as_ref().unwrap();
        assert_eq!(s.balance_of(&addr(1)), 800);
        assert_eq!(s.balance_of(&addr(2)), 200);
        assert_eq!(s.total_supply, 1000);
    }

    #[test]
    #[should_panic(expected = "insufficient balance")]
    fn test_transfer_insufficient() {
        let mut state = setup();
        let args = serde_json::to_vec(&TransferArgs {
            to: addr(2),
            amount: 9999,
        })
        .unwrap();
        dispatch(&mut state, "transfer", &args, addr(1));
    }
}
