use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-5  Soulbound Credentials  (non-transferable tokens)
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Credential {
    pub id: u64,
    pub holder: Address,
    pub issuer: Address,
    pub credential_type: String,
    pub data: BTreeMap<String, String>,
    pub issued_at: u64,
    pub expires_at: Option<u64>,
    pub revoked: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CredentialStatus {
    Valid,
    Expired,
    Revoked,
    NotFound,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CredentialRegistry {
    pub owner: Address,
    /// All credentials by ID
    pub credentials: BTreeMap<u64, Credential>,
    /// Index: holder -> list of credential IDs
    pub holder_creds: BTreeMap<Address, Vec<u64>>,
    /// Authorized issuers per credential type
    pub authorized_issuers: BTreeMap<String, Vec<Address>>,
    /// Next credential ID counter
    pub next_id: u64,
}

impl CredentialRegistry {
    pub fn new(owner: Address) -> Self {
        Self {
            owner,
            credentials: BTreeMap::new(),
            holder_creds: BTreeMap::new(),
            authorized_issuers: BTreeMap::new(),
            next_id: 1,
        }
    }

    // -- Admin ---------------------------------------------------------------

    pub fn add_authorized_issuer(
        &mut self,
        caller: Address,
        credential_type: String,
        issuer: Address,
    ) {
        assert!(
            caller == self.owner,
            "DRC5: only owner can add authorized issuers"
        );
        let issuers = self
            .authorized_issuers
            .entry(credential_type)
            .or_default();
        if !issuers.contains(&issuer) {
            issuers.push(issuer);
        }
    }

    // -- Mutations -----------------------------------------------------------

    pub fn issue(
        &mut self,
        caller: Address,
        holder: Address,
        credential_type: String,
        data: BTreeMap<String, String>,
        issued_at: u64,
        expires_at: Option<u64>,
    ) -> u64 {
        // Caller must be an authorized issuer for this credential type
        let issuers = self
            .authorized_issuers
            .get(&credential_type)
            .expect("DRC5: no authorized issuers for this credential type");
        assert!(
            issuers.contains(&caller),
            "DRC5: caller is not an authorized issuer for '{credential_type}'"
        );

        let id = self.next_id;
        self.next_id += 1;

        let credential = Credential {
            id,
            holder,
            issuer: caller,
            credential_type,
            data,
            issued_at,
            expires_at,
            revoked: false,
        };

        self.credentials.insert(id, credential);
        self.holder_creds
            .entry(holder)
            .or_default()
            .push(id);

        id
    }

    pub fn revoke(&mut self, caller: Address, credential_id: u64) {
        let cred = self
            .credentials
            .get_mut(&credential_id)
            .expect("DRC5: credential not found");
        assert!(
            caller == cred.issuer || caller == self.owner,
            "DRC5: only issuer or owner can revoke"
        );
        assert!(!cred.revoked, "DRC5: credential already revoked");
        cred.revoked = true;
    }

    // -- Queries -------------------------------------------------------------

    pub fn has_credential(&self, holder: &Address, credential_type: &str) -> bool {
        if let Some(ids) = self.holder_creds.get(holder) {
            ids.iter().any(|id| {
                if let Some(cred) = self.credentials.get(id) {
                    cred.credential_type == credential_type && !cred.revoked
                } else {
                    false
                }
            })
        } else {
            false
        }
    }

    pub fn credentials_of(&self, holder: &Address) -> Vec<&Credential> {
        match self.holder_creds.get(holder) {
            Some(ids) => ids
                .iter()
                .filter_map(|id| self.credentials.get(id))
                .collect(),
            None => Vec::new(),
        }
    }

    pub fn verify(&self, credential_id: u64, current_time: u64) -> CredentialStatus {
        match self.credentials.get(&credential_id) {
            None => CredentialStatus::NotFound,
            Some(cred) => {
                if cred.revoked {
                    return CredentialStatus::Revoked;
                }
                if let Some(exp) = cred.expires_at {
                    if current_time > exp {
                        return CredentialStatus::Expired;
                    }
                }
                CredentialStatus::Valid
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct IssueArgs {
    holder: Address,
    credential_type: String,
    data: BTreeMap<String, String>,
    issued_at: u64,
    expires_at: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RevokeArgs {
    credential_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct HasCredentialArgs {
    holder: Address,
    credential_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CredentialsOfArgs {
    holder: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct VerifyArgs {
    credential_id: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddAuthorizedIssuerArgs {
    credential_type: String,
    issuer: Address,
}

pub fn dispatch(
    state: &mut Option<CredentialRegistry>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        // -- Init ------------------------------------------------------------
        "init" => {
            assert!(state.is_none(), "DRC5: already initialised");
            *state = Some(CredentialRegistry::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Admin -----------------------------------------------------------
        "add_authorized_issuer" => {
            let s = state.as_mut().expect("DRC5: not initialised");
            let a: AddAuthorizedIssuerArgs =
                serde_json::from_slice(args).expect("DRC5: bad add_authorized_issuer args");
            s.add_authorized_issuer(caller, a.credential_type, a.issuer);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "issue" => {
            let s = state.as_mut().expect("DRC5: not initialised");
            let a: IssueArgs = serde_json::from_slice(args).expect("DRC5: bad issue args");
            let id = s.issue(
                caller,
                a.holder,
                a.credential_type,
                a.data,
                a.issued_at,
                a.expires_at,
            );
            serde_json::to_vec(&id).unwrap()
        }
        "revoke" => {
            let s = state.as_mut().expect("DRC5: not initialised");
            let a: RevokeArgs = serde_json::from_slice(args).expect("DRC5: bad revoke args");
            s.revoke(caller, a.credential_id);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "has_credential" => {
            let s = state.as_ref().expect("DRC5: not initialised");
            let a: HasCredentialArgs =
                serde_json::from_slice(args).expect("DRC5: bad has_credential args");
            serde_json::to_vec(&s.has_credential(&a.holder, &a.credential_type)).unwrap()
        }
        "credentials_of" => {
            let s = state.as_ref().expect("DRC5: not initialised");
            let a: CredentialsOfArgs =
                serde_json::from_slice(args).expect("DRC5: bad credentials_of args");
            let creds = s.credentials_of(&a.holder);
            serde_json::to_vec(&creds).unwrap()
        }
        "verify" => {
            let s = state.as_ref().expect("DRC5: not initialised");
            let a: VerifyArgs = serde_json::from_slice(args).expect("DRC5: bad verify args");
            let status = s.verify(a.credential_id, a.current_time);
            serde_json::to_vec(&status).unwrap()
        }

        _ => panic!("DRC5: unknown method '{method}'"),
    }
}
