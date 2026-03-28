use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::{DinaError, DinaResult};
use crate::types::Address;

/// The category of smart account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmartAccountType {
    /// DRC-101 agent wallet.
    AgentWallet,
    /// DRC-111 smart wallet.
    SmartWallet,
    /// Multi-signature account.
    MultiSig,
}

/// Metadata for a registered smart account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmartAccountInfo {
    pub address: Address,
    pub account_type: SmartAccountType,
    pub owner: Address,
    pub created_at: u64,
    /// DRC-16 proxy implementation target address.
    pub implementation: Address,
}

/// A scoped, time-limited key granting a subset of account permissions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionKeyInfo {
    pub key: [u8; 32],
    pub account: Address,
    pub permissions: Vec<String>,
    pub expires_at: u64,
    pub max_value_per_tx: u64,
}

/// ERC-4337-style user operation for account abstraction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserOperation {
    pub sender: Address,
    pub nonce: u64,
    pub call_data: Vec<u8>,
    pub call_gas_limit: u64,
    pub verification_gas: u64,
    pub max_fee_per_gas: u64,
    pub paymaster: Option<Address>,
    pub signature: Vec<u8>,
}

/// Account abstraction layer integrating DRC-101 agent wallets and DRC-111
/// smart wallets with session keys and user operations.
pub struct AccountAbstraction {
    smart_accounts: BTreeMap<Address, SmartAccountInfo>,
    session_keys: BTreeMap<Address, Vec<SessionKeyInfo>>,
    nonces: BTreeMap<Address, u64>,
}

impl AccountAbstraction {
    /// Create an empty account abstraction system.
    pub fn new() -> Self {
        Self {
            smart_accounts: BTreeMap::new(),
            session_keys: BTreeMap::new(),
            nonces: BTreeMap::new(),
        }
    }

    /// Register a new smart account. Fails if the address is already registered.
    pub fn register_smart_account(&mut self, info: SmartAccountInfo) -> DinaResult<()> {
        if self.smart_accounts.contains_key(&info.address) {
            return Err(DinaError::AccountAbstractionError(format!(
                "smart account already registered at {}",
                info.address
            )));
        }
        self.nonces.insert(info.address, 0);
        self.smart_accounts.insert(info.address, info);
        Ok(())
    }

    /// Validate a user operation: sender must be a registered smart account,
    /// nonce must match, call data and signature must be non-empty, and gas
    /// limits must be positive.
    pub fn validate_user_operation(&self, op: &UserOperation) -> DinaResult<()> {
        if !self.smart_accounts.contains_key(&op.sender) {
            return Err(DinaError::AccountAbstractionError(format!(
                "sender {} is not a registered smart account",
                op.sender
            )));
        }

        let expected_nonce = self.nonces.get(&op.sender).copied().unwrap_or(0);
        if op.nonce != expected_nonce {
            return Err(DinaError::InvalidNonce {
                expected: expected_nonce,
                got: op.nonce,
            });
        }

        if op.call_data.is_empty() {
            return Err(DinaError::AccountAbstractionError(
                "call_data must not be empty".to_string(),
            ));
        }

        if op.signature.is_empty() {
            return Err(DinaError::AccountAbstractionError(
                "signature must not be empty".to_string(),
            ));
        }

        if op.call_gas_limit == 0 || op.verification_gas == 0 {
            return Err(DinaError::AccountAbstractionError(
                "gas limits must be greater than zero".to_string(),
            ));
        }

        Ok(())
    }

    /// Check whether an address is a registered smart account.
    pub fn is_smart_account(&self, address: &Address) -> bool {
        self.smart_accounts.contains_key(address)
    }

    /// Get the account type for a registered address.
    pub fn get_account_type(&self, address: &Address) -> Option<SmartAccountType> {
        self.smart_accounts.get(address).map(|a| a.account_type)
    }

    /// Add a session key for a smart account. Fails if the account does not
    /// exist or the key is already registered for that account.
    pub fn add_session_key(
        &mut self,
        account: &Address,
        key_info: SessionKeyInfo,
    ) -> DinaResult<()> {
        if !self.smart_accounts.contains_key(account) {
            return Err(DinaError::AccountAbstractionError(format!(
                "account {} not found",
                account
            )));
        }

        let keys = self.session_keys.entry(*account).or_default();
        if keys.iter().any(|k| k.key == key_info.key) {
            return Err(DinaError::AccountAbstractionError(
                "session key already exists for this account".to_string(),
            ));
        }

        keys.push(key_info);
        Ok(())
    }

