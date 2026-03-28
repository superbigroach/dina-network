use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// DRC-16  Upgradeable Proxy (Enhanced)
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PendingUpgrade {
    pub new_code: Vec<u8>,
    pub new_hash: String,
    pub proposed_at: u64,
    pub proposed_by: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpgradeRecord {
    pub version: u64,
    pub old_hash: String,
    pub new_hash: String,
    pub upgraded_at: u64,
    pub upgraded_by: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProxyState {
    pub admin: String,
    pub implementation_code: Vec<u8>,
    pub implementation_hash: String,
    pub storage: HashMap<Vec<u8>, Vec<u8>>,
    pub pending_upgrade: Option<PendingUpgrade>,
    pub upgrade_delay_seconds: u64,
    pub version: u64,
    pub paused: bool,
    pub upgrade_history: Vec<UpgradeRecord>,
}

impl ProxyState {
    pub fn new(admin: String, implementation_code: Vec<u8>, upgrade_delay: u64) -> Self {
        let hash = compute_hash(&implementation_code);
        Self {
            admin,
            implementation_code,
            implementation_hash: hash,
            storage: HashMap::new(),
            pending_upgrade: None,
            upgrade_delay_seconds: upgrade_delay,
            version: 1,
            paused: false,
            upgrade_history: Vec::new(),
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn get_implementation_hash(&self) -> &str {
        &self.implementation_hash
    }

    pub fn get_version(&self) -> u64 {
        self.version
    }

    pub fn get_upgrade_history(&self) -> &[UpgradeRecord] {
        &self.upgrade_history
    }

    pub fn get_pending_upgrade(&self) -> Option<&PendingUpgrade> {
        self.pending_upgrade.as_ref()
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn get_admin(&self) -> &str {
        &self.admin
    }

    // -- Proxy call ----------------------------------------------------------

    /// Forward a call to the implementation contract. Storage persists across
    /// upgrades since it lives in the proxy, not the implementation.
    pub fn proxy_call(&mut self, method: &str, args: &[u8]) -> Vec<u8> {
        assert!(!self.paused, "DRC16: proxy is paused");
        // In a real runtime, this would load the WASM bytecode from
        // `implementation_code`, instantiate it with `self.storage` as the
        // backing store, and call `method(args)`. The result bytes and any
        // storage mutations would be captured and returned/persisted here.
        //
        // For the contract-level implementation we store the call record so
        // tests can verify forwarding behaviour.
        let key = format!("__last_call_{}", method).into_bytes();
        self.storage.insert(key, args.to_vec());
        // Return a placeholder acknowledgement with the method that was called
        serde_json::to_vec(&format!("forwarded:{}", method)).unwrap()
    }

    // -- Upgrade lifecycle ---------------------------------------------------

    pub fn propose_upgrade(&mut self, caller: &str, new_code: Vec<u8>, current_time: u64) {
        assert!(
            caller == self.admin,
            "DRC16: only admin can propose upgrade"
        );
        assert!(
            self.pending_upgrade.is_none(),
            "DRC16: upgrade already pending — cancel first"
        );
        assert!(!new_code.is_empty(), "DRC16: new code cannot be empty");

        let new_hash = compute_hash(&new_code);
        self.pending_upgrade = Some(PendingUpgrade {
            new_code,
            new_hash,
            proposed_at: current_time,
            proposed_by: caller.to_string(),
        });
    }

    pub fn execute_upgrade(&mut self, caller: &str, current_time: u64) {
        // M-5: Only admin can execute an upgrade.
        assert!(
            caller == self.admin,
            "DRC16: only admin can execute upgrade"
        );
        let pending = self
            .pending_upgrade
            .take()
            .expect("DRC16: no pending upgrade");
        assert!(
            current_time >= pending.proposed_at + self.upgrade_delay_seconds,
            "DRC16: timelock not expired (now={}, ready_at={})",
            current_time,
            pending.proposed_at + self.upgrade_delay_seconds
        );

        let old_hash = self.implementation_hash.clone();
        self.implementation_code = pending.new_code;
        self.implementation_hash = pending.new_hash.clone();
        self.version += 1;

        self.upgrade_history.push(UpgradeRecord {
            version: self.version,
            old_hash,
            new_hash: pending.new_hash,
            upgraded_at: current_time,
            upgraded_by: pending.proposed_by,
        });
    }

    pub fn cancel_upgrade(&mut self, caller: &str) {
        assert!(caller == self.admin, "DRC16: only admin can cancel upgrade");
        assert!(
            self.pending_upgrade.is_some(),
            "DRC16: no pending upgrade to cancel"
        );
        self.pending_upgrade = None;
    }

    // -- Admin ---------------------------------------------------------------

    pub fn transfer_admin(&mut self, caller: &str, new_admin: String) {
        assert!(caller == self.admin, "DRC16: only admin can transfer admin");
        assert!(!new_admin.is_empty(), "DRC16: new admin cannot be empty");
        self.admin = new_admin;
    }

    // -- Emergency -----------------------------------------------------------

    pub fn pause(&mut self, caller: &str) {
        assert!(caller == self.admin, "DRC16: only admin can pause");
        assert!(!self.paused, "DRC16: already paused");
        self.paused = true;
    }

    pub fn unpause(&mut self, caller: &str) {
        assert!(caller == self.admin, "DRC16: only admin can unpause");
        assert!(self.paused, "DRC16: not paused");
        self.paused = false;
    }
}

/// Compute a hex-encoded SHA-256 hash for the code.
fn compute_hash(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(data);
    // Encode full 32-byte hash as 64-char hex string
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct DeployProxyArgs {
    admin: String,
    implementation_code: Vec<u8>,
    upgrade_delay: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProxyCallArgs {
    method: String,
    args: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProposeUpgradeArgs {
    new_code: Vec<u8>,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteUpgradeArgs {
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferAdminArgs {
    new_admin: String,
}

pub fn dispatch(
    state: &mut Option<ProxyState>,
    method: &str,
    args: &[u8],
    caller: &str,
) -> Vec<u8> {
    match method {
        "deploy_proxy" => {
            assert!(state.is_none(), "DRC16: already initialised");
            let a: DeployProxyArgs =
                serde_json::from_slice(args).expect("DRC16: bad deploy_proxy args");
            *state = Some(ProxyState::new(
                a.admin,
                a.implementation_code,
                a.upgrade_delay,
            ));
            serde_json::to_vec("ok").unwrap()
        }

        "proxy_call" => {
            let s = state.as_mut().expect("DRC16: not initialised");
            let a: ProxyCallArgs =
                serde_json::from_slice(args).expect("DRC16: bad proxy_call args");
            s.proxy_call(&a.method, &a.args)
        }

        "propose_upgrade" => {
            let s = state.as_mut().expect("DRC16: not initialised");
            let a: ProposeUpgradeArgs =
                serde_json::from_slice(args).expect("DRC16: bad propose_upgrade args");
            s.propose_upgrade(caller, a.new_code, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }

        "execute_upgrade" => {
            let s = state.as_mut().expect("DRC16: not initialised");
            let a: ExecuteUpgradeArgs =
                serde_json::from_slice(args).expect("DRC16: bad execute_upgrade args");
            // M-5: Pass caller so execute_upgrade can verify admin access.
            s.execute_upgrade(caller, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }

        "cancel_upgrade" => {
            let s = state.as_mut().expect("DRC16: not initialised");
            s.cancel_upgrade(caller);
            serde_json::to_vec("ok").unwrap()
        }

        "transfer_admin" => {
            let s = state.as_mut().expect("DRC16: not initialised");
            let a: TransferAdminArgs =
                serde_json::from_slice(args).expect("DRC16: bad transfer_admin args");
            s.transfer_admin(caller, a.new_admin);
            serde_json::to_vec("ok").unwrap()
        }

        "pause" => {
            let s = state.as_mut().expect("DRC16: not initialised");
            s.pause(caller);
            serde_json::to_vec("ok").unwrap()
        }

        "unpause" => {
            let s = state.as_mut().expect("DRC16: not initialised");
            s.unpause(caller);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "get_implementation_hash" => {
            let s = state.as_ref().expect("DRC16: not initialised");
            serde_json::to_vec(s.get_implementation_hash()).unwrap()
        }

        "get_version" => {
            let s = state.as_ref().expect("DRC16: not initialised");
            serde_json::to_vec(&s.get_version()).unwrap()
        }

        "get_upgrade_history" => {
            let s = state.as_ref().expect("DRC16: not initialised");
            serde_json::to_vec(s.get_upgrade_history()).unwrap()
        }

        "get_pending_upgrade" => {
            let s = state.as_ref().expect("DRC16: not initialised");
            serde_json::to_vec(&s.get_pending_upgrade()).unwrap()
        }

        "get_admin" => {
            let s = state.as_ref().expect("DRC16: not initialised");
            serde_json::to_vec(s.get_admin()).unwrap()
        }

        _ => panic!("DRC16: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ADMIN: &str = "admin_address";
    const ALICE: &str = "alice_address";
    const CODE_V1: &[u8] = b"wasm_bytecode_v1";
    const CODE_V2: &[u8] = b"wasm_bytecode_v2";
    const CODE_V3: &[u8] = b"wasm_bytecode_v3";
    const UPGRADE_DELAY: u64 = 3600; // 1 hour

    fn deploy(admin: &str, code: &[u8], delay: u64) -> Option<ProxyState> {
        let mut state = None;
        let args = serde_json::to_vec(&DeployProxyArgs {
            admin: admin.to_string(),
            implementation_code: code.to_vec(),
            upgrade_delay: delay,
        })
        .unwrap();
        dispatch(&mut state, "deploy_proxy", &args, admin);
        state
    }

    #[test]
    fn test_deploy_proxy() {
        let state = deploy(ADMIN, CODE_V1, UPGRADE_DELAY);
        let s = state.as_ref().unwrap();
        assert_eq!(s.admin, ADMIN);
        assert_eq!(s.version, 1);
        assert!(!s.paused);
        assert!(s.pending_upgrade.is_none());
        assert!(s.upgrade_history.is_empty());
        assert_eq!(s.implementation_code, CODE_V1);
    }

    #[test]
    fn test_proxy_call_forwarding() {
        let mut state = deploy(ADMIN, CODE_V1, UPGRADE_DELAY);
        let args = serde_json::to_vec(&ProxyCallArgs {
            method: "transfer".to_string(),
            args: b"some_args".to_vec(),
        })
        .unwrap();
        let result = dispatch(&mut state, "proxy_call", &args, ALICE);
        let result_str: String = serde_json::from_slice(&result).unwrap();
        assert_eq!(result_str, "forwarded:transfer");

        // Verify storage was updated
        let s = state.as_ref().unwrap();
        let key = b"__last_call_transfer".to_vec();
        assert_eq!(s.storage.get(&key).unwrap(), b"some_args");
    }

    #[test]
    fn test_upgrade_lifecycle() {
        let mut state = deploy(ADMIN, CODE_V1, UPGRADE_DELAY);
        let hash_v1 = state.as_ref().unwrap().implementation_hash.clone();

        // Propose upgrade
        let propose_args = serde_json::to_vec(&ProposeUpgradeArgs {
            new_code: CODE_V2.to_vec(),
            current_time: 1000,
        })
        .unwrap();
        dispatch(&mut state, "propose_upgrade", &propose_args, ADMIN);
        assert!(state.as_ref().unwrap().pending_upgrade.is_some());

        // Execute after timelock
        let exec_args = serde_json::to_vec(&ExecuteUpgradeArgs {
            current_time: 1000 + UPGRADE_DELAY,
        })
        .unwrap();
        dispatch(&mut state, "execute_upgrade", &exec_args, ADMIN);

        let s = state.as_ref().unwrap();
        assert_eq!(s.version, 2);
        assert!(s.pending_upgrade.is_none());
        assert_eq!(s.implementation_code, CODE_V2);
        assert_ne!(s.implementation_hash, hash_v1);
        assert_eq!(s.upgrade_history.len(), 1);
        assert_eq!(s.upgrade_history[0].old_hash, hash_v1);
    }

    #[test]
    #[should_panic(expected = "DRC16: timelock not expired")]
    fn test_execute_upgrade_too_early() {
        let mut state = deploy(ADMIN, CODE_V1, UPGRADE_DELAY);

        let propose_args = serde_json::to_vec(&ProposeUpgradeArgs {
            new_code: CODE_V2.to_vec(),
            current_time: 1000,
        })
        .unwrap();
        dispatch(&mut state, "propose_upgrade", &propose_args, ADMIN);

        let exec_args = serde_json::to_vec(&ExecuteUpgradeArgs {
            current_time: 1000 + UPGRADE_DELAY - 1,
        })
        .unwrap();
        dispatch(&mut state, "execute_upgrade", &exec_args, ADMIN);
    }

    #[test]
    fn test_cancel_upgrade() {
        let mut state = deploy(ADMIN, CODE_V1, UPGRADE_DELAY);

        let propose_args = serde_json::to_vec(&ProposeUpgradeArgs {
            new_code: CODE_V2.to_vec(),
            current_time: 1000,
        })
        .unwrap();
        dispatch(&mut state, "propose_upgrade", &propose_args, ADMIN);
        assert!(state.as_ref().unwrap().pending_upgrade.is_some());

        dispatch(&mut state, "cancel_upgrade", b"", ADMIN);
        assert!(state.as_ref().unwrap().pending_upgrade.is_none());
    }

    #[test]
    #[should_panic(expected = "DRC16: only admin can propose upgrade")]
    fn test_non_admin_cannot_propose() {
        let mut state = deploy(ADMIN, CODE_V1, UPGRADE_DELAY);
        let propose_args = serde_json::to_vec(&ProposeUpgradeArgs {
            new_code: CODE_V2.to_vec(),
            current_time: 1000,
        })
        .unwrap();
        dispatch(&mut state, "propose_upgrade", &propose_args, ALICE);
    }

    #[test]
    fn test_transfer_admin() {
        let mut state = deploy(ADMIN, CODE_V1, UPGRADE_DELAY);
        let args = serde_json::to_vec(&TransferAdminArgs {
            new_admin: ALICE.to_string(),
        })
        .unwrap();
        dispatch(&mut state, "transfer_admin", &args, ADMIN);
        assert_eq!(state.as_ref().unwrap().admin, ALICE);
    }

    #[test]
    fn test_pause_unpause() {
        let mut state = deploy(ADMIN, CODE_V1, UPGRADE_DELAY);

        dispatch(&mut state, "pause", b"", ADMIN);
        assert!(state.as_ref().unwrap().paused);

        dispatch(&mut state, "unpause", b"", ADMIN);
        assert!(!state.as_ref().unwrap().paused);
    }

    #[test]
    #[should_panic(expected = "DRC16: proxy is paused")]
    fn test_proxy_call_while_paused() {
        let mut state = deploy(ADMIN, CODE_V1, UPGRADE_DELAY);
        dispatch(&mut state, "pause", b"", ADMIN);

        let args = serde_json::to_vec(&ProxyCallArgs {
            method: "transfer".to_string(),
            args: vec![],
        })
        .unwrap();
        dispatch(&mut state, "proxy_call", &args, ALICE);
    }

    #[test]
    fn test_storage_persists_across_upgrades() {
        let mut state = deploy(ADMIN, CODE_V1, UPGRADE_DELAY);

        // Write to storage via proxy call
        let call_args = serde_json::to_vec(&ProxyCallArgs {
            method: "store_data".to_string(),
            args: b"important_data".to_vec(),
        })
        .unwrap();
        dispatch(&mut state, "proxy_call", &call_args, ALICE);

        // Upgrade
        let propose_args = serde_json::to_vec(&ProposeUpgradeArgs {
            new_code: CODE_V2.to_vec(),
            current_time: 1000,
        })
        .unwrap();
        dispatch(&mut state, "propose_upgrade", &propose_args, ADMIN);

        let exec_args = serde_json::to_vec(&ExecuteUpgradeArgs {
            current_time: 1000 + UPGRADE_DELAY,
        })
        .unwrap();
        dispatch(&mut state, "execute_upgrade", &exec_args, ADMIN);

        // Storage still has the data
        let s = state.as_ref().unwrap();
        let key = b"__last_call_store_data".to_vec();
        assert_eq!(s.storage.get(&key).unwrap(), b"important_data");
        assert_eq!(s.version, 2);
    }

    #[test]
    fn test_multiple_upgrades_history() {
        let mut state = deploy(ADMIN, CODE_V1, UPGRADE_DELAY);

        // Upgrade v1 -> v2
        let propose = serde_json::to_vec(&ProposeUpgradeArgs {
            new_code: CODE_V2.to_vec(),
            current_time: 1000,
        })
        .unwrap();
        dispatch(&mut state, "propose_upgrade", &propose, ADMIN);
        let exec = serde_json::to_vec(&ExecuteUpgradeArgs {
            current_time: 1000 + UPGRADE_DELAY,
        })
        .unwrap();
        dispatch(&mut state, "execute_upgrade", &exec, ADMIN);

        // Upgrade v2 -> v3
        let propose = serde_json::to_vec(&ProposeUpgradeArgs {
            new_code: CODE_V3.to_vec(),
            current_time: 5000,
        })
        .unwrap();
        dispatch(&mut state, "propose_upgrade", &propose, ADMIN);
        let exec = serde_json::to_vec(&ExecuteUpgradeArgs {
            current_time: 5000 + UPGRADE_DELAY,
        })
        .unwrap();
        dispatch(&mut state, "execute_upgrade", &exec, ADMIN);

        let s = state.as_ref().unwrap();
        assert_eq!(s.version, 3);
        assert_eq!(s.upgrade_history.len(), 2);
        assert_eq!(s.upgrade_history[0].version, 2);
        assert_eq!(s.upgrade_history[1].version, 3);
    }

    #[test]
    fn test_query_methods() {
        let state_opt = deploy(ADMIN, CODE_V1, UPGRADE_DELAY);
        let mut state = state_opt;

        let result = dispatch(&mut state, "get_version", b"", ADMIN);
        let version: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(version, 1);

        let result = dispatch(&mut state, "get_admin", b"", ADMIN);
        let admin: String = serde_json::from_slice(&result).unwrap();
        assert_eq!(admin, ADMIN);

        let result = dispatch(&mut state, "get_implementation_hash", b"", ADMIN);
        let hash: String = serde_json::from_slice(&result).unwrap();
        assert!(!hash.is_empty());
    }
}
