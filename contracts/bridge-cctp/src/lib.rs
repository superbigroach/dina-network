use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Serde helper for `[u8; 64]` arrays (Ed25519 signatures).
mod serde_sig64 {
    use serde::{self, Deserialize, Deserializer, Serializer};
    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(bytes)
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let v: Vec<u8> = Vec::deserialize(deserializer)?;
        v.try_into()
            .map_err(|_| serde::de::Error::custom("expected 64 bytes for signature"))
    }
}

// ---------------------------------------------------------------------------
// CCTP MessageTransmitter — Circle Cross-Chain Transfer Protocol for Dina
// ---------------------------------------------------------------------------
//
// Implements a simplified version of Circle's CCTP (Cross-Chain Transfer
// Protocol) MessageTransmitter contract. CCTP enables native USDC transfers
// between blockchains by burning on the source chain and minting on the
// destination chain, with Circle attestation as the trust anchor.
//
// Flow:
//   1. User calls deposit_for_burn() on source chain
//   2. CCTP burns the USDC and emits a MessageSent event
//   3. Circle's attestation service signs the burn message
//   4. Anyone calls receive_message() on destination chain with the
//      signed attestation
//   5. CCTP verifies the signature and mints USDC to the recipient
//
// Domain IDs (matching Circle's official assignments):
//   Ethereum = 0, Arbitrum = 3, Solana = 5, Base = 6, Dina = 99
// ---------------------------------------------------------------------------

/// CCTP domain constants matching Circle's official domain assignments.
pub const DOMAIN_ETHEREUM: u32 = 0;
pub const DOMAIN_ARBITRUM: u32 = 3;
pub const DOMAIN_SOLANA: u32 = 5;
pub const DOMAIN_BASE: u32 = 6;
pub const DOMAIN_DINA: u32 = 99;

/// A CCTP cross-chain message containing the burn details.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CctpMessage {
    /// CCTP version (currently 0)
    pub version: u32,
    /// Source domain ID
    pub source_domain: u32,
    /// Destination domain ID
    pub destination_domain: u32,
    /// Unique nonce for this message
    pub nonce: u64,
    /// Address that initiated the burn (sender on source chain)
    pub sender: [u8; 32],
    /// Recipient on the destination chain
    pub recipient: [u8; 32],
    /// The destination caller — if non-zero, only this address can call
    /// receive_message on the destination chain
    pub destination_caller: [u8; 32],
    /// The burn token address on the source chain
    pub burn_token: [u8; 32],
    /// The mint recipient on the destination chain
    pub mint_recipient: [u8; 32],
    /// Amount of tokens burned/to be minted
    pub amount: u64,
}

impl CctpMessage {
    /// Compute the message hash for attestation verification.
    pub fn hash(&self) -> [u8; 32] {
        let serialized = serde_json::to_vec(self).expect("CCTP: message serialization failed");
        let digest = Sha256::digest(&serialized);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&digest);
        hash
    }
}

/// The full on-chain state for the CCTP MessageTransmitter contract.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CctpState {
    /// Contract owner
    pub owner: [u8; 32],
    /// This chain's CCTP domain ID (99 for Dina)
    pub local_domain: u32,
    /// Next available nonce for outgoing messages
    pub next_nonce: u64,
    /// The attester's public key (simplified — in production this is a
    /// set of attesters with threshold signatures)
    pub attester_pubkey: [u8; 32],
    /// The bridged USDC token contract address on Dina
    pub usdc_token_address: [u8; 32],
    /// Nonces that have been used (prevents replay attacks)
    pub used_nonces: BTreeMap<(u32, u64), bool>,
    /// Whether the transmitter is paused
    pub paused: bool,
    /// Maximum message body size
    pub max_message_body_size: u64,
    /// Mapping of remote domain -> remote token messenger address
    pub remote_token_messengers: BTreeMap<u32, [u8; 32]>,
    /// Emitted messages (stored for indexing — in production these are events)
    pub message_log: Vec<CctpMessage>,
}

impl CctpState {
    /// Initialize a new CCTP MessageTransmitter.
    pub fn new(owner: [u8; 32], attester_pubkey: [u8; 32], usdc_token_address: [u8; 32]) -> Self {
        Self {
            owner,
            local_domain: DOMAIN_DINA,
            next_nonce: 1,
            attester_pubkey,
            usdc_token_address,
            used_nonces: BTreeMap::new(),
            paused: false,
            max_message_body_size: 8192,
            remote_token_messengers: BTreeMap::new(),
            message_log: Vec::new(),
        }
    }

