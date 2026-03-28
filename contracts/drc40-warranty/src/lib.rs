use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-40  Device Warranty
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];
pub type WarrantyId = u64;
pub type ClaimId = u64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CoverageType {
    Basic,
    Extended,
    Comprehensive,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ClaimStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Warranty {
    pub id: WarrantyId,
    pub device_id: String,
    pub manufacturer: Address,
    pub purchase_date: u64,
    pub expiry_date: u64,
    pub coverage_type: CoverageType,
    pub max_claims: u32,
    pub claims_made: u32,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Claim {
    pub id: ClaimId,
    pub warranty_id: WarrantyId,
    pub description: String,
    pub filed_at: u64,
    pub status: ClaimStatus,
    pub resolution: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WarrantyState {
    pub next_warranty_id: WarrantyId,
    pub next_claim_id: ClaimId,
    pub warranties: BTreeMap<WarrantyId, Warranty>,
    pub claims: BTreeMap<ClaimId, Claim>,
}

impl WarrantyState {
    pub fn new() -> Self {
        Self {
            next_warranty_id: 1,
            next_claim_id: 1,
            warranties: BTreeMap::new(),
            claims: BTreeMap::new(),
        }
    }

    pub fn create_warranty(
        &mut self,
        caller: Address,
        device_id: String,
        purchase_date: u64,
        expiry_date: u64,
        coverage_type: CoverageType,
        max_claims: u32,
    ) -> WarrantyId {
        assert!(
            expiry_date > purchase_date,
            "DRC40: expiry must be after purchase"
        );
        assert!(max_claims > 0, "DRC40: max_claims must be positive");
        let id = self.next_warranty_id;
        self.next_warranty_id += 1;
        self.warranties.insert(
            id,
            Warranty {
                id,
                device_id,
                manufacturer: caller,
                purchase_date,
                expiry_date,
                coverage_type,
                max_claims,
                claims_made: 0,
                active: true,
            },
        );
        id
    }

    pub fn file_claim(
        &mut self,
        warranty_id: WarrantyId,
        description: String,
        filed_at: u64,
    ) -> ClaimId {
        let warranty = self
            .warranties
            .get(&warranty_id)
            .expect("DRC40: warranty not found");
        assert!(warranty.active, "DRC40: warranty is inactive");
        assert!(filed_at <= warranty.expiry_date, "DRC40: warranty expired");
        assert!(
            warranty.claims_made < warranty.max_claims,
            "DRC40: max claims reached"
        );

        let claim_id = self.next_claim_id;
        self.next_claim_id += 1;
        self.claims.insert(
            claim_id,
            Claim {
                id: claim_id,
                warranty_id,
                description,
                filed_at,
                status: ClaimStatus::Pending,
                resolution: None,
            },
        );

        let warranty = self.warranties.get_mut(&warranty_id).unwrap();
        warranty.claims_made += 1;

        claim_id
    }

    pub fn approve_claim(&mut self, caller: Address, claim_id: ClaimId, resolution: String) {
        let claim = self.claims.get(&claim_id).expect("DRC40: claim not found");
        let warranty = self
            .warranties
            .get(&claim.warranty_id)
            .expect("DRC40: warranty not found");
        assert!(
            warranty.manufacturer == caller,
            "DRC40: only manufacturer can approve"
        );
        assert!(
            claim.status == ClaimStatus::Pending,
            "DRC40: claim not pending"
        );

        let claim = self.claims.get_mut(&claim_id).unwrap();
        claim.status = ClaimStatus::Approved;
        claim.resolution = Some(resolution);
    }

    pub fn reject_claim(&mut self, caller: Address, claim_id: ClaimId, reason: String) {
        let claim = self.claims.get(&claim_id).expect("DRC40: claim not found");
        let warranty = self
            .warranties
            .get(&claim.warranty_id)
            .expect("DRC40: warranty not found");
        assert!(
            warranty.manufacturer == caller,
            "DRC40: only manufacturer can reject"
        );
        assert!(
            claim.status == ClaimStatus::Pending,
            "DRC40: claim not pending"
        );

        let claim = self.claims.get_mut(&claim_id).unwrap();
        claim.status = ClaimStatus::Rejected;
        claim.resolution = Some(reason);
    }

    pub fn check_warranty(&self, warranty_id: WarrantyId) -> &Warranty {
        self.warranties
            .get(&warranty_id)
            .expect("DRC40: warranty not found")
    }

    pub fn extend_warranty(&mut self, caller: Address, warranty_id: WarrantyId, new_expiry: u64) {
        let warranty = self
            .warranties
            .get(&warranty_id)
            .expect("DRC40: warranty not found");
        assert!(
            warranty.manufacturer == caller,
            "DRC40: only manufacturer can extend"
        );
        assert!(
            new_expiry > warranty.expiry_date,
            "DRC40: new expiry must be later"
        );

        let warranty = self.warranties.get_mut(&warranty_id).unwrap();
        warranty.expiry_date = new_expiry;
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateWarrantyArgs {
    device_id: String,
    purchase_date: u64,
    expiry_date: u64,
    coverage_type: CoverageType,
    max_claims: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct FileClaimArgs {
    warranty_id: WarrantyId,
    description: String,
    filed_at: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApproveClaimArgs {
    claim_id: ClaimId,
    resolution: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RejectClaimArgs {
    claim_id: ClaimId,
    reason: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CheckWarrantyArgs {
    warranty_id: WarrantyId,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExtendWarrantyArgs {
    warranty_id: WarrantyId,
    new_expiry: u64,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<WarrantyState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC40: already initialised");
            *state = Some(WarrantyState::new());
            serde_json::to_vec("ok").unwrap()
        }

        "create_warranty" => {
            let s = state.as_mut().expect("DRC40: not initialised");
            let a: CreateWarrantyArgs =
                serde_json::from_slice(args).expect("DRC40: bad create_warranty args");
            let id = s.create_warranty(
                caller,
                a.device_id,
                a.purchase_date,
                a.expiry_date,
                a.coverage_type,
                a.max_claims,
            );
            serde_json::to_vec(&id).unwrap()
        }

        "file_claim" => {
            let s = state.as_mut().expect("DRC40: not initialised");
            let a: FileClaimArgs =
                serde_json::from_slice(args).expect("DRC40: bad file_claim args");
            let id = s.file_claim(a.warranty_id, a.description, a.filed_at);
            serde_json::to_vec(&id).unwrap()
        }

        "approve_claim" => {
            let s = state.as_mut().expect("DRC40: not initialised");
            let a: ApproveClaimArgs =
                serde_json::from_slice(args).expect("DRC40: bad approve_claim args");
            s.approve_claim(caller, a.claim_id, a.resolution);
            serde_json::to_vec("ok").unwrap()
        }

        "reject_claim" => {
            let s = state.as_mut().expect("DRC40: not initialised");
            let a: RejectClaimArgs =
                serde_json::from_slice(args).expect("DRC40: bad reject_claim args");
            s.reject_claim(caller, a.claim_id, a.reason);
            serde_json::to_vec("ok").unwrap()
        }

        "check_warranty" => {
            let s = state.as_ref().expect("DRC40: not initialised");
            let a: CheckWarrantyArgs =
                serde_json::from_slice(args).expect("DRC40: bad check_warranty args");
            let w = s.check_warranty(a.warranty_id);
            serde_json::to_vec(w).unwrap()
        }

        "extend_warranty" => {
            let s = state.as_mut().expect("DRC40: not initialised");
            let a: ExtendWarrantyArgs =
                serde_json::from_slice(args).expect("DRC40: bad extend_warranty args");
            s.extend_warranty(caller, a.warranty_id, a.new_expiry);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC40: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const MANUFACTURER: Address = [1u8; 32];
    const USER: Address = [2u8; 32];

    fn init() -> Option<WarrantyState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", MANUFACTURER);
        state
    }

    fn create_basic(state: &mut Option<WarrantyState>) -> WarrantyId {
        let args = serde_json::to_vec(&CreateWarrantyArgs {
            device_id: "DEVICE-001".to_string(),
            purchase_date: 1000,
            expiry_date: 2000,
            coverage_type: CoverageType::Basic,
            max_claims: 3,
        })
        .unwrap();
        let result = dispatch(state, "create_warranty", &args, MANUFACTURER);
        serde_json::from_slice(&result).unwrap()
    }

    #[test]
    fn test_create_and_check_warranty() {
        let mut state = init();
        let id = create_basic(&mut state);
        assert_eq!(id, 1);

        let args = serde_json::to_vec(&CheckWarrantyArgs { warranty_id: id }).unwrap();
        let result = dispatch(&mut state, "check_warranty", &args, USER);
        let w: Warranty = serde_json::from_slice(&result).unwrap();
        assert_eq!(w.device_id, "DEVICE-001");
        assert_eq!(w.coverage_type, CoverageType::Basic);
        assert!(w.active);
        assert_eq!(w.claims_made, 0);
    }

    #[test]
    fn test_file_and_approve_claim() {
        let mut state = init();
        let wid = create_basic(&mut state);

        let file_args = serde_json::to_vec(&FileClaimArgs {
            warranty_id: wid,
            description: "Screen cracked".to_string(),
            filed_at: 1500,
        })
        .unwrap();
        let result = dispatch(&mut state, "file_claim", &file_args, USER);
        let claim_id: ClaimId = serde_json::from_slice(&result).unwrap();
        assert_eq!(claim_id, 1);

        let approve_args = serde_json::to_vec(&ApproveClaimArgs {
            claim_id,
            resolution: "Replaced screen".to_string(),
        })
        .unwrap();
        dispatch(&mut state, "approve_claim", &approve_args, MANUFACTURER);

        let claim = state.as_ref().unwrap().claims.get(&claim_id).unwrap();
        assert_eq!(claim.status, ClaimStatus::Approved);
        assert_eq!(claim.resolution.as_deref(), Some("Replaced screen"));
    }

    #[test]
    fn test_reject_claim() {
        let mut state = init();
        let wid = create_basic(&mut state);

        let file_args = serde_json::to_vec(&FileClaimArgs {
            warranty_id: wid,
            description: "User damage".to_string(),
            filed_at: 1200,
        })
        .unwrap();
        let result = dispatch(&mut state, "file_claim", &file_args, USER);
        let cid: ClaimId = serde_json::from_slice(&result).unwrap();

        let reject_args = serde_json::to_vec(&RejectClaimArgs {
            claim_id: cid,
            reason: "Physical damage not covered".to_string(),
        })
        .unwrap();
        dispatch(&mut state, "reject_claim", &reject_args, MANUFACTURER);

        let claim = state.as_ref().unwrap().claims.get(&cid).unwrap();
        assert_eq!(claim.status, ClaimStatus::Rejected);
    }

    #[test]
    fn test_extend_warranty() {
        let mut state = init();
        let wid = create_basic(&mut state);

        let args = serde_json::to_vec(&ExtendWarrantyArgs {
            warranty_id: wid,
            new_expiry: 3000,
        })
        .unwrap();
        dispatch(&mut state, "extend_warranty", &args, MANUFACTURER);

        let w = state.as_ref().unwrap().check_warranty(wid);
        assert_eq!(w.expiry_date, 3000);
    }

    #[test]
    #[should_panic(expected = "warranty expired")]
    fn test_file_claim_after_expiry() {
        let mut state = init();
        let wid = create_basic(&mut state);

        let file_args = serde_json::to_vec(&FileClaimArgs {
            warranty_id: wid,
            description: "Late claim".to_string(),
            filed_at: 2500,
        })
        .unwrap();
        dispatch(&mut state, "file_claim", &file_args, USER);
    }

    #[test]
    #[should_panic(expected = "max claims reached")]
    fn test_max_claims_exceeded() {
        let mut state = init();
        let args = serde_json::to_vec(&CreateWarrantyArgs {
            device_id: "DEV-002".to_string(),
            purchase_date: 1000,
            expiry_date: 2000,
            coverage_type: CoverageType::Extended,
            max_claims: 1,
        })
        .unwrap();
        let result = dispatch(&mut state, "create_warranty", &args, MANUFACTURER);
        let wid: WarrantyId = serde_json::from_slice(&result).unwrap();

        let file1 = serde_json::to_vec(&FileClaimArgs {
            warranty_id: wid,
            description: "Claim 1".to_string(),
            filed_at: 1100,
        })
        .unwrap();
        dispatch(&mut state, "file_claim", &file1, USER);

        let file2 = serde_json::to_vec(&FileClaimArgs {
            warranty_id: wid,
            description: "Claim 2".to_string(),
            filed_at: 1200,
        })
        .unwrap();
        dispatch(&mut state, "file_claim", &file2, USER);
    }
}
