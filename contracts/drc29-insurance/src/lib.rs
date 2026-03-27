use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-29  Insurance Pool
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Policy {
    pub holder: Address,
    pub premium: u64,
    pub coverage: u64,
    pub start: u64,
    pub end: u64,
    pub active: bool,
    pub claimed: bool,
    pub premiums_paid: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InsuranceState {
    pub admin: Address,
    pub policies: BTreeMap<u64, Policy>,
    pub pool_balance: u64,
    pub next_id: u64,
}

impl InsuranceState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            policies: BTreeMap::new(),
            pool_balance: 0,
            next_id: 0,
        }
    }

    // -- Mutations -----------------------------------------------------------

    pub fn create_policy(
        &mut self,
        caller: Address,
        holder: Address,
        premium: u64,
        coverage: u64,
        start: u64,
        end: u64,
    ) -> u64 {
        assert!(caller == self.admin, "DRC29: only admin can create policies");
        assert!(premium > 0, "DRC29: premium must be positive");
        assert!(coverage > 0, "DRC29: coverage must be positive");
        assert!(end > start, "DRC29: end must be after start");

        let id = self.next_id;
        self.next_id += 1;
        self.policies.insert(
            id,
            Policy {
                holder,
                premium,
                coverage,
                start,
                end,
                active: true,
                claimed: false,
                premiums_paid: 0,
            },
        );
        id
    }

    pub fn pay_premium(&mut self, caller: Address, policy_id: u64, amount: u64) {
        let policy = self
            .policies
            .get_mut(&policy_id)
            .expect("DRC29: policy not found");
        assert!(caller == policy.holder, "DRC29: only holder can pay premium");
        assert!(policy.active, "DRC29: policy is not active");
        assert!(
            amount == policy.premium,
            "DRC29: amount must equal premium"
        );

        policy.premiums_paid += amount;
        self.pool_balance += amount;
    }

    pub fn file_claim(&mut self, caller: Address, policy_id: u64, current_time: u64) {
        let policy = self
            .policies
            .get_mut(&policy_id)
            .expect("DRC29: policy not found");
        assert!(caller == policy.holder, "DRC29: only holder can file claim");
        assert!(policy.active, "DRC29: policy is not active");
        assert!(!policy.claimed, "DRC29: claim already filed");
        assert!(
            current_time >= policy.start && current_time <= policy.end,
            "DRC29: claim outside policy period"
        );
        assert!(
            policy.premiums_paid > 0,
            "DRC29: no premiums paid"
        );

        policy.claimed = true;
    }

    pub fn approve_claim(&mut self, caller: Address, policy_id: u64) -> u64 {
        assert!(caller == self.admin, "DRC29: only admin can approve claims");
        let policy = self
            .policies
            .get_mut(&policy_id)
            .expect("DRC29: policy not found");
        assert!(policy.claimed, "DRC29: no claim filed");
        assert!(policy.active, "DRC29: policy is not active");
        assert!(
            self.pool_balance >= policy.coverage,
            "DRC29: insufficient pool balance"
        );

        let payout = policy.coverage;
        self.pool_balance -= payout;
        policy.active = false;
        payout
    }

    pub fn cancel_policy(&mut self, caller: Address, policy_id: u64) {
        let policy = self
            .policies
            .get_mut(&policy_id)
            .expect("DRC29: policy not found");
        assert!(
            caller == self.admin || caller == policy.holder,
            "DRC29: only admin or holder can cancel"
        );
        assert!(policy.active, "DRC29: policy already inactive");
        assert!(!policy.claimed, "DRC29: cannot cancel with pending claim");

        policy.active = false;
    }

    // -- Queries -------------------------------------------------------------

    pub fn get_pool_balance(&self) -> u64 {
        self.pool_balance
    }

    pub fn get_policy(&self, policy_id: u64) -> &Policy {
        self.policies
            .get(&policy_id)
            .expect("DRC29: policy not found")
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreatePolicyArgs {
    holder: Address,
    premium: u64,
    coverage: u64,
    start: u64,
    end: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct PayPremiumArgs {
    policy_id: u64,
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct FileClaimArgs {
    policy_id: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct PolicyIdArgs {
    policy_id: u64,
}

pub fn dispatch(
    state: &mut Option<InsuranceState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC29: already initialised");
            *state = Some(InsuranceState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "create_policy" => {
            let s = state.as_mut().expect("DRC29: not initialised");
            let a: CreatePolicyArgs =
                serde_json::from_slice(args).expect("DRC29: bad create_policy args");
            let id = s.create_policy(caller, a.holder, a.premium, a.coverage, a.start, a.end);
            serde_json::to_vec(&id).unwrap()
        }
        "pay_premium" => {
            let s = state.as_mut().expect("DRC29: not initialised");
            let a: PayPremiumArgs =
                serde_json::from_slice(args).expect("DRC29: bad pay_premium args");
            s.pay_premium(caller, a.policy_id, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "file_claim" => {
            let s = state.as_mut().expect("DRC29: not initialised");
            let a: FileClaimArgs =
                serde_json::from_slice(args).expect("DRC29: bad file_claim args");
            s.file_claim(caller, a.policy_id, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }
        "approve_claim" => {
            let s = state.as_mut().expect("DRC29: not initialised");
            let a: PolicyIdArgs =
                serde_json::from_slice(args).expect("DRC29: bad approve_claim args");
            let payout = s.approve_claim(caller, a.policy_id);
            serde_json::to_vec(&payout).unwrap()
        }
        "cancel_policy" => {
            let s = state.as_mut().expect("DRC29: not initialised");
            let a: PolicyIdArgs =
                serde_json::from_slice(args).expect("DRC29: bad cancel_policy args");
            s.cancel_policy(caller, a.policy_id);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "pool_balance" => {
            let s = state.as_ref().expect("DRC29: not initialised");
            serde_json::to_vec(&s.get_pool_balance()).unwrap()
        }
        "get_policy" => {
            let s = state.as_ref().expect("DRC29: not initialised");
            let a: PolicyIdArgs =
                serde_json::from_slice(args).expect("DRC29: bad get_policy args");
            let policy = s.get_policy(a.policy_id);
            serde_json::to_vec(policy).unwrap()
        }

        _ => panic!("DRC29: unknown method '{method}'"),
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

    fn init(state: &mut Option<InsuranceState>, admin: Address) {
        dispatch(state, "init", b"{}", admin);
    }

    #[test]
    fn test_create_policy_and_pay_premium() {
        let mut state = None;
        let admin = addr(1);
        let holder = addr(2);
        init(&mut state, admin);

        let result = dispatch(
            &mut state,
            "create_policy",
            &serde_json::to_vec(&CreatePolicyArgs {
                holder,
                premium: 100,
                coverage: 5000,
                start: 10,
                end: 1000,
            })
            .unwrap(),
            admin,
        );
        let id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(id, 0);

        // Pay premium
        dispatch(
            &mut state,
            "pay_premium",
            &serde_json::to_vec(&PayPremiumArgs {
                policy_id: 0,
                amount: 100,
            })
            .unwrap(),
            holder,
        );

        let s = state.as_ref().unwrap();
        assert_eq!(s.pool_balance, 100);
        assert_eq!(s.get_policy(0).premiums_paid, 100);
    }

    #[test]
    fn test_full_claim_lifecycle() {
        let mut state = None;
        let admin = addr(1);
        let holder = addr(2);
        init(&mut state, admin);

        dispatch(
            &mut state,
            "create_policy",
            &serde_json::to_vec(&CreatePolicyArgs {
                holder,
                premium: 50,
                coverage: 1000,
                start: 10,
                end: 500,
            })
            .unwrap(),
            admin,
        );

        // Pay multiple premiums to build pool
        for _ in 0..25 {
            dispatch(
                &mut state,
                "pay_premium",
                &serde_json::to_vec(&PayPremiumArgs {
                    policy_id: 0,
                    amount: 50,
                })
                .unwrap(),
                holder,
            );
        }
        assert_eq!(state.as_ref().unwrap().pool_balance, 1250);

        // File claim
        dispatch(
            &mut state,
            "file_claim",
            &serde_json::to_vec(&FileClaimArgs {
                policy_id: 0,
                current_time: 100,
            })
            .unwrap(),
            holder,
        );

        // Approve claim
        let result = dispatch(
            &mut state,
            "approve_claim",
            &serde_json::to_vec(&PolicyIdArgs { policy_id: 0 }).unwrap(),
            admin,
        );
        let payout: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(payout, 1000);
        assert_eq!(state.as_ref().unwrap().pool_balance, 250);
        assert!(!state.as_ref().unwrap().get_policy(0).active);
    }

    #[test]
    fn test_cancel_policy() {
        let mut state = None;
        let admin = addr(1);
        let holder = addr(2);
        init(&mut state, admin);

        dispatch(
            &mut state,
            "create_policy",
            &serde_json::to_vec(&CreatePolicyArgs {
                holder,
                premium: 50,
                coverage: 1000,
                start: 10,
                end: 500,
            })
            .unwrap(),
            admin,
        );

        dispatch(
            &mut state,
            "cancel_policy",
            &serde_json::to_vec(&PolicyIdArgs { policy_id: 0 }).unwrap(),
            holder,
        );

        assert!(!state.as_ref().unwrap().get_policy(0).active);
    }

    #[test]
    #[should_panic(expected = "DRC29: only admin can create policies")]
    fn test_non_admin_cannot_create_policy() {
        let mut state = None;
        init(&mut state, addr(1));

        dispatch(
            &mut state,
            "create_policy",
            &serde_json::to_vec(&CreatePolicyArgs {
                holder: addr(3),
                premium: 50,
                coverage: 1000,
                start: 10,
                end: 500,
            })
            .unwrap(),
            addr(2), // not admin
        );
    }

    #[test]
    #[should_panic(expected = "DRC29: only holder can file claim")]
    fn test_non_holder_cannot_file_claim() {
        let mut state = None;
        let admin = addr(1);
        let holder = addr(2);
        init(&mut state, admin);

        dispatch(
            &mut state,
            "create_policy",
            &serde_json::to_vec(&CreatePolicyArgs {
                holder,
                premium: 50,
                coverage: 1000,
                start: 10,
                end: 500,
            })
            .unwrap(),
            admin,
        );

        dispatch(
            &mut state,
            "pay_premium",
            &serde_json::to_vec(&PayPremiumArgs { policy_id: 0, amount: 50 }).unwrap(),
            holder,
        );

        dispatch(
            &mut state,
            "file_claim",
            &serde_json::to_vec(&FileClaimArgs {
                policy_id: 0,
                current_time: 100,
            })
            .unwrap(),
            addr(3), // not the holder
        );
    }

    #[test]
    #[should_panic(expected = "DRC29: insufficient pool balance")]
    fn test_approve_claim_insufficient_pool() {
        let mut state = None;
        let admin = addr(1);
        let holder = addr(2);
        init(&mut state, admin);

        dispatch(
            &mut state,
            "create_policy",
            &serde_json::to_vec(&CreatePolicyArgs {
                holder,
                premium: 10,
                coverage: 5000,
                start: 10,
                end: 500,
            })
            .unwrap(),
            admin,
        );

        dispatch(
            &mut state,
            "pay_premium",
            &serde_json::to_vec(&PayPremiumArgs { policy_id: 0, amount: 10 }).unwrap(),
            holder,
        );

        dispatch(
            &mut state,
            "file_claim",
            &serde_json::to_vec(&FileClaimArgs { policy_id: 0, current_time: 100 }).unwrap(),
            holder,
        );

        // Pool only has 10, coverage is 5000
        dispatch(
            &mut state,
            "approve_claim",
            &serde_json::to_vec(&PolicyIdArgs { policy_id: 0 }).unwrap(),
            admin,
        );
    }
}
