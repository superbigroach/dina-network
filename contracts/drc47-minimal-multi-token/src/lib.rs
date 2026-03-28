use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-47  Minimal Multi-Token  (ERC-6909 equivalent)
// Simpler version of DRC-7/ERC-1155. Minimal interface, gas efficient.
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MinimalMultiTokenState {
    pub admin: Address,
    /// (owner, token_id) -> balance
    pub balances: BTreeMap<(Address, u64), u64>,
    /// (owner, spender, token_id) -> allowance
    pub allowances: BTreeMap<(Address, Address, u64), u64>,
    /// (owner, operator) -> approved for all token ids
    pub operators: BTreeMap<(Address, Address), bool>,
    pub next_token_id: u64,
}

impl MinimalMultiTokenState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            balances: BTreeMap::new(),
            allowances: BTreeMap::new(),
            operators: BTreeMap::new(),
            next_token_id: 1,
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn balance_of(&self, owner: &Address, id: u64) -> u64 {
        self.balances.get(&(*owner, id)).copied().unwrap_or(0)
    }

    pub fn allowance(&self, owner: &Address, spender: &Address, id: u64) -> u64 {
        self.allowances
            .get(&(*owner, *spender, id))
            .copied()
            .unwrap_or(0)
    }

    pub fn is_operator(&self, owner: &Address, operator: &Address) -> bool {
        self.operators
            .get(&(*owner, *operator))
            .copied()
            .unwrap_or(false)
    }

    // -- Mutations -----------------------------------------------------------

    pub fn mint(&mut self, caller: Address, to: Address, id: u64, amount: u64) {
        assert!(caller == self.admin, "DRC47: only admin can mint");
        assert!(amount > 0, "DRC47: mint amount must be positive");
        let bal = self.balance_of(&to, id);
        self.balances.insert((to, id), bal + amount);
    }

    pub fn transfer(&mut self, caller: Address, to: Address, id: u64, amount: u64) {
        assert!(amount > 0, "DRC47: transfer amount must be positive");
        let from_bal = self.balance_of(&caller, id);
        assert!(from_bal >= amount, "DRC47: insufficient balance");
        self.balances.insert((caller, id), from_bal - amount);
        let to_bal = self.balance_of(&to, id);
        self.balances.insert((to, id), to_bal + amount);
    }

    pub fn transfer_from(
        &mut self,
        caller: Address,
        from: Address,
        to: Address,
        id: u64,
        amount: u64,
    ) {
        assert!(amount > 0, "DRC47: transfer amount must be positive");

        // Check operator or allowance
        if !self.is_operator(&from, &caller) {
            let allowed = self.allowance(&from, &caller, id);
            assert!(allowed >= amount, "DRC47: allowance exceeded");
            self.allowances.insert((from, caller, id), allowed - amount);
        }

        let from_bal = self.balance_of(&from, id);
        assert!(from_bal >= amount, "DRC47: insufficient balance");
        self.balances.insert((from, id), from_bal - amount);
        let to_bal = self.balance_of(&to, id);
        self.balances.insert((to, id), to_bal + amount);
    }

    pub fn approve(&mut self, caller: Address, spender: Address, id: u64, amount: u64) {
        self.allowances.insert((caller, spender, id), amount);
    }

    pub fn set_operator(&mut self, caller: Address, operator: Address, approved: bool) {
        self.operators.insert((caller, operator), approved);
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct MintArgs {
    to: Address,
    id: u64,
    amount: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct TransferArgs {
    to: Address,
    id: u64,
    amount: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct TransferFromArgs {
    from: Address,
    to: Address,
    id: u64,
    amount: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct ApproveArgs {
    spender: Address,
    id: u64,
    amount: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct SetOperatorArgs {
    operator: Address,
    approved: bool,
}
#[derive(Serialize, Deserialize, Debug)]
struct BalanceOfArgs {
    owner: Address,
    id: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct AllowanceArgs {
    owner: Address,
    spender: Address,
    id: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct IsOperatorArgs {
    owner: Address,
    operator: Address,
}

pub fn dispatch(
    state: &mut Option<MinimalMultiTokenState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC47: already initialised");
            *state = Some(MinimalMultiTokenState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "balance_of" => {
            let s = state.as_ref().expect("DRC47: not initialised");
            let a: BalanceOfArgs = serde_json::from_slice(args).expect("DRC47: bad args");
            serde_json::to_vec(&s.balance_of(&a.owner, a.id)).unwrap()
        }
        "allowance" => {
            let s = state.as_ref().expect("DRC47: not initialised");
            let a: AllowanceArgs = serde_json::from_slice(args).expect("DRC47: bad args");
            serde_json::to_vec(&s.allowance(&a.owner, &a.spender, a.id)).unwrap()
        }
        "is_operator" => {
            let s = state.as_ref().expect("DRC47: not initialised");
            let a: IsOperatorArgs = serde_json::from_slice(args).expect("DRC47: bad args");
            serde_json::to_vec(&s.is_operator(&a.owner, &a.operator)).unwrap()
        }
        "mint" => {
            let s = state.as_mut().expect("DRC47: not initialised");
            let a: MintArgs = serde_json::from_slice(args).expect("DRC47: bad args");
            s.mint(caller, a.to, a.id, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer" => {
            let s = state.as_mut().expect("DRC47: not initialised");
            let a: TransferArgs = serde_json::from_slice(args).expect("DRC47: bad args");
            s.transfer(caller, a.to, a.id, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer_from" => {
            let s = state.as_mut().expect("DRC47: not initialised");
            let a: TransferFromArgs = serde_json::from_slice(args).expect("DRC47: bad args");
            s.transfer_from(caller, a.from, a.to, a.id, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "approve" => {
            let s = state.as_mut().expect("DRC47: not initialised");
            let a: ApproveArgs = serde_json::from_slice(args).expect("DRC47: bad args");
            s.approve(caller, a.spender, a.id, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "set_operator" => {
            let s = state.as_mut().expect("DRC47: not initialised");
            let a: SetOperatorArgs = serde_json::from_slice(args).expect("DRC47: bad args");
            s.set_operator(caller, a.operator, a.approved);
            serde_json::to_vec("ok").unwrap()
        }
        _ => panic!("DRC47: unknown method '{method}'"),
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

    fn setup() -> Option<MinimalMultiTokenState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", addr(1));
        // Mint token id 1 and 2
        let m1 = serde_json::to_vec(&MintArgs {
            to: addr(1),
            id: 1,
            amount: 1000,
        })
        .unwrap();
        let m2 = serde_json::to_vec(&MintArgs {
            to: addr(1),
            id: 2,
            amount: 500,
        })
        .unwrap();
        dispatch(&mut state, "mint", &m1, addr(1));
        dispatch(&mut state, "mint", &m2, addr(1));
        state
    }

    #[test]
    fn test_transfer_single_id() {
        let mut state = setup();
        let args = serde_json::to_vec(&TransferArgs {
            to: addr(2),
            id: 1,
            amount: 300,
        })
        .unwrap();
        dispatch(&mut state, "transfer", &args, addr(1));
        let s = state.as_ref().unwrap();
        assert_eq!(s.balance_of(&addr(1), 1), 700);
        assert_eq!(s.balance_of(&addr(2), 1), 300);
        // Token 2 unchanged
        assert_eq!(s.balance_of(&addr(1), 2), 500);
    }

    #[test]
    fn test_approve_and_transfer_from() {
        let mut state = setup();
        let approve = serde_json::to_vec(&ApproveArgs {
            spender: addr(3),
            id: 1,
            amount: 200,
        })
        .unwrap();
        dispatch(&mut state, "approve", &approve, addr(1));

        let xfer = serde_json::to_vec(&TransferFromArgs {
            from: addr(1),
            to: addr(4),
            id: 1,
            amount: 150,
        })
        .unwrap();
        dispatch(&mut state, "transfer_from", &xfer, addr(3));
        let s = state.as_ref().unwrap();
        assert_eq!(s.balance_of(&addr(4), 1), 150);
        assert_eq!(s.allowance(&addr(1), &addr(3), 1), 50);
    }

    #[test]
    fn test_operator_bypasses_allowance() {
        let mut state = setup();
        let op = serde_json::to_vec(&SetOperatorArgs {
            operator: addr(5),
            approved: true,
        })
        .unwrap();
        dispatch(&mut state, "set_operator", &op, addr(1));

        // addr(5) can transfer without per-id allowance
        let xfer = serde_json::to_vec(&TransferFromArgs {
            from: addr(1),
            to: addr(6),
            id: 2,
            amount: 100,
        })
        .unwrap();
        dispatch(&mut state, "transfer_from", &xfer, addr(5));
        let s = state.as_ref().unwrap();
        assert_eq!(s.balance_of(&addr(6), 2), 100);
    }

    #[test]
    #[should_panic(expected = "allowance exceeded")]
    fn test_transfer_from_no_allowance() {
        let mut state = setup();
        let xfer = serde_json::to_vec(&TransferFromArgs {
            from: addr(1),
            to: addr(2),
            id: 1,
            amount: 100,
        })
        .unwrap();
        dispatch(&mut state, "transfer_from", &xfer, addr(99));
    }

    #[test]
    #[should_panic(expected = "insufficient balance")]
    fn test_transfer_insufficient() {
        let mut state = setup();
        let xfer = serde_json::to_vec(&TransferArgs {
            to: addr(2),
            id: 1,
            amount: 9999,
        })
        .unwrap();
        dispatch(&mut state, "transfer", &xfer, addr(1));
    }
}
