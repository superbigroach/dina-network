use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-69  Hierarchical Agent Delegation
// ---------------------------------------------------------------------------
// Delegate authority from one agent to sub-agents with scoped permissions.
// Supports hierarchical sub-delegation chains, budget tracking, and
// time-based expiration.

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Permission {
    Transfer { max_amount: u64 },
    ContractCall { contracts: Vec<Address> },
    DeviceControl { devices: Vec<String> },
    VectorQuery,
    SwarmJoin,
    Custom(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Delegation {
    pub id: u64,
    pub delegator: Address,
    pub delegate: Address,
    pub permissions: Vec<Permission>,
    pub budget: u64,
    pub spent: u64,
    pub expires_at: u64,
    pub revocable: bool,
    pub sub_delegations_allowed: bool,
    pub revoked: bool,
    pub parent_delegation_id: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DelegatedActionKind {
    Transfer { to: Address, amount: u64 },
    ContractCall { contract: Address, method: String },
    DeviceControl { device: String, command: String },
    VectorQuery { index_id: u64 },
    SwarmJoin { swarm_id: u64 },
    Custom { action: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DelegationState {
    pub owner: Address,
    pub delegations: BTreeMap<u64, Delegation>,
    pub next_id: u64,
    // delegate -> list of delegation ids they hold
    pub delegate_index: BTreeMap<String, Vec<u64>>,
}

fn addr_key(a: &Address) -> String {
    a.iter().map(|b| format!("{b:02x}")).collect()
}

impl DelegationState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            delegations: BTreeMap::new(),
            next_id: 1,
            delegate_index: BTreeMap::new(),
        }
    }

    pub fn delegate(
        &mut self,
        caller: Address,
        delegate: Address,
        permissions: Vec<Permission>,
        budget: u64,
        expires_at: u64,
        revocable: bool,
        sub_delegations_allowed: bool,
    ) -> u64 {
        assert!(!permissions.is_empty(), "DRC69: need at least 1 permission");
        assert!(caller != delegate, "DRC69: cannot delegate to self");
        assert!(expires_at > 0, "DRC69: expiration required");

        let id = self.next_id;
        self.next_id += 1;

        self.delegate_index
            .entry(addr_key(&delegate))
            .or_default()
            .push(id);

        self.delegations.insert(
            id,
            Delegation {
                id,
                delegator: caller,
                delegate,
                permissions,
                budget,
                spent: 0,
                expires_at,
                revocable,
                sub_delegations_allowed,
                revoked: false,
                parent_delegation_id: None,
            },
        );
        id
    }

    pub fn sub_delegate(
        &mut self,
        caller: Address,
        parent_delegation_id: u64,
        delegate: Address,
        permissions: Vec<Permission>,
        budget: u64,
        expires_at: u64,
    ) -> u64 {
        let parent = self
            .delegations
            .get(&parent_delegation_id)
            .expect("DRC69: parent delegation not found");
        assert!(caller == parent.delegate, "DRC69: not the delegate");
        assert!(
            parent.sub_delegations_allowed,
            "DRC69: sub-delegation not allowed"
        );
        assert!(!parent.revoked, "DRC69: parent delegation revoked");
        assert!(
            expires_at <= parent.expires_at,
            "DRC69: cannot exceed parent expiry"
        );
        assert!(
            budget <= parent.budget - parent.spent,
            "DRC69: sub-budget exceeds remaining"
        );

        // Verify permissions are a subset
        for p in &permissions {
            assert!(
                parent.permissions.contains(p),
                "DRC69: sub-permission not in parent scope"
            );
        }

        let id = self.next_id;
        self.next_id += 1;

        self.delegate_index
            .entry(addr_key(&delegate))
            .or_default()
            .push(id);

        self.delegations.insert(
            id,
            Delegation {
                id,
                delegator: caller,
                delegate,
                permissions,
                budget,
                spent: 0,
                expires_at,
                revocable: true,
                sub_delegations_allowed: false,
                revoked: false,
                parent_delegation_id: Some(parent_delegation_id),
            },
        );
        id
    }

    pub fn revoke(&mut self, caller: Address, delegation_id: u64) {
        let del = self
            .delegations
            .get_mut(&delegation_id)
            .expect("DRC69: delegation not found");
        assert!(
            caller == del.delegator || caller == self.owner,
            "DRC69: not authorised"
        );
        assert!(del.revocable, "DRC69: delegation is not revocable");
        assert!(!del.revoked, "DRC69: already revoked");
        del.revoked = true;

        // Recursively revoke sub-delegations
        let sub_ids: Vec<u64> = self
            .delegations
            .iter()
            .filter(|(_, d)| d.parent_delegation_id == Some(delegation_id) && !d.revoked)
            .map(|(id, _)| *id)
            .collect();
        for sub_id in sub_ids {
            if let Some(sub) = self.delegations.get_mut(&sub_id) {
                sub.revoked = true;
            }
        }
    }

    pub fn check_permission(
        &self,
        delegation_id: u64,
        action: &DelegatedActionKind,
        current_time: u64,
    ) -> bool {
        let del = match self.delegations.get(&delegation_id) {
            Some(d) => d,
            None => return false,
        };

        if del.revoked || current_time > del.expires_at {
            return false;
        }

        // Check parent chain
        if let Some(parent_id) = del.parent_delegation_id {
            if !self.check_permission(parent_id, action, current_time) {
                return false;
            }
        }

        match action {
            DelegatedActionKind::Transfer { to: _, amount } => {
                del.permissions.iter().any(|p| matches!(p, Permission::Transfer { max_amount } if amount <= max_amount))
                    && del.spent + amount <= del.budget
            }
            DelegatedActionKind::ContractCall { contract, method: _ } => {
                del.permissions.iter().any(|p| matches!(p, Permission::ContractCall { contracts } if contracts.contains(contract)))
            }
            DelegatedActionKind::DeviceControl { device, command: _ } => {
                del.permissions.iter().any(|p| matches!(p, Permission::DeviceControl { devices } if devices.contains(device)))
            }
            DelegatedActionKind::VectorQuery { index_id: _ } => {
                del.permissions.iter().any(|p| matches!(p, Permission::VectorQuery))
            }
            DelegatedActionKind::SwarmJoin { swarm_id: _ } => {
                del.permissions.iter().any(|p| matches!(p, Permission::SwarmJoin))
            }
            DelegatedActionKind::Custom { action: custom } => {
                del.permissions.iter().any(|p| matches!(p, Permission::Custom(s) if s == custom))
            }
        }
    }

    pub fn execute_delegated(
        &mut self,
        caller: Address,
        delegation_id: u64,
        action: DelegatedActionKind,
        current_time: u64,
    ) -> bool {
        let del = self
            .delegations
            .get(&delegation_id)
            .expect("DRC69: delegation not found");
        assert!(caller == del.delegate, "DRC69: not the delegate");
        assert!(
            self.check_permission(delegation_id, &action, current_time),
            "DRC69: permission denied"
        );

        // Track spending for transfers
        if let DelegatedActionKind::Transfer { to: _, amount } = &action {
            let del = self.delegations.get_mut(&delegation_id).unwrap();
            del.spent += amount;
        }
        true
    }

    pub fn delegation_chain(&self, addr: Address) -> Vec<&Delegation> {
        let key = addr_key(&addr);
        self.delegate_index
            .get(&key)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.delegations.get(id))
                    .filter(|d| !d.revoked)
                    .collect()
            })
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct DelegateArgs {
    delegate: Address,
    permissions: Vec<Permission>,
    budget: u64,
    expires_at: u64,
    revocable: bool,
    sub_delegations_allowed: bool,
}
#[derive(Serialize, Deserialize, Debug)]
struct SubDelegateArgs {
    parent_delegation_id: u64,
    delegate: Address,
    permissions: Vec<Permission>,
    budget: u64,
    expires_at: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct DelegationIdArgs {
    delegation_id: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct CheckPermArgs {
    delegation_id: u64,
    action: DelegatedActionKind,
    current_time: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct ExecuteArgs {
    delegation_id: u64,
    action: DelegatedActionKind,
    current_time: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct AddrArgs {
    address: Address,
}

pub fn dispatch(
    state: &mut Option<DelegationState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC69: already initialised");
            *state = Some(DelegationState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "delegate" => {
            let s = state.as_mut().expect("DRC69: not initialised");
            let a: DelegateArgs = serde_json::from_slice(args).expect("DRC69: bad args");
            let id = s.delegate(
                caller,
                a.delegate,
                a.permissions,
                a.budget,
                a.expires_at,
                a.revocable,
                a.sub_delegations_allowed,
            );
            serde_json::to_vec(&id).unwrap()
        }
        "sub_delegate" => {
            let s = state.as_mut().expect("DRC69: not initialised");
            let a: SubDelegateArgs = serde_json::from_slice(args).expect("DRC69: bad args");
            let id = s.sub_delegate(
                caller,
                a.parent_delegation_id,
                a.delegate,
                a.permissions,
                a.budget,
                a.expires_at,
            );
            serde_json::to_vec(&id).unwrap()
        }
        "revoke" => {
            let s = state.as_mut().expect("DRC69: not initialised");
            let a: DelegationIdArgs = serde_json::from_slice(args).expect("DRC69: bad args");
            s.revoke(caller, a.delegation_id);
            serde_json::to_vec("ok").unwrap()
        }
        "check_permission" => {
            let s = state.as_ref().expect("DRC69: not initialised");
            let a: CheckPermArgs = serde_json::from_slice(args).expect("DRC69: bad args");
            let ok = s.check_permission(a.delegation_id, &a.action, a.current_time);
            serde_json::to_vec(&ok).unwrap()
        }
        "execute_delegated" => {
            let s = state.as_mut().expect("DRC69: not initialised");
            let a: ExecuteArgs = serde_json::from_slice(args).expect("DRC69: bad args");
            let ok = s.execute_delegated(caller, a.delegation_id, a.action, a.current_time);
            serde_json::to_vec(&ok).unwrap()
        }
        "delegation_chain" => {
            let s = state.as_ref().expect("DRC69: not initialised");
            let a: AddrArgs = serde_json::from_slice(args).expect("DRC69: bad args");
            serde_json::to_vec(&s.delegation_chain(a.address)).unwrap()
        }
        _ => panic!("DRC69: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const PRINCIPAL: Address = [1u8; 32];
    const AGENT_A: Address = [2u8; 32];
    const AGENT_B: Address = [3u8; 32];
    const TARGET: Address = [9u8; 32];

    fn setup_delegation() -> (DelegationState, u64) {
        let mut s = DelegationState::new(PRINCIPAL);
        let id = s.delegate(
            PRINCIPAL,
            AGENT_A,
            vec![
                Permission::Transfer { max_amount: 1000 },
                Permission::VectorQuery,
            ],
            5000,
            999999,
            true,
            true,
        );
        (s, id)
    }

    #[test]
    fn test_delegate_and_check_permission() {
        let (s, id) = setup_delegation();
        let del = s.delegations.get(&id).unwrap();
        assert_eq!(del.delegate, AGENT_A);
        assert!(!del.revoked);

        assert!(s.check_permission(
            id,
            &DelegatedActionKind::Transfer {
                to: TARGET,
                amount: 500
            },
            100
        ));
        assert!(s.check_permission(id, &DelegatedActionKind::VectorQuery { index_id: 1 }, 100));
        // SwarmJoin not granted
        assert!(!s.check_permission(id, &DelegatedActionKind::SwarmJoin { swarm_id: 1 }, 100));
    }

    #[test]
    fn test_sub_delegation() {
        let (mut s, parent_id) = setup_delegation();
        let sub_id = s.sub_delegate(
            AGENT_A,
            parent_id,
            AGENT_B,
            vec![Permission::VectorQuery],
            1000,
            999999,
        );
        assert!(s.check_permission(
            sub_id,
            &DelegatedActionKind::VectorQuery { index_id: 42 },
            100
        ));
        // Transfer not sub-delegated
        assert!(!s.check_permission(
            sub_id,
            &DelegatedActionKind::Transfer {
                to: TARGET,
                amount: 100
            },
            100
        ));
    }

    #[test]
    fn test_revoke_cascades() {
        let (mut s, parent_id) = setup_delegation();
        let sub_id = s.sub_delegate(
            AGENT_A,
            parent_id,
            AGENT_B,
            vec![Permission::VectorQuery],
            500,
            999999,
        );
        s.revoke(PRINCIPAL, parent_id);
        assert!(s.delegations.get(&parent_id).unwrap().revoked);
        assert!(s.delegations.get(&sub_id).unwrap().revoked);
    }

    #[test]
    fn test_expired_delegation_denied() {
        let (s, id) = setup_delegation();
        // expires_at = 999999, check at time 1_000_000
        assert!(!s.check_permission(
            id,
            &DelegatedActionKind::VectorQuery { index_id: 1 },
            1_000_000
        ));
    }

    #[test]
    fn test_execute_tracks_spending() {
        let (mut s, id) = setup_delegation();
        s.execute_delegated(
            AGENT_A,
            id,
            DelegatedActionKind::Transfer {
                to: TARGET,
                amount: 300,
            },
            100,
        );
        assert_eq!(s.delegations.get(&id).unwrap().spent, 300);

        s.execute_delegated(
            AGENT_A,
            id,
            DelegatedActionKind::Transfer {
                to: TARGET,
                amount: 200,
            },
            200,
        );
        assert_eq!(s.delegations.get(&id).unwrap().spent, 500);
    }

    #[test]
    fn test_delegation_chain() {
        let (s, _) = setup_delegation();
        let chain = s.delegation_chain(AGENT_A);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].delegator, PRINCIPAL);
    }

    #[test]
    #[should_panic(expected = "cannot delegate to self")]
    fn test_self_delegation_rejected() {
        let mut s = DelegationState::new(PRINCIPAL);
        s.delegate(
            PRINCIPAL,
            PRINCIPAL,
            vec![Permission::VectorQuery],
            100,
            9999,
            true,
            false,
        );
    }

    #[test]
    fn test_dispatch_roundtrip() {
        let mut state = None;
        dispatch(&mut state, "init", b"{}", PRINCIPAL);
        let args = serde_json::to_vec(&DelegateArgs {
            delegate: AGENT_A,
            permissions: vec![Permission::VectorQuery],
            budget: 1000,
            expires_at: 99999,
            revocable: true,
            sub_delegations_allowed: false,
        })
        .unwrap();
        let result = dispatch(&mut state, "delegate", &args, PRINCIPAL);
        let id: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(id, 1);
    }
}
