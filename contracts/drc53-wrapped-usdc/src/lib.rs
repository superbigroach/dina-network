use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-53  Wrapped USDC  (like WETH)
// Wrap native USDC into a DRC-1 compatible token for contract interactions.
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WrappedUsdcState {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u64,
    pub admin: Address,
    pub balances: BTreeMap<Address, u64>,
    pub allowances: BTreeMap<(Address, Address), u64>,
    /// Tracks native USDC deposits for accounting.
    pub native_deposits: BTreeMap<Address, u64>,
}

impl WrappedUsdcState {
    pub fn new(admin: Address) -> Self {
        Self {
            name: "Wrapped USDC".to_string(),
            symbol: "wUSDC".to_string(),
            decimals: 6,
            total_supply: 0,
            admin,
            balances: BTreeMap::new(),
            allowances: BTreeMap::new(),
            native_deposits: BTreeMap::new(),
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

    // -- Mutations -----------------------------------------------------------

    /// Wrap native USDC: deposit `amount` of native USDC and receive wUSDC.
    pub fn wrap(&mut self, caller: Address, amount: u64) {
        assert!(amount > 0, "DRC53: wrap amount must be positive");
        // In production, the runtime verifies the native USDC transfer.
        let bal = self.balance_of(&caller);
        assert!(bal.checked_add(amount).is_some(), "DRC53: balance overflow");
        self.balances.insert(caller, bal + amount);
        self.total_supply = self
            .total_supply
            .checked_add(amount)
            .expect("DRC53: total_supply overflow");
        let dep = self.native_deposits.get(&caller).copied().unwrap_or(0);
        assert!(
            dep.checked_add(amount).is_some(),
            "DRC53: native_deposits overflow"
        );
        self.native_deposits.insert(caller, dep + amount);
    }

    /// Unwrap wUSDC: burn wUSDC and receive native USDC back.
    pub fn unwrap(&mut self, caller: Address, amount: u64) {
        assert!(amount > 0, "DRC53: unwrap amount must be positive");
        let bal = self.balance_of(&caller);
        assert!(bal >= amount, "DRC53: insufficient wUSDC balance");
        self.balances.insert(caller, bal - amount);
        self.total_supply = self
            .total_supply
            .checked_sub(amount)
            .expect("DRC53: total_supply underflow");
        // In production, the runtime sends native USDC back.
    }

    pub fn transfer(&mut self, caller: Address, to: Address, amount: u64) {
        assert!(amount > 0, "DRC53: transfer amount must be positive");
        let from_bal = self.balance_of(&caller);
        assert!(from_bal >= amount, "DRC53: insufficient balance");
        self.balances.insert(caller, from_bal - amount);
        let to_bal = self.balance_of(&to);
        assert!(
            to_bal.checked_add(amount).is_some(),
            "DRC53: balance overflow"
        );
        self.balances.insert(to, to_bal + amount);
    }

    pub fn approve(&mut self, caller: Address, spender: Address, amount: u64) {
        self.allowances.insert((caller, spender), amount);
    }

    pub fn transfer_from(&mut self, caller: Address, from: Address, to: Address, amount: u64) {
        assert!(amount > 0, "DRC53: transfer amount must be positive");
        let allowed = self.allowance(&from, &caller);
        assert!(allowed >= amount, "DRC53: allowance exceeded");
        let from_bal = self.balance_of(&from);
        assert!(from_bal >= amount, "DRC53: insufficient balance");
        self.allowances.insert((from, caller), allowed - amount);
        self.balances.insert(from, from_bal - amount);
        let to_bal = self.balance_of(&to);
        assert!(
            to_bal.checked_add(amount).is_some(),
            "DRC53: balance overflow"
        );
        self.balances.insert(to, to_bal + amount);
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct AmountArg {
    amount: u64,
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
struct AddrArg {
    account: Address,
}
#[derive(Serialize, Deserialize, Debug)]
struct AllowanceArgs {
    owner: Address,
    spender: Address,
}

pub fn dispatch(
    state: &mut Option<WrappedUsdcState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC53: already initialised");
            *state = Some(WrappedUsdcState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "name" => {
            let s = state.as_ref().expect("DRC53: not initialised");
            serde_json::to_vec(&s.name).unwrap()
        }
        "symbol" => {
            let s = state.as_ref().expect("DRC53: not initialised");
            serde_json::to_vec(&s.symbol).unwrap()
        }
        "decimals" => {
            let s = state.as_ref().expect("DRC53: not initialised");
            serde_json::to_vec(&s.decimals).unwrap()
        }
        "total_supply" => {
            let s = state.as_ref().expect("DRC53: not initialised");
            serde_json::to_vec(&s.total_supply).unwrap()
        }
        "balance_of" => {
            let s = state.as_ref().expect("DRC53: not initialised");
            let a: AddrArg = serde_json::from_slice(args).expect("DRC53: bad args");
            serde_json::to_vec(&s.balance_of(&a.account)).unwrap()
        }
        "allowance" => {
            let s = state.as_ref().expect("DRC53: not initialised");
            let a: AllowanceArgs = serde_json::from_slice(args).expect("DRC53: bad args");
            serde_json::to_vec(&s.allowance(&a.owner, &a.spender)).unwrap()
        }
        "wrap" => {
            let s = state.as_mut().expect("DRC53: not initialised");
            let a: AmountArg = serde_json::from_slice(args).expect("DRC53: bad args");
            s.wrap(caller, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "unwrap" => {
            let s = state.as_mut().expect("DRC53: not initialised");
            let a: AmountArg = serde_json::from_slice(args).expect("DRC53: bad args");
            s.unwrap(caller, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer" => {
            let s = state.as_mut().expect("DRC53: not initialised");
            let a: TransferArgs = serde_json::from_slice(args).expect("DRC53: bad args");
            s.transfer(caller, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "approve" => {
            let s = state.as_mut().expect("DRC53: not initialised");
            let a: ApproveArgs = serde_json::from_slice(args).expect("DRC53: bad args");
            s.approve(caller, a.spender, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer_from" => {
            let s = state.as_mut().expect("DRC53: not initialised");
            let a: TransferFromArgs = serde_json::from_slice(args).expect("DRC53: bad args");
            s.transfer_from(caller, a.from, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        _ => panic!("DRC53: unknown method '{method}'"),
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

    fn setup() -> Option<WrappedUsdcState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", addr(1));
        state
    }

    #[test]
    fn test_wrap_and_balance() {
        let mut state = setup();
        let args = serde_json::to_vec(&AmountArg { amount: 1000 }).unwrap();
        dispatch(&mut state, "wrap", &args, addr(2));
        let s = state.as_ref().unwrap();
        assert_eq!(s.balance_of(&addr(2)), 1000);
        assert_eq!(s.total_supply, 1000);
    }

    #[test]
    fn test_unwrap() {
        let mut state = setup();
        let wrap = serde_json::to_vec(&AmountArg { amount: 500 }).unwrap();
        dispatch(&mut state, "wrap", &wrap, addr(2));
        let unwrap = serde_json::to_vec(&AmountArg { amount: 200 }).unwrap();
        dispatch(&mut state, "unwrap", &unwrap, addr(2));
        let s = state.as_ref().unwrap();
        assert_eq!(s.balance_of(&addr(2)), 300);
        assert_eq!(s.total_supply, 300);
    }

    #[test]
    fn test_transfer_wusdc() {
        let mut state = setup();
        let wrap = serde_json::to_vec(&AmountArg { amount: 1000 }).unwrap();
        dispatch(&mut state, "wrap", &wrap, addr(2));
        let xfer = serde_json::to_vec(&TransferArgs {
            to: addr(3),
            amount: 400,
        })
        .unwrap();
        dispatch(&mut state, "transfer", &xfer, addr(2));
        let s = state.as_ref().unwrap();
        assert_eq!(s.balance_of(&addr(2)), 600);
        assert_eq!(s.balance_of(&addr(3)), 400);
        assert_eq!(s.total_supply, 1000); // unchanged
    }

    #[test]
    fn test_approve_and_transfer_from() {
        let mut state = setup();
        let wrap = serde_json::to_vec(&AmountArg { amount: 1000 }).unwrap();
        dispatch(&mut state, "wrap", &wrap, addr(2));
        let approve = serde_json::to_vec(&ApproveArgs {
            spender: addr(3),
            amount: 500,
        })
        .unwrap();
        dispatch(&mut state, "approve", &approve, addr(2));
        let xfer = serde_json::to_vec(&TransferFromArgs {
            from: addr(2),
            to: addr(4),
            amount: 300,
        })
        .unwrap();
        dispatch(&mut state, "transfer_from", &xfer, addr(3));
        let s = state.as_ref().unwrap();
        assert_eq!(s.balance_of(&addr(4)), 300);
        assert_eq!(s.allowance(&addr(2), &addr(3)), 200);
    }

    #[test]
    #[should_panic(expected = "insufficient wUSDC balance")]
    fn test_unwrap_insufficient() {
        let mut state = setup();
        let wrap = serde_json::to_vec(&AmountArg { amount: 100 }).unwrap();
        dispatch(&mut state, "wrap", &wrap, addr(2));
        let unwrap = serde_json::to_vec(&AmountArg { amount: 500 }).unwrap();
        dispatch(&mut state, "unwrap", &unwrap, addr(2));
    }

    #[test]
    fn test_name_and_symbol() {
        let state = setup();
        let s = state.as_ref().unwrap();
        assert_eq!(s.name, "Wrapped USDC");
        assert_eq!(s.symbol, "wUSDC");
        assert_eq!(s.decimals, 6);
    }
}
