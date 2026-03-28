use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// DRC-21  N-of-M Multisig Wallet
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

/// Governance actions that require multisig approval before execution.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GovernanceAction {
    AddOwner { new_owner: Address },
    RemoveOwner { owner: Address },
    ChangeThreshold { new_threshold: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PendingTx {
    pub id: u64,
    pub to: Address,
    pub amount: u64,
    pub data: Vec<u8>,
    pub approvals: BTreeSet<Address>,
    pub executed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MultisigState {
    pub owners: BTreeSet<Address>,
    pub threshold: u64,
    pub nonce: u64,
    pub pending_txs: BTreeMap<u64, PendingTx>,
}

impl MultisigState {
    pub fn new(owners: Vec<Address>, threshold: u64) -> Self {
        assert!(!owners.is_empty(), "DRC21: owners list cannot be empty");
        assert!(
            threshold > 0 && threshold <= owners.len() as u64,
            "DRC21: invalid threshold ({threshold} for {} owners)",
            owners.len()
        );
        let owner_set: BTreeSet<Address> = owners.into_iter().collect();
        Self {
            owners: owner_set,
            threshold,
            nonce: 0,
            pending_txs: BTreeMap::new(),
        }
    }

    fn require_owner(&self, caller: &Address) {
        assert!(
            self.owners.contains(caller),
            "DRC21: caller is not an owner"
        );
    }

    /// Submit a new transaction proposal. Returns the transaction id.
    pub fn submit_transaction(
        &mut self,
        caller: Address,
        to: Address,
        amount: u64,
        data: Vec<u8>,
    ) -> u64 {
        self.require_owner(&caller);
        let id = self.nonce;
        self.nonce += 1;

        let mut approvals = BTreeSet::new();
        approvals.insert(caller); // submitter auto-approves

        let tx = PendingTx {
            id,
            to,
            amount,
            data,
            approvals,
            executed: false,
        };
        self.pending_txs.insert(id, tx);
        id
    }

    /// Approve a pending transaction.
    pub fn approve(&mut self, caller: Address, tx_id: u64) {
        self.require_owner(&caller);
        let tx = self
            .pending_txs
            .get_mut(&tx_id)
            .expect("DRC21: transaction not found");
        assert!(!tx.executed, "DRC21: transaction already executed");
        assert!(
            !tx.approvals.contains(&caller),
            "DRC21: already approved by this owner"
        );
        tx.approvals.insert(caller);
    }

    /// Execute a transaction if threshold is met.
    pub fn execute_transaction(&mut self, caller: Address, tx_id: u64) -> &PendingTx {
        self.require_owner(&caller);
        let tx = self
            .pending_txs
            .get_mut(&tx_id)
            .expect("DRC21: transaction not found");
        assert!(!tx.executed, "DRC21: transaction already executed");
        assert!(
            tx.approvals.len() as u64 >= self.threshold,
            "DRC21: not enough approvals ({} of {} required)",
            tx.approvals.len(),
            self.threshold
        );

        tx.executed = true;
        self.pending_txs.get(&tx_id).unwrap()
    }

    // -- Governance operations (require multisig approval) -------------------

    /// Internal: create a governance proposal that must reach threshold approvals
    /// before the action is executed. The caller auto-approves. Returns the tx id.
    fn submit_governance_proposal(&mut self, caller: Address, action_data: Vec<u8>) -> u64 {
        self.require_owner(&caller);
        let id = self.nonce;
        self.nonce += 1;

        let mut approvals = BTreeSet::new();
        approvals.insert(caller);

        let tx = PendingTx {
            id,
            to: [0u8; 32], // governance target (self)
            amount: 0,
            data: action_data,
            approvals,
            executed: false,
        };
        self.pending_txs.insert(id, tx);
        id
    }

    /// Execute a governance transaction if threshold is met. Decodes and applies
    /// the governance action stored in `data`.
    pub fn execute_governance(&mut self, caller: Address, tx_id: u64) {
        self.require_owner(&caller);
        let tx = self
            .pending_txs
            .get(&tx_id)
            .expect("DRC21: transaction not found");
        assert!(!tx.executed, "DRC21: transaction already executed");
        assert!(
            tx.approvals.len() as u64 >= self.threshold,
            "DRC21: not enough approvals ({} of {} required)",
            tx.approvals.len(),
            self.threshold
        );

        // Parse the governance action from data
        let action: GovernanceAction =
            serde_json::from_slice(&tx.data).expect("DRC21: invalid governance action");

        // Mark as executed before applying to prevent re-entrancy
        self.pending_txs.get_mut(&tx_id).unwrap().executed = true;

        match action {
            GovernanceAction::AddOwner { new_owner } => {
                assert!(
                    !self.owners.contains(&new_owner),
                    "DRC21: address is already an owner"
                );
                self.owners.insert(new_owner);
            }
            GovernanceAction::RemoveOwner { owner } => {
                assert!(
                    self.owners.contains(&owner),
                    "DRC21: address is not an owner"
                );
                assert!(
                    self.owners.len() as u64 > self.threshold,
                    "DRC21: cannot remove owner — would go below threshold"
                );
                self.owners.remove(&owner);
            }
            GovernanceAction::ChangeThreshold { new_threshold } => {
                assert!(
                    new_threshold > 0 && new_threshold <= self.owners.len() as u64,
                    "DRC21: invalid threshold ({new_threshold} for {} owners)",
                    self.owners.len()
                );
                self.threshold = new_threshold;
            }
        }
    }

    /// Propose adding a new owner. Returns proposal tx id.
    pub fn add_owner(&mut self, caller: Address, new_owner: Address) -> u64 {
        let action = GovernanceAction::AddOwner { new_owner };
        let data = serde_json::to_vec(&action).unwrap();
        self.submit_governance_proposal(caller, data)
    }

    /// Propose removing an owner. Returns proposal tx id.
    pub fn remove_owner(&mut self, caller: Address, owner: Address) -> u64 {
        let action = GovernanceAction::RemoveOwner { owner };
        let data = serde_json::to_vec(&action).unwrap();
        self.submit_governance_proposal(caller, data)
    }

    /// Propose changing the approval threshold. Returns proposal tx id.
    pub fn change_threshold(&mut self, caller: Address, new_threshold: u64) -> u64 {
        let action = GovernanceAction::ChangeThreshold { new_threshold };
        let data = serde_json::to_vec(&action).unwrap();
        self.submit_governance_proposal(caller, data)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    owners: Vec<Address>,
    threshold: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SubmitTransactionArgs {
    to: Address,
    amount: u64,
    data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApproveArgs {
    tx_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteTransactionArgs {
    tx_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddOwnerArgs {
    new_owner: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct RemoveOwnerArgs {
    owner: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChangeThresholdArgs {
    new_threshold: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetTransactionArgs {
    tx_id: u64,
}

pub fn dispatch(
    state: &mut Option<MultisigState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC21: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC21: bad init args");
            *state = Some(MultisigState::new(a.owners, a.threshold));
            serde_json::to_vec("ok").unwrap()
        }

        "submit_transaction" => {
            let s = state.as_mut().expect("DRC21: not initialised");
            let a: SubmitTransactionArgs =
                serde_json::from_slice(args).expect("DRC21: bad submit_transaction args");
            let id = s.submit_transaction(caller, a.to, a.amount, a.data);
            serde_json::to_vec(&id).unwrap()
        }

        "approve" => {
            let s = state.as_mut().expect("DRC21: not initialised");
            let a: ApproveArgs = serde_json::from_slice(args).expect("DRC21: bad approve args");
            s.approve(caller, a.tx_id);
            serde_json::to_vec("ok").unwrap()
        }

        "execute_transaction" => {
            let s = state.as_mut().expect("DRC21: not initialised");
            let a: ExecuteTransactionArgs =
                serde_json::from_slice(args).expect("DRC21: bad execute_transaction args");
            let tx = s.execute_transaction(caller, a.tx_id);
            serde_json::to_vec(tx).unwrap()
        }

        "add_owner" => {
            let s = state.as_mut().expect("DRC21: not initialised");
            let a: AddOwnerArgs = serde_json::from_slice(args).expect("DRC21: bad add_owner args");
            let tx_id = s.add_owner(caller, a.new_owner);
            serde_json::to_vec(&tx_id).unwrap()
        }

        "remove_owner" => {
            let s = state.as_mut().expect("DRC21: not initialised");
            let a: RemoveOwnerArgs =
                serde_json::from_slice(args).expect("DRC21: bad remove_owner args");
            let tx_id = s.remove_owner(caller, a.owner);
            serde_json::to_vec(&tx_id).unwrap()
        }

        "change_threshold" => {
            let s = state.as_mut().expect("DRC21: not initialised");
            let a: ChangeThresholdArgs =
                serde_json::from_slice(args).expect("DRC21: bad change_threshold args");
            let tx_id = s.change_threshold(caller, a.new_threshold);
            serde_json::to_vec(&tx_id).unwrap()
        }

        "execute_governance" => {
            let s = state.as_mut().expect("DRC21: not initialised");
            let a: ExecuteTransactionArgs =
                serde_json::from_slice(args).expect("DRC21: bad execute_governance args");
            s.execute_governance(caller, a.tx_id);
            serde_json::to_vec("ok").unwrap()
        }

        "get_transaction" => {
            let s = state.as_ref().expect("DRC21: not initialised");
            let a: GetTransactionArgs =
                serde_json::from_slice(args).expect("DRC21: bad get_transaction args");
            serde_json::to_vec(&s.pending_txs.get(&a.tx_id)).unwrap()
        }

        _ => panic!("DRC21: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ALICE: Address = [1u8; 32];
    const BOB: Address = [2u8; 32];
    const CHARLIE: Address = [3u8; 32];
    const DAVE: Address = [4u8; 32];
    const OUTSIDER: Address = [99u8; 32];
    const RECIPIENT: Address = [10u8; 32];

    fn init_2of3() -> Option<MultisigState> {
        let mut state = None;
        let args = serde_json::to_vec(&InitArgs {
            owners: vec![ALICE, BOB, CHARLIE],
            threshold: 2,
        })
        .unwrap();
        dispatch(&mut state, "init", &args, ALICE);
        state
    }

    #[test]
    fn test_submit_approve_execute() {
        let mut state = init_2of3();

        // Alice submits (auto-approves)
        let submit_args = serde_json::to_vec(&SubmitTransactionArgs {
            to: RECIPIENT,
            amount: 5000,
            data: vec![],
        })
        .unwrap();
        let result = dispatch(&mut state, "submit_transaction", &submit_args, ALICE);
        let tx_id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(tx_id, 0);

        // Bob approves (now 2 of 2 needed)
        let approve_args = serde_json::to_vec(&ApproveArgs { tx_id }).unwrap();
        dispatch(&mut state, "approve", &approve_args, BOB);

        // Execute
        let exec_args = serde_json::to_vec(&ExecuteTransactionArgs { tx_id }).unwrap();
        dispatch(&mut state, "execute_transaction", &exec_args, ALICE);

        let s = state.as_ref().unwrap();
        assert!(s.pending_txs.get(&tx_id).unwrap().executed);
    }

    #[test]
    #[should_panic(expected = "DRC21: not enough approvals")]
    fn test_execute_without_enough_approvals() {
        let mut state = init_2of3();

        let submit_args = serde_json::to_vec(&SubmitTransactionArgs {
            to: RECIPIENT,
            amount: 100,
            data: vec![],
        })
        .unwrap();
        let result = dispatch(&mut state, "submit_transaction", &submit_args, ALICE);
        let tx_id: u64 = serde_json::from_slice(&result).unwrap();

        // Only 1 approval (Alice auto), need 2
        let exec_args = serde_json::to_vec(&ExecuteTransactionArgs { tx_id }).unwrap();
        dispatch(&mut state, "execute_transaction", &exec_args, ALICE);
    }

    #[test]
    #[should_panic(expected = "DRC21: caller is not an owner")]
    fn test_non_owner_cannot_submit() {
        let mut state = init_2of3();
        let submit_args = serde_json::to_vec(&SubmitTransactionArgs {
            to: RECIPIENT,
            amount: 100,
            data: vec![],
        })
        .unwrap();
        dispatch(&mut state, "submit_transaction", &submit_args, OUTSIDER);
    }

    #[test]
    #[should_panic(expected = "DRC21: already approved by this owner")]
    fn test_cannot_double_approve() {
        let mut state = init_2of3();
        let submit_args = serde_json::to_vec(&SubmitTransactionArgs {
            to: RECIPIENT,
            amount: 100,
            data: vec![],
        })
        .unwrap();
        let result = dispatch(&mut state, "submit_transaction", &submit_args, ALICE);
        let tx_id: u64 = serde_json::from_slice(&result).unwrap();

        // Alice already auto-approved
        let approve_args = serde_json::to_vec(&ApproveArgs { tx_id }).unwrap();
        dispatch(&mut state, "approve", &approve_args, ALICE);
    }

    #[test]
    fn test_add_and_remove_owner_requires_multisig() {
        let mut state = init_2of3();

        // Alice proposes adding Dave — returns a governance tx id
        let add_args = serde_json::to_vec(&AddOwnerArgs { new_owner: DAVE }).unwrap();
        let result = dispatch(&mut state, "add_owner", &add_args, ALICE);
        let gov_tx_id: u64 = serde_json::from_slice(&result).unwrap();

        // Not yet added — only 1 approval (Alice auto-approves)
        assert_eq!(state.as_ref().unwrap().owners.len(), 3);

        // Bob approves the governance proposal
        let approve_args = serde_json::to_vec(&ApproveArgs { tx_id: gov_tx_id }).unwrap();
        dispatch(&mut state, "approve", &approve_args, BOB);

        // Execute the governance action (now 2 of 2 required)
        let exec_args = serde_json::to_vec(&ExecuteTransactionArgs { tx_id: gov_tx_id }).unwrap();
        dispatch(&mut state, "execute_governance", &exec_args, ALICE);
        assert_eq!(state.as_ref().unwrap().owners.len(), 4);

        // Now propose removing Dave (4 owners > threshold 2, so allowed)
        let rm_args = serde_json::to_vec(&RemoveOwnerArgs { owner: DAVE }).unwrap();
        let result = dispatch(&mut state, "remove_owner", &rm_args, ALICE);
        let rm_tx_id: u64 = serde_json::from_slice(&result).unwrap();

        // Bob approves
        let approve_args2 = serde_json::to_vec(&ApproveArgs { tx_id: rm_tx_id }).unwrap();
        dispatch(&mut state, "approve", &approve_args2, BOB);

        // Execute
        let exec_args2 = serde_json::to_vec(&ExecuteTransactionArgs { tx_id: rm_tx_id }).unwrap();
        dispatch(&mut state, "execute_governance", &exec_args2, ALICE);
        assert_eq!(state.as_ref().unwrap().owners.len(), 3);
    }

    #[test]
    #[should_panic(expected = "DRC21: not enough approvals")]
    fn test_governance_requires_threshold_approvals() {
        let mut state = init_2of3();

        // Alice proposes adding Dave (auto-approves = 1 approval)
        let add_args = serde_json::to_vec(&AddOwnerArgs { new_owner: DAVE }).unwrap();
        let result = dispatch(&mut state, "add_owner", &add_args, ALICE);
        let gov_tx_id: u64 = serde_json::from_slice(&result).unwrap();

        // Try to execute without enough approvals (need 2, have 1)
        let exec_args = serde_json::to_vec(&ExecuteTransactionArgs { tx_id: gov_tx_id }).unwrap();
        dispatch(&mut state, "execute_governance", &exec_args, ALICE);
    }

    #[test]
    #[should_panic(expected = "DRC21: cannot remove owner")]
    fn test_cannot_remove_below_threshold() {
        let mut state = None;
        let args = serde_json::to_vec(&InitArgs {
            owners: vec![ALICE, BOB],
            threshold: 2,
        })
        .unwrap();
        dispatch(&mut state, "init", &args, ALICE);

        // Propose removing BOB
        let rm_args = serde_json::to_vec(&RemoveOwnerArgs { owner: BOB }).unwrap();
        let result = dispatch(&mut state, "remove_owner", &rm_args, ALICE);
        let tx_id: u64 = serde_json::from_slice(&result).unwrap();

        // Bob approves his own removal
        let approve_args = serde_json::to_vec(&ApproveArgs { tx_id }).unwrap();
        dispatch(&mut state, "approve", &approve_args, BOB);

        // Execute — should panic (2 owners, threshold 2, cannot remove)
        let exec_args = serde_json::to_vec(&ExecuteTransactionArgs { tx_id }).unwrap();
        dispatch(&mut state, "execute_governance", &exec_args, ALICE);
    }

    #[test]
    fn test_change_threshold_requires_multisig() {
        let mut state = init_2of3();

        // Alice proposes changing threshold to 3
        let args = serde_json::to_vec(&ChangeThresholdArgs { new_threshold: 3 }).unwrap();
        let result = dispatch(&mut state, "change_threshold", &args, ALICE);
        let tx_id: u64 = serde_json::from_slice(&result).unwrap();

        // Bob approves
        let approve_args = serde_json::to_vec(&ApproveArgs { tx_id }).unwrap();
        dispatch(&mut state, "approve", &approve_args, BOB);

        // Execute
        let exec_args = serde_json::to_vec(&ExecuteTransactionArgs { tx_id }).unwrap();
        dispatch(&mut state, "execute_governance", &exec_args, ALICE);
        assert_eq!(state.as_ref().unwrap().threshold, 3);
    }

    #[test]
    #[should_panic(expected = "DRC21: invalid threshold")]
    fn test_invalid_threshold_fails() {
        let mut state = init_2of3();

        let args = serde_json::to_vec(&ChangeThresholdArgs { new_threshold: 4 }).unwrap();
        let result = dispatch(&mut state, "change_threshold", &args, ALICE);
        let tx_id: u64 = serde_json::from_slice(&result).unwrap();

        let approve_args = serde_json::to_vec(&ApproveArgs { tx_id }).unwrap();
        dispatch(&mut state, "approve", &approve_args, BOB);

        let exec_args = serde_json::to_vec(&ExecuteTransactionArgs { tx_id }).unwrap();
        dispatch(&mut state, "execute_governance", &exec_args, ALICE);
    }

    #[test]
    fn test_multiple_pending_txs() {
        let mut state = init_2of3();

        let submit1 = serde_json::to_vec(&SubmitTransactionArgs {
            to: RECIPIENT,
            amount: 100,
            data: vec![1],
        })
        .unwrap();
        let submit2 = serde_json::to_vec(&SubmitTransactionArgs {
            to: RECIPIENT,
            amount: 200,
            data: vec![2],
        })
        .unwrap();

        let r1 = dispatch(&mut state, "submit_transaction", &submit1, ALICE);
        let r2 = dispatch(&mut state, "submit_transaction", &submit2, BOB);
        let id1: u64 = serde_json::from_slice(&r1).unwrap();
        let id2: u64 = serde_json::from_slice(&r2).unwrap();

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(state.as_ref().unwrap().pending_txs.len(), 2);
    }
}
