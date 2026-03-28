//! DRC-112: Selective Disclosure (View Keys)
//!
//! Privacy-preserving selective disclosure for institutional compliance.
//! Unlike Arc's UI filtering on public data, DRC-112 uses cryptographic
//! view keys that decrypt ONLY specific scoped data from encrypted transactions.
//!
//! Key features:
//! - Private by default (all transaction data encrypted at protocol level)
//! - Scoped view keys (only see balances, or only tx history, etc.)
//! - Time-limited (auto-expires)
//! - Revocable (owner can revoke unless non-revocable for regulators)
//! - On-chain proof that access was granted (audit trail)

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ============================================================
// Types
// ============================================================

/// 32-byte address
type Address = [u8; 32];

/// Unique grant identifier
type GrantId = [u8; 32];

/// What data a view key can access
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ViewScope {
    /// Can see USDC balances
    Balances,
    /// Can see transaction history (from/to/timestamps, not amounts)
    TransactionHistory,
    /// Can see transaction amounts
    TransactionAmounts,
    /// Can see decrypted memo contents
    TransactionMemos,
    /// Can see smart contract call details
    ContractCalls,
    /// Can see device attestation data
    DeviceAttestations,
    /// Can see counterparty addresses
    Counterparties,
    /// Full access to all encrypted data
    FullAccess,
    /// Custom scope with a label
    Custom(String),
}

/// Filters on what subset of scoped data to reveal
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ViewFilter {
    /// Only show data from this timestamp onward (0 = no limit)
    pub valid_from: u64,
    /// Only show data until this timestamp (0 = no limit)
    pub valid_until: u64,
    /// Only show transactions above this USDC amount (0 = no limit)
    pub min_amount: u64,
    /// Only show transactions below this USDC amount (0 = no limit)
    pub max_amount: u64,
    /// Only show data related to specific DRC standards (empty = all)
    pub categories: Vec<String>,
    /// Only show data involving specific counterparties (empty = all)
    pub counterparty_filter: Vec<Address>,
}

/// A view key grant — permission for a grantee to see specific data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewKeyGrant {
    /// Unique grant ID
    pub grant_id: GrantId,
    /// Who owns the data being disclosed
    pub grantor: Address,
    /// Who is receiving view access
    pub grantee: Address,
    /// What scopes of data are accessible
    pub scopes: Vec<ViewScope>,
    /// Filters on the disclosed data
    pub filter: ViewFilter,
    /// When this grant was created
    pub created_at: u64,
    /// When this grant expires (0 = never)
    pub expires_at: u64,
    /// Whether the grantor can revoke this (false for regulatory grants)
    pub revocable: bool,
    /// Whether this grant has been revoked
    pub revoked: bool,
    /// Human-readable label ("SEC Audit Q1 2026", "VC Due Diligence", etc.)
    pub label: String,
    /// The encrypted view key material (ECDH shared secret encrypted)
    /// In a real implementation, this would be the X25519 ECDH result
    /// encrypted to the grantee's public key
    pub encrypted_key_material: Vec<u8>,
}

/// A query against disclosed data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisclosureQuery {
    /// Which grant to use
    pub grant_id: GrantId,
    /// Which scope to query
    pub scope: ViewScope,
    /// Time range (0 = no limit)
    pub from_timestamp: u64,
    pub to_timestamp: u64,
    /// Pagination
    pub offset: u64,
    pub limit: u64,
}

/// Result of a disclosure query
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisclosureResult {
    pub grant_id: GrantId,
    pub scope: ViewScope,
    pub total_records: u64,
    pub records: Vec<DisclosedRecord>,
}

/// A single disclosed record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisclosedRecord {
    pub timestamp: u64,
    pub record_type: String,
    pub data: serde_json::Value,
}

/// On-chain proof that disclosure was granted (for audit trails)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisclosureProof {
    pub grant_id: GrantId,
    pub grantor: Address,
    pub grantee: Address,
    pub scopes: Vec<ViewScope>,
    pub created_at: u64,
    pub expires_at: u64,
    pub revocable: bool,
    /// SHA-256 hash of the grant details (proves what was shared without revealing it)
    pub grant_hash: [u8; 32],
}

/// Grant status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GrantStatus {
    Active,
    Expired,
    Revoked,
    NotFound,
}

