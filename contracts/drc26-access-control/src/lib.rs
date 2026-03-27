use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// DRC-26  Role-Based Access Control
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

/// Default admin role name. This role can manage all other roles.
pub const DEFAULT_ADMIN_ROLE: &str = "DEFAULT_ADMIN";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RoleData {
    pub members: BTreeSet<Address>,
    /// The role that can administrate this role (grant/revoke).
    pub admin_role: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AccessControlState {
    pub admin: Address,
    pub roles: BTreeMap<String, RoleData>,
}

impl AccessControlState {
    pub fn new(admin: Address) -> Self {
        let mut roles = BTreeMap::new();
        let mut admin_members = BTreeSet::new();
        admin_members.insert(admin);
        roles.insert(
            DEFAULT_ADMIN_ROLE.to_string(),
            RoleData {
                members: admin_members,
                admin_role: DEFAULT_ADMIN_ROLE.to_string(),
            },
        );
        Self { admin, roles }
    }

    pub fn has_role(&self, role: &str, account: &Address) -> bool {
        self.roles
            .get(role)
            .map(|r| r.members.contains(account))
            .unwrap_or(false)
    }

    pub fn grant_role(&mut self, caller: Address, role: &str, account: Address) {
        // Caller must have the admin role for the target role
        let admin_role = self.get_admin_role(role);
        assert!(
            self.has_role(&admin_role, &caller),
            "DRC26: caller lacks admin role '{}' required to grant '{}'",
            admin_role,
            role
        );

        let role_data = self
            .roles
            .entry(role.to_string())
            .or_insert_with(|| RoleData {
                members: BTreeSet::new(),
                admin_role: DEFAULT_ADMIN_ROLE.to_string(),
            });
        role_data.members.insert(account);
    }

    pub fn revoke_role(&mut self, caller: Address, role: &str, account: Address) {
        let admin_role = self.get_admin_role(role);
        assert!(
            self.has_role(&admin_role, &caller),
            "DRC26: caller lacks admin role '{}' required to revoke '{}'",
            admin_role,
            role
        );
        assert!(
            !(role == DEFAULT_ADMIN_ROLE && account == self.admin && self.role_member_count(role) <= 1),
            "DRC26: cannot revoke last default admin"
        );

        if let Some(role_data) = self.roles.get_mut(role) {
            role_data.members.remove(&account);
        }
    }

    /// A member can renounce their own role.
    pub fn renounce_role(&mut self, caller: Address, role: &str) {
        assert!(
            self.has_role(role, &caller),
            "DRC26: caller does not have role '{}'",
            role
        );
        assert!(
            !(role == DEFAULT_ADMIN_ROLE && self.role_member_count(role) <= 1),
            "DRC26: cannot renounce last default admin"
        );

        if let Some(role_data) = self.roles.get_mut(role) {
            role_data.members.remove(&caller);
        }
    }

    /// Set which role administrates the given role.
    pub fn set_role_admin(&mut self, caller: Address, role: &str, admin_role: &str) {
        assert!(
            self.has_role(DEFAULT_ADMIN_ROLE, &caller),
            "DRC26: only default admin can change role admins"
        );

        let role_data = self
            .roles
            .entry(role.to_string())
            .or_insert_with(|| RoleData {
                members: BTreeSet::new(),
                admin_role: DEFAULT_ADMIN_ROLE.to_string(),
            });
        role_data.admin_role = admin_role.to_string();
    }

    pub fn get_role_members(&self, role: &str) -> Vec<Address> {
        self.roles
            .get(role)
            .map(|r| r.members.iter().copied().collect())
            .unwrap_or_default()
    }

    fn get_admin_role(&self, role: &str) -> String {
        self.roles
            .get(role)
            .map(|r| r.admin_role.clone())
            .unwrap_or_else(|| DEFAULT_ADMIN_ROLE.to_string())
    }

    fn role_member_count(&self, role: &str) -> usize {
        self.roles
            .get(role)
            .map(|r| r.members.len())
            .unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct HasRoleArgs {
    role: String,
    account: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct GrantRoleArgs {
    role: String,
    account: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct RevokeRoleArgs {
    role: String,
    account: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct RenounceRoleArgs {
    role: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetRoleAdminArgs {
    role: String,
    admin_role: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetRoleMembersArgs {
    role: String,
}

/// Contract-level dispatch.
pub fn dispatch(
    state: &mut Option<AccessControlState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC26: already initialised");
            *state = Some(AccessControlState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "has_role" => {
            let s = state.as_ref().expect("DRC26: not initialised");
            let a: HasRoleArgs =
                serde_json::from_slice(args).expect("DRC26: bad has_role args");
            serde_json::to_vec(&s.has_role(&a.role, &a.account)).unwrap()
        }

        "grant_role" => {
            let s = state.as_mut().expect("DRC26: not initialised");
            let a: GrantRoleArgs =
                serde_json::from_slice(args).expect("DRC26: bad grant_role args");
            s.grant_role(caller, &a.role, a.account);
            serde_json::to_vec("ok").unwrap()
        }

        "revoke_role" => {
            let s = state.as_mut().expect("DRC26: not initialised");
            let a: RevokeRoleArgs =
                serde_json::from_slice(args).expect("DRC26: bad revoke_role args");
            s.revoke_role(caller, &a.role, a.account);
            serde_json::to_vec("ok").unwrap()
        }

        "renounce_role" => {
            let s = state.as_mut().expect("DRC26: not initialised");
            let a: RenounceRoleArgs =
                serde_json::from_slice(args).expect("DRC26: bad renounce_role args");
            s.renounce_role(caller, &a.role);
            serde_json::to_vec("ok").unwrap()
        }

        "set_role_admin" => {
            let s = state.as_mut().expect("DRC26: not initialised");
            let a: SetRoleAdminArgs =
                serde_json::from_slice(args).expect("DRC26: bad set_role_admin args");
            s.set_role_admin(caller, &a.role, &a.admin_role);
            serde_json::to_vec("ok").unwrap()
        }

        "get_role_members" => {
            let s = state.as_ref().expect("DRC26: not initialised");
            let a: GetRoleMembersArgs =
                serde_json::from_slice(args).expect("DRC26: bad get_role_members args");
            let members = s.get_role_members(&a.role);
            serde_json::to_vec(&members).unwrap()
        }

        _ => panic!("DRC26: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ADMIN: Address = [1u8; 32];
    const ALICE: Address = [2u8; 32];
    const BOB: Address = [3u8; 32];
    const NOBODY: Address = [4u8; 32];

    fn init_ac() -> Option<AccessControlState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", ADMIN);
        state
    }

    #[test]
    fn test_admin_has_default_role() {
        let mut state = init_ac();
        let args = serde_json::to_vec(&HasRoleArgs {
            role: DEFAULT_ADMIN_ROLE.into(),
            account: ADMIN,
        })
        .unwrap();
        let result = dispatch(&mut state, "has_role", &args, NOBODY);
        let has: bool = serde_json::from_slice(&result).unwrap();
        assert!(has);
    }

    #[test]
    fn test_grant_and_check_role() {
        let mut state = init_ac();

        let grant = serde_json::to_vec(&GrantRoleArgs {
            role: "MINTER".into(),
            account: ALICE,
        })
        .unwrap();
        dispatch(&mut state, "grant_role", &grant, ADMIN);

        let check = serde_json::to_vec(&HasRoleArgs {
            role: "MINTER".into(),
            account: ALICE,
        })
        .unwrap();
        let result = dispatch(&mut state, "has_role", &check, NOBODY);
        let has: bool = serde_json::from_slice(&result).unwrap();
        assert!(has);
    }

    #[test]
    fn test_revoke_role() {
        let mut state = init_ac();

        let grant = serde_json::to_vec(&GrantRoleArgs {
            role: "BURNER".into(),
            account: ALICE,
        })
        .unwrap();
        dispatch(&mut state, "grant_role", &grant, ADMIN);

        let revoke = serde_json::to_vec(&RevokeRoleArgs {
            role: "BURNER".into(),
            account: ALICE,
        })
        .unwrap();
        dispatch(&mut state, "revoke_role", &revoke, ADMIN);

        let check = serde_json::to_vec(&HasRoleArgs {
            role: "BURNER".into(),
            account: ALICE,
        })
        .unwrap();
        let result = dispatch(&mut state, "has_role", &check, NOBODY);
        let has: bool = serde_json::from_slice(&result).unwrap();
        assert!(!has);
    }

    #[test]
    fn test_renounce_role() {
        let mut state = init_ac();

        // Grant Alice a role
        let grant = serde_json::to_vec(&GrantRoleArgs {
            role: "OPERATOR".into(),
            account: ALICE,
        })
        .unwrap();
        dispatch(&mut state, "grant_role", &grant, ADMIN);

        // Alice renounces
        let renounce = serde_json::to_vec(&RenounceRoleArgs {
            role: "OPERATOR".into(),
        })
        .unwrap();
        dispatch(&mut state, "renounce_role", &renounce, ALICE);

        assert!(!state.as_ref().unwrap().has_role("OPERATOR", &ALICE));
    }

    #[test]
    #[should_panic(expected = "caller lacks admin role")]
    fn test_non_admin_cannot_grant() {
        let mut state = init_ac();
        let grant = serde_json::to_vec(&GrantRoleArgs {
            role: "MINTER".into(),
            account: BOB,
        })
        .unwrap();
        dispatch(&mut state, "grant_role", &grant, NOBODY);
    }

    #[test]
    fn test_set_role_admin_and_delegated_grant() {
        let mut state = init_ac();

        // Grant Alice the MANAGER role
        let grant_manager = serde_json::to_vec(&GrantRoleArgs {
            role: "MANAGER".into(),
            account: ALICE,
        })
        .unwrap();
        dispatch(&mut state, "grant_role", &grant_manager, ADMIN);

        // Set MANAGER as admin for WORKER role
        let set_admin = serde_json::to_vec(&SetRoleAdminArgs {
            role: "WORKER".into(),
            admin_role: "MANAGER".into(),
        })
        .unwrap();
        dispatch(&mut state, "set_role_admin", &set_admin, ADMIN);

        // Alice (MANAGER) can now grant WORKER role to Bob
        let grant_worker = serde_json::to_vec(&GrantRoleArgs {
            role: "WORKER".into(),
            account: BOB,
        })
        .unwrap();
        dispatch(&mut state, "grant_role", &grant_worker, ALICE);

        assert!(state.as_ref().unwrap().has_role("WORKER", &BOB));
    }

    #[test]
    fn test_get_role_members() {
        let mut state = init_ac();

        for account in [ALICE, BOB] {
            let grant = serde_json::to_vec(&GrantRoleArgs {
                role: "VALIDATORS".into(),
                account,
            })
            .unwrap();
            dispatch(&mut state, "grant_role", &grant, ADMIN);
        }

        let query = serde_json::to_vec(&GetRoleMembersArgs {
            role: "VALIDATORS".into(),
        })
        .unwrap();
        let result = dispatch(&mut state, "get_role_members", &query, NOBODY);
        let members: Vec<Address> = serde_json::from_slice(&result).unwrap();
        assert_eq!(members.len(), 2);
        assert!(members.contains(&ALICE));
        assert!(members.contains(&BOB));
    }
}
