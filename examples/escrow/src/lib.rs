// =============================================================================
// USDC Escrow Contract — Secure Two-Party Deals on Dina Network
// =============================================================================
//
// This contract enables trustless transactions between a buyer and a seller:
//
//   1. Buyer creates a deal, describing what they want and the USDC price
//   2. Buyer funds the deal — USDC is locked in the contract's escrow
//   3. Seller delivers the goods/service off-chain
//   4. Seller calls `mark_delivered` to signal completion
//   5. Buyer calls `confirm_delivery` to release USDC to the seller
//
// If something goes wrong:
//   - Buyer can `refund` before the seller marks delivery
//   - Either party can `dispute` after delivery for off-chain resolution
//
// This pattern is the foundation of commerce on Dina Network — agents, robots,
// and humans can trade services without trusting each other.
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// =============================================================================
// Types
// =============================================================================

/// Unique identifier for each deal. Auto-incremented by the contract.
pub type DealId = u64;

/// The lifecycle of a deal. Each status transition has strict rules about
/// who can trigger it and what the prerequisites are.
///
/// State machine:
///   Created --> Funded --> Delivered --> Completed
///                  |          |
///                  v          v
///               Refunded   Disputed
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DealStatus {
    /// Deal has been created but not yet funded with USDC.
    Created,
    /// Buyer has deposited USDC into escrow. Seller can now begin work.
    Funded,
    /// Seller has marked the deal as delivered. Awaiting buyer confirmation.
    Delivered,
    /// Buyer confirmed delivery. USDC has been released to seller. Terminal.
    Completed,
    /// Deal is under dispute. Requires off-chain resolution. Terminal.
    Disputed,
    /// Buyer retrieved their USDC before delivery. Terminal.
    Refunded,
}

/// A single escrow deal between a buyer and a seller.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Deal {
    /// The buyer's address — the party paying USDC.
    pub buyer: [u8; 32],
    /// The seller's address — the party providing goods/services.
    pub seller: [u8; 32],
    /// The USDC amount locked in escrow (in smallest units, 6 decimals).
    /// For example, 10_000_000 = 10.00 USDC.
    pub amount: u64,
    /// Human-readable description of what's being traded.
    pub description: String,
    /// Current status of the deal.
    pub status: DealStatus,
}

// =============================================================================
// Contract State
// =============================================================================

/// The complete on-chain state for the Escrow contract.
///
/// Uses a BTreeMap for deterministic ordering (important for consensus —
/// all nodes must compute the exact same state hash).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EscrowState {
    /// All deals, keyed by their unique ID.
    pub deals: BTreeMap<DealId, Deal>,
    /// Auto-incrementing counter for generating unique deal IDs.
    pub next_id: DealId,
    /// The contract deployer's address (not used for access control here,
    /// but useful for upgrades or admin functions in production).
    pub owner: [u8; 32],
}

// =============================================================================
// Contract Methods
// =============================================================================

impl EscrowState {
    /// Initialize the escrow contract. Called once at deployment.
    pub fn new(owner: [u8; 32]) -> Self {
        Self {
            deals: BTreeMap::new(),
            next_id: 1,
            owner,
        }
    }

    /// Create a new deal. The caller becomes the buyer.
    ///
    /// At this point no USDC is locked — the buyer must call `fund_deal` next.
    /// This two-step process lets buyers review the deal before committing funds.
    ///
    /// # Arguments
    /// * `caller` — The buyer's address (automatically provided by the VM)
    /// * `seller` — The seller's address
    /// * `amount` — USDC amount in smallest units (6 decimals)
    /// * `description` — What the buyer is paying for
    ///
    /// # Returns
    /// The newly created deal's ID.
    pub fn create_deal(
        &mut self,
        caller: [u8; 32],
        seller: [u8; 32],
        amount: u64,
        description: String,
    ) -> DealId {
        // Validate inputs
        assert!(amount > 0, "Escrow: amount must be positive");
        assert!(
            !description.is_empty(),
            "Escrow: description cannot be empty"
        );
        assert!(
            caller != seller,
            "Escrow: buyer and seller must be different addresses"
        );

        let id = self.next_id;
        self.next_id += 1;

        let deal = Deal {
            buyer: caller,
            seller,
            amount,
            description,
            status: DealStatus::Created,
        };

        self.deals.insert(id, deal);
        id
    }