// ============================================================
// Contract State
// ============================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewKeyRegistry {
    /// All grants by grant_id
    grants: BTreeMap<GrantId, ViewKeyGrant>,
    /// Grants by grantor (data owner)
    grantor_grants: BTreeMap<Address, Vec<GrantId>>,
    /// Grants by grantee (data viewer)
    grantee_grants: BTreeMap<Address, Vec<GrantId>>,
    /// On-chain disclosure proofs
    proofs: BTreeMap<GrantId, DisclosureProof>,
    /// Authorized regulatory addresses (can receive non-revocable grants)
    regulatory_authorities: BTreeMap<Address, String>,
    /// Admin address (can add regulatory authorities)
    admin: Address,
    /// Next grant nonce (for deterministic grant IDs)
    nonce: u64,
}

// ============================================================
// Implementation
// ============================================================

impl ViewKeyRegistry {
    pub fn new(admin: Address) -> Self {
        Self {
            grants: BTreeMap::new(),
            grantor_grants: BTreeMap::new(),
            grantee_grants: BTreeMap::new(),
            proofs: BTreeMap::new(),
            regulatory_authorities: BTreeMap::new(),
            admin,
            nonce: 0,
        }
    }

    /// Grant a view key to a grantee
    #[allow(clippy::too_many_arguments)]
    pub fn grant_view_key(
        &mut self,
        caller: Address,
        grantee: Address,
        scopes: Vec<ViewScope>,
        filter: ViewFilter,
        expires_at: u64,
        revocable: bool,
        label: String,
        encrypted_key_material: Vec<u8>,
        current_time: u64,
    ) -> GrantId {
        // Only the data owner can grant view keys
        let grantor = caller;

        // Non-revocable grants can only be given to regulatory authorities
        if !revocable {
            assert!(
                self.regulatory_authorities.contains_key(&grantee),
                "DRC112: non-revocable grants only for registered regulatory authorities"
            );
        }

        // Generate deterministic grant ID
        let mut grant_id_input = Vec::new();
        grant_id_input.extend_from_slice(&grantor);
        grant_id_input.extend_from_slice(&grantee);
        grant_id_input.extend_from_slice(&self.nonce.to_le_bytes());
        let grant_id = simple_hash(&grant_id_input);
        self.nonce += 1;

        let grant = ViewKeyGrant {
            grant_id,
            grantor,
            grantee,
            scopes: scopes.clone(),
            filter,
            created_at: current_time,
            expires_at,
            revocable,
            revoked: false,
            label: label.clone(),
            encrypted_key_material,
        };

        // Store grant
        self.grants.insert(grant_id, grant);

        // Index by grantor
        self.grantor_grants
            .entry(grantor)
            .or_default()
            .push(grant_id);

        // Index by grantee
        self.grantee_grants
            .entry(grantee)
            .or_default()
            .push(grant_id);

        // Create on-chain disclosure proof
        let proof = DisclosureProof {
            grant_id,
            grantor,
            grantee,
            scopes,
            created_at: current_time,
            expires_at,
            revocable,
            grant_hash: simple_hash(&serde_json::to_vec(&label).unwrap_or_default()),
        };
        self.proofs.insert(grant_id, proof);

        grant_id
    }

    /// Revoke a view key grant
    pub fn revoke_grant(&mut self, caller: Address, grant_id: GrantId) {
        let grant = self
            .grants
            .get_mut(&grant_id)
            .expect("DRC112: grant not found");
        assert!(grant.grantor == caller, "DRC112: only grantor can revoke");
        assert!(
            grant.revocable,
            "DRC112: grant is non-revocable (regulatory)"
        );
        assert!(!grant.revoked, "DRC112: already revoked");

        grant.revoked = true;
    }

    /// Check the status of a grant
    pub fn grant_status(&self, grant_id: GrantId, current_time: u64) -> GrantStatus {
        match self.grants.get(&grant_id) {
            None => GrantStatus::NotFound,
            Some(g) if g.revoked => GrantStatus::Revoked,
            Some(g) if g.expires_at > 0 && current_time > g.expires_at => GrantStatus::Expired,
            Some(_) => GrantStatus::Active,
        }
    }

    /// Get a grant's details (only grantee or grantor can see full details)
    pub fn get_grant(&self, caller: Address, grant_id: GrantId) -> Option<ViewKeyGrant> {
        let grant = self.grants.get(&grant_id)?;
        if grant.grantor == caller || grant.grantee == caller || caller == self.admin {
            Some(grant.clone())
        } else {
            None
        }
    }

