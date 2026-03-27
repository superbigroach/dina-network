use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::{DinaError, DinaResult};
use crate::types::{Address, Hash};

/// An account on the Dina blockchain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// The account address (derived from public key).
    pub address: Address,
    /// Balance in the smallest unit (micro-USDC, 6 decimals).
    pub balance: u64,
    /// Nonce -- incremented with each outgoing transaction.
    pub nonce: u64,
    /// If this account is a contract, the hash of its WASM code.
    pub code_hash: Option<Hash>,
    /// Merkle root of this account's contract storage (if any).
    pub storage_root: Option<Hash>,
}

impl Account {
    /// Create a new account with zero balance and nonce.
    pub fn new(address: Address) -> Self {
        Self {
            address,
            balance: 0,
            nonce: 0,
            code_hash: None,
            storage_root: None,
        }
    }

    /// Create a new account with an initial balance.
    pub fn with_balance(address: Address, balance: u64) -> Self {
        Self {
            balance,
            ..Self::new(address)
        }
    }
}

/// In-memory account state manager.
#[derive(Clone, Debug, Default)]
pub struct AccountState {
    accounts: HashMap<Address, Account>,
}

impl AccountState {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    /// Get an account by address. Returns None if the account does not exist.
    pub fn get_account(&self, address: &Address) -> Option<&Account> {
        self.accounts.get(address)
    }

    /// Insert or replace an account.
    pub fn set_account(&mut self, account: Account) {
        self.accounts.insert(account.address, account);
    }

    /// Transfer `amount` from one account to another.
    /// Creates the receiver account if it does not exist.
    pub fn transfer(&mut self, from: &Address, to: &Address, amount: u64) -> DinaResult<()> {
        let sender = self
            .accounts
            .get(from)
            .ok_or_else(|| DinaError::AccountNotFound(from.to_string()))?;

        if sender.balance < amount {
            return Err(DinaError::InsufficientBalance {
                have: sender.balance,
                need: amount,
            });
        }

        if !self.accounts.contains_key(to) {
            self.accounts.insert(*to, Account::new(*to));
        }

        // Check for receiver overflow before mutating state.
        let receiver_balance = self.accounts.get(to).unwrap().balance;
        if receiver_balance.checked_add(amount).is_none() {
            return Err(DinaError::Custom(format!(
                "receiver balance overflow: {} + {} exceeds u64::MAX",
                receiver_balance, amount
            )));
        }

        self.accounts.get_mut(from).unwrap().balance -= amount;
        self.accounts.get_mut(to).unwrap().balance += amount;

        Ok(())
    }

    /// Deduct a fee from an account.
    pub fn deduct_fee(&mut self, address: &Address, fee: u64) -> DinaResult<()> {
        let account = self
            .accounts
            .get_mut(address)
            .ok_or_else(|| DinaError::AccountNotFound(address.to_string()))?;

        if account.balance < fee {
            return Err(DinaError::InsufficientBalance {
                have: account.balance,
                need: fee,
            });
        }

        account.balance -= fee;
        Ok(())
    }

    /// Increment the nonce of an account.
    ///
    /// Returns an error if the nonce would overflow (extremely unlikely in
    /// practice but prevents undefined behavior).
    pub fn increment_nonce(&mut self, address: &Address) -> DinaResult<()> {
        let account = self
            .accounts
            .get_mut(address)
            .ok_or_else(|| DinaError::AccountNotFound(address.to_string()))?;

        account.nonce = account.nonce.checked_add(1).ok_or_else(|| {
            DinaError::Custom(format!(
                "nonce overflow for account {}",
                address
            ))
        })?;
        Ok(())
    }

    /// Credit an amount to an account. Creates the account if it does not exist.
    ///
    /// Uses saturating addition to prevent overflow. In production, balances
    /// should never approach `u64::MAX` (which would represent ~18 quintillion
    /// micro-USDC), but this prevents a panic in adversarial conditions.
    pub fn credit(&mut self, address: &Address, amount: u64) {
        let account = self
            .accounts
            .entry(*address)
            .or_insert_with(|| Account::new(*address));
        account.balance = account.balance.saturating_add(amount);
    }

    /// Return the number of accounts.
    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    /// Check if the state has no accounts.
    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    /// Iterate over all accounts.
    pub fn iter(&self) -> impl Iterator<Item = (&Address, &Account)> {
        self.accounts.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(byte: u8) -> Address {
        Address([byte; 32])
    }

    #[test]
    fn credit_creates_account() {
        let mut state = AccountState::new();
        let a = addr(1);
        state.credit(&a, 500);
        assert_eq!(state.get_account(&a).unwrap().balance, 500);
    }

    #[test]
    fn transfer_works() {
        let mut state = AccountState::new();
        let a = addr(1);
        let b = addr(2);
        state.credit(&a, 1000);
        state.transfer(&a, &b, 300).unwrap();
        assert_eq!(state.get_account(&a).unwrap().balance, 700);
        assert_eq!(state.get_account(&b).unwrap().balance, 300);
    }

    #[test]
    fn transfer_insufficient_balance() {
        let mut state = AccountState::new();
        let a = addr(1);
        let b = addr(2);
        state.credit(&a, 100);
        let err = state.transfer(&a, &b, 200).unwrap_err();
        assert!(matches!(err, DinaError::InsufficientBalance { .. }));
    }

    #[test]
    fn deduct_fee_and_increment_nonce() {
        let mut state = AccountState::new();
        let a = addr(1);
        state.credit(&a, 1000);
        state.deduct_fee(&a, 50).unwrap();
        assert_eq!(state.get_account(&a).unwrap().balance, 950);
        state.increment_nonce(&a).unwrap();
        assert_eq!(state.get_account(&a).unwrap().nonce, 1);
    }

    #[test]
    fn account_not_found() {
        let mut state = AccountState::new();
        let a = addr(99);
        assert!(state.deduct_fee(&a, 10).is_err());
        assert!(state.increment_nonce(&a).is_err());
    }
}
