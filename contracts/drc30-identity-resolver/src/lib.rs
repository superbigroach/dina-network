use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-30  Universal Identity Resolver
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IdentityState {
    pub names: BTreeMap<String, Address>,
    pub addresses: BTreeMap<Address, String>,
    pub device_links: BTreeMap<Address, Vec<Address>>,
    pub name_owners: BTreeMap<String, Address>,
}

impl Default for IdentityState {
    fn default() -> Self {
        Self::new()
    }
}

impl IdentityState {
    pub fn new() -> Self {
        Self {
            names: BTreeMap::new(),
            addresses: BTreeMap::new(),
            device_links: BTreeMap::new(),
            name_owners: BTreeMap::new(),
        }
    }

    // -- Mutations -----------------------------------------------------------

    pub fn register_name(&mut self, caller: Address, name: String) {
        assert!(!name.is_empty(), "DRC30: name must not be empty");
        assert!(
            !self.names.contains_key(&name),
            "DRC30: name already registered"
        );
        assert!(
            !self.addresses.contains_key(&caller),
            "DRC30: address already has a name"
        );

        self.names.insert(name.clone(), caller);
        self.addresses.insert(caller, name.clone());
        self.name_owners.insert(name, caller);
    }

    pub fn transfer_name(&mut self, caller: Address, name: String, new_owner: Address) {
        let owner = self.name_owners.get(&name).expect("DRC30: name not found");
        assert!(*owner == caller, "DRC30: only name owner can transfer");
        assert!(
            !self.addresses.contains_key(&new_owner),
            "DRC30: new owner already has a name"
        );

        // Remove old mappings
        self.addresses.remove(&caller);
        // Transfer device links to new owner
        let devices = self.device_links.remove(&caller).unwrap_or_default();

        // Set new mappings
        self.names.insert(name.clone(), new_owner);
        self.addresses.insert(new_owner, name.clone());
        self.name_owners.insert(name, new_owner);
        if !devices.is_empty() {
            self.device_links.insert(new_owner, devices);
        }
    }

    pub fn link_device(&mut self, caller: Address, device: Address) {
        assert!(
            self.addresses.contains_key(&caller),
            "DRC30: caller has no registered name"
        );
        let devices = self.device_links.entry(caller).or_default();
        assert!(!devices.contains(&device), "DRC30: device already linked");
        devices.push(device);
    }

    // -- Queries -------------------------------------------------------------

    pub fn resolve_name(&self, name: &str) -> Address {
        *self.names.get(name).expect("DRC30: name not found")
    }

    pub fn reverse_resolve(&self, addr: &Address) -> &str {
        self.addresses
            .get(addr)
            .map(|s| s.as_str())
            .expect("DRC30: address has no name")
    }

