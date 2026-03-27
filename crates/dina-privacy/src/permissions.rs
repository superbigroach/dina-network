use dina_core::types::DeviceId;
use dina_core::Address;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("key {0} is not authorized for this action")]
    Unauthorized(String),
    #[error("key {0} not found in permission set")]
    KeyNotFound(String),
    #[error("only the owner can manage keys")]
    NotOwner,
    #[error("session key expired at {expired_at}, current time is {current_time}")]
    SessionExpired { expired_at: u64, current_time: u64 },
    #[error("transfer amount {amount} exceeds max allowed {max}")]
    AmountExceeded { amount: u64, max: u64 },
    #[error("recipient {0} is not in the allowed list")]
    RecipientNotAllowed(String),
    #[error("contract {0} is not in the allowed list")]
    ContractNotAllowed(String),
    #[error("method {0} is not in the allowed list")]
    MethodNotAllowed(String),
    #[error("device {0} is not in the allowed device list")]
    DeviceNotAllowed(String),
}

/// Granular permission levels for authorized keys, inspired by Arc Network's
/// key permission system. Each key in a wallet can have different capabilities.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum KeyPermission {
    /// Unrestricted access — can perform any action.
    FullAccess,

    /// Can only send transfers, optionally limited by amount and recipient list.
    TransferOnly {
        max_amount: Option<u64>,
        allowed_recipients: Vec<Address>,
    },

    /// Can only call specific contracts and methods.
    ContractCallOnly {
        allowed_contracts: Vec<Address>,
        allowed_methods: Vec<String>,
    },

    /// Read-only access — can query state but not submit transactions.
    ViewOnly,

    /// Can only issue commands to specific devices (IoT / robotics use case).
    DeviceControl {
        device_ids: Vec<DeviceId>,
    },

    /// Temporary key with an expiration timestamp and nested permissions.
    SessionKey {
        expires_at: u64,
        permissions: Box<KeyPermission>,
    },

    /// Freeform permission with a label and a list of capability strings.
    Custom {
        label: String,
        capabilities: Vec<String>,
    },
}

/// An action that a key may attempt to perform.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    Transfer {
        to: Address,
        amount: u64,
    },
    ContractCall {
        contract: Address,
        method: String,
    },
    DeviceCommand {
        device_id: DeviceId,
        command: String,
    },
    ViewState,
    ManageKeys,
    EmergencyStop,
}

/// A key that has been granted specific permissions by the wallet owner.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthorizedKey {
    pub pubkey: [u8; 32],
    pub label: String,
    pub permissions: KeyPermission,
    pub created_at: u64,
    pub last_used: Option<u64>,
}

/// The full permission set for a wallet, containing the owner address and all
/// authorized keys with their individual permissions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PermissionSet {
    pub owner: Address,
    pub keys: Vec<AuthorizedKey>,
}

impl PermissionSet {
    /// Create a new permission set for the given owner with no authorized keys.
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            keys: Vec::new(),
        }
    }

    /// Add a new authorized key with the given permissions.
    pub fn add_key(
        &mut self,
        pubkey: [u8; 32],
        label: String,
        permissions: KeyPermission,
        now: u64,
    ) {
        // Remove existing key with same pubkey if present
        self.keys.retain(|k| k.pubkey != pubkey);

        self.keys.push(AuthorizedKey {
            pubkey,
            label,
            permissions,
            created_at: now,
            last_used: None,
        });
    }

    /// Remove an authorized key by its public key.
    pub fn remove_key(&mut self, pubkey: &[u8; 32]) -> Result<(), PermissionError> {
        let before = self.keys.len();
        self.keys.retain(|k| &k.pubkey != pubkey);
        if self.keys.len() == before {
            return Err(PermissionError::KeyNotFound(hex::encode(pubkey)));
        }
        Ok(())
    }

    /// Rotate a key: replace `old_pubkey` with `new_pubkey`, keeping the same
    /// permissions and metadata.
    pub fn rotate_key(
        &mut self,
        old_pubkey: &[u8; 32],
        new_pubkey: [u8; 32],
    ) -> Result<(), PermissionError> {
        let key = self
            .keys
            .iter_mut()
            .find(|k| &k.pubkey == old_pubkey)
            .ok_or_else(|| PermissionError::KeyNotFound(hex::encode(old_pubkey)))?;

        key.pubkey = new_pubkey;
        Ok(())
    }

    /// Check whether a key is authorized for the given action.
    /// Returns `Ok(())` if authorized, or an error describing why not.
    ///
    /// `current_time` is used to validate session key expiration.
    pub fn check_permission(
        &mut self,
        pubkey: &[u8; 32],
        action: &Action,
        current_time: u64,
    ) -> Result<(), PermissionError> {
        let key = self
            .keys
            .iter_mut()
            .find(|k| &k.pubkey == pubkey)
            .ok_or_else(|| PermissionError::KeyNotFound(hex::encode(pubkey)))?;

        key.last_used = Some(current_time);

        check_permission_recursive(&key.permissions, action, current_time)
    }

    /// Convenience: returns `true` if the key is authorized for the action.
    pub fn is_authorized(
        &mut self,
        pubkey: &[u8; 32],
        action: &Action,
        current_time: u64,
    ) -> bool {
        self.check_permission(pubkey, action, current_time).is_ok()
    }
}

