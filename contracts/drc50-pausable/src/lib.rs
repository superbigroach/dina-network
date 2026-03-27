use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-50  Pausable Token  (ERC-20 Pausable equivalent)
// Token that can be paused/unpaused by admin. All transfers blocked when paused.
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PausableTokenState {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u64,
    pub admin: Address,
    pub pauser: Address,
    pub paused: bool,
    pub balances: BTreeMap<Address, u64>,
}

impl PausableTokenState {
    pub fn new(name: String, symbol: String, decimals: u8, admin: Address) -> Self {
        Self {
            name,
            symbol,
            decimals,
            total_supply: 0,
            admin,
            pauser: admin,
            paused: false,
            balances: BTreeMap::new(),
        }
    }

    fn require_not_paused(&self) {
        assert!(!self.paused, "DRC50: contract is paused");
    }

    // -- Queries -------------------------------------------------------------

    pub fn balance_of(&self, account: &Address) -> u64 {
        self.balances.get(account).copied().unwrap_or(0)
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    // -- Mutations -----------------------------------------------------------

    pub fn pause(&mut self, caller: Address) {
        assert!(
            caller == self.pauser || caller == self.admin,
            "DRC50: only pauser or admin can pause"
        );
        assert!(!self.paused, "DRC50: already paused");
        self.paused = true;
    }

    pub fn unpause(&mut self, caller: Address) {
        assert!(
            caller == self.pauser || caller == self.admin,
            "DRC50: only pauser or admin can unpause"
        );
        assert!(self.paused, "DRC50: not paused");
        self.paused = false;
    }

    pub fn set_pauser(&mut self, caller: Address, new_pauser: Address) {
        assert!(caller == self.admin, "DRC50: only admin can set pauser");
        self.pauser = new_pauser;
    }

    pub fn mint(&mut self, caller: Address, to: Address, amount: u64) {
        assert!(caller == self.admin, "DRC50: only admin can mint");
        assert!(amount > 0, "DRC50: mint amount must be positive");
        self.require_not_paused();
        let bal = self.balance_of(&to);
        self.balances.insert(to, bal + amount);
        self.total_supply += amount;
    }

    pub fn burn(&mut self, caller: Address, amount: u64) {
        assert!(amount > 0, "DRC50: burn amount must be positive");
        self.require_not_paused();
        let bal = self.balance_of(&caller);
        assert!(bal >= amount, "DRC50: insufficient balance");
        self.balances.insert(caller, bal - amount);
        self.total_supply -= amount;
    }

    pub fn transfer(&mut self, caller: Address, to: Address, amount: u64) {
        assert!(amount > 0, "DRC50: transfer amount must be positive");
        self.require_not_paused();
        let from_bal = self.balance_of(&caller);
        assert!(from_bal >= amount, "DRC50: insufficient balance");
        self.balances.insert(caller, from_bal - amount);
        let to_bal = self.balance_of(&to);
        self.balances.insert(to, to_bal + amount);
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs { name: String, symbol: String, decimals: u8 }
#[derive(Serialize, Deserialize, Debug)]
struct TransferArgs { to: Address, amount: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct MintArgs { to: Address, amount: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct BurnArgs { amount: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct AddrArg { account: Address }
#[derive(Serialize, Deserialize, Debug)]
struct SetPauserArgs { pauser: Address }

pub fn dispatch(
    state: &mut Option<PausableTokenState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC50: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC50: bad init args");
            *state = Some(PausableTokenState::new(a.name, a.symbol, a.decimals, caller));
            serde_json::to_vec("ok").unwrap()
        }
        "balance_of" => {
            let s = state.as_ref().expect("DRC50: not initialised");
            let a: AddrArg = serde_json::from_slice(args).expect("DRC50: bad args");
            serde_json::to_vec(&s.balance_of(&a.account)).unwrap()
        }
        "total_supply" => {
            let s = state.as_ref().expect("DRC50: not initialised");
            serde_json::to_vec(&s.total_supply).unwrap()
        }
        "is_paused" => {
            let s = state.as_ref().expect("DRC50: not initialised");
            serde_json::to_vec(&s.is_paused()).unwrap()
        }
        "pause" => {
            let s = state.as_mut().expect("DRC50: not initialised");
            s.pause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "unpause" => {
            let s = state.as_mut().expect("DRC50: not initialised");
            s.unpause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "set_pauser" => {
            let s = state.as_mut().expect("DRC50: not initialised");
            let a: SetPauserArgs = serde_json::from_slice(args).expect("DRC50: bad args");
            s.set_pauser(caller, a.pauser);
            serde_json::to_vec("ok").unwrap()
        }
        "mint" => {
            let s = state.as_mut().expect("DRC50: not initialised");
            let a: MintArgs = serde_json::from_slice(args).expect("DRC50: bad args");
            s.mint(caller, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "burn" => {
            let s = state.as_mut().expect("DRC50: not initialised");
            let a: BurnArgs = serde_json::from_slice(args).expect("DRC50: bad args");
            s.burn(caller, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer" => {
            let s = state.as_mut().expect("DRC50: not initialised");
            let a: TransferArgs = serde_json::from_slice(args).expect("DRC50: bad args");
            s.transfer(caller, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        _ => panic!("DRC50: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(n: u8) -> Address { [n; 32] }

    fn setup() -> Option<PausableTokenState> {
        let mut state = None;
        let args = serde_json::to_vec(&InitArgs {
            name: "PauseToken".into(), symbol: "PSE".into(), decimals: 6,
        }).unwrap();
        dispatch(&mut state, "init", &args, addr(1));
        let mint = serde_json::to_vec(&MintArgs { to: addr(1), amount: 10_000 }).unwrap();
        dispatch(&mut state, "mint", &mint, addr(1));
        state
    }

    #[test]
    fn test_transfer_when_not_paused() {
        let mut state = setup();
        let args = serde_json::to_vec(&TransferArgs { to: addr(2), amount: 500 }).unwrap();
        dispatch(&mut state, "transfer", &args, addr(1));
        let s = state.as_ref().unwrap();
        assert_eq!(s.balance_of(&addr(2)), 500);
    }

    #[test]
    #[should_panic(expected = "contract is paused")]
    fn test_transfer_when_paused() {
        let mut state = setup();
        dispatch(&mut state, "pause", b"", addr(1));
        let args = serde_json::to_vec(&TransferArgs { to: addr(2), amount: 100 }).unwrap();
        dispatch(&mut state, "transfer", &args, addr(1));
    }

    #[test]
    fn test_pause_and_unpause() {
        let mut state = setup();
        assert!(!state.as_ref().unwrap().is_paused());
        dispatch(&mut state, "pause", b"", addr(1));
        assert!(state.as_ref().unwrap().is_paused());
        dispatch(&mut state, "unpause", b"", addr(1));
        assert!(!state.as_ref().unwrap().is_paused());
        // Transfer works again
        let args = serde_json::to_vec(&TransferArgs { to: addr(2), amount: 100 }).unwrap();
        dispatch(&mut state, "transfer", &args, addr(1));
        assert_eq!(state.as_ref().unwrap().balance_of(&addr(2)), 100);
    }

    #[test]
    #[should_panic(expected = "only pauser or admin")]
    fn test_pause_non_admin() {
        let mut state = setup();
        dispatch(&mut state, "pause", b"", addr(99));
    }

    #[test]
    fn test_set_pauser() {
        let mut state = setup();
        let args = serde_json::to_vec(&SetPauserArgs { pauser: addr(5) }).unwrap();
        dispatch(&mut state, "set_pauser", &args, addr(1));
        // New pauser can pause
        dispatch(&mut state, "pause", b"", addr(5));
        assert!(state.as_ref().unwrap().is_paused());
    }

    #[test]
    #[should_panic(expected = "contract is paused")]
    fn test_mint_when_paused() {
        let mut state = setup();
        dispatch(&mut state, "pause", b"", addr(1));
        let mint = serde_json::to_vec(&MintArgs { to: addr(2), amount: 100 }).unwrap();
        dispatch(&mut state, "mint", &mint, addr(1));
    }
}
