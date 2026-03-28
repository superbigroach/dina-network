// =============================================================================
// Agent-to-Agent Service Marketplace
// =============================================================================
//
// This contract implements a decentralized marketplace where devices and AI
// agents can list services, hire each other, and pay in USDC. It combines
// patterns from two core Dina standards:
//
//   - DRC-102 (Capability Registry): Devices advertise what they can do
//   - DRC-103 (Service Agreement): Escrow-based service delivery & payment
//
// Example flow:
//   1. A robot arm lists a "pick-and-place" service for 2 USDC per job
//   2. A warehouse agent discovers the listing and calls `hire`
//   3. USDC is locked in escrow within the contract
//   4. The robot arm completes the physical task
//   5. The robot arm calls `complete_service` to release payment
//   6. If the client is unsatisfied, they can call `cancel_hire` (before
//      the provider completes) to get their USDC back
//
// This is the foundational building block for the Machine Economy — autonomous
// agents discovering and paying each other for real-world services.
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// =============================================================================
// Types
// =============================================================================

/// Unique identifier for a service listing.
pub type ListingId = u64;

/// Unique identifier for an active hire (service-in-progress).
pub type HireId = u64;

/// A service listing published by a device or agent.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServiceListing {
    /// The device/agent address offering the service.
    pub provider: [u8; 32],

    /// What type of service this is (e.g. "pick-and-place", "image-classification",
    /// "temperature-reading", "delivery"). Maps to DRC-102 capability types.
    pub service_type: String,

    /// USDC price per service execution (6 decimals).
    pub price: u64,

    /// Human-readable description of the service.
    pub description: String,

    /// Whether the listing is currently accepting new hires.
    /// Providers can toggle this to go offline without deleting the listing.
    pub available: bool,
}

/// The status of a hire (in-progress service).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum HireStatus {
    /// USDC is locked, service is in progress.
    Active,
    /// Provider completed the service, USDC released.
    Completed,
    /// Client cancelled before completion, USDC refunded.
    Cancelled,
}

/// An active hire — a client has paid for a service and is waiting for delivery.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Hire {
    /// Which listing this hire is for.
    pub listing_id: ListingId,
    /// The client who is paying for the service.
    pub client: [u8; 32],
    /// The provider delivering the service.
    pub provider: [u8; 32],
    /// USDC amount locked in escrow for this hire.
    pub amount: u64,
    /// Current status of the hire.
    pub status: HireStatus,
}

// =============================================================================
// Contract State
// =============================================================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MarketplaceState {
    /// All service listings, keyed by unique ID.
    pub listings: BTreeMap<ListingId, ServiceListing>,
    /// All hires (active and historical), keyed by unique ID.
    pub hires: BTreeMap<HireId, Hire>,
    /// Next listing ID counter.
    pub next_listing_id: ListingId,
    /// Next hire ID counter.
    pub next_hire_id: HireId,
    /// Contract deployer.
    pub owner: [u8; 32],
    /// Reverse index: service_type -> list of listing IDs.
    /// Enables efficient discovery of services by type.
    pub type_index: BTreeMap<String, Vec<ListingId>>,
}

// =============================================================================
// Contract Methods
// =============================================================================

impl MarketplaceState {
    pub fn new(owner: [u8; 32]) -> Self {
        Self {
            listings: BTreeMap::new(),
            hires: BTreeMap::new(),
            next_listing_id: 1,
            next_hire_id: 1,
            owner,
            type_index: BTreeMap::new(),
        }
    }

    /// List a new service on the marketplace.
    ///
    /// The caller becomes the service provider. The listing starts as available.
    ///
    /// # Arguments
    /// * `caller` — The provider's address
    /// * `service_type` — Category of service (maps to DRC-102 capability types)
    /// * `price` — USDC price per service execution (6 decimals)
    /// * `description` — Human-readable service description
    ///
    /// # Returns
    /// The new listing's ID.
    pub fn list_service(
        &mut self,
        caller: [u8; 32],
        service_type: String,
        price: u64,
        description: String,
    ) -> ListingId {
        assert!(price > 0, "Marketplace: price must be positive");
        assert!(
            !service_type.is_empty(),
            "Marketplace: service_type cannot be empty"
        );
        assert!(
            !description.is_empty(),
            "Marketplace: description cannot be empty"
        );

        let id = self.next_listing_id;
        self.next_listing_id += 1;

        // Update the type index for service discovery
        self.type_index
            .entry(service_type.clone())
            .or_default()
            .push(id);

        let listing = ServiceListing {
            provider: caller,
            service_type,
            price,
            description,
            available: true,
        };

        self.listings.insert(id, listing);
        id
    }

