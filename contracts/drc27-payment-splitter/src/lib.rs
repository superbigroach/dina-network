use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-27  Payment Splitter  (split USDC between payees by shares)
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SplitterState {
    pub owner: Address,
    pub payees: Vec<(Address, u64)>,
    pub total_shares: u64,
    pub total_released: u64,
    pub total_received: u64,
    pub released: BTreeMap<Address, u64>,
}

impl SplitterState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            payees: Vec::new(),
            total_shares: 0,
            total_released: 0,
            total_received: 0,
            released: BTreeMap::new(),
        }
    }

    // -- Mutations -----------------------------------------------------------

    pub fn add_payee(&mut self, caller: Address, payee: Address, shares: u64) {
        assert!(caller == self.owner, "DRC27: only owner can add payees");
        assert!(shares > 0, "DRC27: shares must be positive");
        assert!(
            !self.payees.iter().any(|(addr, _)| *addr == payee),
            "DRC27: payee already exists"
        );
        self.payees.push((payee, shares));
        self.total_shares += shares;
    }

    /// Simulates receiving funds into the splitter contract.
    pub fn receive_funds(&mut self, amount: u64) {
        assert!(amount > 0, "DRC27: receive amount must be positive");
        self.total_received += amount;
    }

    pub fn release(&mut self, addr: Address) -> u64 {
        let releasable = self.releasable(&addr);
        assert!(releasable > 0, "DRC27: nothing to release for this payee");
        let already_released = self.released.get(&addr).copied().unwrap_or(0);
        self.released.insert(addr, already_released + releasable);
        self.total_released += releasable;
        releasable
    }

    // -- Queries -------------------------------------------------------------

    pub fn releasable(&self, addr: &Address) -> u64 {
        let shares = self.shares_of(addr);
        if shares == 0 || self.total_shares == 0 {
            return 0;
        }
        let total_owed = (self.total_received * shares) / self.total_shares;
        let already_released = self.released.get(addr).copied().unwrap_or(0);
        total_owed.saturating_sub(already_released)
    }

    pub fn shares_of(&self, addr: &Address) -> u64 {
        self.payees
            .iter()
            .find(|(a, _)| a == addr)
            .map(|(_, s)| *s)
            .unwrap_or(0)
    }

    pub fn total_received(&self) -> u64 {
        self.total_received
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct AddPayeeArgs {
    payee: Address,
    shares: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddrArgs {
    addr: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReceiveArgs {
    amount: u64,
}


pub fn dispatch(
    state: &mut Option<SplitterState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC27: already initialised");
            *state = Some(SplitterState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "add_payee" => {
            let s = state.as_mut().expect("DRC27: not initialised");
            let a: AddPayeeArgs =
                serde_json::from_slice(args).expect("DRC27: bad add_payee args");
            s.add_payee(caller, a.payee, a.shares);
            serde_json::to_vec("ok").unwrap()
        }
        "receive" => {
            let s = state.as_mut().expect("DRC27: not initialised");
            let a: ReceiveArgs =
                serde_json::from_slice(args).expect("DRC27: bad receive args");
            s.receive_funds(a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "release" => {
            let s = state.as_mut().expect("DRC27: not initialised");
            let a: AddrArgs =
                serde_json::from_slice(args).expect("DRC27: bad release args");
            let amount = s.release(a.addr);
            serde_json::to_vec(&amount).unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "releasable" => {
            let s = state.as_ref().expect("DRC27: not initialised");
            let a: AddrArgs =
                serde_json::from_slice(args).expect("DRC27: bad releasable args");
            serde_json::to_vec(&s.releasable(&a.addr)).unwrap()
        }
        "shares_of" => {
            let s = state.as_ref().expect("DRC27: not initialised");
            let a: AddrArgs =
                serde_json::from_slice(args).expect("DRC27: bad shares_of args");
            serde_json::to_vec(&s.shares_of(&a.addr)).unwrap()
        }
        "total_received" => {
            let s = state.as_ref().expect("DRC27: not initialised");
            serde_json::to_vec(&s.total_received()).unwrap()
        }

        _ => panic!("DRC27: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(seed: u8) -> Address {
        [seed; 32]
    }

    fn init(state: &mut Option<SplitterState>, owner: Address) {
        dispatch(state, "init", b"{}", owner);
    }

    #[test]
    fn test_add_payees_and_shares() {
        let mut state = None;
        let owner = addr(1);
        init(&mut state, owner);

        let args = serde_json::to_vec(&AddPayeeArgs {
            payee: addr(2),
            shares: 60,
        })
        .unwrap();
        dispatch(&mut state, "add_payee", &args, owner);

        let args = serde_json::to_vec(&AddPayeeArgs {
            payee: addr(3),
            shares: 40,
        })
        .unwrap();
        dispatch(&mut state, "add_payee", &args, owner);

        let s = state.as_ref().unwrap();
        assert_eq!(s.total_shares, 100);
        assert_eq!(s.shares_of(&addr(2)), 60);
        assert_eq!(s.shares_of(&addr(3)), 40);
    }

    #[test]
    fn test_release_proportional() {
        let mut state = None;
        let owner = addr(1);
        init(&mut state, owner);

        dispatch(
            &mut state,
            "add_payee",
            &serde_json::to_vec(&AddPayeeArgs { payee: addr(2), shares: 70 }).unwrap(),
            owner,
        );
        dispatch(
            &mut state,
            "add_payee",
            &serde_json::to_vec(&AddPayeeArgs { payee: addr(3), shares: 30 }).unwrap(),
            owner,
        );

        // Receive 1000 units
        dispatch(
            &mut state,
            "receive",
            &serde_json::to_vec(&ReceiveArgs { amount: 1000 }).unwrap(),
            addr(99),
        );

        // Release for payee 2 (70%)
        let result = dispatch(
            &mut state,
            "release",
            &serde_json::to_vec(&AddrArgs { addr: addr(2) }).unwrap(),
            addr(99),
        );
        let released: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(released, 700);

        // Release for payee 3 (30%)
        let result = dispatch(
            &mut state,
            "release",
            &serde_json::to_vec(&AddrArgs { addr: addr(3) }).unwrap(),
            addr(99),
        );
        let released: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(released, 300);

        assert_eq!(state.as_ref().unwrap().total_released, 1000);
    }

    #[test]
    fn test_incremental_receive_and_release() {
        let mut state = None;
        let owner = addr(1);
        init(&mut state, owner);

        dispatch(
            &mut state,
            "add_payee",
            &serde_json::to_vec(&AddPayeeArgs { payee: addr(2), shares: 50 }).unwrap(),
            owner,
        );
        dispatch(
            &mut state,
            "add_payee",
            &serde_json::to_vec(&AddPayeeArgs { payee: addr(3), shares: 50 }).unwrap(),
            owner,
        );

        // First receive
        dispatch(
            &mut state,
            "receive",
            &serde_json::to_vec(&ReceiveArgs { amount: 200 }).unwrap(),
            addr(99),
        );
        dispatch(
            &mut state,
            "release",
            &serde_json::to_vec(&AddrArgs { addr: addr(2) }).unwrap(),
            addr(99),
        );
        assert_eq!(state.as_ref().unwrap().released.get(&addr(2)).copied().unwrap(), 100);

        // Second receive -- payee 2 should only get the new portion
        dispatch(
            &mut state,
            "receive",
            &serde_json::to_vec(&ReceiveArgs { amount: 400 }).unwrap(),
            addr(99),
        );

        let releasable_bytes = dispatch(
            &mut state,
            "releasable",
            &serde_json::to_vec(&AddrArgs { addr: addr(2) }).unwrap(),
            addr(99),
        );
        let releasable: u64 = serde_json::from_slice(&releasable_bytes).unwrap();
        assert_eq!(releasable, 200); // (600 * 50/100) - 100 already released = 200
    }

    #[test]
    #[should_panic(expected = "DRC27: only owner can add payees")]
    fn test_non_owner_cannot_add_payee() {
        let mut state = None;
        init(&mut state, addr(1));

        let args = serde_json::to_vec(&AddPayeeArgs {
            payee: addr(3),
            shares: 10,
        })
        .unwrap();
        dispatch(&mut state, "add_payee", &args, addr(2));
    }

    #[test]
    #[should_panic(expected = "DRC27: nothing to release")]
    fn test_release_zero_panics() {
        let mut state = None;
        let owner = addr(1);
        init(&mut state, owner);

        dispatch(
            &mut state,
            "add_payee",
            &serde_json::to_vec(&AddPayeeArgs { payee: addr(2), shares: 50 }).unwrap(),
            owner,
        );

        // No funds received yet -- release should panic
        dispatch(
            &mut state,
            "release",
            &serde_json::to_vec(&AddrArgs { addr: addr(2) }).unwrap(),
            addr(99),
        );
    }

    #[test]
    #[should_panic(expected = "DRC27: payee already exists")]
    fn test_duplicate_payee_panics() {
        let mut state = None;
        let owner = addr(1);
        init(&mut state, owner);

        let args = serde_json::to_vec(&AddPayeeArgs {
            payee: addr(2),
            shares: 10,
        })
        .unwrap();
        dispatch(&mut state, "add_payee", &args, owner);
        dispatch(&mut state, "add_payee", &args, owner);
    }
}
