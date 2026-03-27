use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-40  Device Warranty
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ClaimStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Claim {
    pub id: u64,
    pub warranty_id: u64,
    pub description: String,
    pub filed_at: u64,
    pub status: ClaimStatus,
    pub resolution: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Warranty {
    pub id: u64,
    pub device_id: Address,
    pub manufacturer: Address,
    pub owner: Address,
    pub purchase_date: u64,
    pub expiry_date: u64,
    pub coverage_type: String,
    pub max_claims: u64,
    pub claims_made: u64,
    pub active: bool,
    pub claims: Vec<Claim>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WarrantyState {
    pub admin: Address,
    pub warranties: BTreeMap<u64, Warranty>,
    pub next_warranty_id: u64,
    pub next_claim_id: u64,
}

impl WarrantyState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            warranties: BTreeMap::new(),
            next_warranty_id: 1,
            next_claim_id: 1,
        }
    }

    pub fn create_warranty(
        &mut self,
        caller: Address,
        device_id: Address,
        owner: Address,
        purchase_date: u64,
        expiry_date: u64,
        coverage_type: String,
        max_claims: u64,
    ) -> u64 {
        assert!(
            expiry_date > purchase_date,
            "DRC40: expiry must be after purchase"
        );
        assert!(max_claims > 0, "DRC40: max_claims must be positive");
        let id = self.next_warranty_id;
        self.next_warranty_id += 1;
        let warranty = Warranty {
            id,
            device_id,
            manufacturer: caller,
            owner,
            purchase_date,
            expiry_date,
            coverage_type,
            max_claims,
            claims_made: 0,
            active: true,
            claims: Vec::new(),
        };
        self.warranties.insert(id, warranty);
        id
    }

    pub fn file_claim(
        &mut self,
        caller: Address,
        warranty_id: u64,
        description: String,
        filed_at: u64,
    ) -> u64 {
        let warranty = self
            .warranties
            .get_mut(&warranty_id)
            .expect("DRC40: warranty not found");
        assert!(warranty.active, "DRC40: warranty not active");
        assert!(
            warranty.owner == caller,
            "DRC40: only owner can file claims"
        );
        assert!(
            filed_at <= warranty.expiry_date,
            "DRC40: warranty expired"
        );
        assert!(
            warranty.claims_made < warranty.max_claims,
            "DRC40: max claims reached"
        );
        let claim_id = self.next_claim_id;
        self.next_claim_id += 1;
        let claim = Claim {
            id: claim_id,
            warranty_id,
            description,
            filed_at,
            status: ClaimStatus::Pending,
            resolution: String::new(),
        };
        warranty.claims.push(claim);
        warranty.claims_made += 1;
        claim_id
    }

    pub fn approve_claim(
        &mut self,
        caller: Address,
        warranty_id: u64,
        claim_id: u64,
        resolution: String,
    ) {
        let warranty = self
            .warranties
            .get_mut(&warranty_id)
            .expect("DRC40: warranty not found");
        assert!(
            warranty.manufacturer == caller || caller == self.admin,
            "DRC40: not authorized"
        );
        let claim = warranty
            .claims
            .iter_mut()
            .find(|c| c.id == claim_id)
            .expect("DRC40: claim not found");
        assert!(
            claim.status == ClaimStatus::Pending,
            "DRC40: claim not pending"
        );
        claim.status = ClaimStatus::Approved;
        claim.resolution = resolution;
    }

    pub fn reject_claim(
        &mut self,
        caller: Address,
        warranty_id: u64,
        claim_id: u64,
        reason: String,
    ) {
        let warranty = self
            .warranties
            .get_mut(&warranty_id)
            .expect("DRC40: warranty not found");
        assert!(
            warranty.manufacturer == caller || caller == self.admin,
            "DRC40: not authorized"
        );
        let claim = warranty
            .claims
            .iter_mut()
            .find(|c| c.id == claim_id)
            .expect("DRC40: claim not found");
        assert!(
            claim.status == ClaimStatus::Pending,
            "DRC40: claim not pending"
        );
        claim.status = ClaimStatus::Rejected;
        claim.resolution = reason;
    }

    pub fn check_warranty(&self, warranty_id: u64) -> Option<&Warranty> {
        self.warranties.get(&warranty_id)
    }

    pub fn extend_warranty(
        &mut self,
        caller: Address,
        warranty_id: u64,
        new_expiry: u64,
    ) {
        let warranty = self
            .warranties
            .get_mut(&warranty_id)
            .expect("DRC40: warranty not found");
        assert!(
            warranty.manufacturer == caller,
            "DRC40: only manufacturer can extend"
        );
        assert!(
            new_expiry > warranty.expiry_date,
            "DRC40: new expiry must be later"
        );
        warranty.expiry_date = new_expiry;
    }

    pub fn transfer_warranty(
        &mut self,
        caller: Address,
        warranty_id: u64,
        new_owner: Address,
    ) {
        let warranty = self
            .warranties
            .get_mut(&warranty_id)
            .expect("DRC40: warranty not found");
        assert!(
            warranty.owner == caller,
            "DRC40: only owner can transfer"
        );
        warranty.owner = new_owner;
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateWarrantyArgs {
    device_id: Address,
    owner: Address,
    purchase_date: u64,
    expiry_date: u64,
    coverage_type: String,
    max_claims: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct FileClaimArgs {
    warranty_id: u64,
    description: String,
    filed_at: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApproveClaimArgs {
    warranty_id: u64,
    claim_id: u64,
    resolution: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RejectClaimArgs {
    warranty_id: u64,
    claim_id: u64,
    reason: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CheckWarrantyArgs {
    warranty_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExtendWarrantyArgs {
    warranty_id: u64,
    new_expiry: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferWarrantyArgs {
    warranty_id: u64,
    new_owner: Address,
}

pub fn dispatch(
    state: &mut Option<WarrantyState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC40: already initialised");
            *state = Some(WarrantyState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "create_warranty" => {
            let s = state.as_mut().expect("DRC40: not initialised");
            let a: CreateWarrantyArgs =
                serde_json::from_slice(args).expect("DRC40: bad create_warranty args");
            let id = s.create_warranty(
                caller,
                a.device_id,
                a.owner,
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
            let id = s.file_claim(caller, a.warranty_id, a.description, a.filed_at);
            serde_json::to_vec(&id).unwrap()
        }
        "approve_claim" => {
            let s = state.as_mut().expect("DRC40: not initialised");
            let a: ApproveClaimArgs =
                serde_json::from_slice(args).expect("DRC40: bad approve_claim args");
            s.approve_claim(caller, a.warranty_id, a.claim_id, a.resolution);
            serde_json::to_vec("ok").unwrap()
        }
        "reject_claim" => {
            let s = state.as_mut().expect("DRC40: not initialised");
            let a: RejectClaimArgs =
                serde_json::from_slice(args).expect("DRC40: bad reject_claim args");
            s.reject_claim(caller, a.warranty_id, a.claim_id, a.reason);
            serde_json::to_vec("ok").unwrap()
        }
        "check_warranty" => {
            let s = state.as_ref().expect("DRC40: not initialised");
            let a: CheckWarrantyArgs =
                serde_json::from_slice(args).expect("DRC40: bad check_warranty args");
            serde_json::to_vec(&s.check_warranty(a.warranty_id)).unwrap()
        }
        "extend_warranty" => {
            let s = state.as_mut().expect("DRC40: not initialised");
            let a: ExtendWarrantyArgs =
                serde_json::from_slice(args).expect("DRC40: bad extend_warranty args");
            s.extend_warranty(caller, a.warranty_id, a.new_expiry);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer_warranty" => {
            let s = state.as_mut().expect("DRC40: not initialised");
            let a: TransferWarrantyArgs =
                serde_json::from_slice(args).expect("DRC40: bad transfer_warranty args");
            s.transfer_warranty(caller, a.warranty_id, a.new_owner);
            serde_json::to_vec("ok").unwrap()
        }
        _ => panic!("DRC40: unknown method '{method}'"),
    }
}