    pub fn devices_of(&self, addr: &Address) -> &[Address] {
        self.device_links
            .get(addr)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct RegisterNameArgs {
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ResolveNameArgs {
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddrArgs {
    addr: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct LinkDeviceArgs {
    device: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferNameArgs {
    name: String,
    new_owner: Address,
}

pub fn dispatch(
    state: &mut Option<IdentityState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC30: already initialised");
            *state = Some(IdentityState::new());
            serde_json::to_vec("ok").unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "register_name" => {
            let s = state.as_mut().expect("DRC30: not initialised");
            let a: RegisterNameArgs =
                serde_json::from_slice(args).expect("DRC30: bad register_name args");
            s.register_name(caller, a.name);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer_name" => {
            let s = state.as_mut().expect("DRC30: not initialised");
            let a: TransferNameArgs =
                serde_json::from_slice(args).expect("DRC30: bad transfer_name args");
            s.transfer_name(caller, a.name, a.new_owner);
            serde_json::to_vec("ok").unwrap()
        }
        "link_device" => {
            let s = state.as_mut().expect("DRC30: not initialised");
            let a: LinkDeviceArgs =
                serde_json::from_slice(args).expect("DRC30: bad link_device args");
            s.link_device(caller, a.device);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "resolve_name" => {
            let s = state.as_ref().expect("DRC30: not initialised");
            let a: ResolveNameArgs =
                serde_json::from_slice(args).expect("DRC30: bad resolve_name args");
            let addr = s.resolve_name(&a.name);
            serde_json::to_vec(&addr).unwrap()
        }
        "reverse_resolve" => {
            let s = state.as_ref().expect("DRC30: not initialised");
            let a: AddrArgs =
                serde_json::from_slice(args).expect("DRC30: bad reverse_resolve args");
            let name = s.reverse_resolve(&a.addr);
            serde_json::to_vec(name).unwrap()
        }
        "devices_of" => {
            let s = state.as_ref().expect("DRC30: not initialised");
            let a: AddrArgs = serde_json::from_slice(args).expect("DRC30: bad devices_of args");
            let devices = s.devices_of(&a.addr);
            serde_json::to_vec(devices).unwrap()
        }

        _ => panic!("DRC30: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(seed: u8) -> Address {
        [seed; 32]
    }

    fn init(state: &mut Option<IdentityState>) {
        dispatch(state, "init", b"{}", addr(0));
    }

    #[test]
    fn test_register_and_resolve() {
        let mut state = None;
        init(&mut state);
        let user = addr(1);

        dispatch(
            &mut state,
            "register_name",
            &serde_json::to_vec(&RegisterNameArgs {
                name: "alice.dina".to_string(),
            })
            .unwrap(),
            user,
        );

        // Forward resolve
        let result = dispatch(
            &mut state,
            "resolve_name",
            &serde_json::to_vec(&ResolveNameArgs {
                name: "alice.dina".to_string(),
            })
            .unwrap(),
            addr(99),
        );
        let resolved: Address = serde_json::from_slice(&result).unwrap();
        assert_eq!(resolved, user);

        // Reverse resolve
        let result = dispatch(
            &mut state,
            "reverse_resolve",
            &serde_json::to_vec(&AddrArgs { addr: user }).unwrap(),
            addr(99),
        );
        let name: String = serde_json::from_slice(&result).unwrap();
        assert_eq!(name, "alice.dina");
    }

    #[test]
    fn test_link_and_query_devices() {
        let mut state = None;
        init(&mut state);
        let user = addr(1);
        let dev1 = addr(10);
        let dev2 = addr(11);

        dispatch(
            &mut state,
            "register_name",
            &serde_json::to_vec(&RegisterNameArgs {
                name: "bob.dina".to_string(),
            })
            .unwrap(),
            user,
        );

        dispatch(
            &mut state,
            "link_device",
            &serde_json::to_vec(&LinkDeviceArgs { device: dev1 }).unwrap(),
            user,
        );
        dispatch(
            &mut state,
            "link_device",
            &serde_json::to_vec(&LinkDeviceArgs { device: dev2 }).unwrap(),
            user,
        );

        let result = dispatch(
            &mut state,
            "devices_of",
            &serde_json::to_vec(&AddrArgs { addr: user }).unwrap(),
            addr(99),
        );
        let devices: Vec<Address> = serde_json::from_slice(&result).unwrap();
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0], dev1);
        assert_eq!(devices[1], dev2);
    }

    #[test]
    fn test_transfer_name() {
        let mut state = None;
        init(&mut state);
        let alice = addr(1);
        let bob = addr(2);

        dispatch(
            &mut state,
            "register_name",
            &serde_json::to_vec(&RegisterNameArgs {
                name: "cool.dina".to_string(),
            })
            .unwrap(),
            alice,
        );

        // Link a device before transfer
        dispatch(
            &mut state,
            "link_device",
            &serde_json::to_vec(&LinkDeviceArgs { device: addr(10) }).unwrap(),
            alice,
        );

        // Transfer
        dispatch(
            &mut state,
            "transfer_name",
            &serde_json::to_vec(&TransferNameArgs {
                name: "cool.dina".to_string(),
                new_owner: bob,
            })
            .unwrap(),
            alice,
        );

        // Verify new owner resolves
        let result = dispatch(
            &mut state,
            "resolve_name",
            &serde_json::to_vec(&ResolveNameArgs {
                name: "cool.dina".to_string(),
            })
            .unwrap(),
            addr(99),
        );
        let resolved: Address = serde_json::from_slice(&result).unwrap();
        assert_eq!(resolved, bob);

        // Verify devices transferred
        let result = dispatch(
            &mut state,
            "devices_of",
            &serde_json::to_vec(&AddrArgs { addr: bob }).unwrap(),
            addr(99),
        );
        let devices: Vec<Address> = serde_json::from_slice(&result).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0], addr(10));

        // Alice has no devices
        let result = dispatch(
            &mut state,
            "devices_of",
            &serde_json::to_vec(&AddrArgs { addr: alice }).unwrap(),
            addr(99),
        );
        let devices: Vec<Address> = serde_json::from_slice(&result).unwrap();
        assert_eq!(devices.len(), 0);
    }

    #[test]
    #[should_panic(expected = "DRC30: name already registered")]
    fn test_duplicate_name_panics() {
        let mut state = None;
        init(&mut state);

        dispatch(
            &mut state,
            "register_name",
            &serde_json::to_vec(&RegisterNameArgs {
                name: "unique.dina".to_string(),
            })
            .unwrap(),
            addr(1),
        );
        dispatch(
            &mut state,
            "register_name",
            &serde_json::to_vec(&RegisterNameArgs {
                name: "unique.dina".to_string(),
            })
            .unwrap(),
            addr(2),
        );
    }

    #[test]
    #[should_panic(expected = "DRC30: only name owner can transfer")]
    fn test_non_owner_cannot_transfer() {
        let mut state = None;
        init(&mut state);

        dispatch(
            &mut state,
            "register_name",
            &serde_json::to_vec(&RegisterNameArgs {
                name: "mine.dina".to_string(),
            })
            .unwrap(),
            addr(1),
        );

        dispatch(
            &mut state,
            "transfer_name",
            &serde_json::to_vec(&TransferNameArgs {
                name: "mine.dina".to_string(),
                new_owner: addr(3),
            })
            .unwrap(),
            addr(2), // not the owner
        );
    }

    #[test]
    #[should_panic(expected = "DRC30: device already linked")]
    fn test_duplicate_device_link_panics() {
        let mut state = None;
        init(&mut state);
        let user = addr(1);
        let device = addr(10);

        dispatch(
            &mut state,
            "register_name",
            &serde_json::to_vec(&RegisterNameArgs {
                name: "test.dina".to_string(),
            })
            .unwrap(),
            user,
        );

        dispatch(
            &mut state,
            "link_device",
            &serde_json::to_vec(&LinkDeviceArgs { device }).unwrap(),
            user,
        );
        dispatch(
            &mut state,
            "link_device",
            &serde_json::to_vec(&LinkDeviceArgs { device }).unwrap(),
            user,
        );
    }
}
