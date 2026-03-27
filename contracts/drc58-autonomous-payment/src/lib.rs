use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-58  Autonomous Recurring Payments
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaymentAuthorization {
    pub id: u64,
    pub payer: Address,
    pub payee: Address,
    pub max_amount: u64,
    pub frequency_seconds: u64,
    pub total_budget: u64,
    pub spent: u64,
    pub active: bool,
    pub last_payment: u64,
    pub created_at: u64,
    pub payment_count: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaymentRecord {
    pub auth_id: u64,
    pub amount: u64,
    pub timestamp: u64,
    pub payer: Address,
    pub payee: Address,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaymentState {
    pub owner: Address,
    pub authorizations: BTreeMap<u64, PaymentAuthorization>,
    pub payment_history: Vec<PaymentRecord>,
    pub balances: BTreeMap<Address, u64>,
    pub next_auth_id: u64,
}

impl PaymentState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            authorizations: BTreeMap::new(),
            payment_history: Vec::new(),
            balances: BTreeMap::new(),
            next_auth_id: 1,
        }
    }

    pub fn deposit(&mut self, caller: Address, amount: u64) {
        assert!(amount > 0, "DRC58: deposit must be positive");
        let balance = self.balances.entry(caller).or_insert(0);
        *balance += amount;
    }

    pub fn create_authorization(
        &mut self,
        caller: Address,
        payee: Address,
        max_amount: u64,
        frequency_seconds: u64,
        total_budget: u64,
        created_at: u64,
    ) -> u64 {
        assert!(max_amount > 0, "DRC58: max_amount must be positive");
        assert!(frequency_seconds > 0, "DRC58: frequency must be positive");
        assert!(total_budget >= max_amount, "DRC58: budget must cover at least one payment");

        let id = self.next_auth_id;
        self.next_auth_id += 1;
        self.authorizations.insert(id, PaymentAuthorization {
            id,
            payer: caller,
            payee,
            max_amount,
            frequency_seconds,
            total_budget,
            spent: 0,
            active: true,
            last_payment: 0,
            created_at,
            payment_count: 0,
        });
        id
    }

    /// Called by the payee to charge the payer. Checks frequency and budget constraints.
    pub fn charge(&mut self, caller: Address, auth_id: u64, amount: u64, current_time: u64) {
        let auth = self.authorizations.get_mut(&auth_id).expect("DRC58: authorization not found");
        assert!(auth.active, "DRC58: authorization inactive");
        assert!(caller == auth.payee, "DRC58: only payee can charge");
        assert!(amount <= auth.max_amount, "DRC58: amount exceeds max_amount");
        assert!(auth.spent + amount <= auth.total_budget, "DRC58: budget exceeded");

        if auth.last_payment > 0 {
            assert!(
                current_time >= auth.last_payment + auth.frequency_seconds,
                "DRC58: too early, must wait for frequency interval"
            );
        }

        // Deduct from payer balance
        let payer_balance = self.balances.get(&auth.payer).copied().unwrap_or(0);
        assert!(payer_balance >= amount, "DRC58: insufficient payer balance");
        self.balances.insert(auth.payer, payer_balance - amount);

        // Credit payee
        let payee_balance = self.balances.entry(auth.payee).or_insert(0);
        *payee_balance += amount;

        auth.spent += amount;
        auth.last_payment = current_time;
        auth.payment_count += 1;

        self.payment_history.push(PaymentRecord {
            auth_id,
            amount,
            timestamp: current_time,
            payer: auth.payer,
            payee: auth.payee,
        });
    }

    pub fn cancel(&mut self, caller: Address, auth_id: u64) {
        let auth = self.authorizations.get_mut(&auth_id).expect("DRC58: authorization not found");
        assert!(caller == auth.payer, "DRC58: only payer can cancel");
        auth.active = false;
    }

    pub fn modify_budget(&mut self, caller: Address, auth_id: u64, new_budget: u64) {
        let auth = self.authorizations.get_mut(&auth_id).expect("DRC58: authorization not found");
        assert!(caller == auth.payer, "DRC58: only payer can modify budget");
        assert!(new_budget >= auth.spent, "DRC58: new budget less than already spent");
        auth.total_budget = new_budget;
    }

    pub fn payment_history_for(&self, auth_id: u64) -> Vec<&PaymentRecord> {
        self.payment_history.iter().filter(|r| r.auth_id == auth_id).collect()
    }

    pub fn active_authorizations(&self, payer: Address) -> Vec<&PaymentAuthorization> {
        self.authorizations.values()
            .filter(|a| a.payer == payer && a.active)
            .collect()
    }

    pub fn balance_of(&self, addr: &Address) -> u64 {
        self.balances.get(addr).copied().unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs { amount: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct CreateAuthArgs { payee: Address, max_amount: u64, frequency_seconds: u64, total_budget: u64, created_at: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct ChargeArgs { auth_id: u64, amount: u64, current_time: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct CancelArgs { auth_id: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct ModifyBudgetArgs { auth_id: u64, new_budget: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct AuthIdArgs { auth_id: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct PayerArgs { payer: Address }

#[derive(Serialize, Deserialize, Debug)]
struct BalanceArgs { addr: Address }

pub fn dispatch(
    state: &mut Option<PaymentState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC58: already initialised");
            *state = Some(PaymentState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "deposit" => {
            let s = state.as_mut().expect("DRC58: not initialised");
            let a: DepositArgs = serde_json::from_slice(args).expect("DRC58: bad args");
            s.deposit(caller, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "create_authorization" => {
            let s = state.as_mut().expect("DRC58: not initialised");
            let a: CreateAuthArgs = serde_json::from_slice(args).expect("DRC58: bad args");
            let id = s.create_authorization(caller, a.payee, a.max_amount, a.frequency_seconds, a.total_budget, a.created_at);
            serde_json::to_vec(&id).unwrap()
        }
        "charge" => {
            let s = state.as_mut().expect("DRC58: not initialised");
            let a: ChargeArgs = serde_json::from_slice(args).expect("DRC58: bad args");
            s.charge(caller, a.auth_id, a.amount, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }
        "cancel" => {
            let s = state.as_mut().expect("DRC58: not initialised");
            let a: CancelArgs = serde_json::from_slice(args).expect("DRC58: bad args");
            s.cancel(caller, a.auth_id);
            serde_json::to_vec("ok").unwrap()
        }
        "modify_budget" => {
            let s = state.as_mut().expect("DRC58: not initialised");
            let a: ModifyBudgetArgs = serde_json::from_slice(args).expect("DRC58: bad args");
            s.modify_budget(caller, a.auth_id, a.new_budget);
            serde_json::to_vec("ok").unwrap()
        }
        "payment_history" => {
            let s = state.as_ref().expect("DRC58: not initialised");
            let a: AuthIdArgs = serde_json::from_slice(args).expect("DRC58: bad args");
            let history: Vec<&PaymentRecord> = s.payment_history_for(a.auth_id);
            serde_json::to_vec(&history).unwrap()
        }
        "active_authorizations" => {
            let s = state.as_ref().expect("DRC58: not initialised");
            let a: PayerArgs = serde_json::from_slice(args).expect("DRC58: bad args");
            let auths: Vec<&PaymentAuthorization> = s.active_authorizations(a.payer);
            serde_json::to_vec(&auths).unwrap()
        }
        "balance_of" => {
            let s = state.as_ref().expect("DRC58: not initialised");
            let a: BalanceArgs = serde_json::from_slice(args).expect("DRC58: bad args");
            serde_json::to_vec(&s.balance_of(&a.addr)).unwrap()
        }
        _ => panic!("DRC58: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [0u8; 32];
    const PAYER: Address = [1u8; 32];
    const PAYEE: Address = [2u8; 32];

    fn setup() -> PaymentState {
        let mut s = PaymentState::new(OWNER);
        s.deposit(PAYER, 10_000);
        s
    }

    #[test]
    fn test_create_and_charge() {
        let mut s = setup();
        let auth_id = s.create_authorization(PAYER, PAYEE, 100, 3600, 1000, 0);
        s.charge(PAYEE, auth_id, 100, 3600);
        assert_eq!(s.balance_of(&PAYEE), 100);
        assert_eq!(s.balance_of(&PAYER), 9900);
        let auth = s.authorizations.get(&auth_id).unwrap();
        assert_eq!(auth.spent, 100);
        assert_eq!(auth.payment_count, 1);
    }

    #[test]
    fn test_multiple_charges_respect_frequency() {
        let mut s = setup();
        let auth_id = s.create_authorization(PAYER, PAYEE, 50, 100, 500, 0);
        s.charge(PAYEE, auth_id, 50, 100);
        s.charge(PAYEE, auth_id, 50, 200);
        s.charge(PAYEE, auth_id, 50, 300);
        assert_eq!(s.balance_of(&PAYEE), 150);
        let history = s.payment_history_for(auth_id);
        assert_eq!(history.len(), 3);
    }

    #[test]
    #[should_panic(expected = "too early")]
    fn test_charge_too_early() {
        let mut s = setup();
        let auth_id = s.create_authorization(PAYER, PAYEE, 50, 100, 500, 0);
        s.charge(PAYEE, auth_id, 50, 100);
        s.charge(PAYEE, auth_id, 50, 150); // only 50s elapsed, need 100
    }

    #[test]
    #[should_panic(expected = "budget exceeded")]
    fn test_budget_exceeded() {
        let mut s = setup();
        let auth_id = s.create_authorization(PAYER, PAYEE, 100, 10, 150, 0);
        s.charge(PAYEE, auth_id, 100, 10);
        s.charge(PAYEE, auth_id, 100, 20); // 200 > 150 budget
    }

    #[test]
    fn test_cancel_authorization() {
        let mut s = setup();
        let auth_id = s.create_authorization(PAYER, PAYEE, 100, 3600, 1000, 0);
        s.cancel(PAYER, auth_id);
        let auths = s.active_authorizations(PAYER);
        assert!(auths.is_empty());
    }

    #[test]
    fn test_modify_budget() {
        let mut s = setup();
        let auth_id = s.create_authorization(PAYER, PAYEE, 100, 10, 200, 0);
        s.charge(PAYEE, auth_id, 100, 10);
        s.modify_budget(PAYER, auth_id, 500);
        assert_eq!(s.authorizations[&auth_id].total_budget, 500);
    }

    #[test]
    #[should_panic(expected = "only payee can charge")]
    fn test_payer_cannot_self_charge() {
        let mut s = setup();
        let auth_id = s.create_authorization(PAYER, PAYEE, 100, 3600, 1000, 0);
        s.charge(PAYER, auth_id, 100, 3600);
    }
}