/// Recursively check permissions, unwrapping `SessionKey` wrappers.
fn check_permission_recursive(
    permission: &KeyPermission,
    action: &Action,
    current_time: u64,
) -> Result<(), PermissionError> {
    match permission {
        KeyPermission::FullAccess => Ok(()),

        KeyPermission::ViewOnly => match action {
            Action::ViewState => Ok(()),
            _ => Err(PermissionError::Unauthorized(
                "ViewOnly key cannot perform this action".into(),
            )),
        },

        KeyPermission::TransferOnly {
            max_amount,
            allowed_recipients,
        } => match action {
            Action::Transfer { to, amount } => {
                if let Some(max) = max_amount {
                    if amount > max {
                        return Err(PermissionError::AmountExceeded {
                            amount: *amount,
                            max: *max,
                        });
                    }
                }
                if !allowed_recipients.is_empty() && !allowed_recipients.contains(to) {
                    return Err(PermissionError::RecipientNotAllowed(to.to_string()));
                }
                Ok(())
            }
            Action::ViewState => Ok(()), // Transfer keys can view state
            _ => Err(PermissionError::Unauthorized(
                "TransferOnly key can only transfer and view".into(),
            )),
        },

        KeyPermission::ContractCallOnly {
            allowed_contracts,
            allowed_methods,
        } => match action {
            Action::ContractCall { contract, method } => {
                if !allowed_contracts.is_empty() && !allowed_contracts.contains(contract) {
                    return Err(PermissionError::ContractNotAllowed(contract.to_string()));
                }
                if !allowed_methods.is_empty() && !allowed_methods.contains(method) {
                    return Err(PermissionError::MethodNotAllowed(method.clone()));
                }
                Ok(())
            }
            Action::ViewState => Ok(()),
            _ => Err(PermissionError::Unauthorized(
                "ContractCallOnly key can only call contracts and view".into(),
            )),
        },

        KeyPermission::DeviceControl { device_ids } => match action {
            Action::DeviceCommand { device_id, .. } => {
                if !device_ids.contains(device_id) {
                    return Err(PermissionError::DeviceNotAllowed(device_id.to_string()));
                }
                Ok(())
            }
            Action::ViewState => Ok(()),
            _ => Err(PermissionError::Unauthorized(
                "DeviceControl key can only control devices and view".into(),
            )),
        },

        KeyPermission::SessionKey {
            expires_at,
            permissions,
        } => {
            if current_time > *expires_at {
                return Err(PermissionError::SessionExpired {
                    expired_at: *expires_at,
                    current_time,
                });
            }
            check_permission_recursive(permissions, action, current_time)
        }

        KeyPermission::Custom { capabilities, .. } => {
            let required = match action {
                Action::Transfer { .. } => "transfer",
                Action::ContractCall { .. } => "contract_call",
                Action::DeviceCommand { .. } => "device_command",
                Action::ViewState => "view_state",
                Action::ManageKeys => "manage_keys",
                Action::EmergencyStop => "emergency_stop",
            };
            if capabilities.iter().any(|c| c == required || c == "*") {
                Ok(())
            } else {
                Err(PermissionError::Unauthorized(format!(
                    "Custom key lacks capability '{required}'"
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_address(byte: u8) -> Address {
        Address([byte; 32])
    }

    #[test]
    fn full_access_allows_everything() {
        let mut pset = PermissionSet::new(test_address(0x01));
        let key: [u8; 32] = [0xAA; 32];
        pset.add_key(key, "admin".into(), KeyPermission::FullAccess, 1000);

        assert!(pset.is_authorized(&key, &Action::ViewState, 1001));
        assert!(pset.is_authorized(
            &key,
            &Action::Transfer {
                to: test_address(0x02),
                amount: 999_999,
            },
            1001,
        ));
        assert!(pset.is_authorized(&key, &Action::ManageKeys, 1001));
        assert!(pset.is_authorized(&key, &Action::EmergencyStop, 1001));
    }

    #[test]
    fn view_only_blocks_transfers() {
        let mut pset = PermissionSet::new(test_address(0x01));
        let key: [u8; 32] = [0xBB; 32];
        pset.add_key(key, "viewer".into(), KeyPermission::ViewOnly, 1000);

        assert!(pset.is_authorized(&key, &Action::ViewState, 1001));
        assert!(!pset.is_authorized(
            &key,
            &Action::Transfer {
                to: test_address(0x02),
                amount: 100,
            },
            1001,
        ));
    }

    #[test]
    fn transfer_only_respects_max_amount() {
        let mut pset = PermissionSet::new(test_address(0x01));
        let key: [u8; 32] = [0xCC; 32];
        pset.add_key(
            key,
            "limited sender".into(),
            KeyPermission::TransferOnly {
                max_amount: Some(500),
                allowed_recipients: vec![],
            },
            1000,
        );

        // Under limit
        assert!(pset.is_authorized(
            &key,
            &Action::Transfer {
                to: test_address(0x02),
                amount: 500,
            },
            1001,
        ));

        // Over limit
        assert!(!pset.is_authorized(
            &key,
            &Action::Transfer {
                to: test_address(0x02),
                amount: 501,
            },
            1001,
        ));
    }

    #[test]
    fn transfer_only_respects_allowed_recipients() {
        let allowed = test_address(0x02);
        let mut pset = PermissionSet::new(test_address(0x01));
        let key: [u8; 32] = [0xDD; 32];
        pset.add_key(
            key,
            "restricted sender".into(),
            KeyPermission::TransferOnly {
                max_amount: None,
                allowed_recipients: vec![allowed],
            },
            1000,
        );

        assert!(pset.is_authorized(
            &key,
            &Action::Transfer {
                to: allowed,
                amount: 1000,
            },
            1001,
        ));
        assert!(!pset.is_authorized(
            &key,
            &Action::Transfer {
                to: test_address(0x03),
                amount: 1000,
            },
            1001,
        ));
    }

    #[test]
    fn contract_call_only_checks_contract_and_method() {
        let contract = test_address(0x10);
        let mut pset = PermissionSet::new(test_address(0x01));
        let key: [u8; 32] = [0xEE; 32];
        pset.add_key(
            key,
            "contract caller".into(),
            KeyPermission::ContractCallOnly {
                allowed_contracts: vec![contract],
                allowed_methods: vec!["transfer".into(), "approve".into()],
            },
            1000,
        );

        // Allowed
        assert!(pset.is_authorized(
            &key,
            &Action::ContractCall {
                contract,
                method: "transfer".into(),
            },
            1001,
        ));

        // Wrong contract
        assert!(!pset.is_authorized(
            &key,
            &Action::ContractCall {
                contract: test_address(0x20),
                method: "transfer".into(),
            },
            1001,
        ));

        // Wrong method
        assert!(!pset.is_authorized(
            &key,
            &Action::ContractCall {
                contract,
                method: "selfDestruct".into(),
            },
            1001,
        ));
    }

    #[test]
    fn device_control_checks_device_ids() {
        let device = test_address(0x50);
        let mut pset = PermissionSet::new(test_address(0x01));
        let key: [u8; 32] = [0xFF; 32];
        pset.add_key(
            key,
            "device key".into(),
            KeyPermission::DeviceControl {
                device_ids: vec![device],
            },
            1000,
        );

        assert!(pset.is_authorized(
            &key,
            &Action::DeviceCommand {
                device_id: device,
                command: "start_motor".into(),
            },
            1001,
        ));

        assert!(!pset.is_authorized(
            &key,
            &Action::DeviceCommand {
                device_id: test_address(0x60),
                command: "start_motor".into(),
            },
            1001,
        ));
    }

    #[test]
    fn session_key_expires() {
        let mut pset = PermissionSet::new(test_address(0x01));
        let key: [u8; 32] = [0x11; 32];
        pset.add_key(
            key,
            "session".into(),
            KeyPermission::SessionKey {
                expires_at: 2000,
                permissions: Box::new(KeyPermission::FullAccess),
            },
            1000,
        );

        // Before expiry
        assert!(pset.is_authorized(&key, &Action::ViewState, 1999));

        // After expiry
        assert!(!pset.is_authorized(&key, &Action::ViewState, 2001));
    }

    #[test]
    fn session_key_with_nested_restrictions() {
        let mut pset = PermissionSet::new(test_address(0x01));
        let key: [u8; 32] = [0x22; 32];
        pset.add_key(
            key,
            "temp viewer".into(),
            KeyPermission::SessionKey {
                expires_at: 5000,
                permissions: Box::new(KeyPermission::ViewOnly),
            },
            1000,
        );

        // View allowed before expiry
        assert!(pset.is_authorized(&key, &Action::ViewState, 3000));

        // Transfer blocked even before expiry
        assert!(!pset.is_authorized(
            &key,
            &Action::Transfer {
                to: test_address(0x02),
                amount: 100,
            },
            3000,
        ));
    }

    #[test]
    fn custom_permission_checks_capabilities() {
        let mut pset = PermissionSet::new(test_address(0x01));
        let key: [u8; 32] = [0x33; 32];
        pset.add_key(
            key,
            "custom".into(),
            KeyPermission::Custom {
                label: "operator".into(),
                capabilities: vec!["view_state".into(), "emergency_stop".into()],
            },
            1000,
        );

        assert!(pset.is_authorized(&key, &Action::ViewState, 1001));
        assert!(pset.is_authorized(&key, &Action::EmergencyStop, 1001));
        assert!(!pset.is_authorized(
            &key,
            &Action::Transfer {
                to: test_address(0x02),
                amount: 100,
            },
            1001,
        ));
    }

    #[test]
    fn remove_key_works() {
        let mut pset = PermissionSet::new(test_address(0x01));
        let key: [u8; 32] = [0x44; 32];
        pset.add_key(key, "temp".into(), KeyPermission::FullAccess, 1000);
        assert_eq!(pset.keys.len(), 1);

        pset.remove_key(&key).unwrap();
        assert!(pset.keys.is_empty());
    }

    #[test]
    fn remove_nonexistent_key_errors() {
        let mut pset = PermissionSet::new(test_address(0x01));
        let key: [u8; 32] = [0x55; 32];
        assert!(pset.remove_key(&key).is_err());
    }

    #[test]
    fn rotate_key_preserves_permissions() {
        let mut pset = PermissionSet::new(test_address(0x01));
        let old_key: [u8; 32] = [0x66; 32];
        let new_key: [u8; 32] = [0x77; 32];

        pset.add_key(
            old_key,
            "rotating".into(),
            KeyPermission::TransferOnly {
                max_amount: Some(1000),
                allowed_recipients: vec![],
            },
            1000,
        );

        pset.rotate_key(&old_key, new_key).unwrap();

        // Old key should no longer work
        assert!(!pset.is_authorized(
            &old_key,
            &Action::Transfer {
                to: test_address(0x02),
                amount: 500,
            },
            1001,
        ));

        // New key should work with same permissions
        assert!(pset.is_authorized(
            &new_key,
            &Action::Transfer {
                to: test_address(0x02),
                amount: 500,
            },
            1001,
        ));

        // New key should still respect limits
        assert!(!pset.is_authorized(
            &new_key,
            &Action::Transfer {
                to: test_address(0x02),
                amount: 1001,
            },
            1001,
        ));
    }

    #[test]
    fn unknown_key_is_not_authorized() {
        let mut pset = PermissionSet::new(test_address(0x01));
        let unknown: [u8; 32] = [0x99; 32];
        assert!(!pset.is_authorized(&unknown, &Action::ViewState, 1001));
    }

    #[test]
    fn last_used_is_updated() {
        let mut pset = PermissionSet::new(test_address(0x01));
        let key: [u8; 32] = [0xAA; 32];
        pset.add_key(key, "admin".into(), KeyPermission::FullAccess, 1000);

        assert!(pset.keys[0].last_used.is_none());

        pset.check_permission(&key, &Action::ViewState, 5000).unwrap();
        assert_eq!(pset.keys[0].last_used, Some(5000));
    }
}
