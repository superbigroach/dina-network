use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-104  Swarm Membership
// ---------------------------------------------------------------------------

type Address = [u8; 32];
type SwarmId = [u8; 32];
type DeviceId = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpendingLimits {
    pub max_per_tx: u64,
    pub max_per_day: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SwarmConfig {
    pub name: String,
    pub admin: Address,
    pub quorum: u8,
    pub max_members: u64,
    pub spending_limits: SpendingLimits,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MemberSignature {
    pub member: DeviceId,
    pub signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SwarmAction {
    Transfer { to: Address, amount: u64 },
    UpdateConfig(SwarmConfig),
    AddMember(DeviceId),
    RemoveMember(DeviceId),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SwarmRegistry {
    pub owner: Address,
    pub swarms: BTreeMap<SwarmId, SwarmConfig>,
    pub members: BTreeMap<SwarmId, Vec<DeviceId>>,
    pub swarm_wallets: BTreeMap<SwarmId, u64>,
    pub next_swarm_nonce: u64,
}

impl SwarmRegistry {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            swarms: BTreeMap::new(),
            members: BTreeMap::new(),
            swarm_wallets: BTreeMap::new(),
            next_swarm_nonce: 0,
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn get_swarm(&self, swarm_id: &SwarmId) -> &SwarmConfig {
        self.swarms
            .get(swarm_id)
            .expect("DRC104: swarm does not exist")
    }

    pub fn is_member(&self, swarm_id: &SwarmId, device_id: &DeviceId) -> bool {
        self.members
            .get(swarm_id)
            .map(|m| m.contains(device_id))
            .unwrap_or(false)
    }

    pub fn members(&self, swarm_id: &SwarmId) -> Vec<DeviceId> {
        self.members.get(swarm_id).cloned().unwrap_or_default()
    }

    pub fn swarm_wallet(&self, swarm_id: &SwarmId) -> u64 {
        self.swarm_wallets.get(swarm_id).copied().unwrap_or(0)
    }

    // -- Mutations -----------------------------------------------------------

    pub fn create_swarm(&mut self, caller: Address, config: SwarmConfig) -> SwarmId {
        assert!(config.quorum <= 100, "DRC104: quorum must be <= 100");
        assert!(config.max_members > 0, "DRC104: max_members must be > 0");

        // Derive a deterministic swarm ID from caller + nonce
        let mut id_source = [0u8; 64];
        id_source[..32].copy_from_slice(&caller);
        id_source[32..40].copy_from_slice(&self.next_swarm_nonce.to_le_bytes());
        // Simple hash: XOR-fold (production would use SHA-256)
        let mut swarm_id = [0u8; 32];
        for i in 0..32 {
            swarm_id[i] = id_source[i] ^ id_source[i + 32];
        }
        self.next_swarm_nonce += 1;

        self.swarms.insert(swarm_id, config);
        self.members.insert(swarm_id, Vec::new());
        self.swarm_wallets.insert(swarm_id, 0);
        swarm_id
    }

    pub fn add_member(&mut self, caller: Address, swarm_id: SwarmId, device_id: DeviceId) {
        let config = self
            .swarms
            .get(&swarm_id)
            .expect("DRC104: swarm does not exist");
        assert!(
            caller == config.admin,
            "DRC104: only swarm admin can add members"
        );
        let members = self.members.entry(swarm_id).or_insert_with(Vec::new);
        assert!(
            (members.len() as u64) < config.max_members,
            "DRC104: swarm is full"
        );
        assert!(
            !members.contains(&device_id),
            "DRC104: device already a member"
        );
        members.push(device_id);
    }

    pub fn remove_member(&mut self, caller: Address, swarm_id: SwarmId, device_id: DeviceId) {
        let config = self
            .swarms
            .get(&swarm_id)
            .expect("DRC104: swarm does not exist");
        assert!(
            caller == config.admin,
            "DRC104: only swarm admin can remove members"
        );
        let members = self
            .members
            .get_mut(&swarm_id)
            .expect("DRC104: swarm does not exist");
        let pos = members
            .iter()
            .position(|m| m == &device_id)
            .expect("DRC104: device is not a member");
        members.remove(pos);
    }

    pub fn swarm_execute(
        &mut self,
        swarm_id: SwarmId,
        action: SwarmAction,
        signatures: Vec<MemberSignature>,
    ) {
        let config = self
            .swarms
            .get(&swarm_id)
            .expect("DRC104: swarm does not exist");
        let members = self.members.get(&swarm_id).expect("DRC104: swarm has no members list");

        // Verify quorum: count how many signers are actual members
        let valid_signers = signatures
            .iter()
            .filter(|sig| members.contains(&sig.member))
            .count();

        let total = members.len();
        assert!(total > 0, "DRC104: swarm has no members");

        let required = ((total as u64) * (config.quorum as u64) + 99) / 100;
        assert!(
            valid_signers as u64 >= required,
            "DRC104: quorum not met ({valid_signers}/{required} required)"
        );

        match action {
            SwarmAction::Transfer { to: _, amount } => {
                let limits = &config.spending_limits;
                assert!(
                    amount <= limits.max_per_tx,
                    "DRC104: exceeds per-tx spending limit"
                );
                let wallet = self.swarm_wallets.get_mut(&swarm_id).unwrap();
                assert!(*wallet >= amount, "DRC104: insufficient swarm balance");
                *wallet -= amount;
                // In production the transfer would credit the `to` address via
                // a cross-contract call; here we just debit the swarm wallet.
            }
            SwarmAction::UpdateConfig(new_config) => {
                assert!(
                    new_config.quorum <= 100,
                    "DRC104: quorum must be <= 100"
                );
                self.swarms.insert(swarm_id, new_config);
            }
            SwarmAction::AddMember(device_id) => {
                let cfg = self.swarms.get(&swarm_id).unwrap();
                let members = self.members.get_mut(&swarm_id).unwrap();
                assert!(
                    (members.len() as u64) < cfg.max_members,
                    "DRC104: swarm is full"
                );
                assert!(
                    !members.contains(&device_id),
                    "DRC104: device already a member"
                );
                members.push(device_id);
            }
            SwarmAction::RemoveMember(device_id) => {
                let members = self.members.get_mut(&swarm_id).unwrap();
                let pos = members
                    .iter()
                    .position(|m| m == &device_id)
                    .expect("DRC104: device is not a member");
                members.remove(pos);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateSwarmArgs {
    config: SwarmConfig,
}

#[derive(Serialize, Deserialize, Debug)]
struct MemberArgs {
    swarm_id: SwarmId,
    device_id: DeviceId,
}

#[derive(Serialize, Deserialize, Debug)]
struct IsMemberArgs {
    swarm_id: SwarmId,
    device_id: DeviceId,
}

#[derive(Serialize, Deserialize, Debug)]
struct SwarmIdArgs {
    swarm_id: SwarmId,
}

#[derive(Serialize, Deserialize, Debug)]
struct SwarmExecuteArgs {
    swarm_id: SwarmId,
    action: SwarmAction,
    signatures: Vec<MemberSignature>,
}

pub fn dispatch(
    state: &mut Option<SwarmRegistry>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC104: already initialised");
            *state = Some(SwarmRegistry::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "get_swarm" => {
            let s = state.as_ref().expect("DRC104: not initialised");
            let a: SwarmIdArgs =
                serde_json::from_slice(args).expect("DRC104: bad get_swarm args");
            serde_json::to_vec(s.get_swarm(&a.swarm_id)).unwrap()
        }
        "is_member" => {
            let s = state.as_ref().expect("DRC104: not initialised");
            let a: IsMemberArgs =
                serde_json::from_slice(args).expect("DRC104: bad is_member args");
            serde_json::to_vec(&s.is_member(&a.swarm_id, &a.device_id)).unwrap()
        }
        "members" => {
            let s = state.as_ref().expect("DRC104: not initialised");
            let a: SwarmIdArgs =
                serde_json::from_slice(args).expect("DRC104: bad members args");
            serde_json::to_vec(&s.members(&a.swarm_id)).unwrap()
        }
        "swarm_wallet" => {
            let s = state.as_ref().expect("DRC104: not initialised");
            let a: SwarmIdArgs =
                serde_json::from_slice(args).expect("DRC104: bad swarm_wallet args");
            serde_json::to_vec(&s.swarm_wallet(&a.swarm_id)).unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "create_swarm" => {
            let s = state.as_mut().expect("DRC104: not initialised");
            let a: CreateSwarmArgs =
                serde_json::from_slice(args).expect("DRC104: bad create_swarm args");
            let id = s.create_swarm(caller, a.config);
            serde_json::to_vec(&id).unwrap()
        }
        "add_member" => {
            let s = state.as_mut().expect("DRC104: not initialised");
            let a: MemberArgs =
                serde_json::from_slice(args).expect("DRC104: bad add_member args");
            s.add_member(caller, a.swarm_id, a.device_id);
            serde_json::to_vec("ok").unwrap()
        }
        "remove_member" => {
            let s = state.as_mut().expect("DRC104: not initialised");
            let a: MemberArgs =
                serde_json::from_slice(args).expect("DRC104: bad remove_member args");
            s.remove_member(caller, a.swarm_id, a.device_id);
            serde_json::to_vec("ok").unwrap()
        }
        "swarm_execute" => {
            let s = state.as_mut().expect("DRC104: not initialised");
            let a: SwarmExecuteArgs =
                serde_json::from_slice(args).expect("DRC104: bad swarm_execute args");
            s.swarm_execute(a.swarm_id, a.action, a.signatures);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC104: unknown method '{method}'"),
    }
}