    // -- Admin functions -----------------------------------------------------

    /// Set a remote token messenger address for a given domain.
    pub fn set_remote_token_messenger(
        &mut self,
        caller: [u8; 32],
        domain: u32,
        messenger: [u8; 32],
    ) {
        assert!(caller == self.owner, "CCTP: only owner");
        self.remote_token_messengers.insert(domain, messenger);
    }

    /// Update the attester public key.
    pub fn set_attester(&mut self, caller: [u8; 32], new_attester: [u8; 32]) {
        assert!(caller == self.owner, "CCTP: only owner");
        self.attester_pubkey = new_attester;
    }

    /// Pause the transmitter.
    pub fn pause(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "CCTP: only owner");
        self.paused = true;
    }

    /// Unpause the transmitter.
    pub fn unpause(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "CCTP: only owner");
        self.paused = false;
    }

    // -- Core CCTP functions -------------------------------------------------

    /// Burn USDC on Dina to transfer to another chain.
    ///
    /// The caller must have already approved this contract to spend their USDC.
    /// In the real implementation, this contract would call burn() on the USDC
    /// token contract. Here we record the message and return the nonce.
    ///
    /// Returns: (nonce, message_hash) for the caller to track the transfer.
    pub fn deposit_for_burn(
        &mut self,
        caller: [u8; 32],
        amount: u64,
        destination_domain: u32,
        mint_recipient: [u8; 32],
    ) -> (u64, [u8; 32]) {
        assert!(!self.paused, "CCTP: paused");
        assert!(amount > 0, "CCTP: amount must be positive");
        assert!(
            destination_domain != self.local_domain,
            "CCTP: cannot send to local domain"
        );
        assert!(
            self.remote_token_messengers
                .contains_key(&destination_domain),
            "CCTP: unknown destination domain {destination_domain}"
        );

        let nonce = self.next_nonce;
        self.next_nonce += 1;

        let message = CctpMessage {
            version: 0,
            source_domain: self.local_domain,
            destination_domain,
            nonce,
            sender: caller,
            recipient: mint_recipient,
            destination_caller: [0u8; 32], // anyone can relay
            burn_token: self.usdc_token_address,
            mint_recipient,
            amount,
        };

        let hash = message.hash();
        self.message_log.push(message);

        (nonce, hash)
    }

    /// Receive a CCTP message from another chain and mint USDC on Dina.
    ///
    /// The attestation is an Ed25519 signature from Circle's attester over
    /// the message hash. This ensures only Circle-authorized attesters can
    /// approve cross-chain USDC minting.
    ///
    /// Returns true if the message was processed successfully.
    pub fn receive_message(&mut self, message: CctpMessage, attestation: [u8; 64]) -> bool {
        assert!(!self.paused, "CCTP: paused");
        assert!(
            message.destination_domain == self.local_domain,
            "CCTP: message not for this domain"
        );
        assert!(
            message.source_domain != self.local_domain,
            "CCTP: message from local domain"
        );

        // Check nonce hasn't been used (replay protection)
        let nonce_key = (message.source_domain, message.nonce);
        assert!(
            !self.used_nonces.contains_key(&nonce_key),
            "CCTP: nonce already used"
        );

        // Verify Ed25519 attestation from the attester over the message hash
        let message_hash = message.hash();
        let verifying_key = VerifyingKey::from_bytes(&self.attester_pubkey)
            .expect("CCTP: invalid attester public key");
        let signature = Signature::from_bytes(&attestation);
        verifying_key
            .verify(&message_hash, &signature)
            .expect("CCTP: invalid attestation");

        // Mark nonce as used
        self.used_nonces.insert(nonce_key, true);

        // In the real implementation, this would call mint() on the USDC
        // token contract to mint tokens to message.mint_recipient.
        // The bridge integration layer handles the actual token minting.

        true
    }

    /// Check if a nonce has been used for a given source domain.
    pub fn is_nonce_used(&self, source_domain: u32, nonce: u64) -> bool {
        self.used_nonces
            .get(&(source_domain, nonce))
            .copied()
            .unwrap_or(false)
    }

    /// Get the number of messages sent from this domain.
    pub fn message_count(&self) -> usize {
        self.message_log.len()
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    attester_pubkey: [u8; 32],
    usdc_token_address: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct DepositForBurnArgs {
    amount: u64,
    destination_domain: u32,
    mint_recipient: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct ReceiveMessageArgs {
    message: CctpMessage,
    #[serde(with = "serde_sig64")]
    attestation: [u8; 64],
}

#[derive(Serialize, Deserialize, Debug)]
struct SetRemoteMessengerArgs {
    domain: u32,
    messenger: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct SetAttesterArgs {
    new_attester: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct IsNonceUsedArgs {
    source_domain: u32,
    nonce: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct DepositForBurnResult {
    nonce: u64,
    message_hash: [u8; 32],
}

// ---------------------------------------------------------------------------
// Contract dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<CctpState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "CCTP: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("CCTP: bad init args");
            *state = Some(CctpState::new(
                caller,
                a.attester_pubkey,
                a.usdc_token_address,
            ));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Admin -----------------------------------------------------------
        "set_remote_token_messenger" => {
            let s = state.as_mut().expect("CCTP: not initialised");
            let a: SetRemoteMessengerArgs = serde_json::from_slice(args).expect("CCTP: bad args");
            s.set_remote_token_messenger(caller, a.domain, a.messenger);
            serde_json::to_vec("ok").unwrap()
        }
        "set_attester" => {
            let s = state.as_mut().expect("CCTP: not initialised");
            let a: SetAttesterArgs = serde_json::from_slice(args).expect("CCTP: bad args");
            s.set_attester(caller, a.new_attester);
            serde_json::to_vec("ok").unwrap()
        }
        "pause" => {
            let s = state.as_mut().expect("CCTP: not initialised");
            s.pause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "unpause" => {
            let s = state.as_mut().expect("CCTP: not initialised");
            s.unpause(caller);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Core CCTP -------------------------------------------------------
        "deposit_for_burn" => {
            let s = state.as_mut().expect("CCTP: not initialised");
            let a: DepositForBurnArgs =
                serde_json::from_slice(args).expect("CCTP: bad deposit_for_burn args");
            let (nonce, hash) =
                s.deposit_for_burn(caller, a.amount, a.destination_domain, a.mint_recipient);
            serde_json::to_vec(&DepositForBurnResult {
                nonce,
                message_hash: hash,
            })
            .unwrap()
        }
        "receive_message" => {
            let s = state.as_mut().expect("CCTP: not initialised");
            let a: ReceiveMessageArgs =
                serde_json::from_slice(args).expect("CCTP: bad receive_message args");
            let result = s.receive_message(a.message, a.attestation);
            serde_json::to_vec(&result).unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "is_nonce_used" => {
            let s = state.as_ref().expect("CCTP: not initialised");
            let a: IsNonceUsedArgs =
                serde_json::from_slice(args).expect("CCTP: bad is_nonce_used args");
            serde_json::to_vec(&s.is_nonce_used(a.source_domain, a.nonce)).unwrap()
        }
        "local_domain" => {
            let s = state.as_ref().expect("CCTP: not initialised");
            serde_json::to_vec(&s.local_domain).unwrap()
        }
        "message_count" => {
            let s = state.as_ref().expect("CCTP: not initialised");
            serde_json::to_vec(&s.message_count()).unwrap()
        }

        _ => panic!("CCTP: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    fn owner() -> [u8; 32] {
        [1u8; 32]
    }
    fn usdc_addr() -> [u8; 32] {
        [20u8; 32]
    }
    fn alice() -> [u8; 32] {
        [3u8; 32]
    }
    fn remote_messenger() -> [u8; 32] {
        [30u8; 32]
    }

    /// Generate an attester keypair; the public key is stored in contract state.
    fn make_attester_key() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    fn setup() -> (CctpState, SigningKey) {
        let attester_key = make_attester_key();
        let attester_pubkey = attester_key.verifying_key().to_bytes();
        let mut s = CctpState::new(owner(), attester_pubkey, usdc_addr());
        s.set_remote_token_messenger(owner(), DOMAIN_BASE, remote_messenger());
        s.set_remote_token_messenger(owner(), DOMAIN_ETHEREUM, remote_messenger());
        (s, attester_key)
    }

    /// Compute a valid Ed25519 attestation for a given message.
    fn compute_attestation(message: &CctpMessage, attester_key: &SigningKey) -> [u8; 64] {
        let message_hash = message.hash();
        let sig = attester_key.sign(&message_hash);
        sig.to_bytes()
    }

    #[test]
    fn test_init() {
        let attester_key = make_attester_key();
        let attester_pubkey = attester_key.verifying_key().to_bytes();
        let s = CctpState::new(owner(), attester_pubkey, usdc_addr());
        assert_eq!(s.local_domain, DOMAIN_DINA);
        assert_eq!(s.next_nonce, 1);
        assert!(!s.paused);
    }

    #[test]
    fn test_deposit_for_burn() {
        let (mut s, _attester_key) = setup();
        let (nonce, _hash) = s.deposit_for_burn(alice(), 1_000_000, DOMAIN_BASE, alice());
        assert_eq!(nonce, 1);
        assert_eq!(s.next_nonce, 2);
        assert_eq!(s.message_count(), 1);
    }

    #[test]
    #[should_panic(expected = "cannot send to local domain")]
    fn test_deposit_for_burn_to_self_fails() {
        let (mut s, _attester_key) = setup();
        s.deposit_for_burn(alice(), 1_000_000, DOMAIN_DINA, alice());
    }

    #[test]
    fn test_receive_message() {
        let (mut s, attester_key) = setup();
        let message = CctpMessage {
            version: 0,
            source_domain: DOMAIN_BASE,
            destination_domain: DOMAIN_DINA,
            nonce: 42,
            sender: alice(),
            recipient: alice(),
            destination_caller: [0u8; 32],
            burn_token: usdc_addr(),
            mint_recipient: alice(),
            amount: 500_000,
        };

        let attestation = compute_attestation(&message, &attester_key);
        let result = s.receive_message(message, attestation);
        assert!(result);
        assert!(s.is_nonce_used(DOMAIN_BASE, 42));
    }

    #[test]
    #[should_panic(expected = "nonce already used")]
    fn test_replay_prevention() {
        let (mut s, attester_key) = setup();
        let message = CctpMessage {
            version: 0,
            source_domain: DOMAIN_BASE,
            destination_domain: DOMAIN_DINA,
            nonce: 1,
            sender: alice(),
            recipient: alice(),
            destination_caller: [0u8; 32],
            burn_token: usdc_addr(),
            mint_recipient: alice(),
            amount: 500_000,
        };

        let attestation = compute_attestation(&message, &attester_key);
        s.receive_message(message.clone(), attestation);
        // Second time should fail
        let attestation2 = compute_attestation(&message, &attester_key);
        s.receive_message(message, attestation2);
    }

    #[test]
    #[should_panic(expected = "invalid attestation")]
    fn test_bad_attestation_fails() {
        let (mut s, _attester_key) = setup();
        let message = CctpMessage {
            version: 0,
            source_domain: DOMAIN_BASE,
            destination_domain: DOMAIN_DINA,
            nonce: 1,
            sender: alice(),
            recipient: alice(),
            destination_caller: [0u8; 32],
            burn_token: usdc_addr(),
            mint_recipient: alice(),
            amount: 500_000,
        };

        let bad_attestation = [0u8; 64];
        s.receive_message(message, bad_attestation);
    }

    #[test]
    #[should_panic(expected = "invalid attestation")]
    fn test_forged_attestation_fails() {
        let (mut s, _attester_key) = setup();
        let message = CctpMessage {
            version: 0,
            source_domain: DOMAIN_BASE,
            destination_domain: DOMAIN_DINA,
            nonce: 1,
            sender: alice(),
            recipient: alice(),
            destination_caller: [0u8; 32],
            burn_token: usdc_addr(),
            mint_recipient: alice(),
            amount: 500_000,
        };

        // Sign with a different key (attacker's key)
        let attacker_key = make_attester_key();
        let forged_attestation = compute_attestation(&message, &attacker_key);
        s.receive_message(message, forged_attestation);
    }

    #[test]
    #[should_panic(expected = "paused")]
    fn test_paused_blocks_deposit() {
        let (mut s, _attester_key) = setup();
        s.pause(owner());
        s.deposit_for_burn(alice(), 1_000_000, DOMAIN_BASE, alice());
    }

    #[test]
    fn test_domain_constants() {
        assert_eq!(DOMAIN_ETHEREUM, 0);
        assert_eq!(DOMAIN_ARBITRUM, 3);
        assert_eq!(DOMAIN_SOLANA, 5);
        assert_eq!(DOMAIN_BASE, 6);
        assert_eq!(DOMAIN_DINA, 99);
    }
}
