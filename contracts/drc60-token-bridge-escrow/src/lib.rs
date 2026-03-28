use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-60  Cross-Chain Token Bridge Escrow
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum BridgeStatus {
    Locked,
    Released,
    Refunded,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BridgeLock {
    pub id: u64,
    pub sender: Address,
    pub amount: u64,
    pub destination_chain: String,
    pub destination_address: String,
    pub lock_time: u64,
    pub release_time: u64,
    pub timeout: u64,
    pub status: BridgeStatus,
    pub proof: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BridgeState {
    pub owner: Address,
    pub relayer: Address,
    pub locks: BTreeMap<u64, BridgeLock>,
    pub next_lock_id: u64,
    pub total_locked: u64,
    pub total_released: u64,
    pub total_refunded: u64,
    pub balances: BTreeMap<Address, u64>,
    /// Accumulated balance of released funds available for relayer withdrawal.
    pub relayer_balance: u64,
}

impl BridgeState {
    pub fn new(owner: Address, relayer: Address) -> Self {
        Self {
            owner,
            relayer,
            locks: BTreeMap::new(),
            next_lock_id: 1,
            total_locked: 0,
            total_released: 0,
            total_refunded: 0,
            balances: BTreeMap::new(),
            relayer_balance: 0,
        }
    }

    pub fn deposit(&mut self, caller: Address, amount: u64) {
        assert!(amount > 0, "DRC60: deposit must be positive");
        let bal = self.balances.entry(caller).or_insert(0);
        *bal += amount;
    }

    pub fn lock(
        &mut self,
        caller: Address,
        amount: u64,
        destination_chain: String,
        destination_address: String,
        lock_time: u64,
        timeout: u64,
    ) -> u64 {
        assert!(amount > 0, "DRC60: amount must be positive");
        assert!(
            !destination_chain.is_empty(),
            "DRC60: destination chain required"
        );
        assert!(
            !destination_address.is_empty(),
            "DRC60: destination address required"
        );
        assert!(
            timeout > lock_time,
            "DRC60: timeout must be after lock time"
        );

        let balance = self.balances.get(&caller).copied().unwrap_or(0);
        assert!(balance >= amount, "DRC60: insufficient balance");
        self.balances.insert(caller, balance - amount);

        let id = self.next_lock_id;
        self.next_lock_id += 1;
        self.locks.insert(
            id,
            BridgeLock {
                id,
                sender: caller,
                amount,
                destination_chain,
                destination_address,
                lock_time,
                release_time: 0,
                timeout,
                status: BridgeStatus::Locked,
                proof: None,
            },
        );
        self.total_locked += amount;
        id
    }

    /// Relayer confirms the cross-chain transfer and releases escrowed funds.
    pub fn release(&mut self, caller: Address, lock_id: u64, proof: Vec<u8>, current_time: u64) {
        assert!(
            caller == self.relayer || caller == self.owner,
            "DRC60: only relayer/owner can release"
        );
        let lock = self.locks.get_mut(&lock_id).expect("DRC60: lock not found");
        assert!(
            lock.status == BridgeStatus::Locked,
            "DRC60: not in locked state"
        );
        assert!(!proof.is_empty(), "DRC60: proof required");

        lock.status = BridgeStatus::Released;
        lock.release_time = current_time;
        lock.proof = Some(proof);
        self.total_released += lock.amount;
        self.relayer_balance += lock.amount;
    }

    /// Relayer withdraws accumulated released funds.
    pub fn withdraw_released(&mut self, caller: Address) -> u64 {
        assert!(
            caller == self.relayer || caller == self.owner,
            "DRC60: only relayer/owner can withdraw released funds"
        );
        let amount = self.relayer_balance;
        assert!(amount > 0, "DRC60: no released funds to withdraw");
        self.relayer_balance = 0;
        amount
    }

    /// Sender can reclaim funds after timeout if not released.
    pub fn refund(&mut self, caller: Address, lock_id: u64, current_time: u64) {
        let lock = self.locks.get_mut(&lock_id).expect("DRC60: lock not found");
        assert!(
            lock.status == BridgeStatus::Locked,
            "DRC60: not in locked state"
        );
        assert!(caller == lock.sender, "DRC60: only sender can refund");
        assert!(current_time >= lock.timeout, "DRC60: timeout not reached");

        lock.status = BridgeStatus::Refunded;
        self.total_refunded += lock.amount;

        let balance = self.balances.entry(lock.sender).or_insert(0);
        *balance += lock.amount;
    }

    pub fn locked_amount(&self) -> u64 {
        self.locks
            .values()
            .filter(|l| l.status == BridgeStatus::Locked)
            .map(|l| l.amount)
            .sum()
    }

    pub fn pending_releases(&self) -> Vec<&BridgeLock> {
        self.locks
            .values()
            .filter(|l| l.status == BridgeStatus::Locked)
            .collect()
    }

    pub fn get_lock(&self, lock_id: u64) -> Option<&BridgeLock> {
        self.locks.get(&lock_id)
    }

    pub fn balance_of(&self, addr: &Address) -> u64 {
        self.balances.get(addr).copied().unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    relayer: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs {
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct LockArgs {
    amount: u64,
    destination_chain: String,
    destination_address: String,
    lock_time: u64,
    timeout: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReleaseArgs {
    lock_id: u64,
    proof: Vec<u8>,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct RefundArgs {
    lock_id: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct LockIdArgs {
    lock_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddrArgs {
    addr: Address,
}

pub fn dispatch(
    state: &mut Option<BridgeState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC60: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC60: bad init args");
            *state = Some(BridgeState::new(caller, a.relayer));
            serde_json::to_vec("ok").unwrap()
        }
        "deposit" => {
            let s = state.as_mut().expect("DRC60: not initialised");
            let a: DepositArgs = serde_json::from_slice(args).expect("DRC60: bad args");
            s.deposit(caller, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "lock" => {
            let s = state.as_mut().expect("DRC60: not initialised");
            let a: LockArgs = serde_json::from_slice(args).expect("DRC60: bad args");
            let id = s.lock(
                caller,
                a.amount,
                a.destination_chain,
                a.destination_address,
                a.lock_time,
                a.timeout,
            );
            serde_json::to_vec(&id).unwrap()
        }
        "release" => {
            let s = state.as_mut().expect("DRC60: not initialised");
            let a: ReleaseArgs = serde_json::from_slice(args).expect("DRC60: bad args");
            s.release(caller, a.lock_id, a.proof, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }
        "refund" => {
            let s = state.as_mut().expect("DRC60: not initialised");
            let a: RefundArgs = serde_json::from_slice(args).expect("DRC60: bad args");
            s.refund(caller, a.lock_id, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }
        "locked_amount" => {
            let s = state.as_ref().expect("DRC60: not initialised");
            serde_json::to_vec(&s.locked_amount()).unwrap()
        }
        "pending_releases" => {
            let s = state.as_ref().expect("DRC60: not initialised");
            serde_json::to_vec(&s.pending_releases()).unwrap()
        }
        "get_lock" => {
            let s = state.as_ref().expect("DRC60: not initialised");
            let a: LockIdArgs = serde_json::from_slice(args).expect("DRC60: bad args");
            serde_json::to_vec(&s.get_lock(a.lock_id)).unwrap()
        }
        "balance_of" => {
            let s = state.as_ref().expect("DRC60: not initialised");
            let a: AddrArgs = serde_json::from_slice(args).expect("DRC60: bad args");
            serde_json::to_vec(&s.balance_of(&a.addr)).unwrap()
        }
        "withdraw_released" => {
            let s = state.as_mut().expect("DRC60: not initialised");
            let amount = s.withdraw_released(caller);
            serde_json::to_vec(&amount).unwrap()
        }
        _ => panic!("DRC60: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [0u8; 32];
    const RELAYER: Address = [1u8; 32];
    const SENDER: Address = [2u8; 32];

    fn setup() -> BridgeState {
        let mut s = BridgeState::new(OWNER, RELAYER);
        s.deposit(SENDER, 10_000);
        s
    }

    #[test]
    fn test_lock_and_release() {
        let mut s = setup();
        let lock_id = s.lock(SENDER, 5000, "ethereum".into(), "0xABC".into(), 100, 200);
        assert_eq!(s.locked_amount(), 5000);
        assert_eq!(s.balance_of(&SENDER), 5000);

        s.release(RELAYER, lock_id, vec![0xDE, 0xAD], 150);
        let lock = s.get_lock(lock_id).unwrap();
        assert_eq!(lock.status, BridgeStatus::Released);
        assert_eq!(s.locked_amount(), 0);
        assert_eq!(s.total_released, 5000);
    }

    #[test]
    fn test_refund_after_timeout() {
        let mut s = setup();
        let lock_id = s.lock(SENDER, 3000, "polygon".into(), "0x123".into(), 100, 200);
        assert_eq!(s.balance_of(&SENDER), 7000);
        s.refund(SENDER, lock_id, 200);
        assert_eq!(s.balance_of(&SENDER), 10_000);
        assert_eq!(s.get_lock(lock_id).unwrap().status, BridgeStatus::Refunded);
    }

    #[test]
    #[should_panic(expected = "timeout not reached")]
    fn test_refund_before_timeout() {
        let mut s = setup();
        let lock_id = s.lock(SENDER, 3000, "polygon".into(), "0x123".into(), 100, 200);
        s.refund(SENDER, lock_id, 150);
    }

    #[test]
    #[should_panic(expected = "only relayer/owner")]
    fn test_unauthorized_release() {
        let mut s = setup();
        let lock_id = s.lock(SENDER, 1000, "base".into(), "0x999".into(), 100, 200);
        s.release(SENDER, lock_id, vec![0x01], 150);
    }

    #[test]
    fn test_pending_releases_list() {
        let mut s = setup();
        s.lock(SENDER, 1000, "eth".into(), "0xa".into(), 100, 200);
        s.lock(SENDER, 2000, "arb".into(), "0xb".into(), 100, 200);
        assert_eq!(s.pending_releases().len(), 2);
        s.release(RELAYER, 1, vec![0x01], 150);
        assert_eq!(s.pending_releases().len(), 1);
    }

    #[test]
    #[should_panic(expected = "insufficient balance")]
    fn test_lock_exceeds_balance() {
        let mut s = setup();
        s.lock(SENDER, 20_000, "eth".into(), "0xa".into(), 100, 200);
    }
}
