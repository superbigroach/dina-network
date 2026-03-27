use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-78  Device Attestation Chain (Web of Trust for Machines)
// ---------------------------------------------------------------------------

type Address = [u8; 32];
type AttestationId = u64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AttestationType {
    Identity,
    Capability,
    Firmware,
    Location,
    Uptime,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Attestation {
    pub id: AttestationId,
    pub attester: Address,
    pub attested: Address,
    pub attestation_type: AttestationType,
    pub confidence: u64,     // 0-10000 bps
    pub timestamp: u64,
    pub evidence_hash: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AttestationChainState {
    pub owner: Address,
    pub attestations: BTreeMap<AttestationId, Attestation>,
    pub next_id: AttestationId,
    /// Cache of computed trust scores
    pub trust_cache: BTreeMap<Address, u64>,
}

impl AttestationChainState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            attestations: BTreeMap::new(),
            next_id: 1,
            trust_cache: BTreeMap::new(),
        }
    }

    pub fn attest(
        &mut self,
        caller: Address,
        device: Address,
        attestation_type: AttestationType,
        confidence: u64,
        evidence_hash: [u8; 32],
        timestamp: u64,
    ) -> AttestationId {
        assert!(caller != device, "DRC78: cannot attest yourself");
        assert!(confidence <= 10000, "DRC78: confidence max 10000 bps");
        assert!(confidence > 0, "DRC78: confidence must be positive");

        let id = self.next_id;
        self.next_id += 1;
        self.attestations.insert(id, Attestation {
            id, attester: caller, attested: device,
            attestation_type, confidence, timestamp, evidence_hash,
        });

        // Invalidate trust cache for attested device
        self.trust_cache.remove(&device);

        id
    }

    pub fn get_attestations_for(&self, device: &Address) -> Vec<&Attestation> {
        self.attestations.values()
            .filter(|a| &a.attested == device)
            .collect()
    }

    /// Calculate trust score for a device.
    /// Score = sum of (attester_base_score * attestation_confidence) / total_attestations
    /// Attester base score = 1000 (base) + number of attestations they themselves have received * 100
    /// This creates a chain-of-trust: devices attested by well-attested devices score higher.
    pub fn trust_score(&mut self, device: &Address) -> u64 {
        if let Some(&cached) = self.trust_cache.get(device) {
            return cached;
        }

        let attestations: Vec<(Address, u64)> = self.attestations.values()
            .filter(|a| &a.attested == device)
            .map(|a| (a.attester, a.confidence))
            .collect();

        if attestations.is_empty() {
            return 0;
        }

        let mut total_weighted = 0u64;
        let mut total_weight = 0u64;

        for (attester, confidence) in &attestations {
            // Attester's own score: base 1000 + 100 per attestation they received
            let attester_attestation_count = self.attestations.values()
                .filter(|a| &a.attested == attester)
                .count() as u64;
            let attester_base = 1000 + attester_attestation_count * 100;

            total_weighted += attester_base * confidence;
            total_weight += attester_base;
        }

        let score = if total_weight > 0 { total_weighted / total_weight } else { 0 };
        self.trust_cache.insert(*device, score);
        score
    }

    /// Check if two devices have mutually attested each other.
    pub fn mutual_attestation(&self, device_a: &Address, device_b: &Address) -> bool {
        let a_attests_b = self.attestations.values()
            .any(|a| &a.attester == device_a && &a.attested == device_b);
        let b_attests_a = self.attestations.values()
            .any(|a| &a.attester == device_b && &a.attested == device_a);
        a_attests_b && b_attests_a
    }

    /// Get the attestation chain for a device up to a given depth.
    /// Returns all devices in the trust chain and their attestation connections.
    pub fn attestation_chain(&self, device: &Address, max_depth: u32) -> Vec<(Address, Address, AttestationType)> {
        let mut chain = Vec::new();
        let mut visited: Vec<Address> = vec![*device];
        let mut frontier: Vec<Address> = vec![*device];

        for _depth in 0..max_depth {
            let mut next_frontier = Vec::new();
            for current in &frontier {
                for attestation in self.attestations.values() {
                    if &attestation.attested == current && !visited.contains(&attestation.attester) {
                        chain.push((attestation.attester, attestation.attested, attestation.attestation_type.clone()));
                        visited.push(attestation.attester);
                        next_frontier.push(attestation.attester);
                    }
                }
            }
            if next_frontier.is_empty() {
                break;
            }
            frontier = next_frontier;
        }
        chain
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct AttestArgs { device: Address, attestation_type: AttestationType, confidence: u64, evidence_hash: [u8; 32], timestamp: u64 }
#[derive(Serialize, Deserialize, Debug)]
struct DeviceArgs { device: Address }
#[derive(Serialize, Deserialize, Debug)]
struct MutualArgs { device_a: Address, device_b: Address }
#[derive(Serialize, Deserialize, Debug)]
struct ChainArgs { device: Address, depth: u32 }

pub fn dispatch(
    state: &mut Option<AttestationChainState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC78: already initialised");
            *state = Some(AttestationChainState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "attest" => {
            let s = state.as_mut().expect("DRC78: not initialised");
            let a: AttestArgs = serde_json::from_slice(args).expect("DRC78: bad args");
            let id = s.attest(caller, a.device, a.attestation_type, a.confidence, a.evidence_hash, a.timestamp);
            serde_json::to_vec(&id).unwrap()
        }
        "get_attestations_for" => {
            let s = state.as_ref().expect("DRC78: not initialised");
            let a: DeviceArgs = serde_json::from_slice(args).expect("DRC78: bad args");
            serde_json::to_vec(&s.get_attestations_for(&a.device)).unwrap()
        }
        "trust_score" => {
            let s = state.as_mut().expect("DRC78: not initialised");
            let a: DeviceArgs = serde_json::from_slice(args).expect("DRC78: bad args");
            let score = s.trust_score(&a.device);
            serde_json::to_vec(&score).unwrap()
        }
        "mutual_attestation" => {
            let s = state.as_ref().expect("DRC78: not initialised");
            let a: MutualArgs = serde_json::from_slice(args).expect("DRC78: bad args");
            serde_json::to_vec(&s.mutual_attestation(&a.device_a, &a.device_b)).unwrap()
        }
        "attestation_chain" => {
            let s = state.as_ref().expect("DRC78: not initialised");
            let a: ChainArgs = serde_json::from_slice(args).expect("DRC78: bad args");
            serde_json::to_vec(&s.attestation_chain(&a.device, a.depth)).unwrap()
        }
        _ => panic!("DRC78: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [0u8; 32];
    const DEV_A: Address = [1u8; 32];
    const DEV_B: Address = [2u8; 32];
    const DEV_C: Address = [3u8; 32];
    const DEV_D: Address = [4u8; 32];

    fn setup() -> AttestationChainState {
        let mut s = AttestationChainState::new(OWNER);
        // A attests B (identity, high confidence)
        s.attest(DEV_A, DEV_B, AttestationType::Identity, 9000, [0xAA; 32], 1000);
        // A attests C (capability)
        s.attest(DEV_A, DEV_C, AttestationType::Capability, 7000, [0xBB; 32], 1001);
        // B attests C (firmware)
        s.attest(DEV_B, DEV_C, AttestationType::Firmware, 8000, [0xCC; 32], 1002);
        s
    }

    #[test]
    fn test_attest_and_query() {
        let s = setup();
        let attestations = s.get_attestations_for(&DEV_C);
        assert_eq!(attestations.len(), 2); // from A and B
    }

    #[test]
    fn test_trust_score_basic() {
        let mut s = setup();
        let score_b = s.trust_score(&DEV_B);
        assert!(score_b > 0); // B is attested by A
        let score_d = s.trust_score(&DEV_D);
        assert_eq!(score_d, 0); // D has no attestations
    }

    #[test]
    fn test_trust_score_chain_effect() {
        let mut s = setup();
        // C is attested by both A and B.
        // B itself is attested (by A), so B's attestation of C carries more weight.
        let score_c = s.trust_score(&DEV_C);
        let score_b = s.trust_score(&DEV_B);
        // Both should be positive
        assert!(score_c > 0);
        assert!(score_b > 0);
    }

    #[test]
    fn test_mutual_attestation() {
        let mut s = setup();
        assert!(!s.mutual_attestation(&DEV_A, &DEV_B)); // only A->B

        // Add B->A
        s.attest(DEV_B, DEV_A, AttestationType::Identity, 8500, [0xDD; 32], 2000);
        assert!(s.mutual_attestation(&DEV_A, &DEV_B));
    }

    #[test]
    fn test_attestation_chain() {
        let s = setup();
        // Chain from C: C is attested by A and B. B is attested by A.
        let chain = s.attestation_chain(&DEV_C, 3);
        assert!(chain.len() >= 2); // at least A->C and B->C
        // With depth 2, should also pick up A->B (A attests B, and B is in the chain)
    }

    #[test]
    #[should_panic(expected = "cannot attest yourself")]
    fn test_cannot_self_attest() {
        let mut s = AttestationChainState::new(OWNER);
        s.attest(DEV_A, DEV_A, AttestationType::Identity, 10000, [0; 32], 1000);
    }

    #[test]
    fn test_dispatch_roundtrip() {
        let mut state: Option<AttestationChainState> = None;
        dispatch(&mut state, "init", b"", OWNER);
        let args = serde_json::to_vec(&AttestArgs {
            device: DEV_B, attestation_type: AttestationType::Uptime,
            confidence: 5000, evidence_hash: [0xFF; 32], timestamp: 100,
        }).unwrap();
        let id_bytes = dispatch(&mut state, "attest", &args, DEV_A);
        let id: u64 = serde_json::from_slice(&id_bytes).unwrap();
        assert_eq!(id, 1);
    }
}
