use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// DRC-19  Batch Transfer  (payroll / airdrop)
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BatchTransferState {
    pub admin: Address,
    pub total_sent: u64,
    pub total_transfers: u64,
}

impl BatchTransferState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            total_sent: 0,
            total_transfers: 0,
        }
    }

    /// Execute a batch transfer to multiple recipients.
    /// In a real deployment this would debit from a token contract;
    /// here we track totals and validate the batch structure.
    pub fn batch_transfer(
        &mut self,
        caller: Address,
        recipients: Vec<(Address, u64)>,
    ) -> Vec<(Address, u64)> {
        assert!(caller == self.admin, "DRC19: only admin can batch transfer");
        assert!(!recipients.is_empty(), "DRC19: recipients list is empty");

        let mut executed: Vec<(Address, u64)> = Vec::with_capacity(recipients.len());
        for (recipient, amount) in &recipients {
            assert!(*amount > 0, "DRC19: transfer amount must be positive");
            assert!(
                *recipient != [0u8; 32],
                "DRC19: cannot transfer to zero address"
            );
            self.total_sent += amount;
            self.total_transfers += 1;
            executed.push((*recipient, *amount));
        }
        executed
    }

    pub fn total_sent(&self) -> u64 {
        self.total_sent
    }

    pub fn total_transfers(&self) -> u64 {
        self.total_transfers
    }

    pub fn set_admin(&mut self, caller: Address, new_admin: Address) {
        assert!(caller == self.admin, "DRC19: only admin can set admin");
        assert!(new_admin != [0u8; 32], "DRC19: new admin cannot be zero");
        self.admin = new_admin;
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct BatchTransferArgs {
    recipients: Vec<(Address, u64)>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetAdminArgs {
    new_admin: Address,
}

pub fn dispatch(
    state: &mut Option<BatchTransferState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC19: already initialised");
            *state = Some(BatchTransferState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "batch_transfer" => {
            let s = state.as_mut().expect("DRC19: not initialised");
            let a: BatchTransferArgs =
                serde_json::from_slice(args).expect("DRC19: bad batch_transfer args");
            let result = s.batch_transfer(caller, a.recipients);
            serde_json::to_vec(&result).unwrap()
        }

        "total_sent" => {
            let s = state.as_ref().expect("DRC19: not initialised");
            serde_json::to_vec(&s.total_sent()).unwrap()
        }

        "total_transfers" => {
            let s = state.as_ref().expect("DRC19: not initialised");
            serde_json::to_vec(&s.total_transfers()).unwrap()
        }

        "set_admin" => {
            let s = state.as_mut().expect("DRC19: not initialised");
            let a: SetAdminArgs = serde_json::from_slice(args).expect("DRC19: bad set_admin args");
            s.set_admin(caller, a.new_admin);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC19: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ADMIN: Address = [1u8; 32];
    const ALICE: Address = [2u8; 32];
    const BOB: Address = [3u8; 32];
    const CHARLIE: Address = [4u8; 32];
    const DAVE: Address = [5u8; 32];
    const EVE: Address = [6u8; 32];

    fn init_state() -> Option<BatchTransferState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", ADMIN);
        state
    }

    #[test]
    fn test_init_and_query() {
        let state = init_state();
        let s = state.as_ref().unwrap();
        assert_eq!(s.admin, ADMIN);
        assert_eq!(s.total_sent, 0);
        assert_eq!(s.total_transfers, 0);
    }

    #[test]
    fn test_batch_transfer_payroll() {
        let mut state = init_state();
        let recipients = vec![(ALICE, 1000u64), (BOB, 2000), (CHARLIE, 500)];
        let args = serde_json::to_vec(&BatchTransferArgs { recipients }).unwrap();
        dispatch(&mut state, "batch_transfer", &args, ADMIN);

        let s = state.as_ref().unwrap();
        assert_eq!(s.total_sent, 3500);
        assert_eq!(s.total_transfers, 3);
    }

    #[test]
    fn test_multiple_batches_accumulate() {
        let mut state = init_state();

        let args1 = serde_json::to_vec(&BatchTransferArgs {
            recipients: vec![(ALICE, 100), (BOB, 200)],
        })
        .unwrap();
        dispatch(&mut state, "batch_transfer", &args1, ADMIN);

        let args2 = serde_json::to_vec(&BatchTransferArgs {
            recipients: vec![(CHARLIE, 300), (DAVE, 400), (EVE, 500)],
        })
        .unwrap();
        dispatch(&mut state, "batch_transfer", &args2, ADMIN);

        let s = state.as_ref().unwrap();
        assert_eq!(s.total_sent, 1500);
        assert_eq!(s.total_transfers, 5);
    }

    #[test]
    #[should_panic(expected = "DRC19: only admin can batch transfer")]
    fn test_non_admin_cannot_batch_transfer() {
        let mut state = init_state();
        let args = serde_json::to_vec(&BatchTransferArgs {
            recipients: vec![(BOB, 100)],
        })
        .unwrap();
        dispatch(&mut state, "batch_transfer", &args, ALICE);
    }

    #[test]
    #[should_panic(expected = "DRC19: recipients list is empty")]
    fn test_empty_recipients_fails() {
        let mut state = init_state();
        let args = serde_json::to_vec(&BatchTransferArgs { recipients: vec![] }).unwrap();
        dispatch(&mut state, "batch_transfer", &args, ADMIN);
    }

    #[test]
    #[should_panic(expected = "DRC19: transfer amount must be positive")]
    fn test_zero_amount_fails() {
        let mut state = init_state();
        let args = serde_json::to_vec(&BatchTransferArgs {
            recipients: vec![(ALICE, 0)],
        })
        .unwrap();
        dispatch(&mut state, "batch_transfer", &args, ADMIN);
    }

    #[test]
    #[should_panic(expected = "DRC19: cannot transfer to zero address")]
    fn test_zero_address_fails() {
        let mut state = init_state();
        let args = serde_json::to_vec(&BatchTransferArgs {
            recipients: vec![([0u8; 32], 100)],
        })
        .unwrap();
        dispatch(&mut state, "batch_transfer", &args, ADMIN);
    }

    #[test]
    fn test_set_admin() {
        let mut state = init_state();
        let args = serde_json::to_vec(&SetAdminArgs { new_admin: ALICE }).unwrap();
        dispatch(&mut state, "set_admin", &args, ADMIN);
        assert_eq!(state.as_ref().unwrap().admin, ALICE);

        // Old admin can no longer batch transfer
        let batch_args = serde_json::to_vec(&BatchTransferArgs {
            recipients: vec![(BOB, 100)],
        })
        .unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut s = state.clone();
            dispatch(&mut s, "batch_transfer", &batch_args, ADMIN);
        }));
        assert!(result.is_err());
    }
}