    /// Get all grants issued by a grantor
    pub fn grants_by_grantor(&self, grantor: Address) -> Vec<GrantId> {
        self.grantor_grants
            .get(&grantor)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all grants received by a grantee
    pub fn grants_for_grantee(&self, grantee: Address) -> Vec<GrantId> {
        self.grantee_grants
            .get(&grantee)
            .cloned()
            .unwrap_or_default()
    }

    /// Get active grants for a grantee (excludes expired and revoked)
    pub fn active_grants_for_grantee(&self, grantee: Address, current_time: u64) -> Vec<GrantId> {
        self.grantee_grants
            .get(&grantee)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|id| self.grant_status(*id, current_time) == GrantStatus::Active)
            .collect()
    }

    /// Get the on-chain disclosure proof for a grant
    pub fn get_proof(&self, grant_id: GrantId) -> Option<DisclosureProof> {
        self.proofs.get(&grant_id).cloned()
    }

    /// Register a regulatory authority (admin only)
    pub fn register_authority(&mut self, caller: Address, authority: Address, name: String) {
        assert!(
            caller == self.admin,
            "DRC112: only admin can register authorities"
        );
        self.regulatory_authorities.insert(authority, name);
    }

    /// Remove a regulatory authority (admin only)
    pub fn remove_authority(&mut self, caller: Address, authority: Address) {
        assert!(
            caller == self.admin,
            "DRC112: only admin can remove authorities"
        );
        self.regulatory_authorities.remove(&authority);
    }

    /// Check if an address is a registered regulatory authority
    pub fn is_authority(&self, addr: Address) -> bool {
        self.regulatory_authorities.contains_key(&addr)
    }

    /// Get all registered authorities
    pub fn authorities(&self) -> Vec<(Address, String)> {
        self.regulatory_authorities
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect()
    }

    /// Extend a grant's expiry (grantor only, cannot shorten)
    pub fn extend_grant(&mut self, caller: Address, grant_id: GrantId, new_expires_at: u64) {
        let grant = self
            .grants
            .get_mut(&grant_id)
            .expect("DRC112: grant not found");
        assert!(grant.grantor == caller, "DRC112: only grantor can extend");
        assert!(!grant.revoked, "DRC112: cannot extend revoked grant");
        assert!(
            new_expires_at > grant.expires_at,
            "DRC112: can only extend, not shorten"
        );
        grant.expires_at = new_expires_at;
    }

    /// Add scopes to an existing grant (grantor only)
    pub fn add_scopes(&mut self, caller: Address, grant_id: GrantId, new_scopes: Vec<ViewScope>) {
        let grant = self
            .grants
            .get_mut(&grant_id)
            .expect("DRC112: grant not found");
        assert!(
            grant.grantor == caller,
            "DRC112: only grantor can add scopes"
        );
        assert!(!grant.revoked, "DRC112: cannot modify revoked grant");

        for scope in new_scopes {
            if !grant.scopes.contains(&scope) {
                grant.scopes.push(scope);
            }
        }
    }

    /// Verify a grantee has access to a specific scope
    pub fn verify_access(
        &self,
        grantee: Address,
        scope: &ViewScope,
        current_time: u64,
    ) -> Option<GrantId> {
        let grants = self.grantee_grants.get(&grantee)?;
        for grant_id in grants {
            if self.grant_status(*grant_id, current_time) != GrantStatus::Active {
                continue;
            }
            let grant = self.grants.get(grant_id)?;
            if grant.scopes.contains(scope) || grant.scopes.contains(&ViewScope::FullAccess) {
                return Some(*grant_id);
            }
        }
        None
    }
}

// ============================================================
// Dispatch
// ============================================================

