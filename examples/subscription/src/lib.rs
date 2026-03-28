// =============================================================================
// Recurring USDC Subscription Payments
// =============================================================================
//
// This contract enables recurring subscription payments between a subscriber
// and a service provider. Think Stripe subscriptions, but on-chain with USDC:
//
//   1. Provider creates a subscription plan with a price and period
//   2. Subscriber activates the subscription
//   3. Provider calls `charge` periodically — the contract enforces that at
//      least one full period has elapsed since the last charge
//   4. Either party can cancel the subscription
//
// Real-world use cases on Dina Network:
//   - Robot fleet maintenance contracts (monthly per-device fees)
//   - AI agent API access (weekly compute billing)
//   - Sensor data feeds (daily data subscriptions)
//   - SaaS-style service billing for autonomous agents
//
// Design notes:
//   - The provider calls `charge`, not the subscriber. This matches the
//     real-world pattern where service providers initiate billing.
//   - The contract only tracks the schedule and authorization. Actual USDC
//     transfers are executed by the VM based on the return values.
//   - Overdue charges are not batched — the provider must call `charge`
//     once per period. This prevents surprise large debits.
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// =============================================================================
// Types
// =============================================================================

/// Unique identifier for each subscription.
pub type SubId = u64;

/// A recurring subscription between a subscriber and a provider.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Subscription {
    /// The address paying for the subscription.
    pub subscriber: [u8; 32],

    /// The address receiving payments (the service provider).
    pub provider: [u8; 32],

    /// USDC amount charged per period (in smallest units, 6 decimals).
    /// Example: 5_000_000 = 5.00 USDC per period.
    pub amount_per_period: u64,

    /// Length of one billing period in seconds.
    /// Common values:
    ///   - 86_400     = 1 day
    ///   - 604_800    = 1 week
    ///   - 2_592_000  = 30 days (approx. 1 month)
    pub period_seconds: u64,

    /// Unix timestamp of the last successful charge.
    /// The provider cannot charge again until `last_payment + period_seconds`.
    pub last_payment: u64,

    /// Whether the subscription is currently active.
    /// Cancelled subscriptions cannot be charged.
    pub active: bool,
}

// =============================================================================
// Contract State
// =============================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SubscriptionState {
    /// All subscriptions, keyed by unique ID.
    pub subscriptions: BTreeMap<SubId, Subscription>,
    /// Auto-incrementing ID counter.
    pub next_id: SubId,
    /// Contract deployer.
    pub owner: [u8; 32],
}

// =============================================================================
// Contract Methods
// =============================================================================

impl SubscriptionState {
    pub fn new(owner: [u8; 32]) -> Self {
        Self {
            subscriptions: BTreeMap::new(),
            next_id: 1,
            owner,
        }
    }

    /// Create a new subscription.
    ///
    /// The caller becomes the subscriber. The subscription starts active and
    /// the `last_payment` is set to the current time (so the first charge
    /// can happen after one full period elapses).
    ///
    /// # Arguments
    /// * `caller` — The subscriber's address
    /// * `provider` — The service provider's address
    /// * `amount_per_period` — USDC per period (6 decimals)
    /// * `period_seconds` — Billing cycle length in seconds
    /// * `current_time` — Current Unix timestamp
    ///
    /// # Returns
    /// The new subscription's ID.
    pub fn create_subscription(
        &mut self,
        caller: [u8; 32],
        provider: [u8; 32],
        amount_per_period: u64,
        period_seconds: u64,
        current_time: u64,
    ) -> SubId {
        assert!(
            amount_per_period > 0,
            "Subscription: amount must be positive"
        );
        assert!(period_seconds > 0, "Subscription: period must be positive");
        assert!(
            caller != provider,
            "Subscription: subscriber and provider must be different"
        );

        let id = self.next_id;
        self.next_id += 1;

        let sub = Subscription {
            subscriber: caller,
            provider,
            amount_per_period,
            period_seconds,
            // Set last_payment to now so the first charge is due after one period
            last_payment: current_time,
            active: true,
        };

        self.subscriptions.insert(id, sub);
        id
    }

