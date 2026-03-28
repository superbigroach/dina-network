use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-15  Meta-Transactions / Gasless  (ERC-2771 equivalent)
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ForwarderInfo {
    pub name: String,
    pub enabled: bool,
    pub registered_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ForwardRequest {
    pub from: Address,
    pub to: Address,
    pub value: u64,
    pub nonce: u64,
    pub data: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MetaTxRegistry {
    pub trusted_forwarders: BTreeMap<Address, ForwarderInfo>,
    pub nonces: BTreeMap<Address, u64>,
    pub admin: Address,
}

impl MetaTxRegistry {
    pub fn new(admin: Address) -> Self {
        Self {
            trusted_forwarders: BTreeMap::new(),
            nonces: BTreeMap::new(),
            admin,
        }
    }

    /// Register a trusted forwarder. Admin only.
    pub fn register_forwarder(
        &mut self,
        caller: Address,
        addr: Address,
        name: String,
        timestamp: u64,
    ) {
        assert!(
            caller == self.admin,
            "DRC15: only admin can register forwarders"
        );
        self.trusted_forwarders.insert(
            addr,
            ForwarderInfo {
                name,
                enabled: true,
                registered_at: timestamp,
            },
        );
    }

    /// Remove a trusted forwarder. Admin only.
    pub fn remove_forwarder(&mut self, caller: Address, addr: Address) {
        assert!(
            caller == self.admin,
            "DRC15: only admin can remove forwarders"
        );
        self.trusted_forwarders.remove(&addr);
    }

    /// Check if an address is a trusted (and enabled) forwarder.
    pub fn is_trusted_forwarder(&self, addr: &Address) -> bool {
        self.trusted_forwarders
            .get(addr)
            .map(|f| f.enabled)
            .unwrap_or(false)
    }

    /// Get the current nonce for an address.
    pub fn get_nonce(&self, addr: &Address) -> u64 {
        self.nonces.get(addr).copied().unwrap_or(0)
    }

    /// Verify a forward request and execute it (increment nonce).
    /// The caller must be a trusted forwarder.
    /// Returns true if verification passed and the request was "executed".
    pub fn verify_and_execute(&mut self, caller: Address, request: ForwardRequest) -> bool {
        // Caller must be a trusted forwarder
        assert!(
            self.is_trusted_forwarder(&caller),
            "DRC15: caller is not a trusted forwarder"
        );

        // Check nonce
        let expected_nonce = self.get_nonce(&request.from);
        assert!(
            request.nonce == expected_nonce,
            "DRC15: invalid nonce (expected {expected_nonce}, got {})",
            request.nonce
        );

        // Verify Ed25519 signature over SHA-256(from || to || value || data || nonce)
        assert!(
            request.signature.len() >= 64,
            "DRC15: signature must be at least 64 bytes"
        );
        let mut hasher = Sha256::new();
        hasher.update(request.from);
        hasher.update(request.to);
        hasher.update(request.value.to_le_bytes());
        hasher.update(&request.data);
        hasher.update(request.nonce.to_le_bytes());
        let message_hash = hasher.finalize();

        let vk = VerifyingKey::from_bytes(&request.from).expect("DRC15: invalid signer pubkey");
        let sig_bytes: [u8; 64] = request.signature[..64]
            .try_into()
            .expect("DRC15: signature too short");
        let sig = Signature::from_bytes(&sig_bytes);
        assert!(
            vk.verify(&message_hash, &sig).is_ok(),
            "DRC15: invalid signature"
        );

        // Increment nonce to prevent replay
        self.nonces.insert(request.from, expected_nonce + 1);

        true
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct RegisterForwarderArgs {
    addr: Address,
    name: String,
    #[serde(default)]
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddressArg {
    addr: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct VerifyAndExecuteArgs {
    request: ForwardRequest,
}

pub fn dispatch(
    state: &mut Option<MetaTxRegistry>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC15: already initialised");
            *state = Some(MetaTxRegistry::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "register_forwarder" => {
            let s = state.as_mut().expect("DRC15: not initialised");
            let a: RegisterForwarderArgs =
                serde_json::from_slice(args).expect("DRC15: bad register_forwarder args");
            s.register_forwarder(caller, a.addr, a.name, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }

        "remove_forwarder" => {
            let s = state.as_mut().expect("DRC15: not initialised");
            let a: AddressArg =
                serde_json::from_slice(args).expect("DRC15: bad remove_forwarder args");
            s.remove_forwarder(caller, a.addr);
            serde_json::to_vec("ok").unwrap()
        }

        "is_trusted_forwarder" => {
            let s = state.as_ref().expect("DRC15: not initialised");
            let a: AddressArg =
                serde_json::from_slice(args).expect("DRC15: bad is_trusted_forwarder args");
            serde_json::to_vec(&s.is_trusted_forwarder(&a.addr)).unwrap()
        }

        "get_nonce" => {
            let s = state.as_ref().expect("DRC15: not initialised");
            let a: AddressArg = serde_json::from_slice(args).expect("DRC15: bad get_nonce args");
            serde_json::to_vec(&s.get_nonce(&a.addr)).unwrap()
        }

        "verify_and_execute" => {
            let s = state.as_mut().expect("DRC15: not initialised");
            let a: VerifyAndExecuteArgs =
                serde_json::from_slice(args).expect("DRC15: bad verify_and_execute args");
            let result = s.verify_and_execute(caller, a.request);
            serde_json::to_vec(&result).unwrap()
        }

        _ => panic!("DRC15: unknown method '{method}'"),
    }
}
