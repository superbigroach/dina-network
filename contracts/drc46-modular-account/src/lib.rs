use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-46  Modular Accounts  (ERC-6900 equivalent)
// Smart accounts with installable plugins/modules.
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModularAccountState {
    pub admin: Address,
    /// Account owners.
    pub accounts: BTreeMap<Address, AccountInfo>,
    /// (account, module_name) -> ModuleInfo
    pub installed_modules: BTreeMap<(Address, String), ModuleInfo>,
    /// Log of module executions.
    pub execution_log: Vec<ExecutionRecord>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AccountInfo {
    pub owner: Address,
    pub created_at: u64,
    pub nonce: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModuleInfo {
    pub name: String,
    pub version: String,
    pub permissions: Vec<String>,
    pub installed_at: u64,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExecutionRecord {
    pub account: Address,
    pub module: String,
    pub method: String,
    pub args: Vec<u8>,
    pub timestamp: u64,
}

impl ModularAccountState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            accounts: BTreeMap::new(),
            installed_modules: BTreeMap::new(),
            execution_log: Vec::new(),
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn has_module(&self, account: &Address, module_name: &str) -> bool {
        self.installed_modules
            .get(&(*account, module_name.to_string()))
            .map_or(false, |m| m.active)
    }

    pub fn list_modules(&self, account: &Address) -> Vec<&ModuleInfo> {
        self.installed_modules
            .iter()
            .filter(|((a, _), m)| a == account && m.active)
            .map(|(_, m)| m)
            .collect()
    }

    pub fn get_module(&self, account: &Address, module_name: &str) -> Option<&ModuleInfo> {
        self.installed_modules
            .get(&(*account, module_name.to_string()))
    }

    // -- Mutations -----------------------------------------------------------

    pub fn create_account(&mut self, caller: Address, timestamp: u64) {
        assert!(
            !self.accounts.contains_key(&caller),
            "DRC46: account already exists"
        );
        self.accounts.insert(
            caller,
            AccountInfo {
                owner: caller,
                created_at: timestamp,
                nonce: 0,
            },
        );
    }

    pub fn install_module(
        &mut self,
        caller: Address,
        account: Address,
        module_name: String,
        version: String,
        permissions: Vec<String>,
        timestamp: u64,
    ) {
        let acc = self
            .accounts
            .get(&account)
            .expect("DRC46: account not found");
        assert!(
            acc.owner == caller,
            "DRC46: only account owner can install modules"
        );
        assert!(
            !self.has_module(&account, &module_name),
            "DRC46: module already installed"
        );
        self.installed_modules.insert(
            (account, module_name.clone()),
            ModuleInfo {
                name: module_name,
                version,
                permissions,
                installed_at: timestamp,
                active: true,
            },
        );
    }

    pub fn uninstall_module(&mut self, caller: Address, account: Address, module_name: String) {
        let acc = self
            .accounts
            .get(&account)
            .expect("DRC46: account not found");
        assert!(
            acc.owner == caller,
            "DRC46: only account owner can uninstall"
        );
        let module = self
            .installed_modules
            .get_mut(&(account, module_name.clone()))
            .expect("DRC46: module not found");
        assert!(module.active, "DRC46: module already inactive");
        module.active = false;
    }

    pub fn execute_via_module(
        &mut self,
        caller: Address,
        account: Address,
        module_name: String,
        exec_method: String,
        exec_args: Vec<u8>,
        timestamp: u64,
    ) -> Vec<u8> {
        let acc = self
            .accounts
            .get_mut(&account)
            .expect("DRC46: account not found");
        assert!(acc.owner == caller, "DRC46: only account owner can execute");

        let module = self
            .installed_modules
            .get(&(account, module_name.clone()))
            .expect("DRC46: module not found");
        assert!(module.active, "DRC46: module is not active");

        // Check permission
        assert!(
            module.permissions.contains(&exec_method)
                || module.permissions.contains(&"*".to_string()),
            "DRC46: module does not have permission for '{exec_method}'"
        );

        acc.nonce += 1;

        self.execution_log.push(ExecutionRecord {
            account,
            module: module_name,
            method: exec_method,
            args: exec_args.clone(),
            timestamp,
        });

        // Return the args as a pass-through (real system would route to module logic)
        exec_args
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateAccountArgs {
    timestamp: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct InstallModuleArgs {
    account: Address,
    module_name: String,
    version: String,
    permissions: Vec<String>,
    timestamp: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct UninstallModuleArgs {
    account: Address,
    module_name: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct ExecuteArgs {
    account: Address,
    module_name: String,
    method: String,
    args: Vec<u8>,
    timestamp: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct HasModuleArgs {
    account: Address,
    module_name: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct ListModulesArgs {
    account: Address,
}

pub fn dispatch(
    state: &mut Option<ModularAccountState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC46: already initialised");
            *state = Some(ModularAccountState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "create_account" => {
            let s = state.as_mut().expect("DRC46: not initialised");
            let a: CreateAccountArgs = serde_json::from_slice(args).expect("DRC46: bad args");
            s.create_account(caller, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }
        "install_module" => {
            let s = state.as_mut().expect("DRC46: not initialised");
            let a: InstallModuleArgs = serde_json::from_slice(args).expect("DRC46: bad args");
            s.install_module(
                caller,
                a.account,
                a.module_name,
                a.version,
                a.permissions,
                a.timestamp,
            );
            serde_json::to_vec("ok").unwrap()
        }
        "uninstall_module" => {
            let s = state.as_mut().expect("DRC46: not initialised");
            let a: UninstallModuleArgs = serde_json::from_slice(args).expect("DRC46: bad args");
            s.uninstall_module(caller, a.account, a.module_name);
            serde_json::to_vec("ok").unwrap()
        }
        "execute_via_module" => {
            let s = state.as_mut().expect("DRC46: not initialised");
            let a: ExecuteArgs = serde_json::from_slice(args).expect("DRC46: bad args");
            let result = s.execute_via_module(
                caller,
                a.account,
                a.module_name,
                a.method,
                a.args,
                a.timestamp,
            );
            serde_json::to_vec(&result).unwrap()
        }
        "has_module" => {
            let s = state.as_ref().expect("DRC46: not initialised");
            let a: HasModuleArgs = serde_json::from_slice(args).expect("DRC46: bad args");
            serde_json::to_vec(&s.has_module(&a.account, &a.module_name)).unwrap()
        }
        "list_modules" => {
            let s = state.as_ref().expect("DRC46: not initialised");
            let a: ListModulesArgs = serde_json::from_slice(args).expect("DRC46: bad args");
            let modules = s.list_modules(&a.account);
            serde_json::to_vec(&modules).unwrap()
        }
        _ => panic!("DRC46: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(n: u8) -> Address {
        [n; 32]
    }

    fn setup() -> Option<ModularAccountState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", addr(1));
        // Create an account for addr(1)
        let ca = serde_json::to_vec(&CreateAccountArgs { timestamp: 1000 }).unwrap();
        dispatch(&mut state, "create_account", &ca, addr(1));
        state
    }

    #[test]
    fn test_install_and_list_modules() {
        let mut state = setup();
        let args = serde_json::to_vec(&InstallModuleArgs {
            account: addr(1),
            module_name: "token-guard".into(),
            version: "1.0.0".into(),
            permissions: vec!["transfer".into(), "approve".into()],
            timestamp: 2000,
        })
        .unwrap();
        dispatch(&mut state, "install_module", &args, addr(1));

        let s = state.as_ref().unwrap();
        assert!(s.has_module(&addr(1), "token-guard"));
        assert_eq!(s.list_modules(&addr(1)).len(), 1);
    }

    #[test]
    fn test_uninstall_module() {
        let mut state = setup();
        let install = serde_json::to_vec(&InstallModuleArgs {
            account: addr(1),
            module_name: "logger".into(),
            version: "1.0".into(),
            permissions: vec!["*".into()],
            timestamp: 2000,
        })
        .unwrap();
        dispatch(&mut state, "install_module", &install, addr(1));
        assert!(state.as_ref().unwrap().has_module(&addr(1), "logger"));

        let uninstall = serde_json::to_vec(&UninstallModuleArgs {
            account: addr(1),
            module_name: "logger".into(),
        })
        .unwrap();
        dispatch(&mut state, "uninstall_module", &uninstall, addr(1));
        assert!(!state.as_ref().unwrap().has_module(&addr(1), "logger"));
    }

    #[test]
    fn test_execute_via_module() {
        let mut state = setup();
        let install = serde_json::to_vec(&InstallModuleArgs {
            account: addr(1),
            module_name: "swap".into(),
            version: "2.0".into(),
            permissions: vec!["execute_swap".into()],
            timestamp: 3000,
        })
        .unwrap();
        dispatch(&mut state, "install_module", &install, addr(1));

        let exec = serde_json::to_vec(&ExecuteArgs {
            account: addr(1),
            module_name: "swap".into(),
            method: "execute_swap".into(),
            args: vec![1, 2, 3],
            timestamp: 4000,
        })
        .unwrap();
        dispatch(&mut state, "execute_via_module", &exec, addr(1));

        let s = state.as_ref().unwrap();
        assert_eq!(s.execution_log.len(), 1);
        assert_eq!(s.accounts.get(&addr(1)).unwrap().nonce, 1);
    }

    #[test]
    #[should_panic(expected = "does not have permission")]
    fn test_execute_without_permission() {
        let mut state = setup();
        let install = serde_json::to_vec(&InstallModuleArgs {
            account: addr(1),
            module_name: "limited".into(),
            version: "1.0".into(),
            permissions: vec!["read".into()],
            timestamp: 3000,
        })
        .unwrap();
        dispatch(&mut state, "install_module", &install, addr(1));

        let exec = serde_json::to_vec(&ExecuteArgs {
            account: addr(1),
            module_name: "limited".into(),
            method: "write".into(),
            args: vec![],
            timestamp: 4000,
        })
        .unwrap();
        dispatch(&mut state, "execute_via_module", &exec, addr(1));
    }

    #[test]
    #[should_panic(expected = "only account owner")]
    fn test_install_non_owner() {
        let mut state = setup();
        let args = serde_json::to_vec(&InstallModuleArgs {
            account: addr(1),
            module_name: "hacker".into(),
            version: "1.0".into(),
            permissions: vec!["*".into()],
            timestamp: 5000,
        })
        .unwrap();
        dispatch(&mut state, "install_module", &args, addr(99));
    }
}
