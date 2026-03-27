use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-56  Data NFTs — Ownership of Datasets with Access Control
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AccessType {
    View,
    Download,
    Compute,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum License {
    OpenData,
    ResearchOnly,
    Commercial,
    Exclusive,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataNFT {
    pub id: u64,
    pub owner: Address,
    pub data_hash: [u8; 32],
    pub encryption_key_hash: [u8; 32],
    pub size_bytes: u64,
    pub description: String,
    pub license: License,
    pub access_count: u64,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AccessGrant {
    pub nft_id: u64,
    pub grantee: Address,
    pub expires_at: u64,
    pub access_type: AccessType,
    pub granted_at: u64,
    pub revoked: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AccessLogEntry {
    pub nft_id: u64,
    pub accessor: Address,
    pub access_type: AccessType,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataNFTState {
    pub owner: Address,
    pub data_nfts: BTreeMap<u64, DataNFT>,
    /// Key: (nft_id, grantee)
    pub access_grants: BTreeMap<(u64, Address), AccessGrant>,
    pub access_log: Vec<AccessLogEntry>,
    pub next_id: u64,
}

impl DataNFTState {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            data_nfts: BTreeMap::new(),
            access_grants: BTreeMap::new(),
            access_log: Vec::new(),
            next_id: 1,
        }
    }

    pub fn mint_data_nft(
        &mut self,
        caller: Address,
        data_hash: [u8; 32],
        encryption_key_hash: [u8; 32],
        size_bytes: u64,
        description: String,
        license: License,
        created_at: u64,
    ) -> u64 {
        assert!(size_bytes > 0, "DRC56: data must have nonzero size");
        let id = self.next_id;
        self.next_id += 1;
        self.data_nfts.insert(id, DataNFT {
            id,
            owner: caller,
            data_hash,
            encryption_key_hash,
            size_bytes,
            description,
            license,
            access_count: 0,
            created_at,
        });
        id
    }

    pub fn grant_access(
        &mut self,
        caller: Address,
        nft_id: u64,
        grantee: Address,
        access_type: AccessType,
        expires_at: u64,
        granted_at: u64,
    ) {
        let nft = self.data_nfts.get(&nft_id).expect("DRC56: NFT not found");
        assert!(caller == nft.owner, "DRC56: only owner can grant access");
        let grant = AccessGrant {
            nft_id,
            grantee,
            expires_at,
            access_type,
            granted_at,
            revoked: false,
        };
        self.access_grants.insert((nft_id, grantee), grant);
    }

    pub fn revoke_access(&mut self, caller: Address, nft_id: u64, grantee: Address) {
        let nft = self.data_nfts.get(&nft_id).expect("DRC56: NFT not found");
        assert!(caller == nft.owner, "DRC56: only owner can revoke access");
        let grant = self.access_grants.get_mut(&(nft_id, grantee)).expect("DRC56: grant not found");
        grant.revoked = true;
    }

    pub fn transfer(&mut self, caller: Address, nft_id: u64, new_owner: Address) {
        let nft = self.data_nfts.get_mut(&nft_id).expect("DRC56: NFT not found");
        assert!(caller == nft.owner, "DRC56: only owner can transfer");
        nft.owner = new_owner;
    }

    pub fn has_access(&self, nft_id: u64, account: Address, current_time: u64) -> bool {
        let nft = self.data_nfts.get(&nft_id);
        if nft.is_none() { return false; }
        let nft = nft.unwrap();
        if account == nft.owner { return true; }
        if let Some(grant) = self.access_grants.get(&(nft_id, account)) {
            return !grant.revoked && current_time <= grant.expires_at;
        }
        false
    }

    pub fn record_access(&mut self, nft_id: u64, accessor: Address, access_type: AccessType, timestamp: u64) {
        assert!(self.has_access(nft_id, accessor, timestamp), "DRC56: no access");
        let nft = self.data_nfts.get_mut(&nft_id).expect("DRC56: NFT not found");
        nft.access_count += 1;
        self.access_log.push(AccessLogEntry {
            nft_id,
            accessor,
            access_type,
            timestamp,
        });
    }

    pub fn access_log_for(&self, nft_id: u64) -> Vec<&AccessLogEntry> {
        self.access_log.iter().filter(|e| e.nft_id == nft_id).collect()
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct MintArgs {
    data_hash: [u8; 32], encryption_key_hash: [u8; 32],
    size_bytes: u64, description: String, license: License, created_at: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GrantAccessArgs { nft_id: u64, grantee: Address, access_type: AccessType, expires_at: u64, granted_at: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct RevokeAccessArgs { nft_id: u64, grantee: Address }

#[derive(Serialize, Deserialize, Debug)]
struct TransferArgs { nft_id: u64, new_owner: Address }

#[derive(Serialize, Deserialize, Debug)]
struct HasAccessArgs { nft_id: u64, account: Address, current_time: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct RecordAccessArgs { nft_id: u64, accessor: Address, access_type: AccessType, timestamp: u64 }

#[derive(Serialize, Deserialize, Debug)]
struct NftIdArgs { nft_id: u64 }

pub fn dispatch(
    state: &mut Option<DataNFTState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC56: already initialised");
            *state = Some(DataNFTState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }
        "mint_data_nft" => {
            let s = state.as_mut().expect("DRC56: not initialised");
            let a: MintArgs = serde_json::from_slice(args).expect("DRC56: bad args");
            let id = s.mint_data_nft(caller, a.data_hash, a.encryption_key_hash, a.size_bytes, a.description, a.license, a.created_at);
            serde_json::to_vec(&id).unwrap()
        }
        "grant_access" => {
            let s = state.as_mut().expect("DRC56: not initialised");
            let a: GrantAccessArgs = serde_json::from_slice(args).expect("DRC56: bad args");
            s.grant_access(caller, a.nft_id, a.grantee, a.access_type, a.expires_at, a.granted_at);
            serde_json::to_vec("ok").unwrap()
        }
        "revoke_access" => {
            let s = state.as_mut().expect("DRC56: not initialised");
            let a: RevokeAccessArgs = serde_json::from_slice(args).expect("DRC56: bad args");
            s.revoke_access(caller, a.nft_id, a.grantee);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer" => {
            let s = state.as_mut().expect("DRC56: not initialised");
            let a: TransferArgs = serde_json::from_slice(args).expect("DRC56: bad args");
            s.transfer(caller, a.nft_id, a.new_owner);
            serde_json::to_vec("ok").unwrap()
        }
        "has_access" => {
            let s = state.as_ref().expect("DRC56: not initialised");
            let a: HasAccessArgs = serde_json::from_slice(args).expect("DRC56: bad args");
            serde_json::to_vec(&s.has_access(a.nft_id, a.account, a.current_time)).unwrap()
        }
        "record_access" => {
            let s = state.as_mut().expect("DRC56: not initialised");
            let a: RecordAccessArgs = serde_json::from_slice(args).expect("DRC56: bad args");
            s.record_access(a.nft_id, a.accessor, a.access_type, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }
        "access_log" => {
            let s = state.as_ref().expect("DRC56: not initialised");
            let a: NftIdArgs = serde_json::from_slice(args).expect("DRC56: bad args");
            let log: Vec<&AccessLogEntry> = s.access_log_for(a.nft_id);
            serde_json::to_vec(&log).unwrap()
        }
        _ => panic!("DRC56: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [1u8; 32];
    const USER_A: Address = [2u8; 32];
    const USER_B: Address = [3u8; 32];
    const DATA_HASH: [u8; 32] = [0xAB; 32];
    const KEY_HASH: [u8; 32] = [0xCD; 32];

    fn setup() -> DataNFTState {
        DataNFTState::new(OWNER)
    }

    #[test]
    fn test_mint_and_query() {
        let mut s = setup();
        let id = s.mint_data_nft(OWNER, DATA_HASH, KEY_HASH, 1_000_000, "Test dataset".into(), License::OpenData, 100);
        assert_eq!(id, 1);
        let nft = s.data_nfts.get(&id).unwrap();
        assert_eq!(nft.size_bytes, 1_000_000);
        assert_eq!(nft.license, License::OpenData);
    }

    #[test]
    fn test_grant_and_check_access() {
        let mut s = setup();
        let id = s.mint_data_nft(OWNER, DATA_HASH, KEY_HASH, 5000, "My data".into(), License::ResearchOnly, 100);
        assert!(!s.has_access(id, USER_A, 200));
        s.grant_access(OWNER, id, USER_A, AccessType::View, 500, 200);
        assert!(s.has_access(id, USER_A, 200));
        assert!(s.has_access(id, USER_A, 500)); // at expiry
        assert!(!s.has_access(id, USER_A, 501)); // after expiry
    }

    #[test]
    fn test_revoke_access() {
        let mut s = setup();
        let id = s.mint_data_nft(OWNER, DATA_HASH, KEY_HASH, 5000, "Data".into(), License::Commercial, 100);
        s.grant_access(OWNER, id, USER_A, AccessType::Download, 9999, 200);
        assert!(s.has_access(id, USER_A, 300));
        s.revoke_access(OWNER, id, USER_A);
        assert!(!s.has_access(id, USER_A, 300));
    }

    #[test]
    fn test_transfer_ownership() {
        let mut s = setup();
        let id = s.mint_data_nft(OWNER, DATA_HASH, KEY_HASH, 5000, "Data".into(), License::Exclusive, 100);
        assert!(s.has_access(id, OWNER, 200));
        s.transfer(OWNER, id, USER_B);
        assert!(s.has_access(id, USER_B, 200));
        // Old owner no longer has implicit access
        assert!(!s.has_access(id, OWNER, 200));
    }

    #[test]
    fn test_access_log_recording() {
        let mut s = setup();
        let id = s.mint_data_nft(OWNER, DATA_HASH, KEY_HASH, 5000, "Data".into(), License::OpenData, 100);
        s.grant_access(OWNER, id, USER_A, AccessType::Compute, 9999, 200);
        s.record_access(id, USER_A, AccessType::Compute, 300);
        s.record_access(id, USER_A, AccessType::Compute, 400);
        let log = s.access_log_for(id);
        assert_eq!(log.len(), 2);
        assert_eq!(s.data_nfts[&id].access_count, 2);
    }

    #[test]
    #[should_panic(expected = "only owner can grant")]
    fn test_non_owner_cannot_grant() {
        let mut s = setup();
        let id = s.mint_data_nft(OWNER, DATA_HASH, KEY_HASH, 5000, "Data".into(), License::OpenData, 100);
        s.grant_access(USER_A, id, USER_B, AccessType::View, 9999, 200);
    }

    #[test]
    fn test_owner_always_has_access() {
        let mut s = setup();
        let id = s.mint_data_nft(OWNER, DATA_HASH, KEY_HASH, 5000, "Data".into(), License::OpenData, 100);
        assert!(s.has_access(id, OWNER, 999999));
    }
}
