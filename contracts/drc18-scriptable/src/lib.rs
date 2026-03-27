use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-18  Scriptable Tokens
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScriptableTokenState {
    pub owner: Address,
    pub scripts: BTreeMap<u64, Vec<String>>,
    pub creators: BTreeMap<u64, Address>,
    pub next_token_id: u64,
}

impl ScriptableTokenState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            scripts: BTreeMap::new(),
            creators: BTreeMap::new(),
            next_token_id: 1,
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn script_uri(&self, token_id: u64) -> Vec<String> {
        self.scripts.get(&token_id).cloned().unwrap_or_default()
    }

    // -- Mutations -----------------------------------------------------------

    pub fn create_token(&mut self, caller: Address) -> u64 {
        let token_id = self.next_token_id;
        self.next_token_id += 1;
        self.creators.insert(token_id, caller);
        self.scripts.insert(token_id, Vec::new());
        token_id
    }

    pub fn set_script_uri(&mut self, caller: Address, token_id: u64, scripts: Vec<String>) {
        let creator = self
            .creators
            .get(&token_id)
            .expect("DRC18: token does not exist");
        assert!(
            caller == *creator,
            "DRC18: only creator can set script URI"
        );
        self.scripts.insert(token_id, scripts);
    }

    pub fn add_script(&mut self, caller: Address, token_id: u64, script_uri: String) {
        let creator = self
            .creators
            .get(&token_id)
            .expect("DRC18: token does not exist");
        assert!(caller == *creator, "DRC18: only creator can add script");
        self.scripts
            .entry(token_id)
            .or_insert_with(Vec::new)
            .push(script_uri);
    }

    pub fn remove_script(&mut self, caller: Address, token_id: u64, index: usize) {
        let creator = self
            .creators
            .get(&token_id)
            .expect("DRC18: token does not exist");
        assert!(
            caller == *creator,
            "DRC18: only creator can remove script"
        );
        let scripts = self
            .scripts
            .get_mut(&token_id)
            .expect("DRC18: no scripts for token");
        assert!(index < scripts.len(), "DRC18: script index out of bounds");
        scripts.remove(index);
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct ScriptUriArgs {
    token_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetScriptUriArgs {
    token_id: u64,
    scripts: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddScriptArgs {
    token_id: u64,
    script_uri: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RemoveScriptArgs {
    token_id: u64,
    index: usize,
}

pub fn dispatch(
    state: &mut Option<ScriptableTokenState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC18: already initialised");
            *state = Some(ScriptableTokenState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "script_uri" => {
            let s = state.as_ref().expect("DRC18: not initialised");
            let a: ScriptUriArgs =
                serde_json::from_slice(args).expect("DRC18: bad script_uri args");
            serde_json::to_vec(&s.script_uri(a.token_id)).unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "create_token" => {
            let s = state.as_mut().expect("DRC18: not initialised");
            let id = s.create_token(caller);
            serde_json::to_vec(&id).unwrap()
        }
        "set_script_uri" => {
            let s = state.as_mut().expect("DRC18: not initialised");
            let a: SetScriptUriArgs =
                serde_json::from_slice(args).expect("DRC18: bad set_script_uri args");
            s.set_script_uri(caller, a.token_id, a.scripts);
            serde_json::to_vec("ok").unwrap()
        }
        "add_script" => {
            let s = state.as_mut().expect("DRC18: not initialised");
            let a: AddScriptArgs =
                serde_json::from_slice(args).expect("DRC18: bad add_script args");
            s.add_script(caller, a.token_id, a.script_uri);
            serde_json::to_vec("ok").unwrap()
        }
        "remove_script" => {
            let s = state.as_mut().expect("DRC18: not initialised");
            let a: RemoveScriptArgs =
                serde_json::from_slice(args).expect("DRC18: bad remove_script args");
            s.remove_script(caller, a.token_id, a.index);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC18: unknown method '{method}'"),
    }
}
