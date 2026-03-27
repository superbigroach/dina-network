use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-79  Micro Payment Streaming (Pay Per Second)
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaymentStream {
    pub id: u64,
    pub sender: Address,
    pub receiver: Address,
    pub rate_per_second: u64,
    pub total_deposited: u64,
    pub total_withdrawn: u64,
    pub start_time: u64,
    pub last_withdrawal_time: u64,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StreamBalance {
    pub withdrawable: u64,
    pub remaining_deposit: u64,
    pub elapsed_seconds: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaymentStreamState {
    pub owner: Address,
    pub streams: BTreeMap<u64, PaymentStream>,
    pub next_id: u64,
    pub balances: BTreeMap<Address, u64>,
}

impl PaymentStreamState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            streams: BTreeMap::new(),
            next_id: 1,
            balances: BTreeMap::new(),
        }
    }

    pub fn deposit(&mut self, caller: Address, amount: u64) {
        assert!(amount > 0, "DRC79: deposit must be positive");
        *self.balances.entry(caller).or_insert(0) += amount;
    }

    pub fn create_stream(
        &mut self,
        caller: Address,
        receiver: Address,
        rate_per_second: u64,
        deposit_amount: u64,
        start_time: u64,
    ) -> u64 {
        assert!(rate_per_second > 0, "DRC79: rate must be positive");
        assert!(deposit_amount > 0, "DRC79: deposit must be positive");
        assert!(caller != receiver, "DRC79: cannot stream to yourself");

        let bal = self.balances.get(&caller).copied().unwrap_or(0);
        assert!(bal >= deposit_amount, "DRC79: insufficient balance");
        self.balances.insert(caller, bal - deposit_amount);

        let id = self.next_id;
        self.next_id += 1;
        self.streams.insert(id, PaymentStream {
            id,
            sender: caller,
            receiver,
            rate_per_second,
            total_deposited: deposit_amount,
            total_withdrawn: 0,
            start_time,
            last_withdrawal_time: start_time,
            active: true,
        });
        id
    }

    /// Withdraw accumulated payments from a stream. Only the receiver can withdraw.
    pub fn withdraw(&mut self, caller: Address, stream_id: u64, current_time: u64) -> u64 {
        let stream = self.streams.get_mut(&stream_id).expect("DRC79: stream not found");
        assert!(stream.active, "DRC79: stream not active");
        assert!(caller == stream.receiver, "DRC79: only receiver can withdraw");
        assert!(current_time >= stream.last_withdrawal_time, "DRC79: invalid time");

        let elapsed = current_time - stream.last_withdrawal_time;
        let accrued = elapsed * stream.rate_per_second;
        let remaining = stream.total_deposited - stream.total_withdrawn;
        let withdrawable = accrued.min(remaining);

        if withdrawable > 0 {
            stream.total_withdrawn += withdrawable;
            stream.last_withdrawal_time = current_time;
            *self.balances.entry(caller).or_insert(0) += withdrawable;

            // Auto-deactivate if fully consumed
            if stream.total_withdrawn >= stream.total_deposited {
                stream.active = false;
            }
        }
        withdrawable
    }

    /// Top up an existing stream with additional deposit.
    pub fn top_up(&mut self, caller: Address, stream_id: u64, amount: u64) {
        assert!(amount > 0, "DRC79: amount must be positive");
        let stream = self.streams.get_mut(&stream_id).expect("DRC79: stream not found");
        assert!(caller == stream.sender, "DRC79: only sender can top up");

        let bal = self.balances.get(&caller).copied().unwrap_or(0);
        assert!(bal >= amount, "DRC79: insufficient balance");
        self.balances.insert(caller, bal - amount);

        stream.total_deposited += amount;
        if !stream.active && stream.total_deposited > stream.total_withdrawn {
            stream.active = true;
        }
    }

    /// Cancel a stream, returning un-streamed funds to sender.
    pub fn cancel_stream(&mut self, caller: Address, stream_id: u64, current_time: u64) {
        let stream = self.streams.get_mut(&stream_id).expect("DRC79: stream not found");
        assert!(caller == stream.sender, "DRC79: only sender can cancel");
        assert!(stream.active, "DRC79: stream not active");

        // Calculate what receiver is owed up to now
        let elapsed = current_time.saturating_sub(stream.last_withdrawal_time);
        let accrued = elapsed * stream.rate_per_second;
        let remaining = stream.total_deposited - stream.total_withdrawn;
        let owed = accrued.min(remaining);

        // Pay receiver what they're owed
        if owed > 0 {
            let receiver = stream.receiver;
            stream.total_withdrawn += owed;
            *self.balances.entry(receiver).or_insert(0) += owed;
        }

        // Refund remaining to sender
        let refund = stream.total_deposited - stream.total_withdrawn;
        if refund > 0 {
            *self.balances.entry(caller).or_insert(0) += refund;
        }
        stream.active = false;
    }

    /// Check stream balance without withdrawing.
    pub fn stream_balance(&self, stream_id: u64, current_time: u64) -> StreamBalance {
        let stream = self.streams.get(&stream_id).expect("DRC79: stream not found");
        let elapsed = current_time.saturating_sub(stream.last_withdrawal_time);
        let accrued = elapsed * stream.rate_per_second;
        let remaining = stream.total_deposited - stream.total_withdrawn;
        let withdrawable = accrued.min(remaining);

        StreamBalance {
            withdrawable,
            remaining_deposit: remaining - withdrawable,
            elapsed_seconds: elapsed,
        }
    }

    pub fn active_streams_of(&self, addr: &Address) -> Vec<&PaymentStream> {
        self.streams.values()
            .filter(|s| s.active && (&s.sender == addr || &s.receiver == addr))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateStreamArgs { receiver: Address, rate_per_second: u64, deposit_amount: u64, start_time: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct WithdrawArgs { stream_id: u64, current_time: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct TopUpArgs { stream_id: u64, amount: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct CancelArgs { stream_id: u64, current_time: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct BalanceArgs { stream_id: u64, current_time: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct AddrArgs { addr: Address }
#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs { amount: u64 }

pub fn dispatch(
    state: &mut Option<PaymentStreamState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC79: already initialised");
            *state = Some(PaymentStreamState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "deposit" => {
            let s = state.as_mut().expect("DRC79: not initialised");
            let a: DepositArgs = serde_json::from_slice(args).expect("DRC79: bad args");
            s.deposit(caller, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "create_stream" => {
            let s = state.as_mut().expect("DRC79: not initialised");
            let a: CreateStreamArgs = serde_json::from_slice(args).expect("DRC79: bad args");
            let id = s.create_stream(caller, a.receiver, a.rate_per_second, a.deposit_amount, a.start_time);
            serde_json::to_vec(&id).unwrap()
        }
        "withdraw" => {
            let s = state.as_mut().expect("DRC79: not initialised");
            let a: WithdrawArgs = serde_json::from_slice(args).expect("DRC79: bad args");
            let amount = s.withdraw(caller, a.stream_id, a.current_time);
            serde_json::to_vec(&amount).unwrap()
        }
        "top_up" => {
            let s = state.as_mut().expect("DRC79: not initialised");
            let a: TopUpArgs = serde_json::from_slice(args).expect("DRC79: bad args");
            s.top_up(caller, a.stream_id, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "cancel_stream" => {
            let s = state.as_mut().expect("DRC79: not initialised");
            let a: CancelArgs = serde_json::from_slice(args).expect("DRC79: bad args");
            s.cancel_stream(caller, a.stream_id, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }
        "stream_balance" => {
            let s = state.as_ref().expect("DRC79: not initialised");
            let a: BalanceArgs = serde_json::from_slice(args).expect("DRC79: bad args");
            serde_json::to_vec(&s.stream_balance(a.stream_id, a.current_time)).unwrap()
        }
        "active_streams_of" => {
            let s = state.as_ref().expect("DRC79: not initialised");
            let a: AddrArgs = serde_json::from_slice(args).expect("DRC79: bad args");
            serde_json::to_vec(&s.active_streams_of(&a.addr)).unwrap()
        }
        _ => panic!("DRC79: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [0u8; 32];
    const ALICE: Address = [1u8; 32]; // sender
    const BOB: Address = [2u8; 32];   // receiver

    fn setup() -> (PaymentStreamState, u64) {
        let mut s = PaymentStreamState::new(OWNER);
        s.deposit(ALICE, 100_000);
        // 10 tokens per second, 10000 deposited, starts at t=0
        let stream_id = s.create_stream(ALICE, BOB, 10, 10_000, 0);
        (s, stream_id)
    }

    #[test]
    fn test_create_stream_and_balance() {
        let (s, sid) = setup();
        assert_eq!(s.balances.get(&ALICE).copied().unwrap(), 90_000);
        let bal = s.stream_balance(sid, 100); // 100 seconds elapsed
        assert_eq!(bal.withdrawable, 1000); // 10 * 100
        assert_eq!(bal.remaining_deposit, 9000);
    }

    #[test]
    fn test_withdraw() {
        let (mut s, sid) = setup();
        let withdrawn = s.withdraw(BOB, sid, 50); // 50 seconds
        assert_eq!(withdrawn, 500); // 10 * 50
        assert_eq!(s.balances.get(&BOB).copied().unwrap(), 500);

        // Withdraw again at t=100
        let withdrawn2 = s.withdraw(BOB, sid, 100);
        assert_eq!(withdrawn2, 500); // another 50 seconds * 10
    }

    #[test]
    fn test_stream_exhaustion() {
        let (mut s, sid) = setup();
        // At t=2000, 2000*10 = 20000 owed but only 10000 deposited
        let withdrawn = s.withdraw(BOB, sid, 2000);
        assert_eq!(withdrawn, 10_000);
        assert!(!s.streams.get(&sid).unwrap().active);
    }

    #[test]
    fn test_top_up() {
        let (mut s, sid) = setup();
        s.top_up(ALICE, sid, 5000);
        assert_eq!(s.streams.get(&sid).unwrap().total_deposited, 15_000);
        assert_eq!(s.balances.get(&ALICE).copied().unwrap(), 85_000);
    }

    #[test]
    fn test_cancel_stream() {
        let (mut s, sid) = setup();
        // Cancel at t=100: receiver owed 1000, sender refunded 9000
        s.cancel_stream(ALICE, sid, 100);
        assert!(!s.streams.get(&sid).unwrap().active);
        assert_eq!(s.balances.get(&BOB).copied().unwrap(), 1000);
        assert_eq!(s.balances.get(&ALICE).copied().unwrap(), 99_000); // 90000 + 9000 refund
    }

    #[test]
    fn test_active_streams() {
        let (s, _sid) = setup();
        assert_eq!(s.active_streams_of(&ALICE).len(), 1);
        assert_eq!(s.active_streams_of(&BOB).len(), 1);
    }

    #[test]
    #[should_panic(expected = "only receiver can withdraw")]
    fn test_sender_cannot_withdraw() {
        let (mut s, sid) = setup();
        s.withdraw(ALICE, sid, 50);
    }
}