    /// Fund a deal by locking USDC in escrow.
    ///
    /// Only the buyer can fund their own deal. The `usdc_attached` parameter
    /// represents the USDC sent with this transaction (similar to `msg.value`
    /// in Ethereum, but for USDC on Dina).
    ///
    /// # Arguments
    /// * `caller` — Must be the deal's buyer
    /// * `deal_id` — The deal to fund
    /// * `usdc_attached` — The amount of USDC attached to this transaction
    pub fn fund_deal(&mut self, caller: [u8; 32], deal_id: DealId, usdc_attached: u64) {
        let deal = self
            .deals
            .get_mut(&deal_id)
            .expect("Escrow: deal not found");

        assert!(
            caller == deal.buyer,
            "Escrow: only the buyer can fund the deal"
        );
        assert!(
            deal.status == DealStatus::Created,
            "Escrow: deal is not in Created status"
        );
        assert!(
            usdc_attached >= deal.amount,
            "Escrow: insufficient USDC attached (need {}, got {})",
            deal.amount,
            usdc_attached
        );

        deal.status = DealStatus::Funded;
    }

    /// Seller marks the deal as delivered.
    ///
    /// This signals to the buyer that the work is done and they should
    /// inspect and confirm delivery. The USDC remains locked until
    /// the buyer confirms.
    ///
    /// # Arguments
    /// * `caller` — Must be the deal's seller
    /// * `deal_id` — The deal to mark as delivered
    pub fn mark_delivered(&mut self, caller: [u8; 32], deal_id: DealId) {
        let deal = self
            .deals
            .get_mut(&deal_id)
            .expect("Escrow: deal not found");

        assert!(
            caller == deal.seller,
            "Escrow: only the seller can mark as delivered"
        );
        assert!(
            deal.status == DealStatus::Funded,
            "Escrow: deal is not in Funded status"
        );

        deal.status = DealStatus::Delivered;
    }

    /// Buyer confirms delivery and releases USDC to the seller.
    ///
    /// This is the happy-path terminal state. After this call, the USDC
    /// is transferred to the seller's account and the deal is complete.
    ///
    /// # Arguments
    /// * `caller` — Must be the deal's buyer
    /// * `deal_id` — The deal to confirm
    ///
    /// # Returns
    /// A tuple of (amount released, seller address) so the VM can execute
    /// the actual USDC transfer.
    pub fn confirm_delivery(&mut self, caller: [u8; 32], deal_id: DealId) -> (u64, [u8; 32]) {
        let deal = self
            .deals
            .get_mut(&deal_id)
            .expect("Escrow: deal not found");

        assert!(
            caller == deal.buyer,
            "Escrow: only the buyer can confirm delivery"
        );
        assert!(
            deal.status == DealStatus::Delivered,
            "Escrow: deal is not in Delivered status"
        );

        deal.status = DealStatus::Completed;

        // Return the payout info so the VM can execute the USDC transfer
        (deal.amount, deal.seller)
    }

    /// Dispute a deal. Either the buyer or seller can dispute.
    ///
    /// Disputes freeze the USDC in escrow and require off-chain resolution
    /// (e.g., via a DAO vote, arbitration service, or admin intervention).
    /// In production, you'd integrate DRC-107 (Reputation) to track disputes.
    ///
    /// # Arguments
    /// * `caller` — Must be either the buyer or seller
    /// * `deal_id` — The deal to dispute
    pub fn dispute(&mut self, caller: [u8; 32], deal_id: DealId) {
        let deal = self
            .deals
            .get_mut(&deal_id)
            .expect("Escrow: deal not found");

        assert!(
            caller == deal.buyer || caller == deal.seller,
            "Escrow: only buyer or seller can dispute"
        );
        assert!(
            deal.status == DealStatus::Funded || deal.status == DealStatus::Delivered,
            "Escrow: can only dispute Funded or Delivered deals"
        );

        deal.status = DealStatus::Disputed;
    }

