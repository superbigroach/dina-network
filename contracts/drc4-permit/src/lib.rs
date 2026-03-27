use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-4  Gasless Approvals  (ERC-2612 equivalent)
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PermitRegistry {
    /// The DRC-1 token contract address this permit registry serves
    pub token_contract: Address,
    /// Nonces per owner to prevent replay attacks
    pub nonces: BTreeMap<Address, u64>,
    /// Approvals: (owner, spender) -> amount
    pub allowances: BTreeMap<(Address, Address), u64>,
    /// Chain ID for domain separation
    pub chain_id: u64,
}

impl PermitRegistry {
    pub fn new(token_contract: Address, chain_id: u64) -> Self {
        Self {
            token_contract,
            chain_id,
            nonces: BTreeMap::new(),
            allowances: BTreeMap::new(),
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn nonces(&self, owner: &Address) -> u64 {
        self.nonces.get(owner).copied().unwrap_or(0)
    }

    pub fn allowance(&self, owner: &Address, spender: &Address) -> u64 {
        self.allowances
            .get(&(*owner, *spender))
            .copied()
            .unwrap_or(0)
    }

    pub fn domain_separator(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"DRC4-Permit-v1");
        hasher.update(self.chain_id.to_le_bytes());
        hasher.update(self.token_contract);
        let result = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result);
        out
    }

    // -- Mutations -----------------------------------------------------------

    /// Apply a gasless approval via a signed permit.
    ///
    /// In production the `signature_bytes` would be verified against the
    /// owner's public key over a hash of (domain_separator, owner, spender,
    /// amount, nonce, deadline).  Here we build the same digest and verify
    /// that `signature_bytes` equals the SHA-256 of that digest (a
    /// deterministic stand-in until ed25519 host-call is wired up).
    pub fn permit(
        &mut self,
        owner: Address,
        spender: Address,
        amount: u64,
        deadline: u64,
        current_time: u64,
        signature_bytes: &[u8],
    ) {
        assert!(
            current_time <= deadline,
            "DRC4: permit expired (now={current_time}, deadline={deadline})"
        );

        let nonce = self.nonces(&owner);

        // Build the permit digest
        let digest = self.build_permit_digest(&owner, &spender, amount, nonce, deadline);

        // Verify signature: in the on-chain runtime this would call an
        // ed25519-verify host function.  For now we accept SHA-256(digest)
        // as a valid "signature" so the contract is fully testable without
        // a crypto host import.
        let expected_sig = Sha256::digest(digest);
        assert!(
            signature_bytes == expected_sig.as_slice(),
            "DRC4: invalid permit signature"
        );

        // Apply approval and bump nonce
        self.allowances.insert((owner, spender), amount);
        self.nonces.insert(owner, nonce + 1);
    }

    /// Build the hash that must be signed for a permit.
    pub fn build_permit_digest(
        &self,
        owner: &Address,
        spender: &Address,
        amount: u64,
        nonce: u64,
        deadline: u64,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.domain_separator());
        hasher.update(owner);
        hasher.update(spender);
        hasher.update(amount.to_le_bytes());
        hasher.update(nonce.to_le_bytes());
        hasher.update(deadline.to_le_bytes());
        let result = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result);
        out
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    token_contract: Address,
    chain_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct PermitArgs {
    owner: Address,
    spender: Address,
    amount: u64,
    deadline: u64,
    current_time: u64,
    signature_bytes: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct NoncesArgs {
    owner: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct AllowanceArgs {
    owner: Address,
    spender: Address,
}

pub fn dispatch(
    state: &mut Option<PermitRegistry>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        // -- Init ------------------------------------------------------------
        "init" => {
            assert!(state.is_none(), "DRC4: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC4: bad init args");
            *state = Some(PermitRegistry::new(a.token_contract, a.chain_id));
            let _ = caller;
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "nonces" => {
            let s = state.as_ref().expect("DRC4: not initialised");
            let a: NoncesArgs = serde_json::from_slice(args).expect("DRC4: bad nonces args");
            serde_json::to_vec(&s.nonces(&a.owner)).unwrap()
        }
        "domain_separator" => {
            let s = state.as_ref().expect("DRC4: not initialised");
            serde_json::to_vec(&s.domain_separator()).unwrap()
        }
        "allowance" => {
            let s = state.as_ref().expect("DRC4: not initialised");
            let a: AllowanceArgs =
                serde_json::from_slice(args).expect("DRC4: bad allowance args");
            serde_json::to_vec(&s.allowance(&a.owner, &a.spender)).unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "permit" => {
            let s = state.as_mut().expect("DRC4: not initialised");
            let a: PermitArgs = serde_json::from_slice(args).expect("DRC4: bad permit args");
            s.permit(
                a.owner,
                a.spender,
                a.amount,
                a.deadline,
                a.current_time,
                &a.signature_bytes,
            );
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC4: unknown method '{method}'"),
    }
}
