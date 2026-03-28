use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-28  Crowdfunding with Refund
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Campaign {
    pub creator: Address,
    pub goal: u64,
    pub raised: u64,
    pub deadline: u64,
    pub contributions: BTreeMap<Address, u64>,
    pub claimed: bool,
    pub refunded: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CrowdfundState {
    pub campaigns: BTreeMap<u64, Campaign>,
    pub next_id: u64,
}

impl Default for CrowdfundState {
    fn default() -> Self {
        Self::new()
    }
}

impl CrowdfundState {
    pub fn new() -> Self {
        Self {
            campaigns: BTreeMap::new(),
            next_id: 0,
        }
    }

    // -- Mutations -----------------------------------------------------------

    pub fn create_campaign(&mut self, caller: Address, goal: u64, deadline: u64) -> u64 {
        assert!(goal > 0, "DRC28: goal must be positive");
        assert!(deadline > 0, "DRC28: deadline must be positive");
        let id = self.next_id;
        self.next_id += 1;
        self.campaigns.insert(
            id,
            Campaign {
                creator: caller,
                goal,
                raised: 0,
                deadline,
                contributions: BTreeMap::new(),
                claimed: false,
                refunded: false,
            },
        );
        id
    }

    pub fn contribute(
        &mut self,
        caller: Address,
        campaign_id: u64,
        amount: u64,
        current_time: u64,
    ) {
        assert!(amount > 0, "DRC28: contribution must be positive");
        let campaign = self
            .campaigns
            .get_mut(&campaign_id)
            .expect("DRC28: campaign not found");
        assert!(
            current_time <= campaign.deadline,
            "DRC28: campaign has ended"
        );
        assert!(!campaign.claimed, "DRC28: campaign already claimed");
        assert!(!campaign.refunded, "DRC28: campaign already refunded");

        let existing = campaign.contributions.get(&caller).copied().unwrap_or(0);
        campaign.contributions.insert(caller, existing + amount);
        campaign.raised += amount;
    }

    pub fn claim(&mut self, caller: Address, campaign_id: u64, current_time: u64) -> u64 {
        let campaign = self
            .campaigns
            .get_mut(&campaign_id)
            .expect("DRC28: campaign not found");
        assert!(caller == campaign.creator, "DRC28: only creator can claim");
        assert!(campaign.raised >= campaign.goal, "DRC28: goal not met");
        assert!(
            current_time > campaign.deadline,
            "DRC28: campaign still active"
        );
        assert!(!campaign.claimed, "DRC28: already claimed");
        assert!(!campaign.refunded, "DRC28: campaign was refunded");

        campaign.claimed = true;
        campaign.raised
    }

    pub fn refund(&mut self, caller: Address, campaign_id: u64, current_time: u64) -> u64 {
        let campaign = self
            .campaigns
            .get_mut(&campaign_id)
            .expect("DRC28: campaign not found");
        assert!(
            current_time > campaign.deadline,
            "DRC28: campaign still active"
        );
        assert!(
            campaign.raised < campaign.goal,
            "DRC28: goal was met, cannot refund"
        );
        assert!(!campaign.claimed, "DRC28: campaign was claimed");

        let contributed = campaign
            .contributions
            .get(&caller)
            .copied()
            .expect("DRC28: caller has no contribution");
        assert!(contributed > 0, "DRC28: nothing to refund");

        campaign.contributions.insert(caller, 0);
        campaign.raised -= contributed;

        // Mark refunded once all funds have been withdrawn
        if campaign.raised == 0 {
            campaign.refunded = true;
        }

        contributed
    }

    // -- Queries -------------------------------------------------------------

    pub fn get_campaign(&self, campaign_id: u64) -> &Campaign {
        self.campaigns
            .get(&campaign_id)
            .expect("DRC28: campaign not found")
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateCampaignArgs {
    goal: u64,
    deadline: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ContributeArgs {
    campaign_id: u64,
    amount: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ClaimArgs {
    campaign_id: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct RefundArgs {
    campaign_id: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct CampaignIdArgs {
    campaign_id: u64,
}

pub fn dispatch(
    state: &mut Option<CrowdfundState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC28: already initialised");
            *state = Some(CrowdfundState::new());
            serde_json::to_vec("ok").unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "create_campaign" => {
            let s = state.as_mut().expect("DRC28: not initialised");
            let a: CreateCampaignArgs =
                serde_json::from_slice(args).expect("DRC28: bad create_campaign args");
            let id = s.create_campaign(caller, a.goal, a.deadline);
            serde_json::to_vec(&id).unwrap()
        }
        "contribute" => {
            let s = state.as_mut().expect("DRC28: not initialised");
            let a: ContributeArgs =
                serde_json::from_slice(args).expect("DRC28: bad contribute args");
            s.contribute(caller, a.campaign_id, a.amount, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }
        "claim" => {
            let s = state.as_mut().expect("DRC28: not initialised");
            let a: ClaimArgs = serde_json::from_slice(args).expect("DRC28: bad claim args");
            let amount = s.claim(caller, a.campaign_id, a.current_time);
            serde_json::to_vec(&amount).unwrap()
        }
        "refund" => {
            let s = state.as_mut().expect("DRC28: not initialised");
            let a: RefundArgs = serde_json::from_slice(args).expect("DRC28: bad refund args");
            let amount = s.refund(caller, a.campaign_id, a.current_time);
            serde_json::to_vec(&amount).unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "get_campaign" => {
            let s = state.as_ref().expect("DRC28: not initialised");
            let a: CampaignIdArgs =
                serde_json::from_slice(args).expect("DRC28: bad get_campaign args");
            let campaign = s.get_campaign(a.campaign_id);
            serde_json::to_vec(campaign).unwrap()
        }

        _ => panic!("DRC28: unknown method '{method}'"),
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

    fn init(state: &mut Option<CrowdfundState>) {
        dispatch(state, "init", b"{}", addr(0));
    }

    #[test]
    fn test_create_and_contribute() {
        let mut state = None;
        init(&mut state);

        let creator = addr(1);
        let result = dispatch(
            &mut state,
            "create_campaign",
            &serde_json::to_vec(&CreateCampaignArgs {
                goal: 1000,
                deadline: 100,
            })
            .unwrap(),
            creator,
        );
        let id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(id, 0);

        // Contribute
        dispatch(
            &mut state,
            "contribute",
            &serde_json::to_vec(&ContributeArgs {
                campaign_id: 0,
                amount: 500,
                current_time: 50,
            })
            .unwrap(),
            addr(2),
        );

        let s = state.as_ref().unwrap();
        let c = s.get_campaign(0);
        assert_eq!(c.raised, 500);
        assert_eq!(c.contributions.get(&addr(2)).copied().unwrap(), 500);
    }

    #[test]
    fn test_successful_claim() {
        let mut state = None;
        init(&mut state);
        let creator = addr(1);

        dispatch(
            &mut state,
            "create_campaign",
            &serde_json::to_vec(&CreateCampaignArgs {
                goal: 500,
                deadline: 100,
            })
            .unwrap(),
            creator,
        );

        dispatch(
            &mut state,
            "contribute",
            &serde_json::to_vec(&ContributeArgs {
                campaign_id: 0,
                amount: 600,
                current_time: 50,
            })
            .unwrap(),
            addr(2),
        );

        let result = dispatch(
            &mut state,
            "claim",
            &serde_json::to_vec(&ClaimArgs {
                campaign_id: 0,
                current_time: 101,
            })
            .unwrap(),
            creator,
        );
        let claimed: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(claimed, 600);
        assert!(state.as_ref().unwrap().get_campaign(0).claimed);
    }

    #[test]
    fn test_refund_on_failure() {
        let mut state = None;
        init(&mut state);
        let creator = addr(1);

        dispatch(
            &mut state,
            "create_campaign",
            &serde_json::to_vec(&CreateCampaignArgs {
                goal: 1000,
                deadline: 100,
            })
            .unwrap(),
            creator,
        );

        dispatch(
            &mut state,
            "contribute",
            &serde_json::to_vec(&ContributeArgs {
                campaign_id: 0,
                amount: 300,
                current_time: 50,
            })
            .unwrap(),
            addr(2),
        );

        // Deadline passed, goal not met -- refund
        let result = dispatch(
            &mut state,
            "refund",
            &serde_json::to_vec(&RefundArgs {
                campaign_id: 0,
                current_time: 101,
            })
            .unwrap(),
            addr(2),
        );
        let refunded: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(refunded, 300);
        assert!(state.as_ref().unwrap().get_campaign(0).refunded);
    }

    #[test]
    #[should_panic(expected = "DRC28: goal not met")]
    fn test_cannot_claim_unmet_goal() {
        let mut state = None;
        init(&mut state);
        let creator = addr(1);

        dispatch(
            &mut state,
            "create_campaign",
            &serde_json::to_vec(&CreateCampaignArgs {
                goal: 1000,
                deadline: 100,
            })
            .unwrap(),
            creator,
        );

        dispatch(
            &mut state,
            "contribute",
            &serde_json::to_vec(&ContributeArgs {
                campaign_id: 0,
                amount: 100,
                current_time: 50,
            })
            .unwrap(),
            addr(2),
        );

        dispatch(
            &mut state,
            "claim",
            &serde_json::to_vec(&ClaimArgs {
                campaign_id: 0,
                current_time: 101,
            })
            .unwrap(),
            creator,
        );
    }

    #[test]
    #[should_panic(expected = "DRC28: campaign still active")]
    fn test_cannot_refund_before_deadline() {
        let mut state = None;
        init(&mut state);

        dispatch(
            &mut state,
            "create_campaign",
            &serde_json::to_vec(&CreateCampaignArgs {
                goal: 1000,
                deadline: 100,
            })
            .unwrap(),
            addr(1),
        );

        dispatch(
            &mut state,
            "contribute",
            &serde_json::to_vec(&ContributeArgs {
                campaign_id: 0,
                amount: 100,
                current_time: 50,
            })
            .unwrap(),
            addr(2),
        );

        dispatch(
            &mut state,
            "refund",
            &serde_json::to_vec(&RefundArgs {
                campaign_id: 0,
                current_time: 50,
            })
            .unwrap(),
            addr(2),
        );
    }

    #[test]
    fn test_multiple_contributors_refund() {
        let mut state = None;
        init(&mut state);

        dispatch(
            &mut state,
            "create_campaign",
            &serde_json::to_vec(&CreateCampaignArgs {
                goal: 1000,
                deadline: 100,
            })
            .unwrap(),
            addr(1),
        );

        // Two contributors
        dispatch(
            &mut state,
            "contribute",
            &serde_json::to_vec(&ContributeArgs {
                campaign_id: 0,
                amount: 200,
                current_time: 50,
            })
            .unwrap(),
            addr(2),
        );
        dispatch(
            &mut state,
            "contribute",
            &serde_json::to_vec(&ContributeArgs {
                campaign_id: 0,
                amount: 150,
                current_time: 60,
            })
            .unwrap(),
            addr(3),
        );

        // Both refund after deadline
        let r1 = dispatch(
            &mut state,
            "refund",
            &serde_json::to_vec(&RefundArgs {
                campaign_id: 0,
                current_time: 101,
            })
            .unwrap(),
            addr(2),
        );
        let r2 = dispatch(
            &mut state,
            "refund",
            &serde_json::to_vec(&RefundArgs {
                campaign_id: 0,
                current_time: 101,
            })
            .unwrap(),
            addr(3),
        );

        let v1: u64 = serde_json::from_slice(&r1).unwrap();
        let v2: u64 = serde_json::from_slice(&r2).unwrap();
        assert_eq!(v1, 200);
        assert_eq!(v2, 150);
        assert!(state.as_ref().unwrap().get_campaign(0).refunded);
    }
}