    /// Buyer requests a refund. Only possible before the seller marks delivery.
    ///
    /// This protects buyers from sellers who never deliver. Once the seller
    /// marks delivery, the buyer must either confirm or dispute.
    ///
    /// # Arguments
    /// * `caller` — Must be the deal's buyer
    /// * `deal_id` — The deal to refund
    ///
    /// # Returns
    /// The refunded USDC amount.
    pub fn refund(&mut self, caller: [u8; 32], deal_id: DealId) -> u64 {
        let deal = self
            .deals
            .get_mut(&deal_id)
            .expect("Escrow: deal not found");

        assert!(
            caller == deal.buyer,
            "Escrow: only the buyer can request a refund"
        );
        assert!(
            deal.status == DealStatus::Funded,
            "Escrow: can only refund Funded deals (before delivery)"
        );

        deal.status = DealStatus::Refunded;
        deal.amount
    }

    /// Get a deal by its ID. Returns None if the deal doesn't exist.
    pub fn get_deal(&self, deal_id: DealId) -> Option<&Deal> {
        self.deals.get(&deal_id)
    }
}

// =============================================================================
// Dispatch Argument Types
// =============================================================================

#[derive(Serialize, Deserialize, Debug)]
struct CreateDealArgs {
    seller: [u8; 32],
    amount: u64,
    description: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct FundDealArgs {
    deal_id: DealId,
    usdc_attached: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct DealIdArgs {
    deal_id: DealId,
}

/// Result returned from confirm_delivery for the VM to execute transfers.
#[derive(Serialize, Deserialize, Debug)]
struct ConfirmResult {
    amount: u64,
    seller: [u8; 32],
}

// =============================================================================
// Dispatch Function
// =============================================================================

pub fn dispatch(
    state: &mut Option<EscrowState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        // -- Initialization --------------------------------------------------
        "init" => {
            assert!(state.is_none(), "Escrow: already initialised");
            *state = Some(EscrowState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "create_deal" => {
            let s = state.as_mut().expect("Escrow: not initialised");
            let a: CreateDealArgs =
                serde_json::from_slice(args).expect("Escrow: bad create_deal args");
            let id = s.create_deal(caller, a.seller, a.amount, a.description);
            serde_json::to_vec(&id).unwrap()
        }

        "fund_deal" => {
            let s = state.as_mut().expect("Escrow: not initialised");
            let a: FundDealArgs = serde_json::from_slice(args).expect("Escrow: bad fund_deal args");
            s.fund_deal(caller, a.deal_id, a.usdc_attached);
            serde_json::to_vec("ok").unwrap()
        }

        "mark_delivered" => {
            let s = state.as_mut().expect("Escrow: not initialised");
            let a: DealIdArgs =
                serde_json::from_slice(args).expect("Escrow: bad mark_delivered args");
            s.mark_delivered(caller, a.deal_id);
            serde_json::to_vec("ok").unwrap()
        }

        "confirm_delivery" => {
            let s = state.as_mut().expect("Escrow: not initialised");
            let a: DealIdArgs =
                serde_json::from_slice(args).expect("Escrow: bad confirm_delivery args");
            let (amount, seller) = s.confirm_delivery(caller, a.deal_id);
            serde_json::to_vec(&ConfirmResult { amount, seller }).unwrap()
        }

        "dispute" => {
            let s = state.as_mut().expect("Escrow: not initialised");
            let a: DealIdArgs = serde_json::from_slice(args).expect("Escrow: bad dispute args");
            s.dispute(caller, a.deal_id);
            serde_json::to_vec("ok").unwrap()
        }

        "refund" => {
            let s = state.as_mut().expect("Escrow: not initialised");
            let a: DealIdArgs = serde_json::from_slice(args).expect("Escrow: bad refund args");
            let refunded = s.refund(caller, a.deal_id);
            serde_json::to_vec(&refunded).unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "get_deal" => {
            let s = state.as_ref().expect("Escrow: not initialised");
            let a: DealIdArgs = serde_json::from_slice(args).expect("Escrow: bad get_deal args");
            serde_json::to_vec(&s.get_deal(a.deal_id)).unwrap()
        }

        _ => panic!("Escrow: unknown method '{method}'"),
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

    #[test]
    fn test_happy_path() {
        let mut state: Option<EscrowState> = None;
        let buyer = addr(1);
        let seller = addr(2);

        // Deploy
        dispatch(&mut state, "init", b"{}", buyer);

        // Buyer creates a deal for 10 USDC
        let create_args = serde_json::to_vec(&CreateDealArgs {
            seller,
            amount: 10_000_000, // 10 USDC (6 decimals)
            description: "Build me a robot arm".to_string(),
        })
        .unwrap();
        let result = dispatch(&mut state, "create_deal", &create_args, buyer);
        let deal_id: DealId = serde_json::from_slice(&result).unwrap();
        assert_eq!(deal_id, 1);

        // Buyer funds the deal
        let fund_args = serde_json::to_vec(&FundDealArgs {
            deal_id: 1,
            usdc_attached: 10_000_000,
        })
        .unwrap();
        dispatch(&mut state, "fund_deal", &fund_args, buyer);

        // Seller delivers
        let deliver_args = serde_json::to_vec(&DealIdArgs { deal_id: 1 }).unwrap();
        dispatch(&mut state, "mark_delivered", &deliver_args, seller);

        // Buyer confirms
        let confirm_args = serde_json::to_vec(&DealIdArgs { deal_id: 1 }).unwrap();
        let result = dispatch(&mut state, "confirm_delivery", &confirm_args, buyer);
        let confirm: ConfirmResult = serde_json::from_slice(&result).unwrap();
        assert_eq!(confirm.amount, 10_000_000);
        assert_eq!(confirm.seller, seller);

        // Verify final status
        let get_args = serde_json::to_vec(&DealIdArgs { deal_id: 1 }).unwrap();
        let result = dispatch(&mut state, "get_deal", &get_args, buyer);
        let deal: Option<Deal> = serde_json::from_slice(&result).unwrap();
        assert_eq!(deal.unwrap().status, DealStatus::Completed);
    }

    #[test]
    fn test_refund_before_delivery() {
        let mut state: Option<EscrowState> = None;
        let buyer = addr(1);
        let seller = addr(2);

        dispatch(&mut state, "init", b"{}", buyer);

        let create_args = serde_json::to_vec(&CreateDealArgs {
            seller,
            amount: 5_000_000,
            description: "Test deal".to_string(),
        })
        .unwrap();
        dispatch(&mut state, "create_deal", &create_args, buyer);

        let fund_args = serde_json::to_vec(&FundDealArgs {
            deal_id: 1,
            usdc_attached: 5_000_000,
        })
        .unwrap();
        dispatch(&mut state, "fund_deal", &fund_args, buyer);

        // Buyer refunds before delivery
        let refund_args = serde_json::to_vec(&DealIdArgs { deal_id: 1 }).unwrap();
        let result = dispatch(&mut state, "refund", &refund_args, buyer);
        let refunded: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(refunded, 5_000_000);
    }

    #[test]
    #[should_panic(expected = "only the buyer can request a refund")]
    fn test_seller_cannot_refund() {
        let mut state: Option<EscrowState> = None;
        let buyer = addr(1);
        let seller = addr(2);

        dispatch(&mut state, "init", b"{}", buyer);

        let create_args = serde_json::to_vec(&CreateDealArgs {
            seller,
            amount: 5_000_000,
            description: "Test deal".to_string(),
        })
        .unwrap();
        dispatch(&mut state, "create_deal", &create_args, buyer);

        let fund_args = serde_json::to_vec(&FundDealArgs {
            deal_id: 1,
            usdc_attached: 5_000_000,
        })
        .unwrap();
        dispatch(&mut state, "fund_deal", &fund_args, buyer);

        // Seller tries to refund — should fail
        let refund_args = serde_json::to_vec(&DealIdArgs { deal_id: 1 }).unwrap();
        dispatch(&mut state, "refund", &refund_args, seller);
    }
}