    /// Hire a service. The caller becomes the client and USDC is locked.
    ///
    /// The `usdc_attached` parameter represents USDC sent with this transaction.
    /// It must be at least equal to the listing's price.
    ///
    /// # Arguments
    /// * `caller` — The client's address (who is hiring)
    /// * `listing_id` — Which service to hire
    /// * `usdc_attached` — USDC attached to this transaction
    ///
    /// # Returns
    /// The new hire's ID.
    pub fn hire(&mut self, caller: [u8; 32], listing_id: ListingId, usdc_attached: u64) -> HireId {
        let listing = self
            .listings
            .get(&listing_id)
            .expect("Marketplace: listing not found");

        assert!(listing.available, "Marketplace: listing is not available");
        assert!(
            caller != listing.provider,
            "Marketplace: cannot hire yourself"
        );
        assert!(
            usdc_attached >= listing.price,
            "Marketplace: insufficient USDC (need {}, got {})",
            listing.price,
            usdc_attached
        );

        let hire_id = self.next_hire_id;
        self.next_hire_id += 1;

        let hire = Hire {
            listing_id,
            client: caller,
            provider: listing.provider,
            amount: listing.price,
            status: HireStatus::Active,
        };

        self.hires.insert(hire_id, hire);
        hire_id
    }

    /// Provider completes the service and receives USDC payment.
    ///
    /// Only the provider of the hired service can call this. Upon completion,
    /// the escrowed USDC is released to the provider.
    ///
    /// # Arguments
    /// * `caller` — Must be the hire's provider
    /// * `hire_id` — The hire to complete
    ///
    /// # Returns
    /// A tuple of (amount released, provider address) for the VM to transfer USDC.
    pub fn complete_service(&mut self, caller: [u8; 32], hire_id: HireId) -> (u64, [u8; 32]) {
        let hire = self
            .hires
            .get_mut(&hire_id)
            .expect("Marketplace: hire not found");

        assert!(
            caller == hire.provider,
            "Marketplace: only the provider can complete the service"
        );
        assert!(
            hire.status == HireStatus::Active,
            "Marketplace: hire is not active"
        );

        hire.status = HireStatus::Completed;
        (hire.amount, hire.provider)
    }

    /// Client cancels the hire and gets a USDC refund.
    ///
    /// Only possible while the hire is still Active (before the provider
    /// completes). This protects clients from non-responsive providers.
    ///
    /// # Arguments
    /// * `caller` — Must be the hire's client
    /// * `hire_id` — The hire to cancel
    ///
    /// # Returns
    /// The refunded USDC amount.
    pub fn cancel_hire(&mut self, caller: [u8; 32], hire_id: HireId) -> u64 {
        let hire = self
            .hires
            .get_mut(&hire_id)
            .expect("Marketplace: hire not found");

        assert!(
            caller == hire.client,
            "Marketplace: only the client can cancel the hire"
        );
        assert!(
            hire.status == HireStatus::Active,
            "Marketplace: hire is not active"
        );

        hire.status = HireStatus::Cancelled;
        hire.amount
    }

    /// Toggle a listing's availability.
    ///
    /// Providers use this to go "offline" without deleting their listing.
    /// When unavailable, no new hires can be created for this listing.
    pub fn set_available(&mut self, caller: [u8; 32], listing_id: ListingId, available: bool) {
        let listing = self
            .listings
            .get_mut(&listing_id)
            .expect("Marketplace: listing not found");

        assert!(
            caller == listing.provider,
            "Marketplace: only the provider can change availability"
        );

        listing.available = available;
    }

    /// Find all listings for a given service type.
    ///
    /// Uses the reverse index for O(1) lookup by type. This is how agents
    /// discover services — they query by capability type (e.g. "pick-and-place")
    /// and get back all providers offering that service.
    pub fn find_by_type(&self, service_type: &str) -> Vec<(ListingId, &ServiceListing)> {
        let listing_ids = match self.type_index.get(service_type) {
            Some(ids) => ids,
            None => return Vec::new(),
        };

        listing_ids
            .iter()
            .filter_map(|id| {
                let listing = self.listings.get(id)?;
                Some((*id, listing))
            })
            .collect()
    }

    /// Get a specific listing by ID.
    pub fn get_listing(&self, listing_id: ListingId) -> Option<&ServiceListing> {
        self.listings.get(&listing_id)
    }

