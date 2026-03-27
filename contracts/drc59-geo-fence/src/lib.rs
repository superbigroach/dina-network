use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// DRC-59  Geographic Fence for Devices
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeoFence {
    pub id: u64,
    pub name: String,
    pub owner: Address,
    pub center_lat: f64,
    pub center_lng: f64,
    pub radius_meters: f64,
    pub allowed_devices: BTreeSet<Address>,
    /// If true, devices MUST stay inside. If false, devices MUST stay outside.
    pub restricted: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeviceLocation {
    pub device: Address,
    pub lat: f64,
    pub lng: f64,
    pub updated_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeoFenceState {
    pub owner: Address,
    pub fences: BTreeMap<u64, GeoFence>,
    pub device_locations: BTreeMap<Address, DeviceLocation>,
    pub next_fence_id: u64,
}

/// Haversine distance in meters between two lat/lng points.
fn haversine_meters(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const R: f64 = 6_371_000.0; // Earth radius in meters
    let d_lat = (lat2 - lat1).to_radians();
    let d_lng = (lng2 - lng1).to_radians();
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();
    let a = (d_lat / 2.0).sin().powi(2) + lat1_r.cos() * lat2_r.cos() * (d_lng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    R * c
}

impl GeoFenceState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            fences: BTreeMap::new(),
            device_locations: BTreeMap::new(),
            next_fence_id: 1,
        }
    }

    pub fn create_fence(
        &mut self,
        caller: Address,
        name: String,
        center_lat: f64,
        center_lng: f64,
        radius_meters: f64,
        restricted: bool,
    ) -> u64 {
        assert!(radius_meters > 0.0, "DRC59: radius must be positive");
        assert!(center_lat >= -90.0 && center_lat <= 90.0, "DRC59: invalid latitude");
        assert!(center_lng >= -180.0 && center_lng <= 180.0, "DRC59: invalid longitude");
        let id = self.next_fence_id;
        self.next_fence_id += 1;
        self.fences.insert(id, GeoFence {
            id,
            name,
            owner: caller,
            center_lat,
            center_lng,
            radius_meters,
            allowed_devices: BTreeSet::new(),
            restricted,
        });
        id
    }

    pub fn update_location(&mut self, device: Address, lat: f64, lng: f64, timestamp: u64) {
        self.device_locations.insert(device, DeviceLocation {
            device,
            lat,
            lng,
            updated_at: timestamp,
        });
    }

    pub fn is_within_fence(&self, device: &Address, fence_id: u64) -> bool {
        let fence = self.fences.get(&fence_id).expect("DRC59: fence not found");
        let loc = self.device_locations.get(device);
        if loc.is_none() { return false; }
        let loc = loc.unwrap();
        let dist = haversine_meters(fence.center_lat, fence.center_lng, loc.lat, loc.lng);
        dist <= fence.radius_meters
    }

    /// Check compliance: for restricted fences the device must be inside, for non-restricted it must stay outside.
    /// Returns Vec of (fence_id, compliant) tuples for all fences this device is enrolled in.
    pub fn check_all_fences(&self, device: &Address) -> Vec<(u64, bool)> {
        let mut results = Vec::new();
        for (fence_id, fence) in &self.fences {
            if !fence.allowed_devices.contains(device) { continue; }
            let inside = self.is_within_fence(device, *fence_id);
            let compliant = if fence.restricted { inside } else { !inside };
            results.push((*fence_id, compliant));
        }
        results
    }

    pub fn add_device_to_fence(&mut self, caller: Address, fence_id: u64, device: Address) {
        let fence = self.fences.get_mut(&fence_id).expect("DRC59: fence not found");
        assert!(caller == fence.owner || caller == self.owner, "DRC59: not authorised");
        fence.allowed_devices.insert(device);
    }

    pub fn remove_device_from_fence(&mut self, caller: Address, fence_id: u64, device: Address) {
        let fence = self.fences.get_mut(&fence_id).expect("DRC59: fence not found");
        assert!(caller == fence.owner || caller == self.owner, "DRC59: not authorised");
        fence.allowed_devices.remove(&device);
    }

    pub fn get_fence(&self, fence_id: u64) -> Option<&GeoFence> {
        self.fences.get(&fence_id)
    }

    pub fn get_location(&self, device: &Address) -> Option<&DeviceLocation> {
        self.device_locations.get(device)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateFenceArgs { name: String, center_lat: f64, center_lng: f64, radius_meters: f64, restricted: bool }

#[derive(Serialize, Deserialize, Debug)]
struct UpdateLocationArgs { device: Address, lat: f64, lng: f64, timestamp: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct FenceDeviceArgs { fence_id: u64, device: Address }

#[derive(Serialize, Deserialize, Debug)]
struct IsWithinArgs { device: Address, fence_id: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct CheckAllArgs { device: Address }

#[derive(Serialize, Deserialize, Debug)]
struct FenceIdArgs { fence_id: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct DeviceArgs { device: Address }

pub fn dispatch(
    state: &mut Option<GeoFenceState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC59: already initialised");
            *state = Some(GeoFenceState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "create_fence" => {
            let s = state.as_mut().expect("DRC59: not initialised");
            let a: CreateFenceArgs = serde_json::from_slice(args).expect("DRC59: bad args");
            let id = s.create_fence(caller, a.name, a.center_lat, a.center_lng, a.radius_meters, a.restricted);
            serde_json::to_vec(&id).unwrap()
        }
        "update_location" => {
            let s = state.as_mut().expect("DRC59: not initialised");
            let a: UpdateLocationArgs = serde_json::from_slice(args).expect("DRC59: bad args");
            s.update_location(a.device, a.lat, a.lng, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }
        "is_within_fence" => {
            let s = state.as_ref().expect("DRC59: not initialised");
            let a: IsWithinArgs = serde_json::from_slice(args).expect("DRC59: bad args");
            serde_json::to_vec(&s.is_within_fence(&a.device, a.fence_id)).unwrap()
        }
        "check_all_fences" => {
            let s = state.as_ref().expect("DRC59: not initialised");
            let a: CheckAllArgs = serde_json::from_slice(args).expect("DRC59: bad args");
            serde_json::to_vec(&s.check_all_fences(&a.device)).unwrap()
        }
        "add_device_to_fence" => {
            let s = state.as_mut().expect("DRC59: not initialised");
            let a: FenceDeviceArgs = serde_json::from_slice(args).expect("DRC59: bad args");
            s.add_device_to_fence(caller, a.fence_id, a.device);
            serde_json::to_vec("ok").unwrap()
        }
        "remove_device_from_fence" => {
            let s = state.as_mut().expect("DRC59: not initialised");
            let a: FenceDeviceArgs = serde_json::from_slice(args).expect("DRC59: bad args");
            s.remove_device_from_fence(caller, a.fence_id, a.device);
            serde_json::to_vec("ok").unwrap()
        }
        "get_fence" => {
            let s = state.as_ref().expect("DRC59: not initialised");
            let a: FenceIdArgs = serde_json::from_slice(args).expect("DRC59: bad args");
            serde_json::to_vec(&s.get_fence(a.fence_id)).unwrap()
        }
        "get_location" => {
            let s = state.as_ref().expect("DRC59: not initialised");
            let a: DeviceArgs = serde_json::from_slice(args).expect("DRC59: bad args");
            serde_json::to_vec(&s.get_location(&a.device)).unwrap()
        }
        _ => panic!("DRC59: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [0u8; 32];
    const DEVICE_A: Address = [1u8; 32];
    const DEVICE_B: Address = [2u8; 32];

    // Toronto: 43.6532, -79.3832
    // ~100m away: 43.6541, -79.3832

    #[test]
    fn test_haversine_known_distance() {
        let dist = haversine_meters(43.6532, -79.3832, 43.6541, -79.3832);
        assert!(dist > 90.0 && dist < 110.0, "expected ~100m, got {dist}");
    }

    #[test]
    fn test_create_fence_and_check_inside() {
        let mut s = GeoFenceState::new(OWNER);
        let fence_id = s.create_fence(OWNER, "Office".into(), 43.6532, -79.3832, 200.0, true);
        s.update_location(DEVICE_A, 43.6535, -79.3830, 100);
        assert!(s.is_within_fence(&DEVICE_A, fence_id));
    }

    #[test]
    fn test_device_outside_fence() {
        let mut s = GeoFenceState::new(OWNER);
        let fence_id = s.create_fence(OWNER, "Office".into(), 43.6532, -79.3832, 50.0, true);
        // Place device ~1km away
        s.update_location(DEVICE_A, 43.6632, -79.3832, 100);
        assert!(!s.is_within_fence(&DEVICE_A, fence_id));
    }

    #[test]
    fn test_check_all_fences_compliance() {
        let mut s = GeoFenceState::new(OWNER);
        // restricted=true: must be inside
        let f1 = s.create_fence(OWNER, "Warehouse".into(), 43.6532, -79.3832, 500.0, true);
        // restricted=false: must stay outside (exclusion zone)
        let f2 = s.create_fence(OWNER, "Danger Zone".into(), 43.7000, -79.4000, 100.0, false);

        s.add_device_to_fence(OWNER, f1, DEVICE_A);
        s.add_device_to_fence(OWNER, f2, DEVICE_A);
        s.update_location(DEVICE_A, 43.6535, -79.3830, 100);

        let results = s.check_all_fences(&DEVICE_A);
        assert_eq!(results.len(), 2);
        // Inside warehouse (compliant for restricted)
        assert!(results.iter().any(|&(id, c)| id == f1 && c));
        // Outside danger zone (compliant for non-restricted)
        assert!(results.iter().any(|&(id, c)| id == f2 && c));
    }

    #[test]
    fn test_add_remove_device() {
        let mut s = GeoFenceState::new(OWNER);
        let f = s.create_fence(OWNER, "Zone".into(), 0.0, 0.0, 1000.0, true);
        s.add_device_to_fence(OWNER, f, DEVICE_A);
        s.add_device_to_fence(OWNER, f, DEVICE_B);
        assert_eq!(s.fences[&f].allowed_devices.len(), 2);
        s.remove_device_from_fence(OWNER, f, DEVICE_A);
        assert_eq!(s.fences[&f].allowed_devices.len(), 1);
    }

    #[test]
    fn test_no_location_returns_false() {
        let mut s = GeoFenceState::new(OWNER);
        let fence_id = s.create_fence(OWNER, "Zone".into(), 0.0, 0.0, 1000.0, true);
        // DEVICE_A has no location
        assert!(!s.is_within_fence(&DEVICE_A, fence_id));
    }

    #[test]
    #[should_panic(expected = "invalid latitude")]
    fn test_invalid_latitude_rejected() {
        let mut s = GeoFenceState::new(OWNER);
        s.create_fence(OWNER, "Bad".into(), 91.0, 0.0, 100.0, true);
    }
}