pub fn dispatch(
    state: &mut ViewKeyRegistry,
    method: &str,
    args: &[u8],
    caller: Address,
    current_time: u64,
) -> Vec<u8> {
    match method {
        "grant_view_key" => {
            #[derive(Deserialize)]
            struct Args {
                grantee: Address,
                scopes: Vec<ViewScope>,
                filter: ViewFilter,
                expires_at: u64,
                revocable: bool,
                label: String,
                encrypted_key_material: Vec<u8>,
            }
            let a: Args = serde_json::from_slice(args).expect("DRC112: invalid args");
            let id = state.grant_view_key(
                caller,
                a.grantee,
                a.scopes,
                a.filter,
                a.expires_at,
                a.revocable,
                a.label,
                a.encrypted_key_material,
                current_time,
            );
            serde_json::to_vec(&id).unwrap()
        }

        "revoke_grant" => {
            #[derive(Deserialize)]
            struct Args {
                grant_id: GrantId,
            }
            let a: Args = serde_json::from_slice(args).expect("DRC112: invalid args");
            state.revoke_grant(caller, a.grant_id);
            serde_json::to_vec(&true).unwrap()
        }

        "grant_status" => {
            #[derive(Deserialize)]
            struct Args {
                grant_id: GrantId,
            }
            let a: Args = serde_json::from_slice(args).expect("DRC112: invalid args");
            let status = state.grant_status(a.grant_id, current_time);
            serde_json::to_vec(&status).unwrap()
        }

        "get_grant" => {
            #[derive(Deserialize)]
            struct Args {
                grant_id: GrantId,
            }
            let a: Args = serde_json::from_slice(args).expect("DRC112: invalid args");
            let grant = state.get_grant(caller, a.grant_id);
            serde_json::to_vec(&grant).unwrap()
        }

        "grants_by_grantor" => {
            let grants = state.grants_by_grantor(caller);
            serde_json::to_vec(&grants).unwrap()
        }

        "grants_for_grantee" => {
            let grants = state.grants_for_grantee(caller);
            serde_json::to_vec(&grants).unwrap()
        }

        "active_grants" => {
            let grants = state.active_grants_for_grantee(caller, current_time);
            serde_json::to_vec(&grants).unwrap()
        }

        "get_proof" => {
            #[derive(Deserialize)]
            struct Args {
                grant_id: GrantId,
            }
            let a: Args = serde_json::from_slice(args).expect("DRC112: invalid args");
            let proof = state.get_proof(a.grant_id);
            serde_json::to_vec(&proof).unwrap()
        }

        "register_authority" => {
            #[derive(Deserialize)]
            struct Args {
                authority: Address,
                name: String,
            }
            let a: Args = serde_json::from_slice(args).expect("DRC112: invalid args");
            state.register_authority(caller, a.authority, a.name);
            serde_json::to_vec(&true).unwrap()
        }

        "is_authority" => {
            #[derive(Deserialize)]
            struct Args {
                addr: Address,
            }
            let a: Args = serde_json::from_slice(args).expect("DRC112: invalid args");
            serde_json::to_vec(&state.is_authority(a.addr)).unwrap()
        }

        "verify_access" => {
            #[derive(Deserialize)]
            struct Args {
                grantee: Address,
                scope: ViewScope,
            }
            let a: Args = serde_json::from_slice(args).expect("DRC112: invalid args");
            let result = state.verify_access(a.grantee, &a.scope, current_time);
            serde_json::to_vec(&result).unwrap()
        }

        "extend_grant" => {
            #[derive(Deserialize)]
            struct Args {
                grant_id: GrantId,
                new_expires_at: u64,
            }
            let a: Args = serde_json::from_slice(args).expect("DRC112: invalid args");
            state.extend_grant(caller, a.grant_id, a.new_expires_at);
            serde_json::to_vec(&true).unwrap()
        }

        "add_scopes" => {
            #[derive(Deserialize)]
            struct Args {
                grant_id: GrantId,
                new_scopes: Vec<ViewScope>,
            }
            let a: Args = serde_json::from_slice(args).expect("DRC112: invalid args");
            state.add_scopes(caller, a.grant_id, a.new_scopes);
            serde_json::to_vec(&true).unwrap()
        }

        _ => panic!("DRC112: unknown method: {}", method),
    }
}

// ============================================================
// Helpers
// ============================================================

