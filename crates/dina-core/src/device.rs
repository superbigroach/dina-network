use serde::{Deserialize, Serialize};

use crate::types::{Address, DeviceId, Hash};

/// Type of device registered on the Dina Network.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    /// Cognitum Seed -- portable personal AI/health device.
    CognitumSeed,
    /// Cognitum Appliance -- stationary smart home device.
    CognitumAppliance,
    /// Autonomous or semi-autonomous robot.
    Robot,
    /// Aerial autonomous vehicle.
    Drone,
    /// Internet of Things sensor (temperature, humidity, etc.).
    IoTSensor,
    /// Software-only virtual agent (no hardware).
    VirtualAgent,
    /// Custom device type.
    Custom(String),
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceType::CognitumSeed => write!(f, "CognitumSeed"),
            DeviceType::CognitumAppliance => write!(f, "CognitumAppliance"),
            DeviceType::Robot => write!(f, "Robot"),
            DeviceType::Drone => write!(f, "Drone"),
            DeviceType::IoTSensor => write!(f, "IoTSensor"),
            DeviceType::VirtualAgent => write!(f, "VirtualAgent"),
            DeviceType::Custom(name) => write!(f, "Custom({name})"),
        }
    }
}

/// Geographic location of a device.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeoLocation {
    /// Latitude in degrees (-90 to 90).
    pub latitude: f64,
    /// Longitude in degrees (-180 to 180).
    pub longitude: f64,
    /// Optional altitude in meters.
    pub altitude: Option<f64>,
    /// Accuracy radius in meters.
    pub accuracy: Option<f64>,
}

/// Additional metadata associated with a device.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceMetadata {
    /// Human-readable name for the device.
    pub name: Option<String>,
    /// Manufacturer identifier.
    pub manufacturer: Option<String>,
    /// Model number or name.
    pub model: Option<String>,
    /// Firmware version string.
    pub firmware_version: Option<String>,
    /// Last known location of the device.
    pub location: Option<GeoLocation>,
    /// Hardware interface identifiers (camera, GPS, motor, etc.).
    pub interfaces: Vec<u32>,
    /// Arbitrary key-value metadata.
    pub extra: std::collections::HashMap<String, String>,
}

impl Default for DeviceMetadata {
    fn default() -> Self {
        Self {
            name: None,
            manufacturer: None,
            model: None,
            firmware_version: None,
            location: None,
            interfaces: Vec::new(),
            extra: std::collections::HashMap::new(),
        }
    }
}

/// On-chain identity of a registered device.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceIdentity {
    /// Unique device identifier (derived from the device's public key).
    pub id: DeviceId,
    /// The device's Ed25519 public key (raw 32 bytes).
    pub pubkey: [u8; 32],
    /// Address of the device's owner.
    pub owner: Address,
    /// Type of device.
    pub device_type: DeviceType,
    /// SHA-256 hash of the device's firmware.
    pub firmware_hash: Hash,
    /// Merkle root of the device's witness history.
    pub witness_root: Hash,
    /// Unix timestamp when the device was registered.
    pub registered_at: u64,
    /// Whether the device is currently active and authorized.
    pub active: bool,
    /// Additional device metadata.
    pub metadata: DeviceMetadata,
}

impl DeviceIdentity {
    /// Create a new device identity from registration data.
    pub fn new(
        pubkey: [u8; 32],
        owner: Address,
        device_type: DeviceType,
        firmware_hash: Hash,
        witness_root: Hash,
        registered_at: u64,
    ) -> Self {
        use crate::crypto::hash_bytes;

        let id_hash = hash_bytes(&pubkey);
        let id = Address(id_hash.0);

        Self {
            id,
            pubkey,
            owner,
            device_type,
            firmware_hash,
            witness_root,
            registered_at,
            active: true,
            metadata: DeviceMetadata::default(),
        }
    }

    /// Deactivate this device.
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Update the firmware hash (e.g., after an OTA update).
    pub fn update_firmware(&mut self, new_hash: Hash) {
        self.firmware_hash = new_hash;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_device_identity() {
        let pubkey = [0x42; 32];
        let owner = Address([0x01; 32]);
        let device = DeviceIdentity::new(
            pubkey,
            owner,
            DeviceType::CognitumSeed,
            Hash([0xaa; 32]),
            Hash::ZERO,
            1_700_000_000,
        );
        assert!(device.active);
        assert_eq!(device.owner, owner);
        assert_eq!(device.device_type, DeviceType::CognitumSeed);
    }

    #[test]
    fn deactivate_device() {
        let mut device = DeviceIdentity::new(
            [0x01; 32],
            Address::ZERO,
            DeviceType::IoTSensor,
            Hash::ZERO,
            Hash::ZERO,
            0,
        );
        assert!(device.active);
        device.deactivate();
        assert!(!device.active);
    }

    #[test]
    fn device_type_display() {
        assert_eq!(DeviceType::Robot.to_string(), "Robot");
        assert_eq!(
            DeviceType::Custom("MyBot".to_string()).to_string(),
            "Custom(MyBot)"
        );
    }
}