    /// Get a specific hire by ID.
    pub fn get_hire(&self, hire_id: HireId) -> Option<&Hire> {
        self.hires.get(&hire_id)
    }
}

// =============================================================================
// Dispatch Argument Types
// =============================================================================

#[derive(Serialize, Deserialize, Debug)]
struct ListServiceArgs {
    service_type: String,
    price: u64,
    description: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct HireArgs {
    listing_id: ListingId,
    usdc_attached: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct HireIdArgs {
    hire_id: HireId,
}

#[derive(Serialize, Deserialize, Debug)]
struct ListingIdArgs {
    listing_id: ListingId,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetAvailableArgs {
    listing_id: ListingId,
    available: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct FindByTypeArgs {
    service_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CompleteResult {
    amount: u64,
    provider: [u8; 32],
}

/// Serializable listing entry for search results.
#[derive(Serialize, Deserialize, Debug)]
struct ListingEntry {
    id: ListingId,
    listing: ServiceListing,
}

// =============================================================================
// Dispatch Function
// =============================================================================

pub fn dispatch(
    state: &mut Option<MarketplaceState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "Marketplace: already initialised");
            *state = Some(MarketplaceState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "list_service" => {
            let s = state.as_mut().expect("Marketplace: not initialised");
            let a: ListServiceArgs =
                serde_json::from_slice(args).expect("Marketplace: bad list_service args");
            let id = s.list_service(caller, a.service_type, a.price, a.description);
            serde_json::to_vec(&id).unwrap()
        }

        "hire" => {
            let s = state.as_mut().expect("Marketplace: not initialised");
            let a: HireArgs = serde_json::from_slice(args).expect("Marketplace: bad hire args");
            let id = s.hire(caller, a.listing_id, a.usdc_attached);
            serde_json::to_vec(&id).unwrap()
        }

        "complete_service" => {
            let s = state.as_mut().expect("Marketplace: not initialised");
            let a: HireIdArgs =
                serde_json::from_slice(args).expect("Marketplace: bad complete_service args");
            let (amount, provider) = s.complete_service(caller, a.hire_id);
            serde_json::to_vec(&CompleteResult { amount, provider }).unwrap()
        }

        "cancel_hire" => {
            let s = state.as_mut().expect("Marketplace: not initialised");
            let a: HireIdArgs =
                serde_json::from_slice(args).expect("Marketplace: bad cancel_hire args");
            let refund = s.cancel_hire(caller, a.hire_id);
            serde_json::to_vec(&refund).unwrap()
        }

        "set_available" => {
            let s = state.as_mut().expect("Marketplace: not initialised");
            let a: SetAvailableArgs =
                serde_json::from_slice(args).expect("Marketplace: bad set_available args");
            s.set_available(caller, a.listing_id, a.available);
            serde_json::to_vec("ok").unwrap()
        }

        "find_by_type" => {
            let s = state.as_ref().expect("Marketplace: not initialised");
            let a: FindByTypeArgs =
                serde_json::from_slice(args).expect("Marketplace: bad find_by_type args");
            let results: Vec<ListingEntry> = s
                .find_by_type(&a.service_type)
                .into_iter()
                .map(|(id, listing)| ListingEntry {
                    id,
                    listing: listing.clone(),
                })
                .collect();
            serde_json::to_vec(&results).unwrap()
        }

        "get_listing" => {
            let s = state.as_ref().expect("Marketplace: not initialised");
            let a: ListingIdArgs =
                serde_json::from_slice(args).expect("Marketplace: bad get_listing args");
            serde_json::to_vec(&s.get_listing(a.listing_id)).unwrap()
        }

        "get_hire" => {
            let s = state.as_ref().expect("Marketplace: not initialised");
            let a: HireIdArgs =
                serde_json::from_slice(args).expect("Marketplace: bad get_hire args");
            serde_json::to_vec(&s.get_hire(a.hire_id)).unwrap()
        }

        _ => panic!("Marketplace: unknown method '{method}'"),
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
    fn test_list_hire_complete() {
        let mut state: Option<MarketplaceState> = None;
        let robot = addr(1); // provider
        let agent = addr(2); // client

        dispatch(&mut state, "init", b"{}", robot);

        // Robot lists a pick-and-place service for 2 USDC
        let list_args = serde_json::to_vec(&ListServiceArgs {
            service_type: "pick-and-place".to_string(),
            price: 2_000_000,
            description: "6-axis robot arm pick and place".to_string(),
        })
        .unwrap();
        let result = dispatch(&mut state, "list_service", &list_args, robot);
        let listing_id: ListingId = serde_json::from_slice(&result).unwrap();
        assert_eq!(listing_id, 1);

        // Agent discovers the service
        let find_args = serde_json::to_vec(&FindByTypeArgs {
            service_type: "pick-and-place".to_string(),
        })
        .unwrap();
        let result = dispatch(&mut state, "find_by_type", &find_args, agent);
        let entries: Vec<ListingEntry> = serde_json::from_slice(&result).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].listing.price, 2_000_000);

        // Agent hires the robot
        let hire_args = serde_json::to_vec(&HireArgs {
            listing_id: 1,
            usdc_attached: 2_000_000,
        })
        .unwrap();
        let result = dispatch(&mut state, "hire", &hire_args, agent);
        let hire_id: HireId = serde_json::from_slice(&result).unwrap();
        assert_eq!(hire_id, 1);

        // Robot completes the service
        let complete_args = serde_json::to_vec(&HireIdArgs { hire_id: 1 }).unwrap();
        let result = dispatch(&mut state, "complete_service", &complete_args, robot);
        let complete: CompleteResult = serde_json::from_slice(&result).unwrap();
        assert_eq!(complete.amount, 2_000_000);
        assert_eq!(complete.provider, robot);

        // Verify hire status
        let get_args = serde_json::to_vec(&HireIdArgs { hire_id: 1 }).unwrap();
        let result = dispatch(&mut state, "get_hire", &get_args, agent);
        let hire: Option<Hire> = serde_json::from_slice(&result).unwrap();
        assert_eq!(hire.unwrap().status, HireStatus::Completed);
    }

    #[test]
    fn test_cancel_hire() {
        let mut state: Option<MarketplaceState> = None;
        let robot = addr(1);
        let agent = addr(2);

        dispatch(&mut state, "init", b"{}", robot);

        let list_args = serde_json::to_vec(&ListServiceArgs {
            service_type: "delivery".to_string(),
            price: 1_000_000,
            description: "Package delivery".to_string(),
        })
        .unwrap();
        dispatch(&mut state, "list_service", &list_args, robot);

        let hire_args = serde_json::to_vec(&HireArgs {
            listing_id: 1,
            usdc_attached: 1_000_000,
        })
        .unwrap();
        dispatch(&mut state, "hire", &hire_args, agent);

        // Agent cancels
        let cancel_args = serde_json::to_vec(&HireIdArgs { hire_id: 1 }).unwrap();
        let result = dispatch(&mut state, "cancel_hire", &cancel_args, agent);
        let refund: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(refund, 1_000_000);
    }

    #[test]
    #[should_panic(expected = "cannot hire yourself")]
    fn test_cannot_hire_self() {
        let mut state: Option<MarketplaceState> = None;
        let robot = addr(1);

        dispatch(&mut state, "init", b"{}", robot);

        let list_args = serde_json::to_vec(&ListServiceArgs {
            service_type: "test".to_string(),
            price: 1_000_000,
            description: "Test service".to_string(),
        })
        .unwrap();
        dispatch(&mut state, "list_service", &list_args, robot);

        // Robot tries to hire itself — should fail
        let hire_args = serde_json::to_vec(&HireArgs {
            listing_id: 1,
            usdc_attached: 1_000_000,
        })
        .unwrap();
        dispatch(&mut state, "hire", &hire_args, robot);
    }

    #[test]
    fn test_toggle_availability() {
        let mut state: Option<MarketplaceState> = None;
        let robot = addr(1);
        let agent = addr(2);

        dispatch(&mut state, "init", b"{}", robot);

        let list_args = serde_json::to_vec(&ListServiceArgs {
            service_type: "test".to_string(),
            price: 1_000_000,
            description: "Test".to_string(),
        })
        .unwrap();
        dispatch(&mut state, "list_service", &list_args, robot);

        // Take offline
        let set_args = serde_json::to_vec(&SetAvailableArgs {
            listing_id: 1,
            available: false,
        })
        .unwrap();
        dispatch(&mut state, "set_available", &set_args, robot);

        // Verify listing is unavailable
        let get_args = serde_json::to_vec(&ListingIdArgs { listing_id: 1 }).unwrap();
        let result = dispatch(&mut state, "get_listing", &get_args, agent);
        let listing: Option<ServiceListing> = serde_json::from_slice(&result).unwrap();
        assert!(!listing.unwrap().available);
    }
}
