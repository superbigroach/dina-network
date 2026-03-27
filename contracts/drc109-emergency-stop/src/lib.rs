use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-109  Emergency Stop
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FreezeScope {
    WalletOnly,
    FullFreeze,
    CapabilityFreeze,
    NetworkBan,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FreezeInfo {
    pub target: [u8; 32],
    pub frozen_by: [u8; 32],
    pub reason: String,
    pub frozen_at: u64,
    pub scope: FreezeScope,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EmergencyStopState {
    /// target -> FreezeInfo
    pub frozen: BTreeMap<[u8; 32], FreezeInfo>,
    /// Set of authorised emergency responders
    pub responders: BTreeMap<[u8; 32], String>,
    pub admin: [u8; 32],
    pub total_freezes: u64,
    pub total_unfreezes: u64,
}

impl EmergencyStopState {
    pub fn new(admin: [u8; 32]) -> Self {
        let mut responders = BTreeMap::new();
        responders.insert(admin, "admin".to_string());
        Self {
            frozen: BTreeMap::new(),
            responders,
            admin,
            total_freezes: 0,
            total_unfreezes: 0,
        }
    }

    pub fn freeze(
        &mut self,
        caller: [u8; 32],
        target: [u8; 32],
        reason: String,
        scope: FreezeScope,
        timestamp: u64,
    ) {
        assert!(
            self.responders.contains_key(&caller),
            "DRC109: caller is not an authorised responder"
        );
        assert!(
            !self.frozen.contains_key(&target),
            "DRC109: target is already frozen"
        );

        self.frozen.insert(
            target,
            FreezeInfo {
                target,
                frozen_by: caller,
                reason,
                frozen_at: timestamp,
                scope,
            },
        );
        self.total_freezes += 1;
    }

    pub fn unfreeze(&mut self, caller: [u8; 32], target: [u8; 32]) {
        assert!(
            self.responders.contains_key(&caller),
            "DRC109: caller is not an authorised responder"
        );
        let info = self
            .frozen
            .get(&target)
            .expect("DRC109: target is not frozen");

        // NetworkBan can only be lifted by admin
        if info.scope == FreezeScope::NetworkBan {
            assert!(
                caller == self.admin,
                "DRC109: only admin can lift a NetworkBan"
            );
        }

        self.frozen.remove(&target);
        self.total_unfreezes += 1;
    }

    pub fn is_frozen(&self, target: &[u8; 32]) -> bool {
        self.frozen.contains_key(target)
    }

    pub fn freeze_info(&self, target: &[u8; 32]) -> Option<&FreezeInfo> {
        self.frozen.get(target)
    }

    pub fn register_responder(
        &mut self,
        caller: [u8; 32],
        responder: [u8; 32],
        label: String,
    ) {
        assert!(
            caller == self.admin,
            "DRC109: only admin can register responders"
        );
        self.responders.insert(responder, label);
    }

    pub fn remove_responder(&mut self, caller: [u8; 32], responder: [u8; 32]) {
        assert!(
            caller == self.admin,
            "DRC109: only admin can remove responders"
        );
        assert!(
            responder != self.admin,
            "DRC109: cannot remove admin as responder"
        );
        self.responders.remove(&responder);
    }
}

// ---------------------------------------------------------------------------
// Dispatch arg types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct FreezeArgs {
    target: [u8; 32],
    reason: String,
    scope: FreezeScope,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct UnfreezeArgs {
    target: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct TargetArgs {
    target: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct RegisterResponderArgs {
    responder: [u8; 32],
    label: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RemoveResponderArgs {
    responder: [u8; 32],
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<EmergencyStopState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC109: already initialised");
            *state = Some(EmergencyStopState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "freeze" => {
            let s = state.as_mut().expect("DRC109: not initialised");
            let a: FreezeArgs =
                serde_json::from_slice(args).expect("DRC109: bad freeze args");
            s.freeze(caller, a.target, a.reason, a.scope, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }

        "unfreeze" => {
            let s = state.as_mut().expect("DRC109: not initialised");
            let a: UnfreezeArgs =
                serde_json::from_slice(args).expect("DRC109: bad unfreeze args");
            s.unfreeze(caller, a.target);
            serde_json::to_vec("ok").unwrap()
        }

        "is_frozen" => {
            let s = state.as_ref().expect("DRC109: not initialised");
            let a: TargetArgs =
                serde_json::from_slice(args).expect("DRC109: bad is_frozen args");
            serde_json::to_vec(&s.is_frozen(&a.target)).unwrap()
        }

        "freeze_info" => {
            let s = state.as_ref().expect("DRC109: not initialised");
            let a: TargetArgs =
                serde_json::from_slice(args).expect("DRC109: bad freeze_info args");
            serde_json::to_vec(&s.freeze_info(&a.target)).unwrap()
        }

        "register_responder" => {
            let s = state.as_mut().expect("DRC109: not initialised");
            let a: RegisterResponderArgs =
                serde_json::from_slice(args).expect("DRC109: bad register_responder args");
            s.register_responder(caller, a.responder, a.label);
            serde_json::to_vec("ok").unwrap()
        }

        "remove_responder" => {
            let s = state.as_mut().expect("DRC109: not initialised");
            let a: RemoveResponderArgs =
                serde_json::from_slice(args).expect("DRC109: bad remove_responder args");
            s.remove_responder(caller, a.responder);
            serde_json::to_vec("ok").unwrap()
        }

        "stats" => {
            let s = state.as_ref().expect("DRC109: not initialised");
            #[derive(Serialize)]
            struct Stats {
                total_freezes: u64,
                total_unfreezes: u64,
                currently_frozen: usize,
                responder_count: usize,
            }
            serde_json::to_vec(&Stats {
                total_freezes: s.total_freezes,
                total_unfreezes: s.total_unfreezes,
                currently_frozen: s.frozen.len(),
                responder_count: s.responders.len(),
            })
            .unwrap()
        }

        _ => panic!("DRC109: unknown method '{method}'"),
    }
}
