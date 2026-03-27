use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// DRC-16  Upgradeable Proxy
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProxyState {
    pub implementation: Address,
    pub admin: Address,
    pub pending_admin: Option<Address>,
}

impl ProxyState {
    pub fn new(implementation: Address, admin: Address) -> Self {
        Self {
            implementation,
            admin,
            pending_admin: None,
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn implementation(&self) -> Address {
        self.implementation
    }

    pub fn proxy_admin(&self) -> Address {
        self.admin
    }

    // -- Mutations -----------------------------------------------------------

    pub fn upgrade_to(&mut self, caller: Address, new_impl: Address) {
        assert!(caller == self.admin, "DRC16: only admin can upgrade");
        assert!(
            new_impl != [0u8; 32],
            "DRC16: implementation cannot be zero address"
        );
        self.implementation = new_impl;
    }

    pub fn upgrade_to_and_call(
        &mut self,
        caller: Address,
        new_impl: Address,
        _init_data: Vec<u8>,
    ) {
        assert!(caller == self.admin, "DRC16: only admin can upgrade");
        assert!(
            new_impl != [0u8; 32],
            "DRC16: implementation cannot be zero address"
        );
        self.implementation = new_impl;
        // init_data would be forwarded to the new implementation in a real
        // runtime; here we store the upgrade and the runtime handles the call.
    }

    pub fn change_admin(&mut self, caller: Address, new_admin: Address) {
        assert!(caller == self.admin, "DRC16: only admin can change admin");
        assert!(
            new_admin != [0u8; 32],
            "DRC16: new admin cannot be zero address"
        );
        self.pending_admin = Some(new_admin);
    }

    pub fn accept_admin(&mut self, caller: Address) {
        let pending = self
            .pending_admin
            .expect("DRC16: no pending admin transfer");
        assert!(caller == pending, "DRC16: only pending admin can accept");
        self.admin = pending;
        self.pending_admin = None;
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    implementation: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpgradeToArgs {
    new_impl: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpgradeToAndCallArgs {
    new_impl: Address,
    init_data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChangeAdminArgs {
    new_admin: Address,
}

pub fn dispatch(
    state: &mut Option<ProxyState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC16: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC16: bad init args");
            *state = Some(ProxyState::new(a.implementation, caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "implementation" => {
            let s = state.as_ref().expect("DRC16: not initialised");
            serde_json::to_vec(&s.implementation()).unwrap()
        }
        "proxy_admin" => {
            let s = state.as_ref().expect("DRC16: not initialised");
            serde_json::to_vec(&s.proxy_admin()).unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "upgrade_to" => {
            let s = state.as_mut().expect("DRC16: not initialised");
            let a: UpgradeToArgs =
                serde_json::from_slice(args).expect("DRC16: bad upgrade_to args");
            s.upgrade_to(caller, a.new_impl);
            serde_json::to_vec("ok").unwrap()
        }
        "upgrade_to_and_call" => {
            let s = state.as_mut().expect("DRC16: not initialised");
            let a: UpgradeToAndCallArgs =
                serde_json::from_slice(args).expect("DRC16: bad upgrade_to_and_call args");
            s.upgrade_to_and_call(caller, a.new_impl, a.init_data);
            serde_json::to_vec("ok").unwrap()
        }
        "change_admin" => {
            let s = state.as_mut().expect("DRC16: not initialised");
            let a: ChangeAdminArgs =
                serde_json::from_slice(args).expect("DRC16: bad change_admin args");
            s.change_admin(caller, a.new_admin);
            serde_json::to_vec("ok").unwrap()
        }
        "accept_admin" => {
            let s = state.as_mut().expect("DRC16: not initialised");
            s.accept_admin(caller);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC16: unknown method '{method}'"),
    }
}