    /// Charge a subscription. Only the provider can call this.
    ///
    /// The contract enforces that at least one full period has elapsed since
    /// the last charge. This prevents double-charging within a period.
    ///
    /// # Arguments
    /// * `caller` — Must be the subscription's provider
    /// * `sub_id` — The subscription to charge
    /// * `current_time` — Current Unix timestamp
    ///
    /// # Returns
    /// A tuple of (amount charged, subscriber address) so the VM can execute
    /// the USDC transfer from subscriber to provider.
    pub fn charge(
        &mut self,
        caller: [u8; 32],
        sub_id: SubId,
        current_time: u64,
    ) -> (u64, [u8; 32], [u8; 32]) {
        let sub = self
            .subscriptions
            .get_mut(&sub_id)
            .expect("Subscription: not found");

        // Only the provider can initiate charges
        assert!(
            caller == sub.provider,
            "Subscription: only the provider can charge"
        );

        // Subscription must be active
        assert!(sub.active, "Subscription: subscription is not active");

        // Enforce the billing period — at least `period_seconds` must have
        // elapsed since the last charge. This is the key timing invariant.
        let next_charge_time = sub.last_payment + sub.period_seconds;
        assert!(
            current_time >= next_charge_time,
            "Subscription: too early to charge (next charge at {})",
            next_charge_time
        );

        // Record this charge
        sub.last_payment = current_time;

        // Return the charge details for the VM to execute the USDC transfer
        (sub.amount_per_period, sub.subscriber, sub.provider)
    }

    /// Cancel a subscription. Either the subscriber or provider can cancel.
    ///
    /// Once cancelled, no more charges can be made. The subscription record
    /// remains on-chain for history/audit purposes.
    ///
    /// # Arguments
    /// * `caller` — Must be either the subscriber or provider
    /// * `sub_id` — The subscription to cancel
    pub fn cancel(&mut self, caller: [u8; 32], sub_id: SubId) {
        let sub = self
            .subscriptions
            .get_mut(&sub_id)
            .expect("Subscription: not found");

        assert!(
            caller == sub.subscriber || caller == sub.provider,
            "Subscription: only subscriber or provider can cancel"
        );
        assert!(
            sub.active,
            "Subscription: subscription is already cancelled"
        );

        sub.active = false;
    }

    /// Get a subscription by its ID.
    pub fn get_subscription(&self, sub_id: SubId) -> Option<&Subscription> {
        self.subscriptions.get(&sub_id)
    }
}

// =============================================================================
// Dispatch Argument Types
// =============================================================================

