use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-82  Advanced Agent Marketplace V2
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum HireStatus {
    Active,
    Completed,
    Cancelled,
    Disputed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentListing {
    pub id: u64,
    pub agent: Address,
    pub service_type: String,
    pub capabilities: Vec<String>,
    pub price_per_task: u64,
    pub price_per_hour: u64,
    pub availability_hours: Vec<(u64, u64)>, // (start, end) time windows
    pub rating_avg: u32,                     // bps, 0-10000 => 0-5 stars * 2000
    pub total_completed: u64,
    pub total_reviews: u64,
    pub device_id: Option<Address>,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Hire {
    pub id: u64,
    pub client: Address,
    pub agent: Address,
    pub listing_id: u64,
    pub budget: u64,
    pub tasks_completed: u64,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub status: HireStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Review {
    pub hire_id: u64,
    pub reviewer: Address,
    pub rating: u8, // 1-5
    pub comment_hash: [u8; 32],
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentProfile {
    pub listing: AgentListing,
    pub active_hires: usize,
    pub total_earned: u64,
    pub avg_rating: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentMarketplaceState {
    pub owner: Address,
    pub listings: BTreeMap<u64, AgentListing>,
    pub hires: BTreeMap<u64, Hire>,
    pub reviews: BTreeMap<u64, Vec<Review>>, // hire_id -> reviews
    pub next_listing_id: u64,
    pub next_hire_id: u64,
    pub balances: BTreeMap<Address, u64>,
    pub earnings: BTreeMap<Address, u64>,
}

impl AgentMarketplaceState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            listings: BTreeMap::new(),
            hires: BTreeMap::new(),
            reviews: BTreeMap::new(),
            next_listing_id: 1,
            next_hire_id: 1,
            balances: BTreeMap::new(),
            earnings: BTreeMap::new(),
        }
    }

    pub fn deposit(&mut self, caller: Address, amount: u64) {
        assert!(amount > 0, "DRC82: deposit must be positive");
        *self.balances.entry(caller).or_insert(0) += amount;
    }

    pub fn list_service(
        &mut self,
        caller: Address,
        service_type: String,
        capabilities: Vec<String>,
        price_per_task: u64,
        price_per_hour: u64,
        availability_hours: Vec<(u64, u64)>,
        device_id: Option<Address>,
    ) -> u64 {
        assert!(!service_type.is_empty(), "DRC82: service type required");
        assert!(
            !capabilities.is_empty(),
            "DRC82: at least one capability required"
        );
        assert!(
            price_per_task > 0 || price_per_hour > 0,
            "DRC82: must set a price"
        );

        let id = self.next_listing_id;
        self.next_listing_id += 1;
        self.listings.insert(
            id,
            AgentListing {
                id,
                agent: caller,
                service_type,
                capabilities,
                price_per_task,
                price_per_hour,
                availability_hours,
                rating_avg: 0,
                total_completed: 0,
                total_reviews: 0,
                device_id,
                active: true,
            },
        );
        id
    }

    pub fn update_listing(
        &mut self,
        caller: Address,
        listing_id: u64,
        price_per_task: Option<u64>,
        price_per_hour: Option<u64>,
        active: Option<bool>,
    ) {
        let listing = self
            .listings
            .get_mut(&listing_id)
            .expect("DRC82: listing not found");
        assert!(
            listing.agent == caller,
            "DRC82: only agent can update listing"
        );
        if let Some(p) = price_per_task {
            listing.price_per_task = p;
        }
        if let Some(p) = price_per_hour {
            listing.price_per_hour = p;
        }
        if let Some(a) = active {
            listing.active = a;
        }
    }

    pub fn hire_agent(
        &mut self,
        caller: Address,
        listing_id: u64,
        budget: u64,
        start_time: u64,
    ) -> u64 {
        let listing = self
            .listings
            .get(&listing_id)
            .expect("DRC82: listing not found");
        assert!(listing.active, "DRC82: listing not active");
        assert!(caller != listing.agent, "DRC82: cannot hire yourself");

        let bal = self.balances.get(&caller).copied().unwrap_or(0);
        assert!(bal >= budget, "DRC82: insufficient balance");
        self.balances.insert(caller, bal - budget);

        let agent = listing.agent;
        let id = self.next_hire_id;
        self.next_hire_id += 1;
        self.hires.insert(
            id,
            Hire {
                id,
                client: caller,
                agent,
                listing_id,
                budget,
                tasks_completed: 0,
                start_time,
                end_time: None,
                status: HireStatus::Active,
            },
        );
        id
    }

    pub fn complete_task_in_hire(&mut self, caller: Address, hire_id: u64) {
        let hire = self.hires.get_mut(&hire_id).expect("DRC82: hire not found");
        assert!(hire.status == HireStatus::Active, "DRC82: hire not active");
        assert!(
            hire.agent == caller,
            "DRC82: only hired agent can complete tasks"
        );

        let listing = self
            .listings
            .get(&hire.listing_id)
            .expect("DRC82: listing not found");
        let task_cost = listing.price_per_task;
        let remaining = hire.budget - (hire.tasks_completed * task_cost);
        assert!(
            remaining >= task_cost,
            "DRC82: insufficient budget for task"
        );

        hire.tasks_completed += 1;
        // Pay the agent immediately per task
        *self.balances.entry(caller).or_insert(0) += task_cost;
        *self.earnings.entry(caller).or_insert(0) += task_cost;
    }

    pub fn end_hire(&mut self, caller: Address, hire_id: u64, end_time: u64) {
        let hire = self.hires.get_mut(&hire_id).expect("DRC82: hire not found");
        assert!(hire.status == HireStatus::Active, "DRC82: hire not active");
        assert!(
            hire.client == caller || hire.agent == caller,
            "DRC82: not authorized"
        );

        hire.status = HireStatus::Completed;
        hire.end_time = Some(end_time);

        // Refund unused budget to client
        let listing = self.listings.get(&hire.listing_id).unwrap();
        let spent = hire.tasks_completed * listing.price_per_task;
        let refund = hire.budget.saturating_sub(spent);
        if refund > 0 {
            *self.balances.entry(hire.client).or_insert(0) += refund;
        }

        // Update listing stats
        let listing_id = hire.listing_id;
        let listing = self.listings.get_mut(&listing_id).unwrap();
        listing.total_completed += hire.tasks_completed;
    }

    pub fn leave_review(
        &mut self,
        caller: Address,
        hire_id: u64,
        rating: u8,
        comment_hash: [u8; 32],
        timestamp: u64,
    ) {
        assert!(rating >= 1 && rating <= 5, "DRC82: rating must be 1-5");
        let hire = self.hires.get(&hire_id).expect("DRC82: hire not found");
        assert!(
            hire.status == HireStatus::Completed,
            "DRC82: hire not completed"
        );
        assert!(hire.client == caller, "DRC82: only client can review");

        let reviews = self.reviews.entry(hire_id).or_default();
        assert!(reviews.is_empty(), "DRC82: already reviewed");

        reviews.push(Review {
            hire_id,
            reviewer: caller,
            rating,
            comment_hash,
            timestamp,
        });

        // Update agent listing rating (rolling average in bps)
        let listing = self.listings.get_mut(&hire.listing_id).unwrap();
        let old_total = listing.rating_avg as u64 * listing.total_reviews;
        listing.total_reviews += 1;
        let rating_bps = rating as u64 * 2000; // 1-5 star => 2000-10000 bps
        listing.rating_avg = ((old_total + rating_bps) / listing.total_reviews) as u32;
    }

    pub fn search_agents(
        &self,
        capability: &str,
        max_price: Option<u64>,
        min_rating: Option<u32>,
    ) -> Vec<&AgentListing> {
        self.listings
            .values()
            .filter(|l| {
                l.active
                    && l.capabilities.iter().any(|c| c == capability)
                    && max_price.map_or(true, |mp| l.price_per_task <= mp)
                    && min_rating.map_or(true, |mr| l.rating_avg >= mr)
            })
            .collect()
    }

    pub fn agent_profile(&self, agent: &Address) -> Option<AgentProfile> {
        let listing = self.listings.values().find(|l| &l.agent == agent)?;
        let active_hires = self
            .hires
            .values()
            .filter(|h| &h.agent == agent && h.status == HireStatus::Active)
            .count();
        let total_earned = self.earnings.get(agent).copied().unwrap_or(0);
        let avg_rating = if listing.total_reviews > 0 {
            listing.rating_avg as f64 / 2000.0
        } else {
            0.0
        };

        Some(AgentProfile {
            listing: listing.clone(),
            active_hires,
            total_earned,
            avg_rating,
        })
    }

    pub fn top_agents(&self, limit: usize) -> Vec<&AgentListing> {
        let mut listings: Vec<&AgentListing> = self
            .listings
            .values()
            .filter(|l| l.active && l.total_reviews > 0)
            .collect();
        listings.sort_by(|a, b| {
            b.rating_avg
                .cmp(&a.rating_avg)
                .then(b.total_completed.cmp(&a.total_completed))
        });
        listings.truncate(limit);
        listings
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct ListServiceArgs {
    service_type: String,
    capabilities: Vec<String>,
    price_per_task: u64,
    price_per_hour: u64,
    availability_hours: Vec<(u64, u64)>,
    device_id: Option<Address>,
}
#[derive(Serialize, Deserialize, Debug)]
struct UpdateListingArgs {
    listing_id: u64,
    price_per_task: Option<u64>,
    price_per_hour: Option<u64>,
    active: Option<bool>,
}
#[derive(Serialize, Deserialize, Debug)]
struct HireArgs {
    listing_id: u64,
    budget: u64,
    start_time: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct HireIdArgs {
    hire_id: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct EndHireArgs {
    hire_id: u64,
    end_time: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct ReviewArgs {
    hire_id: u64,
    rating: u8,
    comment_hash: [u8; 32],
    timestamp: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct SearchArgs {
    capability: String,
    max_price: Option<u64>,
    min_rating: Option<u32>,
}
#[derive(Serialize, Deserialize, Debug)]
struct AgentArgs {
    agent: Address,
}
#[derive(Serialize, Deserialize, Debug)]
struct TopArgs {
    limit: usize,
}
#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs {
    amount: u64,
}

pub fn dispatch(
    state: &mut Option<AgentMarketplaceState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC82: already initialised");
            *state = Some(AgentMarketplaceState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "deposit" => {
            let s = state.as_mut().expect("DRC82: not initialised");
            let a: DepositArgs = serde_json::from_slice(args).expect("DRC82: bad args");
            s.deposit(caller, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "list_service" => {
            let s = state.as_mut().expect("DRC82: not initialised");
            let a: ListServiceArgs = serde_json::from_slice(args).expect("DRC82: bad args");
            let id = s.list_service(
                caller,
                a.service_type,
                a.capabilities,
                a.price_per_task,
                a.price_per_hour,
                a.availability_hours,
                a.device_id,
            );
            serde_json::to_vec(&id).unwrap()
        }
        "update_listing" => {
            let s = state.as_mut().expect("DRC82: not initialised");
            let a: UpdateListingArgs = serde_json::from_slice(args).expect("DRC82: bad args");
            s.update_listing(
                caller,
                a.listing_id,
                a.price_per_task,
                a.price_per_hour,
                a.active,
            );
            serde_json::to_vec("ok").unwrap()
        }
        "hire_agent" => {
            let s = state.as_mut().expect("DRC82: not initialised");
            let a: HireArgs = serde_json::from_slice(args).expect("DRC82: bad args");
            let id = s.hire_agent(caller, a.listing_id, a.budget, a.start_time);
            serde_json::to_vec(&id).unwrap()
        }
        "complete_task_in_hire" => {
            let s = state.as_mut().expect("DRC82: not initialised");
            let a: HireIdArgs = serde_json::from_slice(args).expect("DRC82: bad args");
            s.complete_task_in_hire(caller, a.hire_id);
            serde_json::to_vec("ok").unwrap()
        }
        "end_hire" => {
            let s = state.as_mut().expect("DRC82: not initialised");
            let a: EndHireArgs = serde_json::from_slice(args).expect("DRC82: bad args");
            s.end_hire(caller, a.hire_id, a.end_time);
            serde_json::to_vec("ok").unwrap()
        }
        "leave_review" => {
            let s = state.as_mut().expect("DRC82: not initialised");
            let a: ReviewArgs = serde_json::from_slice(args).expect("DRC82: bad args");
            s.leave_review(caller, a.hire_id, a.rating, a.comment_hash, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }
        "search_agents" => {
            let s = state.as_ref().expect("DRC82: not initialised");
            let a: SearchArgs = serde_json::from_slice(args).expect("DRC82: bad args");
            serde_json::to_vec(&s.search_agents(&a.capability, a.max_price, a.min_rating)).unwrap()
        }
        "agent_profile" => {
            let s = state.as_ref().expect("DRC82: not initialised");
            let a: AgentArgs = serde_json::from_slice(args).expect("DRC82: bad args");
            serde_json::to_vec(&s.agent_profile(&a.agent)).unwrap()
        }
        "top_agents" => {
            let s = state.as_ref().expect("DRC82: not initialised");
            let a: TopArgs = serde_json::from_slice(args).expect("DRC82: bad args");
            serde_json::to_vec(&s.top_agents(a.limit)).unwrap()
        }
        _ => panic!("DRC82: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [0u8; 32];
    const AGENT_A: Address = [1u8; 32];
    const AGENT_B: Address = [2u8; 32];
    const CLIENT: Address = [3u8; 32];

    fn setup() -> AgentMarketplaceState {
        let mut s = AgentMarketplaceState::new(OWNER);
        s.deposit(CLIENT, 100_000);
        s.list_service(
            AGENT_A,
            "ml-inference".into(),
            vec!["text-generation".into(), "summarization".into()],
            100,
            500,
            vec![(0, 86400)],
            None,
        );
        s.list_service(
            AGENT_B,
            "data-processing".into(),
            vec!["text-generation".into(), "translation".into()],
            80,
            400,
            vec![(0, 86400)],
            Some([0xDD; 32]),
        );
        s
    }

    #[test]
    fn test_list_and_search() {
        let s = setup();
        let results = s.search_agents("text-generation", None, None);
        assert_eq!(results.len(), 2);
        let results = s.search_agents("translation", None, None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].agent, AGENT_B);
    }

    #[test]
    fn test_search_with_max_price() {
        let s = setup();
        let results = s.search_agents("text-generation", Some(90), None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].agent, AGENT_B); // 80 <= 90
    }

    #[test]
    fn test_hire_complete_review() {
        let mut s = setup();
        let hire_id = s.hire_agent(CLIENT, 1, 500, 1000);
        assert_eq!(s.balances.get(&CLIENT).copied().unwrap(), 99_500);

        // Complete 3 tasks
        s.complete_task_in_hire(AGENT_A, hire_id);
        s.complete_task_in_hire(AGENT_A, hire_id);
        s.complete_task_in_hire(AGENT_A, hire_id);
        assert_eq!(s.balances.get(&AGENT_A).copied().unwrap(), 300);

        // End hire
        s.end_hire(CLIENT, hire_id, 2000);
        assert_eq!(s.hires.get(&hire_id).unwrap().status, HireStatus::Completed);
        // Refund: 500 - 300 = 200
        assert_eq!(s.balances.get(&CLIENT).copied().unwrap(), 99_700);

        // Leave review
        s.leave_review(CLIENT, hire_id, 4, [0xEE; 32], 3000);
        let listing = s.listings.get(&1).unwrap();
        assert_eq!(listing.total_reviews, 1);
        assert_eq!(listing.rating_avg, 8000); // 4 stars * 2000 bps
        assert_eq!(listing.total_completed, 3);
    }

    #[test]
    fn test_agent_profile() {
        let mut s = setup();
        let hire_id = s.hire_agent(CLIENT, 1, 500, 1000);
        s.complete_task_in_hire(AGENT_A, hire_id);
        let profile = s.agent_profile(&AGENT_A).unwrap();
        assert_eq!(profile.active_hires, 1);
        assert_eq!(profile.total_earned, 100);
    }

    #[test]
    fn test_top_agents() {
        let mut s = setup();
        // Hire and review agent A
        let h1 = s.hire_agent(CLIENT, 1, 500, 1000);
        s.complete_task_in_hire(AGENT_A, h1);
        s.end_hire(CLIENT, h1, 2000);
        s.leave_review(CLIENT, h1, 5, [0; 32], 3000);

        // Hire and review agent B
        let h2 = s.hire_agent(CLIENT, 2, 500, 1000);
        s.complete_task_in_hire(AGENT_B, h2);
        s.end_hire(CLIENT, h2, 2000);
        s.leave_review(CLIENT, h2, 3, [0; 32], 3001);

        let top = s.top_agents(10);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].agent, AGENT_A); // 5 star > 3 star
    }

    #[test]
    fn test_update_listing() {
        let mut s = setup();
        s.update_listing(AGENT_A, 1, Some(200), None, None);
        assert_eq!(s.listings.get(&1).unwrap().price_per_task, 200);
    }

    #[test]
    #[should_panic(expected = "cannot hire yourself")]
    fn test_cannot_self_hire() {
        let mut s = setup();
        s.deposit(AGENT_A, 1000);
        s.hire_agent(AGENT_A, 1, 500, 1000);
    }
}
