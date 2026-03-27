use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Dev-Factory — Deploy contracts from registered templates
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeployedContract {
    pub address: String,
    pub template: String,
    pub deployer: String,
    pub timestamp: u64,
    pub init_args: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FactoryState {
    pub owner: String,
    pub templates: HashMap<String, Vec<u8>>,
    pub deployed: Vec<DeployedContract>,
    pub deploy_count: u64,
}

impl FactoryState {
    pub fn new(owner: String) -> Self {
        Self {
            owner,
            templates: HashMap::new(),
            deployed: Vec::new(),
            deploy_count: 0,
        }
    }

    /// Register a new contract template (WASM bytecode). Owner only.
    pub fn register_template(&mut self, caller: &str, name: String, wasm_code: Vec<u8>) {
        assert!(
            caller == self.owner,
            "Factory: only owner can register templates"
        );
        assert!(!name.is_empty(), "Factory: template name cannot be empty");
        assert!(
            !wasm_code.is_empty(),
            "Factory: template code cannot be empty"
        );
        self.templates.insert(name, wasm_code);
    }

    /// Deploy an instance from a registered template. Anyone can deploy.
    /// Returns the derived address of the deployed contract.
    pub fn deploy_from_template(
        &mut self,
        caller: &str,
        template_name: &str,
        init_args: Vec<u8>,
        current_time: u64,
    ) -> String {
        assert!(
            self.templates.contains_key(template_name),
            "Factory: template '{}' not found",
            template_name
        );

        self.deploy_count += 1;

        // Derive a deterministic address from deployer + count
        let address = format!(
            "dina1_factory_{}_{}_{:016x}",
            template_name, self.deploy_count, current_time
        );

        let deployed = DeployedContract {
            address: address.clone(),
            template: template_name.to_string(),
            deployer: caller.to_string(),
            timestamp: current_time,
            init_args,
        };
        self.deployed.push(deployed);
        address
    }

    pub fn get_deployed(&self, index: usize) -> Option<&DeployedContract> {
        self.deployed.get(index)
    }

    pub fn get_template_count(&self) -> usize {
        self.templates.len()
    }

    pub fn get_deploy_count(&self) -> u64 {
        self.deploy_count
    }

    pub fn has_template(&self, name: &str) -> bool {
        self.templates.contains_key(name)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    owner: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RegisterTemplateArgs {
    name: String,
    wasm_code: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeployArgs {
    template_name: String,
    init_args: Vec<u8>,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetDeployedArgs {
    index: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct HasTemplateArgs {
    name: String,
}

pub fn dispatch(
    state: &mut Option<FactoryState>,
    method: &str,
    args: &[u8],
    caller: &str,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "Factory: already initialised");
            let a: InitArgs =
                serde_json::from_slice(args).expect("Factory: bad init args");
            *state = Some(FactoryState::new(a.owner));
            serde_json::to_vec("ok").unwrap()
        }

        "register_template" => {
            let s = state.as_mut().expect("Factory: not initialised");
            let a: RegisterTemplateArgs =
                serde_json::from_slice(args).expect("Factory: bad register_template args");
            s.register_template(caller, a.name, a.wasm_code);
            serde_json::to_vec("ok").unwrap()
        }

        "deploy_from_template" => {
            let s = state.as_mut().expect("Factory: not initialised");
            let a: DeployArgs =
                serde_json::from_slice(args).expect("Factory: bad deploy args");
            let address =
                s.deploy_from_template(caller, &a.template_name, a.init_args, a.current_time);
            serde_json::to_vec(&address).unwrap()
        }

        "get_deployed" => {
            let s = state.as_ref().expect("Factory: not initialised");
            let a: GetDeployedArgs =
                serde_json::from_slice(args).expect("Factory: bad get_deployed args");
            serde_json::to_vec(&s.get_deployed(a.index)).unwrap()
        }

        "get_template_count" => {
            let s = state.as_ref().expect("Factory: not initialised");
            serde_json::to_vec(&s.get_template_count()).unwrap()
        }

        "get_deploy_count" => {
            let s = state.as_ref().expect("Factory: not initialised");
            serde_json::to_vec(&s.get_deploy_count()).unwrap()
        }

        "has_template" => {
            let s = state.as_ref().expect("Factory: not initialised");
            let a: HasTemplateArgs =
                serde_json::from_slice(args).expect("Factory: bad has_template args");
            serde_json::to_vec(&s.has_template(&a.name)).unwrap()
        }

        _ => panic!("Factory: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: &str = "owner_addr";
    const ALICE: &str = "alice_addr";
    const BOB: &str = "bob_addr";

    fn init() -> Option<FactoryState> {
        let mut state = None;
        let args = serde_json::to_vec(&InitArgs {
            owner: OWNER.to_string(),
        })
        .unwrap();
        dispatch(&mut state, "init", &args, OWNER);
        state
    }

    fn register(state: &mut Option<FactoryState>, name: &str) {
        let args = serde_json::to_vec(&RegisterTemplateArgs {
            name: name.to_string(),
            wasm_code: vec![0x00, 0x61, 0x73, 0x6d], // WASM magic bytes
        })
        .unwrap();
        dispatch(state, "register_template", &args, OWNER);
    }

    #[test]
    fn test_register_and_deploy() {
        let mut state = init();
        register(&mut state, "erc20");

        let args = serde_json::to_vec(&DeployArgs {
            template_name: "erc20".to_string(),
            init_args: b"init_data".to_vec(),
            current_time: 1000,
        })
        .unwrap();
        let result = dispatch(&mut state, "deploy_from_template", &args, ALICE);
        let address: String = serde_json::from_slice(&result).unwrap();
        assert!(address.contains("erc20"));

        assert_eq!(state.as_ref().unwrap().deploy_count, 1);
    }

    #[test]
    fn test_multiple_deploys() {
        let mut state = init();
        register(&mut state, "token");

        for i in 0..3 {
            let args = serde_json::to_vec(&DeployArgs {
                template_name: "token".to_string(),
                init_args: vec![i as u8],
                current_time: 1000 + i,
            })
            .unwrap();
            dispatch(&mut state, "deploy_from_template", &args, ALICE);
        }

        let result = dispatch(&mut state, "get_deploy_count", b"", ALICE);
        let count: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    #[should_panic(expected = "Factory: only owner can register templates")]
    fn test_non_owner_cannot_register() {
        let mut state = init();
        let args = serde_json::to_vec(&RegisterTemplateArgs {
            name: "token".to_string(),
            wasm_code: vec![1, 2, 3],
        })
        .unwrap();
        dispatch(&mut state, "register_template", &args, ALICE);
    }

    #[test]
    #[should_panic(expected = "Factory: template 'missing' not found")]
    fn test_deploy_missing_template() {
        let mut state = init();
        let args = serde_json::to_vec(&DeployArgs {
            template_name: "missing".to_string(),
            init_args: vec![],
            current_time: 1000,
        })
        .unwrap();
        dispatch(&mut state, "deploy_from_template", &args, ALICE);
    }

    #[test]
    fn test_get_deployed() {
        let mut state = init();
        register(&mut state, "nft");

        let args = serde_json::to_vec(&DeployArgs {
            template_name: "nft".to_string(),
            init_args: b"my_nft".to_vec(),
            current_time: 2000,
        })
        .unwrap();
        dispatch(&mut state, "deploy_from_template", &args, BOB);

        let get_args = serde_json::to_vec(&GetDeployedArgs { index: 0 }).unwrap();
        let result = dispatch(&mut state, "get_deployed", &get_args, ALICE);
        let deployed: Option<DeployedContract> = serde_json::from_slice(&result).unwrap();
        let d = deployed.unwrap();
        assert_eq!(d.template, "nft");
        assert_eq!(d.deployer, BOB);
        assert_eq!(d.timestamp, 2000);
    }

    #[test]
    fn test_template_count() {
        let mut state = init();
        register(&mut state, "token");
        register(&mut state, "nft");
        register(&mut state, "vault");

        let result = dispatch(&mut state, "get_template_count", b"", ALICE);
        let count: usize = serde_json::from_slice(&result).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_has_template() {
        let mut state = init();
        register(&mut state, "token");

        let args = serde_json::to_vec(&HasTemplateArgs {
            name: "token".to_string(),
        })
        .unwrap();
        let result = dispatch(&mut state, "has_template", &args, ALICE);
        let exists: bool = serde_json::from_slice(&result).unwrap();
        assert!(exists);

        let args = serde_json::to_vec(&HasTemplateArgs {
            name: "missing".to_string(),
        })
        .unwrap();
        let result = dispatch(&mut state, "has_template", &args, ALICE);
        let exists: bool = serde_json::from_slice(&result).unwrap();
        assert!(!exists);
    }
}
