use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-105  Sensor Attestation
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SensorValue {
    Float(f64),
    Integer(i64),
    Location {
        lat: f64,
        lng: f64,
        accuracy: f64,
    },
    Vector(Vec<f64>),
    Json(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SensorReading {
    pub device_id: [u8; 32],
    pub sensor_type: String,
    pub value: SensorValue,
    pub timestamp: u64,
    pub witness_hash: [u8; 32],
    pub device_signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Attestation {
    pub id: u64,
    pub reading: SensorReading,
    pub attester: [u8; 32],
    pub attested_at: u64,
    pub verified: bool,
    pub verifier: Option<[u8; 32]>,
    pub verified_at: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SensorAttestationState {
    pub attestations: BTreeMap<u64, Attestation>,
    /// device_id -> list of attestation ids
    pub device_attestations: BTreeMap<[u8; 32], Vec<u64>>,
    pub next_id: u64,
}

impl SensorAttestationState {
    pub fn new() -> Self {
        Self {
            attestations: BTreeMap::new(),
            device_attestations: BTreeMap::new(),
            next_id: 1,
        }
    }

    /// Submit a sensor reading as an attestation.
    pub fn attest(
        &mut self,
        caller: [u8; 32],
        reading: SensorReading,
        timestamp: u64,
    ) -> u64 {
        assert!(
            !reading.device_signature.is_empty(),
            "DRC105: device signature is required"
        );
        assert!(
            reading.timestamp <= timestamp,
            "DRC105: reading timestamp cannot be in the future"
        );

        let id = self.next_id;
        self.next_id += 1;

        let device_id = reading.device_id;

        let attestation = Attestation {
            id,
            reading,
            attester: caller,
            attested_at: timestamp,
            verified: false,
            verifier: None,
            verified_at: None,
        };

        self.attestations.insert(id, attestation);
        self.device_attestations
            .entry(device_id)
            .or_insert_with(Vec::new)
            .push(id);

        id
    }

    /// A verifier confirms the attestation is valid (e.g. cross-checked the
    /// device signature against DRC-2 registry).
    pub fn verify(
        &mut self,
        caller: [u8; 32],
        attestation_id: u64,
        timestamp: u64,
    ) {
        let attestation = self
            .attestations
            .get_mut(&attestation_id)
            .expect("DRC105: attestation not found");
        assert!(
            !attestation.verified,
            "DRC105: attestation already verified"
        );
        assert!(
            caller != attestation.attester,
            "DRC105: attester cannot self-verify"
        );

        attestation.verified = true;
        attestation.verifier = Some(caller);
        attestation.verified_at = Some(timestamp);
    }

    /// Get all attestation IDs for a device.
    pub fn attestations_of(&self, device_id: &[u8; 32]) -> Vec<&Attestation> {
        self.device_attestations
            .get(device_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.attestations.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get a single attestation by ID.
    pub fn get_attestation(&self, attestation_id: u64) -> Option<&Attestation> {
        self.attestations.get(&attestation_id)
    }
}

// ---------------------------------------------------------------------------
// Dispatch arg types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct AttestArgs {
    reading: SensorReading,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct VerifyArgs {
    attestation_id: u64,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeviceIdArgs {
    device_id: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct AttestationIdArgs {
    attestation_id: u64,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<SensorAttestationState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC105: already initialised");
            *state = Some(SensorAttestationState::new());
            serde_json::to_vec("ok").unwrap()
        }

        "attest" => {
            let s = state.as_mut().expect("DRC105: not initialised");
            let a: AttestArgs =
                serde_json::from_slice(args).expect("DRC105: bad attest args");
            let id = s.attest(caller, a.reading, a.timestamp);
            serde_json::to_vec(&id).unwrap()
        }

        "verify" => {
            let s = state.as_mut().expect("DRC105: not initialised");
            let a: VerifyArgs =
                serde_json::from_slice(args).expect("DRC105: bad verify args");
            s.verify(caller, a.attestation_id, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }

        "attestations_of" => {
            let s = state.as_ref().expect("DRC105: not initialised");
            let a: DeviceIdArgs =
                serde_json::from_slice(args).expect("DRC105: bad attestations_of args");
            let attestations: Vec<&Attestation> = s.attestations_of(&a.device_id);
            let owned: Vec<Attestation> = attestations.into_iter().cloned().collect();
            serde_json::to_vec(&owned).unwrap()
        }

        "get_attestation" => {
            let s = state.as_ref().expect("DRC105: not initialised");
            let a: AttestationIdArgs =
                serde_json::from_slice(args).expect("DRC105: bad get_attestation args");
            serde_json::to_vec(&s.get_attestation(a.attestation_id)).unwrap()
        }

        _ => panic!("DRC105: unknown method '{method}'"),
    }
}