    /// Validate that a session key is active (not expired at `current_time`)
    /// and has the required permission for the given action.
    pub fn validate_session_key(
        &self,
        account: &Address,
        key: &[u8; 32],
        action: &str,
        current_time: u64,
    ) -> bool {
        let Some(keys) = self.session_keys.get(account) else {
            return false;
        };
        keys.iter().any(|k| {
            k.key == *key
                && k.expires_at > current_time
                && k.permissions.iter().any(|p| p == action)
        })
    }

    /// Revoke (remove) a session key from an account.
    pub fn revoke_session_key(&mut self, account: &Address, key: &[u8; 32]) -> DinaResult<()> {
        let keys = self.session_keys.get_mut(account).ok_or_else(|| {
            DinaError::AccountAbstractionError(format!("no session keys for account {}", account))
        })?;

        let initial_len = keys.len();
        keys.retain(|k| k.key != *key);
        if keys.len() == initial_len {
            return Err(DinaError::AccountAbstractionError(
                "session key not found".to_string(),
            ));
        }
        Ok(())
    }

    /// Return all non-expired session keys for an account.
    pub fn active_session_keys(
        &self,
        account: &Address,
        current_time: u64,
    ) -> Vec<&SessionKeyInfo> {
        self.session_keys
            .get(account)
            .map(|keys| {
                keys.iter()
                    .filter(|k| k.expires_at > current_time)
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for AccountAbstraction {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_address(byte: u8) -> Address {
        Address([byte; 32])
    }

    fn make_account_info(addr_byte: u8, owner_byte: u8, typ: SmartAccountType) -> SmartAccountInfo {
        SmartAccountInfo {
            address: make_address(addr_byte),
            account_type: typ,
            owner: make_address(owner_byte),
            created_at: 1000,
            implementation: make_address(0xFF),
        }
    }

    fn make_session_key(key_byte: u8, account_byte: u8, expires: u64) -> SessionKeyInfo {
        SessionKeyInfo {
            key: [key_byte; 32],
            account: make_address(account_byte),
            permissions: vec!["transfer".to_string(), "approve".to_string()],
            expires_at: expires,
            max_value_per_tx: 1000,
        }
    }

    fn make_user_op(sender_byte: u8, nonce: u64) -> UserOperation {
        UserOperation {
            sender: make_address(sender_byte),
            nonce,
            call_data: vec![1, 2, 3],
            call_gas_limit: 100_000,
            verification_gas: 50_000,
            max_fee_per_gas: 10,
            paymaster: None,
            signature: vec![0xAA; 64],
        }
    }

    #[test]
    fn register_and_query_smart_account() {
        let mut aa = AccountAbstraction::new();
        let info = make_account_info(1, 10, SmartAccountType::AgentWallet);
        aa.register_smart_account(info).unwrap();
        assert!(aa.is_smart_account(&make_address(1)));
        assert_eq!(
            aa.get_account_type(&make_address(1)),
            Some(SmartAccountType::AgentWallet)
        );
    }

    #[test]
    fn register_duplicate_account_fails() {
        let mut aa = AccountAbstraction::new();
        aa.register_smart_account(make_account_info(1, 10, SmartAccountType::SmartWallet))
            .unwrap();
        let result =
            aa.register_smart_account(make_account_info(1, 10, SmartAccountType::SmartWallet));
        assert!(result.is_err());
    }

    #[test]
    fn is_smart_account_false_for_unknown() {
        let aa = AccountAbstraction::new();
        assert!(!aa.is_smart_account(&make_address(99)));
    }

    #[test]
    fn get_account_type_none_for_unknown() {
        let aa = AccountAbstraction::new();
        assert_eq!(aa.get_account_type(&make_address(99)), None);
    }

    #[test]
    fn validate_user_operation_success() {
        let mut aa = AccountAbstraction::new();
        aa.register_smart_account(make_account_info(1, 10, SmartAccountType::AgentWallet))
            .unwrap();
        let op = make_user_op(1, 0);
        aa.validate_user_operation(&op).unwrap();
    }

    #[test]
    fn validate_user_operation_unknown_sender() {
        let aa = AccountAbstraction::new();
        let op = make_user_op(99, 0);
        let result = aa.validate_user_operation(&op);
        assert!(result.is_err());
    }

    #[test]
    fn validate_user_operation_bad_nonce() {
        let mut aa = AccountAbstraction::new();
        aa.register_smart_account(make_account_info(1, 10, SmartAccountType::AgentWallet))
            .unwrap();
        let op = make_user_op(1, 5); // expected 0
        let result = aa.validate_user_operation(&op);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("nonce"));
    }

    #[test]
    fn validate_user_operation_empty_call_data() {
        let mut aa = AccountAbstraction::new();
        aa.register_smart_account(make_account_info(1, 10, SmartAccountType::AgentWallet))
            .unwrap();
        let mut op = make_user_op(1, 0);
        op.call_data = vec![];
        assert!(aa.validate_user_operation(&op).is_err());
    }

    #[test]
    fn validate_user_operation_empty_signature() {
        let mut aa = AccountAbstraction::new();
        aa.register_smart_account(make_account_info(1, 10, SmartAccountType::AgentWallet))
            .unwrap();
        let mut op = make_user_op(1, 0);
        op.signature = vec![];
        assert!(aa.validate_user_operation(&op).is_err());
    }

    #[test]
    fn validate_user_operation_zero_gas() {
        let mut aa = AccountAbstraction::new();
        aa.register_smart_account(make_account_info(1, 10, SmartAccountType::AgentWallet))
            .unwrap();
        let mut op = make_user_op(1, 0);
        op.call_gas_limit = 0;
        assert!(aa.validate_user_operation(&op).is_err());
    }

    #[test]
    fn add_and_validate_session_key() {
        let mut aa = AccountAbstraction::new();
        aa.register_smart_account(make_account_info(1, 10, SmartAccountType::SmartWallet))
            .unwrap();
        let sk = make_session_key(0xAA, 1, 5000);
        aa.add_session_key(&make_address(1), sk).unwrap();
        assert!(aa.validate_session_key(&make_address(1), &[0xAA; 32], "transfer", 1000));
        assert!(!aa.validate_session_key(&make_address(1), &[0xAA; 32], "transfer", 6000));
        assert!(!aa.validate_session_key(&make_address(1), &[0xAA; 32], "destroy", 1000));
    }

    #[test]
    fn add_session_key_unknown_account_fails() {
        let mut aa = AccountAbstraction::new();
        let sk = make_session_key(0xAA, 99, 5000);
        assert!(aa.add_session_key(&make_address(99), sk).is_err());
    }

    #[test]
    fn add_duplicate_session_key_fails() {
        let mut aa = AccountAbstraction::new();
        aa.register_smart_account(make_account_info(1, 10, SmartAccountType::SmartWallet))
            .unwrap();
        let sk = make_session_key(0xAA, 1, 5000);
        aa.add_session_key(&make_address(1), sk.clone()).unwrap();
        assert!(aa.add_session_key(&make_address(1), sk).is_err());
    }

    #[test]
    fn revoke_session_key_success() {
        let mut aa = AccountAbstraction::new();
        aa.register_smart_account(make_account_info(1, 10, SmartAccountType::SmartWallet))
            .unwrap();
        aa.add_session_key(&make_address(1), make_session_key(0xAA, 1, 5000))
            .unwrap();
        aa.revoke_session_key(&make_address(1), &[0xAA; 32])
            .unwrap();
        assert!(!aa.validate_session_key(&make_address(1), &[0xAA; 32], "transfer", 1000));
    }

    #[test]
    fn revoke_nonexistent_session_key_fails() {
        let mut aa = AccountAbstraction::new();
        aa.register_smart_account(make_account_info(1, 10, SmartAccountType::SmartWallet))
            .unwrap();
        aa.add_session_key(&make_address(1), make_session_key(0xAA, 1, 5000))
            .unwrap();
        let result = aa.revoke_session_key(&make_address(1), &[0xBB; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn active_session_keys_filters_expired() {
        let mut aa = AccountAbstraction::new();
        aa.register_smart_account(make_account_info(1, 10, SmartAccountType::SmartWallet))
            .unwrap();
        aa.add_session_key(&make_address(1), make_session_key(0xAA, 1, 2000))
            .unwrap();
        aa.add_session_key(&make_address(1), make_session_key(0xBB, 1, 5000))
            .unwrap();
        let active = aa.active_session_keys(&make_address(1), 3000);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].key, [0xBB; 32]);
    }

    #[test]
    fn active_session_keys_empty_for_unknown_account() {
        let aa = AccountAbstraction::new();
        assert!(aa.active_session_keys(&make_address(99), 0).is_empty());
    }

    #[test]
    fn default_creates_empty_aa() {
        let aa = AccountAbstraction::default();
        assert!(!aa.is_smart_account(&make_address(1)));
    }

    #[test]
    fn multisig_account_type() {
        let mut aa = AccountAbstraction::new();
        aa.register_smart_account(make_account_info(1, 10, SmartAccountType::MultiSig))
            .unwrap();
        assert_eq!(
            aa.get_account_type(&make_address(1)),
            Some(SmartAccountType::MultiSig)
        );
    }
}
