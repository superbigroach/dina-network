use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-102  Machine Capability Registry
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapabilityStatus {
    Online,
    Busy,
    Offline,
    Maintenance,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PricingModel {
    PerCall(u64),
    PerMinute(u64),
    PerUnit(u64, String),
    Negotiable,
    Free,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Capability {
    pub capability_type: String,
    pub version: String,
    pub status: CapabilityStatus,
    pub pricing: PricingModel,
    pub metadata: BTreeMap<String, String>,
    pub registered_at: u64,
    pub last_updated: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CapabilityRegistryState {
    /// device_id -> list of capabilities
    pub device_capabilities: BTreeMap<[u8; 32], Vec<Capability>>,
    /// capability_type -> list of device_ids that offer it
    pub capability_index: BTreeMap<String, Vec<[u8; 32]>>,
    pub admin: [u8; 32],
}

impl CapabilityRegistryState {
    pub fn new(admin: [u8; 32]) -> Self {
        Self {
            device_capabilities: BTreeMap::new(),
            capability_index: BTreeMap::new(),
            admin,
        }
    }

    pub fn register_capabilities(
        &mut self,
        caller: [u8; 32],
        device_id: [u8; 32],
        capabilities: Vec<Capability>,
    ) {
        // The caller must be the device owner. In a real deployment this would
        // cross-check DRC-2, but here we trust the caller == device controller.
        assert!(
            !capabilities.is_empty(),
            "DRC102: must register at least one capability"
        );

        let existing = self
            .device_capabilities
            .entry(device_id)
            .or_default();

        for cap in &capabilities {
            // Update the reverse index
            let entry = self
                .capability_index
                .entry(cap.capability_type.clone())
                .or_default();
            if !entry.contains(&device_id) {
                entry.push(device_id);
            }
        }

        existing.extend(capabilities);
        let _ = caller; // used for auth context
    }

    pub fn find_by_capability(&self, capability_type: &str) -> Vec<([u8; 32], Vec<&Capability>)> {
        let device_ids = match self.capability_index.get(capability_type) {
            Some(ids) => ids,
            None => return Vec::new(),
        };

        device_ids
            .iter()
            .filter_map(|id| {
                let caps = self.device_capabilities.get(id)?;
                let matching: Vec<&Capability> = caps
                    .iter()
                    .filter(|c| c.capability_type == capability_type)
                    .collect();
                if matching.is_empty() {
                    None
                } else {
                    Some((*id, matching))
                }
            })
            .collect()
    }

    pub fn capabilities_of(&self, device_id: &[u8; 32]) -> Vec<&Capability> {
        self.device_capabilities
            .get(device_id)
            .map(|caps| caps.iter().collect())
            .unwrap_or_default()
    }

    pub fn has_capability(&self, device_id: &[u8; 32], capability_type: &str) -> bool {
        self.device_capabilities
            .get(device_id)
            .map(|caps| caps.iter().any(|c| c.capability_type == capability_type))
            .unwrap_or(false)
    }

    pub fn update_status(
        &mut self,
        caller: [u8; 32],
        device_id: [u8; 32],
        capability_type: String,
        new_status: CapabilityStatus,
        timestamp: u64,
    ) {
        let caps = self
            .device_capabilities
            .get_mut(&device_id)
            .expect("DRC102: device has no capabilities");

        let mut found = false;
        for cap in caps.iter_mut() {
            if cap.capability_type == capability_type {
                cap.status = new_status.clone();
                cap.last_updated = timestamp;
                found = true;
            }
        }
        assert!(found, "DRC102: capability '{capability_type}' not found on device");
        let _ = caller;
    }

    pub fn set_pricing(
        &mut self,
        caller: [u8; 32],
        device_id: [u8; 32],
        capability_type: String,
        pricing: PricingModel,
        timestamp: u64,
    ) {
        let caps = self
            .device_capabilities
            .get_mut(&device_id)
            .expect("DRC102: device has no capabilities");

        let mut found = false;
        for cap in caps.iter_mut() {
            if cap.capability_type == capability_type {
                cap.pricing = pricing.clone();
                cap.last_updated = timestamp;
                found = true;
            }
        }
        assert!(found, "DRC102: capability '{capability_type}' not found on device");
        let _ = caller;
    }
}

// ---------------------------------------------------------------------------
// Dispatch arg types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct RegisterCapabilitiesArgs {
    device_id: [u8; 32],
    capabilities: Vec<Capability>,
}

#[derive(Serialize, Deserialize, Debug)]
struct FindByCapabilityArgs {
    capability_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeviceIdArgs {
    device_id: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct HasCapabilityArgs {
    device_id: [u8; 32],
    capability_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateStatusArgs {
    device_id: [u8; 32],
    capability_type: String,
    new_status: CapabilityStatus,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetPricingArgs {
    device_id: [u8; 32],
    capability_type: String,
    pricing: PricingModel,
    timestamp: u64,
}

// ---------------------------------------------------------------------------
// Serializable search result for dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CapabilitySearchResult {
    device_id: [u8; 32],
    capabilities: Vec<Capability>,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<CapabilityRegistryState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC102: already initialised");
            *state = Some(CapabilityRegistryState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "register_capabilities" => {
            let s = state.as_mut().expect("DRC102: not initialised");
            let a: RegisterCapabilitiesArgs =
                serde_json::from_slice(args).expect("DRC102: bad register_capabilities args");
            s.register_capabilities(caller, a.device_id, a.capabilities);
            serde_json::to_vec("ok").unwrap()
        }

        "find_by_capability" => {
            let s = state.as_ref().expect("DRC102: not initialised");
            let a: FindByCapabilityArgs =
                serde_json::from_slice(args).expect("DRC102: bad find_by_capability args");
            let results: Vec<CapabilitySearchResult> = s
                .find_by_capability(&a.capability_type)
                .into_iter()
                .map(|(device_id, caps)| CapabilitySearchResult {
                    device_id,
                    capabilities: caps.into_iter().cloned().collect(),
                })
                .collect();
            serde_json::to_vec(&results).unwrap()
        }

        "capabilities_of" => {
            let s = state.as_ref().expect("DRC102: not initialised");
            let a: DeviceIdArgs =
                serde_json::from_slice(args).expect("DRC102: bad capabilities_of args");
            let caps: Vec<&Capability> = s.capabilities_of(&a.device_id);
            let owned: Vec<Capability> = caps.into_iter().cloned().collect();
            serde_json::to_vec(&owned).unwrap()
        }

        "has_capability" => {
            let s = state.as_ref().expect("DRC102: not initialised");
            let a: HasCapabilityArgs =
                serde_json::from_slice(args).expect("DRC102: bad has_capability args");
            serde_json::to_vec(&s.has_capability(&a.device_id, &a.capability_type)).unwrap()
        }

        "update_status" => {
            let s = state.as_mut().expect("DRC102: not initialised");
            let a: UpdateStatusArgs =
                serde_json::from_slice(args).expect("DRC102: bad update_status args");
            s.update_status(caller, a.device_id, a.capability_type, a.new_status, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }

        "set_pricing" => {
            let s = state.as_mut().expect("DRC102: not initialised");
            let a: SetPricingArgs =
                serde_json::from_slice(args).expect("DRC102: bad set_pricing args");
            s.set_pricing(caller, a.device_id, a.capability_type, a.pricing, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC102: unknown method '{method}'"),
    }
}
