use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-110  Firmware Attestation
// ---------------------------------------------------------------------------

type Address = [u8; 32];
type DeviceId = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FirmwareRecord {
    pub device_id: DeviceId,
    pub firmware_hash: [u8; 32],
    pub boot_hash: [u8; 32],
    pub version: String,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrustedFirmware {
    pub hash: [u8; 32],
    pub version: String,
    pub manufacturer: Address,
    pub registered_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FirmwareStatus {
    Trusted { version: String, last_attested: u64 },
    Unknown { firmware_hash: [u8; 32] },
    Outdated { current: String, latest: String },
    Compromised { reason: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FirmwareRegistry {
    pub admin: Address,
    pub device_firmware: BTreeMap<DeviceId, FirmwareRecord>,
    pub trusted_firmware: BTreeMap<[u8; 32], TrustedFirmware>,
    pub firmware_history: BTreeMap<DeviceId, Vec<FirmwareRecord>>,
    pub manufacturers: BTreeMap<Address, String>,
}

impl FirmwareRegistry {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            device_firmware: BTreeMap::new(),
            trusted_firmware: BTreeMap::new(),
            firmware_history: BTreeMap::new(),
            manufacturers: BTreeMap::new(),
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn verify_firmware(&self, device_id: &DeviceId) -> FirmwareStatus {
        let record = match self.device_firmware.get(device_id) {
            Some(r) => r,
            None => {
                return FirmwareStatus::Unknown {
                    firmware_hash: [0u8; 32],
                };
            }
        };

        // Check if firmware hash is in trusted list
        if let Some(trusted) = self.trusted_firmware.get(&record.firmware_hash) {
            // Check if this is the latest version
            let latest = self.latest_trusted_version_inner();
            if trusted.version == latest {
                FirmwareStatus::Trusted {
                    version: trusted.version.clone(),
                    last_attested: record.timestamp,
                }
            } else {
                FirmwareStatus::Outdated {
                    current: trusted.version.clone(),
                    latest,
                }
            }
        } else {
            FirmwareStatus::Unknown {
                firmware_hash: record.firmware_hash,
            }
        }
    }

    pub fn is_trusted_firmware(&self, hash: &[u8; 32]) -> bool {
        self.trusted_firmware.contains_key(hash)
    }

    pub fn firmware_history(&self, device_id: &DeviceId) -> Vec<FirmwareRecord> {
        self.firmware_history
            .get(device_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn latest_trusted_version(&self) -> String {
        self.latest_trusted_version_inner()
    }

    fn latest_trusted_version_inner(&self) -> String {
        // Return the version string of the most recently registered trusted firmware
        self.trusted_firmware
            .values()
            .max_by_key(|tf| tf.registered_at)
            .map(|tf| tf.version.clone())
            .unwrap_or_default()
    }

    // -- Mutations -----------------------------------------------------------

    pub fn attest_firmware(&mut self, record: FirmwareRecord) {
        let device_id = record.device_id;
        self.firmware_history
            .entry(device_id)
            .or_default()
            .push(record.clone());
        self.device_firmware.insert(device_id, record);
    }

    pub fn register_trusted_firmware(
        &mut self,
        caller: Address,
        hash: [u8; 32],
        version: String,
        timestamp: u64,
    ) {
        assert!(
            self.manufacturers.contains_key(&caller),
            "DRC110: only registered manufacturers can register firmware"
        );
        let trusted = TrustedFirmware {
            hash,
            version,
            manufacturer: caller,
            registered_at: timestamp,
        };
        self.trusted_firmware.insert(hash, trusted);
    }

    pub fn register_manufacturer(&mut self, caller: Address, addr: Address, name: String) {
        assert!(
            caller == self.admin,
            "DRC110: only admin can register manufacturers"
        );
        self.manufacturers.insert(addr, name);
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct AttestFirmwareArgs {
    record: FirmwareRecord,
}

#[derive(Serialize, Deserialize, Debug)]
struct VerifyFirmwareArgs {
    device_id: DeviceId,
}

#[derive(Serialize, Deserialize, Debug)]
struct RegisterTrustedFirmwareArgs {
    hash: [u8; 32],
    version: String,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct IsTrustedFirmwareArgs {
    hash: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct FirmwareHistoryArgs {
    device_id: DeviceId,
}

#[derive(Serialize, Deserialize, Debug)]
struct RegisterManufacturerArgs {
    addr: Address,
    name: String,
}

pub fn dispatch(
    state: &mut Option<FirmwareRegistry>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC110: already initialised");
            *state = Some(FirmwareRegistry::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "verify_firmware" => {
            let s = state.as_ref().expect("DRC110: not initialised");
            let a: VerifyFirmwareArgs =
                serde_json::from_slice(args).expect("DRC110: bad verify_firmware args");
            serde_json::to_vec(&s.verify_firmware(&a.device_id)).unwrap()
        }
        "is_trusted_firmware" => {
            let s = state.as_ref().expect("DRC110: not initialised");
            let a: IsTrustedFirmwareArgs =
                serde_json::from_slice(args).expect("DRC110: bad is_trusted_firmware args");
            serde_json::to_vec(&s.is_trusted_firmware(&a.hash)).unwrap()
        }
        "firmware_history" => {
            let s = state.as_ref().expect("DRC110: not initialised");
            let a: FirmwareHistoryArgs =
                serde_json::from_slice(args).expect("DRC110: bad firmware_history args");
            serde_json::to_vec(&s.firmware_history(&a.device_id)).unwrap()
        }
        "latest_trusted_version" => {
            let s = state.as_ref().expect("DRC110: not initialised");
            serde_json::to_vec(&s.latest_trusted_version()).unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "attest_firmware" => {
            let s = state.as_mut().expect("DRC110: not initialised");
            let a: AttestFirmwareArgs =
                serde_json::from_slice(args).expect("DRC110: bad attest_firmware args");
            s.attest_firmware(a.record);
            serde_json::to_vec("ok").unwrap()
        }
        "register_trusted_firmware" => {
            let s = state.as_mut().expect("DRC110: not initialised");
            let a: RegisterTrustedFirmwareArgs =
                serde_json::from_slice(args).expect("DRC110: bad register_trusted_firmware args");
            s.register_trusted_firmware(caller, a.hash, a.version, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }
        "register_manufacturer" => {
            let s = state.as_mut().expect("DRC110: not initialised");
            let a: RegisterManufacturerArgs =
                serde_json::from_slice(args).expect("DRC110: bad register_manufacturer args");
            s.register_manufacturer(caller, a.addr, a.name);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC110: unknown method '{method}'"),
    }
}