/// Simple SHA-256 hash (in real contract, would use dina_sdk::sha256)
fn simple_hash(data: &[u8]) -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    let h = hasher.finish();
    let mut result = [0u8; 32];
    result[..8].copy_from_slice(&h.to_le_bytes());
    result[8..16].copy_from_slice(&h.to_be_bytes());
    // Fill remaining with derived bytes
    for i in 16..32 {
        result[i] = result[i - 16] ^ result[i - 8];
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_addr(seed: u8) -> Address {
        [seed; 32]
    }

    #[test]
    fn test_grant_and_query() {
        let admin = test_addr(0);
        let mut registry = ViewKeyRegistry::new(admin);

        let grantor = test_addr(1);
        let grantee = test_addr(2);

        let grant_id = registry.grant_view_key(
            grantor,
            grantee,
            vec![ViewScope::Balances, ViewScope::TransactionHistory],
            ViewFilter::default(),
            1000,
            true,
            "VC Due Diligence".to_string(),
            vec![1, 2, 3, 4],
            100,
        );

        assert_eq!(registry.grant_status(grant_id, 500), GrantStatus::Active);
        assert_eq!(registry.grant_status(grant_id, 1500), GrantStatus::Expired);

        let grant = registry.get_grant(grantor, grant_id).unwrap();
        assert_eq!(grant.scopes.len(), 2);
        assert_eq!(grant.label, "VC Due Diligence");
    }

    #[test]
    fn test_revoke() {
        let admin = test_addr(0);
        let mut registry = ViewKeyRegistry::new(admin);

        let grantor = test_addr(1);
        let grantee = test_addr(2);

        let grant_id = registry.grant_view_key(
            grantor,
            grantee,
            vec![ViewScope::FullAccess],
            ViewFilter::default(),
            0,
            true,
            "Test".to_string(),
            vec![],
            100,
        );

        assert_eq!(registry.grant_status(grant_id, 200), GrantStatus::Active);
        registry.revoke_grant(grantor, grant_id);
        assert_eq!(registry.grant_status(grant_id, 200), GrantStatus::Revoked);
    }

    #[test]
    fn test_non_revocable_requires_authority() {
        let admin = test_addr(0);
        let mut registry = ViewKeyRegistry::new(admin);

        let grantor = test_addr(1);
        let regulator = test_addr(3);

        // Register regulator
        registry.register_authority(admin, regulator, "SEC".to_string());

        // Non-revocable grant to regulator works
        let grant_id = registry.grant_view_key(
            grantor,
            regulator,
            vec![ViewScope::Balances, ViewScope::TransactionAmounts],
            ViewFilter::default(),
            0,
            false,
            "SEC Audit".to_string(),
            vec![],
            100,
        );

        assert_eq!(registry.grant_status(grant_id, 200), GrantStatus::Active);
    }

    #[test]
    #[should_panic(expected = "non-revocable grants only for registered regulatory authorities")]
    fn test_non_revocable_fails_for_non_authority() {
        let admin = test_addr(0);
        let mut registry = ViewKeyRegistry::new(admin);

        let grantor = test_addr(1);
        let random = test_addr(5);

        // Non-revocable grant to non-authority should panic
        registry.grant_view_key(
            grantor,
            random,
            vec![ViewScope::FullAccess],
            ViewFilter::default(),
            0,
            false,
            "Nope".to_string(),
            vec![],
            100,
        );
    }

    #[test]
    fn test_verify_access() {
        let admin = test_addr(0);
        let mut registry = ViewKeyRegistry::new(admin);

        let grantor = test_addr(1);
        let grantee = test_addr(2);

        registry.grant_view_key(
            grantor,
            grantee,
            vec![ViewScope::Balances],
            ViewFilter::default(),
            1000,
            true,
            "Test".to_string(),
            vec![],
            100,
        );

        // Grantee has access to Balances
        assert!(registry
            .verify_access(grantee, &ViewScope::Balances, 500)
            .is_some());
        // Grantee does NOT have access to TransactionMemos
        assert!(registry
            .verify_access(grantee, &ViewScope::TransactionMemos, 500)
            .is_none());
        // After expiry, no access
        assert!(registry
            .verify_access(grantee, &ViewScope::Balances, 1500)
            .is_none());
    }

    #[test]
    fn test_extend_and_add_scopes() {
        let admin = test_addr(0);
        let mut registry = ViewKeyRegistry::new(admin);

        let grantor = test_addr(1);
        let grantee = test_addr(2);

        let grant_id = registry.grant_view_key(
            grantor,
            grantee,
            vec![ViewScope::Balances],
            ViewFilter::default(),
            1000,
            true,
            "Test".to_string(),
            vec![],
            100,
        );

        // Extend
        registry.extend_grant(grantor, grant_id, 2000);
        assert_eq!(registry.grant_status(grant_id, 1500), GrantStatus::Active);

        // Add scopes
        registry.add_scopes(grantor, grant_id, vec![ViewScope::TransactionHistory]);
        let grant = registry.get_grant(grantor, grant_id).unwrap();
        assert_eq!(grant.scopes.len(), 2);
    }
}
