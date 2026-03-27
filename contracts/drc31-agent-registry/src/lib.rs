use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-31  Global Agent Registry
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AgentType {
    AI,
    Robot,
    IoT,
    Service,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentProfile {
    pub address: Address,
    pub name: String,
    pub agent_type: AgentType,
    pub capabilities: Vec<String>,
    pub owner: Address,
    pub created_at: u64,
    pub reputation_score: u64,
    pub active: bool,
    pub location: Option<(i64, i64)>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegistryState {
    pub admin: Address,
    pub agents: BTreeMap<Address, AgentProfile>,
    pub next_id: u64,
}

impl RegistryState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            agents: BTreeMap::new(),
            next_id: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_agent(
        &mut self,
        caller: Address,
        agent_address: Address,
        name: String,
        agent_type: AgentType,
        capabilities: Vec<String>,
        created_at: u64,
        location: Option<(i64, i64)>,
    ) {
        assert!(
            !self.agents.contains_key(&agent_address),
            "DRC31: agent already registered"
        );
        let profile = AgentProfile {
            address: agent_address,
            name,
            agent_type,
            capabilities,
            owner: caller,
            created_at,
            reputation_score: 0,
            active: true,
            location,
        };
        self.agents.insert(agent_address, profile);
        self.next_id += 1;
    }

    pub fn update_profile(
        &mut self,
        caller: Address,
        agent_address: Address,
        name: Option<String>,
        capabilities: Option<Vec<String>>,
        location: Option<(i64, i64)>,
    ) {
        let profile = self
            .agents
            .get_mut(&agent_address)
            .expect("DRC31: agent not found");
        assert!(
            profile.owner == caller,
            "DRC31: only owner can update profile"
        );
        if let Some(n) = name {
            profile.name = n;
        }
        if let Some(c) = capabilities {
            profile.capabilities = c;
        }
        if let Some(loc) = location {
            profile.location = Some(loc);
        }
    }

    pub fn deactivate(&mut self, caller: Address, agent_address: Address) {
        let profile = self
            .agents
            .get_mut(&agent_address)
            .expect("DRC31: agent not found");
        assert!(
            profile.owner == caller || caller == self.admin,
            "DRC31: not authorized"
        );
        profile.active = false;
    }

    pub fn search_by_type(&self, agent_type: &AgentType) -> Vec<&AgentProfile> {
        self.agents
            .values()
            .filter(|a| a.active && a.agent_type == *agent_type)
            .collect()
    }

    pub fn search_by_capability(&self, capability: &str) -> Vec<&AgentProfile> {
        self.agents
            .values()
            .filter(|a| a.active && a.capabilities.iter().any(|c| c == capability))
            .collect()
    }

    pub fn agents_near_location(&self, lat: i64, lon: i64, radius: i64) -> Vec<&AgentProfile> {
        self.agents
            .values()
            .filter(|a| {
                a.active
                    && a.location.is_some_and(|(alat, alon)| {
                        let dlat = alat - lat;
                        let dlon = alon - lon;
                        dlat * dlat + dlon * dlon <= radius * radius
                    })
            })
            .collect()
    }

    pub fn total_agents(&self) -> usize {
        self.agents.values().filter(|a| a.active).count()
    }

    pub fn get_agent(&self, address: &Address) -> Option<&AgentProfile> {
        self.agents.get(address)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct RegisterAgentArgs {
    agent_address: Address,
    name: String,
    agent_type: AgentType,
    capabilities: Vec<String>,
    created_at: u64,
    location: Option<(i64, i64)>,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateProfileArgs {
    agent_address: Address,
    name: Option<String>,
    capabilities: Option<Vec<String>>,
    location: Option<(i64, i64)>,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeactivateArgs {
    agent_address: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct SearchByTypeArgs {
    agent_type: AgentType,
}

#[derive(Serialize, Deserialize, Debug)]
struct SearchByCapabilityArgs {
    capability: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct NearLocationArgs {
    lat: i64,
    lon: i64,
    radius: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetAgentArgs {
    address: Address,
}

pub fn dispatch(
    state: &mut Option<RegistryState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC31: already initialised");
            *state = Some(RegistryState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "register_agent" => {
            let s = state.as_mut().expect("DRC31: not initialised");
            let a: RegisterAgentArgs =
                serde_json::from_slice(args).expect("DRC31: bad register_agent args");
            s.register_agent(
                caller,
                a.agent_address,
                a.name,
                a.agent_type,
                a.capabilities,
                a.created_at,
                a.location,
            );
            serde_json::to_vec("ok").unwrap()
        }
        "update_profile" => {
            let s = state.as_mut().expect("DRC31: not initialised");
            let a: UpdateProfileArgs =
                serde_json::from_slice(args).expect("DRC31: bad update_profile args");
            s.update_profile(caller, a.agent_address, a.name, a.capabilities, a.location);
            serde_json::to_vec("ok").unwrap()
        }
        "deactivate" => {
            let s = state.as_mut().expect("DRC31: not initialised");
            let a: DeactivateArgs =
                serde_json::from_slice(args).expect("DRC31: bad deactivate args");
            s.deactivate(caller, a.agent_address);
            serde_json::to_vec("ok").unwrap()
        }
        "search_by_type" => {
            let s = state.as_ref().expect("DRC31: not initialised");
            let a: SearchByTypeArgs =
                serde_json::from_slice(args).expect("DRC31: bad search_by_type args");
            let results = s.search_by_type(&a.agent_type);
            serde_json::to_vec(&results).unwrap()
        }
        "search_by_capability" => {
            let s = state.as_ref().expect("DRC31: not initialised");
            let a: SearchByCapabilityArgs =
                serde_json::from_slice(args).expect("DRC31: bad search_by_capability args");
            let results = s.search_by_capability(&a.capability);
            serde_json::to_vec(&results).unwrap()
        }
        "agents_near_location" => {
            let s = state.as_ref().expect("DRC31: not initialised");
            let a: NearLocationArgs =
                serde_json::from_slice(args).expect("DRC31: bad agents_near_location args");
            let results = s.agents_near_location(a.lat, a.lon, a.radius);
            serde_json::to_vec(&results).unwrap()
        }
        "total_agents" => {
            let s = state.as_ref().expect("DRC31: not initialised");
            serde_json::to_vec(&s.total_agents()).unwrap()
        }
        "get_agent" => {
            let s = state.as_ref().expect("DRC31: not initialised");
            let a: GetAgentArgs =
                serde_json::from_slice(args).expect("DRC31: bad get_agent args");
            serde_json::to_vec(&s.get_agent(&a.address)).unwrap()
        }
        _ => panic!("DRC31: unknown method '{method}'"),
    }
}
