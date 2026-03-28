use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-14  Contract Signature Verification  (ERC-1271 equivalent)
// ---------------------------------------------------------------------------

type Address = [u8; 32];

/// Returned when the signature is valid for the given hash + signer.
pub const MAGIC_VALUE: u32 = 0x1626ba7e;

/// Returned when the signature is invalid.
pub const INVALID: u32 = 0xffffffff;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SignatureVerifier {
    /// Maps contract address -> list of authorised signers.
    pub authorized_signers: BTreeMap<Address, Vec<Address>>,
    pub owner: Address,
}

impl SignatureVerifier {
    pub fn new(owner: Address) -> Self {
        Self {
            authorized_signers: BTreeMap::new(),
            owner,
        }
    }

    /// Check if `signer` is authorized for any contract and return the
    /// magic value or invalid sentinel.
    pub fn is_valid_signature(&self, _hash: [u8; 32], signer: Address) -> u32 {
        // Search all contracts for this signer
        for signers in self.authorized_signers.values() {
            if signers.contains(&signer) {
                return MAGIC_VALUE;
            }
        }
        INVALID
    }

    /// Check if `signer` is authorized for a specific contract address.
    pub fn is_valid_signature_for(
        &self,
        _hash: [u8; 32],
        contract_addr: Address,
        signer: Address,
    ) -> u32 {
        if let Some(signers) = self.authorized_signers.get(&contract_addr) {
            if signers.contains(&signer) {
                return MAGIC_VALUE;
            }
        }
        INVALID
    }

    /// Add an authorized signer for a contract address. Owner only.
    pub fn add_signer(&mut self, caller: Address, contract_addr: Address, signer: Address) {
        assert!(caller == self.owner, "DRC14: only owner can add signers");
        let signers = self.authorized_signers.entry(contract_addr).or_default();
        if !signers.contains(&signer) {
            signers.push(signer);
        }
    }

    /// Remove an authorized signer for a contract address. Owner only.
    pub fn remove_signer(&mut self, caller: Address, contract_addr: Address, signer: Address) {
        assert!(caller == self.owner, "DRC14: only owner can remove signers");
        if let Some(signers) = self.authorized_signers.get_mut(&contract_addr) {
            signers.retain(|s| s != &signer);
        }
    }

    /// Get all authorized signers for a contract address.
    pub fn signers_of(&self, contract_addr: &Address) -> Vec<Address> {
        self.authorized_signers
            .get(contract_addr)
            .cloned()
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct IsValidSignatureArgs {
    hash: [u8; 32],
    signer: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct IsValidSignatureForArgs {
    hash: [u8; 32],
    contract_addr: Address,
    signer: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddSignerArgs {
    contract_addr: Address,
    signer: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct RemoveSignerArgs {
    contract_addr: Address,
    signer: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct SignersOfArgs {
    contract_addr: Address,
}

pub fn dispatch(
    state: &mut Option<SignatureVerifier>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC14: already initialised");
            *state = Some(SignatureVerifier::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "is_valid_signature" => {
            let s = state.as_ref().expect("DRC14: not initialised");
            let a: IsValidSignatureArgs =
                serde_json::from_slice(args).expect("DRC14: bad is_valid_signature args");
            let result = s.is_valid_signature(a.hash, a.signer);
            serde_json::to_vec(&result).unwrap()
        }

        "is_valid_signature_for" => {
            let s = state.as_ref().expect("DRC14: not initialised");
            let a: IsValidSignatureForArgs =
                serde_json::from_slice(args).expect("DRC14: bad is_valid_signature_for args");
            let result = s.is_valid_signature_for(a.hash, a.contract_addr, a.signer);
            serde_json::to_vec(&result).unwrap()
        }

        "add_signer" => {
            let s = state.as_mut().expect("DRC14: not initialised");
            let a: AddSignerArgs =
                serde_json::from_slice(args).expect("DRC14: bad add_signer args");
            s.add_signer(caller, a.contract_addr, a.signer);
            serde_json::to_vec("ok").unwrap()
        }

        "remove_signer" => {
            let s = state.as_mut().expect("DRC14: not initialised");
            let a: RemoveSignerArgs =
                serde_json::from_slice(args).expect("DRC14: bad remove_signer args");
            s.remove_signer(caller, a.contract_addr, a.signer);
            serde_json::to_vec("ok").unwrap()
        }

        "signers_of" => {
            let s = state.as_ref().expect("DRC14: not initialised");
            let a: SignersOfArgs =
                serde_json::from_slice(args).expect("DRC14: bad signers_of args");
            serde_json::to_vec(&s.signers_of(&a.contract_addr)).unwrap()
        }

        _ => panic!("DRC14: unknown method '{method}'"),
    }
}
