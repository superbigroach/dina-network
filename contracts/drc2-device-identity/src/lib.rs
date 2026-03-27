use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-2  Device Identity Registry
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeviceType {
    Sensor,
    Actuator,
    Robot,
    Drone,
    Vehicle,
    Gateway,
    Compute,
    Storage,
    Custom(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeviceMetadata {
    pub manufacturer: String,
    pub model: String,
    pub firmware_version: String,
    pub capabilities: Vec<String>,
    pub location: Option<String>,
    pub custom: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeviceAttestation {
    pub witness: [u8; 32],
    pub timestamp: u64,
    pub signature: Vec<u8>,
    pub attestation_data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeviceIdentity {
    pub device_id: [u8; 32],
    pub owner: [u8; 32],
    pub public_key: [u8; 32],
    pub device_type: DeviceType,
    pub metadata: DeviceMetadata,
    pub registered_at: u64,
    pub active: bool,
    pub attestations: Vec<DeviceAttestation>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeviceRegistryState {
    pub devices: BTreeMap<[u8; 32], DeviceIdentity>,
    pub pubkey_to_id: BTreeMap<[u8; 32], [u8; 32]>,
    pub owner_devices: BTreeMap<[u8; 32], Vec<[u8; 32]>>,
    pub total_devices: u64,
    pub admin: [u8; 32],
}

impl DeviceRegistryState {
    pub fn new(admin: [u8; 32]) -> Self {
        Self {
            devices: BTreeMap::new(),
            pubkey_to_id: BTreeMap::new(),
            owner_devices: BTreeMap::new(),
            total_devices: 0,
            admin,
        }
    }

    pub fn register_device(
        &mut self,
        caller: [u8; 32],
        device_id: [u8; 32],
        public_key: [u8; 32],
        device_type: DeviceType,
        metadata: DeviceMetadata,
        timestamp: u64,
    ) {
        assert!(
            !self.devices.contains_key(&device_id),
            "DRC2: device already registered"
        );
        assert!(
            !self.pubkey_to_id.contains_key(&public_key),
            "DRC2: public key already bound to another device"
        );

        let identity = DeviceIdentity {
            device_id,
            owner: caller,
            public_key,
            device_type,
            metadata,
            registered_at: timestamp,
            active: true,
            attestations: Vec::new(),
        };

        self.devices.insert(device_id, identity);
        self.pubkey_to_id.insert(public_key, device_id);
        self.owner_devices
            .entry(caller)
            .or_insert_with(Vec::new)
            .push(device_id);
        self.total_devices += 1;
    }

    pub fn resolve(&self, device_id: &[u8; 32]) -> Option<&DeviceIdentity> {
        self.devices.get(device_id)
    }

    pub fn resolve_by_pubkey(&self, pubkey: &[u8; 32]) -> Option<&DeviceIdentity> {
        self.pubkey_to_id
            .get(pubkey)
            .and_then(|id| self.devices.get(id))
    }

    pub fn update_metadata(
        &mut self,
        caller: [u8; 32],
        device_id: [u8; 32],
        metadata: DeviceMetadata,
    ) {
        let device = self
            .devices
            .get_mut(&device_id)
            .expect("DRC2: device not found");
        assert!(
            device.owner == caller,
            "DRC2: only owner can update metadata"
        );
        device.metadata = metadata;
    }

    pub fn verify_witness(
        &mut self,
        caller: [u8; 32],
        device_id: [u8; 32],
        attestation: DeviceAttestation,
    ) {
        let device = self
            .devices
            .get_mut(&device_id)
            .expect("DRC2: device not found");
        assert!(device.active, "DRC2: device is not active");
        assert!(
            attestation.witness == caller,
            "DRC2: caller must be the witness"
        );
        device.attestations.push(attestation);
    }

    pub fn revoke(&mut self, caller: [u8; 32], device_id: [u8; 32]) {
        let device = self
            .devices
            .get_mut(&device_id)
            .expect("DRC2: device not found");
        assert!(
            device.owner == caller || caller == self.admin,
            "DRC2: only owner or admin can revoke"
        );
        device.active = false;
    }

    pub fn is_active(&self, device_id: &[u8; 32]) -> bool {
        self.devices
            .get(device_id)
            .map(|d| d.active)
            .unwrap_or(false)
    }

    pub fn devices_of(&self, owner: &[u8; 32]) -> Vec<DeviceIdentity> {
        self.owner_devices
            .get(owner)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.devices.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Dispatch arg types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct RegisterDeviceArgs {
    device_id: [u8; 32],
    public_key: [u8; 32],
    device_type: DeviceType,
    metadata: DeviceMetadata,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeviceIdArgs {
    device_id: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct PubkeyArgs {
    public_key: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateMetadataArgs {
    device_id: [u8; 32],
    metadata: DeviceMetadata,
}

#[derive(Serialize, Deserialize, Debug)]
struct VerifyWitnessArgs {
    device_id: [u8; 32],
    attestation: DeviceAttestation,
}

#[derive(Serialize, Deserialize, Debug)]
struct OwnerArgs {
    owner: [u8; 32],
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<DeviceRegistryState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC2: already initialised");
            *state = Some(DeviceRegistryState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "register_device" => {
            let s = state.as_mut().expect("DRC2: not initialised");
            let a: RegisterDeviceArgs =
                serde_json::from_slice(args).expect("DRC2: bad register_device args");
            s.register_device(
                caller,
                a.device_id,
                a.public_key,
                a.device_type,
                a.metadata,
                a.timestamp,
            );
            serde_json::to_vec("ok").unwrap()
        }

        "resolve" => {
            let s = state.as_ref().expect("DRC2: not initialised");
            let a: DeviceIdArgs =
                serde_json::from_slice(args).expect("DRC2: bad resolve args");
            serde_json::to_vec(&s.resolve(&a.device_id)).unwrap()
        }

        "resolve_by_pubkey" => {
            let s = state.as_ref().expect("DRC2: not initialised");
            let a: PubkeyArgs =
                serde_json::from_slice(args).expect("DRC2: bad resolve_by_pubkey args");
            serde_json::to_vec(&s.resolve_by_pubkey(&a.public_key)).unwrap()
        }

        "update_metadata" => {
            let s = state.as_mut().expect("DRC2: not initialised");
            let a: UpdateMetadataArgs =
                serde_json::from_slice(args).expect("DRC2: bad update_metadata args");
            s.update_metadata(caller, a.device_id, a.metadata);
            serde_json::to_vec("ok").unwrap()
        }

        "verify_witness" => {
            let s = state.as_mut().expect("DRC2: not initialised");
            let a: VerifyWitnessArgs =
                serde_json::from_slice(args).expect("DRC2: bad verify_witness args");
            s.verify_witness(caller, a.device_id, a.attestation);
            serde_json::to_vec("ok").unwrap()
        }

        "revoke" => {
            let s = state.as_mut().expect("DRC2: not initialised");
            let a: DeviceIdArgs =
                serde_json::from_slice(args).expect("DRC2: bad revoke args");
            s.revoke(caller, a.device_id);
            serde_json::to_vec("ok").unwrap()
        }

        "is_active" => {
            let s = state.as_ref().expect("DRC2: not initialised");
            let a: DeviceIdArgs =
                serde_json::from_slice(args).expect("DRC2: bad is_active args");
            serde_json::to_vec(&s.is_active(&a.device_id)).unwrap()
        }

        "devices_of" => {
            let s = state.as_ref().expect("DRC2: not initialised");
            let a: OwnerArgs =
                serde_json::from_slice(args).expect("DRC2: bad devices_of args");
            serde_json::to_vec(&s.devices_of(&a.owner)).unwrap()
        }

        "total_devices" => {
            let s = state.as_ref().expect("DRC2: not initialised");
            serde_json::to_vec(&s.total_devices).unwrap()
        }

        _ => panic!("DRC2: unknown method '{method}'"),
    }
}