#[derive(Serialize, Deserialize, Debug)]
struct CreateSubscriptionArgs {
    provider: [u8; 32],
    amount_per_period: u64,
    period_seconds: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChargeArgs {
    sub_id: SubId,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SubIdArgs {
    sub_id: SubId,
}

/// Result returned from charge for the VM to execute the USDC transfer.
#[derive(Serialize, Deserialize, Debug)]
struct ChargeResult {
    amount: u64,
    from: [u8; 32],
    to: [u8; 32],
}

// =============================================================================
// Dispatch Function
// =============================================================================

pub fn dispatch(
    state: &mut Option<SubscriptionState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "Subscription: already initialised");
            *state = Some(SubscriptionState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "create_subscription" => {
            let s = state.as_mut().expect("Subscription: not initialised");
            let a: CreateSubscriptionArgs =
                serde_json::from_slice(args).expect("Subscription: bad create args");
            let id = s.create_subscription(
                caller,
                a.provider,
                a.amount_per_period,
                a.period_seconds,
                a.current_time,
            );
            serde_json::to_vec(&id).unwrap()
        }

        "charge" => {
            let s = state.as_mut().expect("Subscription: not initialised");
            let a: ChargeArgs =
                serde_json::from_slice(args).expect("Subscription: bad charge args");
            let (amount, from, to) = s.charge(caller, a.sub_id, a.current_time);
            serde_json::to_vec(&ChargeResult { amount, from, to }).unwrap()
        }

        "cancel" => {
            let s = state.as_mut().expect("Subscription: not initialised");
            let a: SubIdArgs = serde_json::from_slice(args).expect("Subscription: bad cancel args");
            s.cancel(caller, a.sub_id);
            serde_json::to_vec("ok").unwrap()
        }

        "get_subscription" => {
            let s = state.as_ref().expect("Subscription: not initialised");
            let a: SubIdArgs = serde_json::from_slice(args).expect("Subscription: bad get args");
            serde_json::to_vec(&s.get_subscription(a.sub_id)).unwrap()
        }

        _ => panic!("Subscription: unknown method '{method}'"),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    const DAY: u64 = 86_400;
    const FIVE_USDC: u64 = 5_000_000;

    #[test]
    fn test_subscription_lifecycle() {
        let mut state: Option<SubscriptionState> = None;
        let subscriber = addr(1);
        let provider = addr(2);
        let t0: u64 = 1_000_000;

        dispatch(&mut state, "init", b"{}", subscriber);

        // Create a daily subscription for 5 USDC
        let create_args = serde_json::to_vec(&CreateSubscriptionArgs {
            provider,
            amount_per_period: FIVE_USDC,
            period_seconds: DAY,
            current_time: t0,
        })
        .unwrap();
        let result = dispatch(&mut state, "create_subscription", &create_args, subscriber);
        let sub_id: SubId = serde_json::from_slice(&result).unwrap();
        assert_eq!(sub_id, 1);

        // Charge after one day
        let charge_args = serde_json::to_vec(&ChargeArgs {
            sub_id: 1,
            current_time: t0 + DAY,
        })
        .unwrap();
        let result = dispatch(&mut state, "charge", &charge_args, provider);
        let charge: ChargeResult = serde_json::from_slice(&result).unwrap();
        assert_eq!(charge.amount, FIVE_USDC);
        assert_eq!(charge.from, subscriber);
        assert_eq!(charge.to, provider);

        // Charge after another day
        let charge_args = serde_json::to_vec(&ChargeArgs {
            sub_id: 1,
            current_time: t0 + 2 * DAY,
        })
        .unwrap();
        dispatch(&mut state, "charge", &charge_args, provider);
    }

    #[test]
    #[should_panic(expected = "too early to charge")]
    fn test_double_charge_fails() {
        let mut state: Option<SubscriptionState> = None;
        let subscriber = addr(1);
        let provider = addr(2);
        let t0: u64 = 1_000_000;

        dispatch(&mut state, "init", b"{}", subscriber);

        let create_args = serde_json::to_vec(&CreateSubscriptionArgs {
            provider,
            amount_per_period: FIVE_USDC,
            period_seconds: DAY,
            current_time: t0,
        })
        .unwrap();
        dispatch(&mut state, "create_subscription", &create_args, subscriber);

        // First charge at day 1
        let charge_args = serde_json::to_vec(&ChargeArgs {
            sub_id: 1,
            current_time: t0 + DAY,
        })
        .unwrap();
        dispatch(&mut state, "charge", &charge_args, provider);

        // Try to charge again same day — should fail
        let charge_args = serde_json::to_vec(&ChargeArgs {
            sub_id: 1,
            current_time: t0 + DAY + 100,
        })
        .unwrap();
        dispatch(&mut state, "charge", &charge_args, provider);
    }

    #[test]
    #[should_panic(expected = "not active")]
    fn test_charge_after_cancel_fails() {
        let mut state: Option<SubscriptionState> = None;
        let subscriber = addr(1);
        let provider = addr(2);
        let t0: u64 = 1_000_000;

        dispatch(&mut state, "init", b"{}", subscriber);

        let create_args = serde_json::to_vec(&CreateSubscriptionArgs {
            provider,
            amount_per_period: FIVE_USDC,
            period_seconds: DAY,
            current_time: t0,
        })
        .unwrap();
        dispatch(&mut state, "create_subscription", &create_args, subscriber);

        // Cancel
        let cancel_args = serde_json::to_vec(&SubIdArgs { sub_id: 1 }).unwrap();
        dispatch(&mut state, "cancel", &cancel_args, subscriber);

        // Try to charge — should fail
        let charge_args = serde_json::to_vec(&ChargeArgs {
            sub_id: 1,
            current_time: t0 + DAY,
        })
        .unwrap();
        dispatch(&mut state, "charge", &charge_args, provider);
    }
}
