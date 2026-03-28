use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-75  Conditional Payment Channels
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ConditionType {
    PriceAbove {
        pair: String,
        threshold: u64,
    },
    PriceBelow {
        pair: String,
        threshold: u64,
    },
    SensorReading {
        device: Address,
        metric: String,
        threshold: u64,
    },
    BlockHeight {
        height: u64,
    },
    TimePassed {
        timestamp: u64,
    },
    CustomOracle {
        oracle_addr: Address,
        key: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PaymentStatus {
    Pending,
    Executed,
    Cancelled,
    Expired,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConditionalPayment {
    pub id: u64,
    pub payer: Address,
    pub payee: Address,
    pub amount: u64,
    pub condition: ConditionType,
    pub deadline: u64,
    pub status: PaymentStatus,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OracleData {
    pub price: Option<u64>,
    pub sensor_value: Option<u64>,
    pub block_height: Option<u64>,
    pub current_time: Option<u64>,
    pub oracle_result: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConditionalPaymentState {
    pub owner: Address,
    pub payments: BTreeMap<u64, ConditionalPayment>,
    pub next_id: u64,
    pub balances: BTreeMap<Address, u64>,
}

impl ConditionalPaymentState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            payments: BTreeMap::new(),
            next_id: 1,
            balances: BTreeMap::new(),
        }
    }

    pub fn deposit(&mut self, caller: Address, amount: u64) {
        assert!(amount > 0, "DRC75: deposit must be positive");
        *self.balances.entry(caller).or_insert(0) += amount;
    }

    pub fn create_conditional_payment(
        &mut self,
        caller: Address,
        payee: Address,
        amount: u64,
        condition: ConditionType,
        deadline: u64,
        created_at: u64,
    ) -> u64 {
        assert!(amount > 0, "DRC75: amount must be positive");
        assert!(
            deadline > created_at,
            "DRC75: deadline must be in the future"
        );

        let bal = self.balances.get(&caller).copied().unwrap_or(0);
        assert!(bal >= amount, "DRC75: insufficient balance");
        self.balances.insert(caller, bal - amount);

        let id = self.next_id;
        self.next_id += 1;
        self.payments.insert(
            id,
            ConditionalPayment {
                id,
                payer: caller,
                payee,
                amount,
                condition,
                deadline,
                status: PaymentStatus::Pending,
                created_at,
            },
        );
        id
    }

    /// Check whether the condition is satisfied and execute the payment.
    pub fn check_and_execute(&mut self, payment_id: u64, oracle: &OracleData) -> bool {
        let payment = self
            .payments
            .get(&payment_id)
            .expect("DRC75: payment not found");
        assert!(
            payment.status == PaymentStatus::Pending,
            "DRC75: payment not pending"
        );

        let satisfied = match &payment.condition {
            ConditionType::PriceAbove { threshold, .. } => {
                oracle.price.map_or(false, |p| p > *threshold)
            }
            ConditionType::PriceBelow { threshold, .. } => {
                oracle.price.map_or(false, |p| p < *threshold)
            }
            ConditionType::SensorReading { threshold, .. } => {
                oracle.sensor_value.map_or(false, |v| v >= *threshold)
            }
            ConditionType::BlockHeight { height } => {
                oracle.block_height.map_or(false, |h| h >= *height)
            }
            ConditionType::TimePassed { timestamp } => {
                oracle.current_time.map_or(false, |t| t >= *timestamp)
            }
            ConditionType::CustomOracle { .. } => oracle.oracle_result.unwrap_or(false),
        };

        if satisfied {
            let payment = self.payments.get_mut(&payment_id).unwrap();
            payment.status = PaymentStatus::Executed;
            let payee = payment.payee;
            let amount = payment.amount;
            *self.balances.entry(payee).or_insert(0) += amount;
        }
        satisfied
    }

    /// Cancel all expired pending payments, returning funds to payers.
    pub fn cancel_expired(&mut self, current_time: u64) -> u32 {
        let mut cancelled = 0u32;
        let expired: Vec<u64> = self
            .payments
            .values()
            .filter(|p| p.status == PaymentStatus::Pending && current_time > p.deadline)
            .map(|p| p.id)
            .collect();

        for id in expired {
            let payment = self.payments.get_mut(&id).unwrap();
            payment.status = PaymentStatus::Expired;
            let payer = payment.payer;
            let amount = payment.amount;
            *self.balances.entry(payer).or_insert(0) += amount;
            cancelled += 1;
        }
        cancelled
    }

    pub fn my_conditional_payments(&self, addr: &Address) -> Vec<&ConditionalPayment> {
        self.payments
            .values()
            .filter(|p| &p.payer == addr || &p.payee == addr)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateArgs {
    payee: Address,
    amount: u64,
    condition: ConditionType,
    deadline: u64,
    created_at: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct CheckArgs {
    payment_id: u64,
    oracle_data: OracleData,
}
#[derive(Serialize, Deserialize, Debug)]
struct CancelExpiredArgs {
    current_time: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct AddrArgs {
    addr: Address,
}
#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs {
    amount: u64,
}

pub fn dispatch(
    state: &mut Option<ConditionalPaymentState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC75: already initialised");
            *state = Some(ConditionalPaymentState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "deposit" => {
            let s = state.as_mut().expect("DRC75: not initialised");
            let a: DepositArgs = serde_json::from_slice(args).expect("DRC75: bad args");
            s.deposit(caller, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "create_conditional_payment" => {
            let s = state.as_mut().expect("DRC75: not initialised");
            let a: CreateArgs = serde_json::from_slice(args).expect("DRC75: bad args");
            let id = s.create_conditional_payment(
                caller,
                a.payee,
                a.amount,
                a.condition,
                a.deadline,
                a.created_at,
            );
            serde_json::to_vec(&id).unwrap()
        }
        "check_and_execute" => {
            let s = state.as_mut().expect("DRC75: not initialised");
            let a: CheckArgs = serde_json::from_slice(args).expect("DRC75: bad args");
            let result = s.check_and_execute(a.payment_id, &a.oracle_data);
            serde_json::to_vec(&result).unwrap()
        }
        "cancel_expired" => {
            let s = state.as_mut().expect("DRC75: not initialised");
            let a: CancelExpiredArgs = serde_json::from_slice(args).expect("DRC75: bad args");
            let count = s.cancel_expired(a.current_time);
            serde_json::to_vec(&count).unwrap()
        }
        "my_conditional_payments" => {
            let s = state.as_ref().expect("DRC75: not initialised");
            let a: AddrArgs = serde_json::from_slice(args).expect("DRC75: bad args");
            serde_json::to_vec(&s.my_conditional_payments(&a.addr)).unwrap()
        }
        _ => panic!("DRC75: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [0u8; 32];
    const ALICE: Address = [1u8; 32];
    const BOB: Address = [2u8; 32];

    fn setup() -> ConditionalPaymentState {
        let mut s = ConditionalPaymentState::new(OWNER);
        s.deposit(ALICE, 50_000);
        s
    }

    #[test]
    fn test_create_price_above_and_execute() {
        let mut s = setup();
        let id = s.create_conditional_payment(
            ALICE,
            BOB,
            1000,
            ConditionType::PriceAbove {
                pair: "DINA/USD".into(),
                threshold: 500,
            },
            10000,
            1000,
        );
        assert_eq!(s.balances.get(&ALICE).copied().unwrap(), 49000);

        // Price not met
        let oracle = OracleData {
            price: Some(400),
            sensor_value: None,
            block_height: None,
            current_time: None,
            oracle_result: None,
        };
        assert!(!s.check_and_execute(id, &oracle));
        assert_eq!(s.payments.get(&id).unwrap().status, PaymentStatus::Pending);

        // Price met
        let oracle = OracleData {
            price: Some(600),
            sensor_value: None,
            block_height: None,
            current_time: None,
            oracle_result: None,
        };
        assert!(s.check_and_execute(id, &oracle));
        assert_eq!(s.payments.get(&id).unwrap().status, PaymentStatus::Executed);
        assert_eq!(s.balances.get(&BOB).copied().unwrap(), 1000);
    }

    #[test]
    fn test_sensor_reading_condition() {
        let mut s = setup();
        let device: Address = [99u8; 32];
        let id = s.create_conditional_payment(
            ALICE,
            BOB,
            500,
            ConditionType::SensorReading {
                device,
                metric: "temperature".into(),
                threshold: 100,
            },
            10000,
            1000,
        );
        let oracle = OracleData {
            price: None,
            sensor_value: Some(100),
            block_height: None,
            current_time: None,
            oracle_result: None,
        };
        assert!(s.check_and_execute(id, &oracle));
    }

    #[test]
    fn test_cancel_expired() {
        let mut s = setup();
        s.create_conditional_payment(
            ALICE,
            BOB,
            1000,
            ConditionType::TimePassed { timestamp: 5000 },
            3000,
            1000, // deadline 3000
        );
        s.create_conditional_payment(
            ALICE,
            BOB,
            2000,
            ConditionType::TimePassed { timestamp: 5000 },
            6000,
            1000, // deadline 6000
        );

        // At time 4000, first payment expired but second is still pending
        let count = s.cancel_expired(4000);
        assert_eq!(count, 1);
        assert_eq!(s.balances.get(&ALICE).copied().unwrap(), 48000); // got 1000 back
    }

    #[test]
    fn test_my_conditional_payments() {
        let mut s = setup();
        s.create_conditional_payment(
            ALICE,
            BOB,
            100,
            ConditionType::BlockHeight { height: 1000 },
            5000,
            100,
        );
        let alice_payments = s.my_conditional_payments(&ALICE);
        assert_eq!(alice_payments.len(), 1);
        let bob_payments = s.my_conditional_payments(&BOB);
        assert_eq!(bob_payments.len(), 1); // BOB is payee
    }

    #[test]
    fn test_custom_oracle_condition() {
        let mut s = setup();
        let oracle_addr: Address = [88u8; 32];
        let id = s.create_conditional_payment(
            ALICE,
            BOB,
            300,
            ConditionType::CustomOracle {
                oracle_addr,
                key: "task_complete".into(),
            },
            10000,
            1000,
        );
        let oracle = OracleData {
            price: None,
            sensor_value: None,
            block_height: None,
            current_time: None,
            oracle_result: Some(true),
        };
        assert!(s.check_and_execute(id, &oracle));
        assert_eq!(s.balances.get(&BOB).copied().unwrap(), 300);
    }

    #[test]
    #[should_panic(expected = "insufficient balance")]
    fn test_insufficient_balance() {
        let mut s = setup();
        s.create_conditional_payment(
            ALICE,
            BOB,
            999_999,
            ConditionType::BlockHeight { height: 10 },
            5000,
            100,
        );
    }
}
