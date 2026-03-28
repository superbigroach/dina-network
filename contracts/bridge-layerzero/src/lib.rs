use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::collections::BTreeMap;

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------
// LayerZero Integration — OFT (Omnichain Fungible Token) for Dina Network
// ---------------------------------------------------------------------------
//
// Implements a LayerZero-compatible OFT (Omnichain Fungible Token) bridge
// for Dina Network. LayerZero is an omnichain interoperability protocol
// that enables cross-chain messaging through Ultra Light Nodes (ULNs)
// and decentralized verifier networks (DVNs).
//
// The OFT standard allows a single token to exist natively across multiple
// chains — when tokens are sent from one chain, they are burned on the
// source and minted on the destination.
//
// Key concepts:
//   - Trusted Remotes: each chain must configure the trusted contract
//     address on every other chain it communicates with
//   - Adapter Params: per-message gas configuration
//   - Failed Messages: messages that fail on receive are stored for retry
//   - Nonces: per-path sequential nonces for ordering
//
// LayerZero Chain IDs (Endpoint v2):
//   Ethereum = 30101, Base = 30184, Arbitrum = 30110, Solana = 30168
//   Dina = 30099 (proposed)
// ---------------------------------------------------------------------------

/// LayerZero chain ID constants (Endpoint v2 format).
pub const LZ_ETHEREUM: u16 = 30101;
pub const LZ_ARBITRUM: u16 = 30110;
pub const LZ_SOLANA: u16 = 30168;
pub const LZ_BASE: u16 = 30184;
pub const LZ_DINA: u16 = 30099;

/// Adapter parameters for controlling gas on the destination chain.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdapterParams {
    /// Version of the adapter params (1 = basic, 2 = airdrop native gas)
    pub version: u16,
    /// Gas limit for the destination chain execution
    pub dst_gas_limit: u64,
    /// Native gas amount to airdrop on destination (version 2 only)
    pub native_for_dst: u64,
    /// Address to receive airdropped native gas (version 2 only)
    pub native_dst_address: [u8; 32],
}

impl Default for AdapterParams {
    fn default() -> Self {
        Self {
            version: 1,
            dst_gas_limit: 200_000,
            native_for_dst: 0,
            native_dst_address: [0u8; 32],
        }
    }
}

/// A failed message that can be retried.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FailedMessage {
    /// Source chain ID
    pub src_chain_id: u16,
    /// Source address
    pub src_address: [u8; 32],
    /// Nonce of the failed message
    pub nonce: u64,
    /// The payload that failed
    pub payload: Vec<u8>,
    /// Reason for failure
    pub reason: String,
}

/// An outgoing cross-chain message (stored for event emission).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OutboundMessage {
    /// Destination chain ID
    pub dst_chain_id: u16,
    /// Destination address
    pub dst_address: [u8; 32],
    /// Sender on this chain
    pub sender: [u8; 32],
    /// Amount of tokens
    pub amount: u64,
    /// Nonce for this path
    pub nonce: u64,
}

/// OFT message payload (the data sent cross-chain).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OftPayload {
    /// Recipient address on the destination chain
    pub to: [u8; 32],
    /// Amount of tokens (in shared decimals)
    pub amount: u64,
    /// Optional compose message for further processing
    pub compose_msg: Option<Vec<u8>>,
}

/// Full on-chain state for the LayerZero OFT bridge on Dina.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LayerZeroState {
    /// Contract owner
    pub owner: [u8; 32],
    /// This chain's LayerZero chain ID
    pub local_chain_id: u16,
    /// The LayerZero endpoint address (the LZ protocol contract)
    pub lz_endpoint: [u8; 32],
    /// Ed25519 public key of the LZ endpoint for cryptographic message
    /// verification. Inbound messages must carry a valid Ed25519 signature
    /// over the message hash, signed by this key.
    pub endpoint_pubkey: [u8; 32],
    /// The bridged USDC token address on Dina
    pub usdc_token: [u8; 32],
    /// Trusted remote addresses: chain_id -> remote contract address
    /// Messages are only accepted from trusted remotes
    pub trusted_remotes: BTreeMap<u16, [u8; 32]>,
    /// Inbound nonces per source path: (src_chain_id, src_address) -> last nonce
    pub inbound_nonces: BTreeMap<(u16, [u8; 32]), u64>,
    /// Outbound nonces per destination path: dst_chain_id -> next nonce
    pub outbound_nonces: BTreeMap<u16, u64>,
    /// Failed messages that can be retried
    pub failed_messages: Vec<FailedMessage>,
    /// Outbound message log (for event emission / indexing)
    pub outbound_log: Vec<OutboundMessage>,
    /// Default adapter params per chain
    pub default_adapter_params: BTreeMap<u16, AdapterParams>,
    /// Whether the bridge is paused
    pub paused: bool,
    /// Minimum gas limit for destination execution
    pub min_dst_gas: BTreeMap<u16, u64>,
}

impl LayerZeroState {
    /// Initialize a new LayerZero OFT bridge.
    pub fn new(
        owner: [u8; 32],
        lz_endpoint: [u8; 32],
        endpoint_pubkey: [u8; 32],
        usdc_token: [u8; 32],
    ) -> Self {
        Self {
            owner,
            local_chain_id: LZ_DINA,
            lz_endpoint,
            endpoint_pubkey,
            usdc_token,
            trusted_remotes: BTreeMap::new(),
            inbound_nonces: BTreeMap::new(),
            outbound_nonces: BTreeMap::new(),
            failed_messages: Vec::new(),
            outbound_log: Vec::new(),
            default_adapter_params: BTreeMap::new(),
            paused: false,
            min_dst_gas: BTreeMap::new(),
        }
    }

    // -- Admin ---------------------------------------------------------------

    /// Configure a trusted remote contract address for a given chain.
    /// Only messages from trusted remotes are accepted.
    pub fn set_trusted_remote(
        &mut self,
        caller: [u8; 32],
        chain_id: u16,
        remote_address: [u8; 32],
    ) {
        assert!(caller == self.owner, "LZ: only owner");
        assert!(
            chain_id != self.local_chain_id,
            "LZ: cannot set self as remote"
        );
        self.trusted_remotes.insert(chain_id, remote_address);
    }

    /// Set the minimum destination gas for a chain.
    pub fn set_min_dst_gas(&mut self, caller: [u8; 32], chain_id: u16, min_gas: u64) {
        assert!(caller == self.owner, "LZ: only owner");
        self.min_dst_gas.insert(chain_id, min_gas);
    }

    /// Set default adapter params for a destination chain.
    pub fn set_default_adapter_params(
        &mut self,
        caller: [u8; 32],
        chain_id: u16,
        params: AdapterParams,
    ) {
        assert!(caller == self.owner, "LZ: only owner");
        self.default_adapter_params.insert(chain_id, params);
    }

    /// Pause the bridge.
    pub fn pause(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "LZ: only owner");
        self.paused = true;
    }

    /// Unpause the bridge.
    pub fn unpause(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "LZ: only owner");
        self.paused = false;
    }

    /// Update the endpoint public key. Only callable by owner.
    pub fn set_endpoint_pubkey(&mut self, caller: [u8; 32], new_pubkey: [u8; 32]) {
        assert!(caller == self.owner, "LZ: only owner");
        self.endpoint_pubkey = new_pubkey;
    }

    /// Verify an Ed25519 signature over the given message hash.
    fn verify_endpoint_signature(&self, message_hash: &[u8; 32], signature: &[u8; 64]) {
        let verifying_key = VerifyingKey::from_bytes(&self.endpoint_pubkey)
            .expect("LZ: invalid endpoint public key");
        let sig = Signature::from_bytes(signature);
        verifying_key
            .verify(message_hash, &sig)
            .expect("LZ: invalid endpoint signature");
    }

    /// Compute the message hash for lz_receive() verification.
    /// Hash covers: src_chain_id, src_address, nonce, serialized payload.
    fn compute_receive_hash(
        src_chain_id: u16,
        src_address: &[u8; 32],
        nonce: u64,
        payload: &OftPayload,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(src_chain_id.to_le_bytes());
        hasher.update(src_address);
        hasher.update(nonce.to_le_bytes());
        // Hash the payload fields deterministically
        hasher.update(payload.to.as_slice());
        hasher.update(payload.amount.to_le_bytes());
        if let Some(ref compose) = payload.compose_msg {
            hasher.update(compose);
        }
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    // -- Core OFT functions --------------------------------------------------

    /// Send tokens from Dina to another chain via LayerZero.
    ///
    /// Burns the tokens on Dina and emits a cross-chain message that
    /// LayerZero's DVN network will relay to the destination chain.
    ///
    /// Returns the nonce of the outbound message.
    pub fn send(
        &mut self,
        caller: [u8; 32],
        dst_chain_id: u16,
        dst_address: [u8; 32],
        amount: u64,
        adapter_params: Option<AdapterParams>,
    ) -> u64 {
        assert!(!self.paused, "LZ: paused");
        assert!(amount > 0, "LZ: amount must be positive");
        assert!(
            self.trusted_remotes.contains_key(&dst_chain_id),
            "LZ: destination not trusted"
        );

        // Validate adapter params gas limit
        let params = adapter_params.unwrap_or_else(|| {
            self.default_adapter_params
                .get(&dst_chain_id)
                .cloned()
                .unwrap_or_default()
        });

        if let Some(&min_gas) = self.min_dst_gas.get(&dst_chain_id) {
            assert!(
                params.dst_gas_limit >= min_gas,
                "LZ: gas limit below minimum ({} < {min_gas})",
                params.dst_gas_limit
            );
        }

        // Get and increment nonce
        let nonce = self.outbound_nonces.entry(dst_chain_id).or_insert(0);
        *nonce += 1;
        let current_nonce = *nonce;

        // Log the outbound message
        self.outbound_log.push(OutboundMessage {
            dst_chain_id,
            dst_address,
            sender: caller,
            amount,
            nonce: current_nonce,
        });

        // In production: burn tokens from caller via USDC.e token contract,
        // then call lz_endpoint.send() to emit the cross-chain message

        current_nonce
    }

    /// Receive a cross-chain message from LayerZero.
    ///
    /// This is called by the LayerZero endpoint when a message arrives
    /// from a trusted remote. It verifies the source, checks the nonce
    /// ordering, and mints tokens to the recipient.
    ///
    /// The `endpoint_signature` parameter is an Ed25519 signature over the
    /// SHA-256 hash of (src_chain_id || src_address || nonce || payload),
    /// produced by the endpoint's signing key. This provides cryptographic
    /// verification beyond the caller address check.
    ///
    /// Returns the amount minted.
    pub fn lz_receive(
        &mut self,
        caller: [u8; 32],
        src_chain_id: u16,
        src_address: [u8; 32],
        nonce: u64,
        payload: OftPayload,
        endpoint_signature: [u8; 64],
    ) -> u64 {
        assert!(!self.paused, "LZ: paused");
        // Only the LZ endpoint can call lz_receive
        assert!(
            caller == self.lz_endpoint,
            "LZ: only endpoint can call lz_receive"
        );

        // Cryptographic verification: verify endpoint Ed25519 signature
        let message_hash = Self::compute_receive_hash(src_chain_id, &src_address, nonce, &payload);
        self.verify_endpoint_signature(&message_hash, &endpoint_signature);

        // Verify the source is a trusted remote
        let trusted = self.trusted_remotes.get(&src_chain_id);
        assert!(
            trusted.is_some() && *trusted.unwrap() == src_address,
            "LZ: untrusted source"
        );

        // Verify nonce ordering (must be sequential)
        let expected_nonce = self
            .inbound_nonces
            .get(&(src_chain_id, src_address))
            .copied()
            .unwrap_or(0)
            + 1;

        if nonce != expected_nonce {
            // Store as failed message for later retry
            let payload_bytes = serde_json::to_vec(&payload).unwrap_or_default();
            self.failed_messages.push(FailedMessage {
                src_chain_id,
                src_address,
                nonce,
                payload: payload_bytes,
                reason: format!("nonce mismatch: expected {expected_nonce}, got {nonce}"),
            });
            return 0;
        }

        // Update inbound nonce
        self.inbound_nonces
            .insert((src_chain_id, src_address), nonce);

        // In production: mint tokens via USDC.e token contract
        // to payload.to for payload.amount

        payload.amount
    }

    /// Retry a failed message. Only callable by owner.
    pub fn retry_message(
        &mut self,
        caller: [u8; 32],
        src_chain_id: u16,
        src_address: [u8; 32],
        nonce: u64,
    ) -> u64 {
        assert!(caller == self.owner, "LZ: only owner");

        // Find and remove the failed message
        let idx = self
            .failed_messages
            .iter()
            .position(|m| {
                m.src_chain_id == src_chain_id && m.src_address == src_address && m.nonce == nonce
            })
            .expect("LZ: failed message not found");

        let failed = self.failed_messages.remove(idx);
        let payload: OftPayload = serde_json::from_slice(&failed.payload).expect("LZ: bad payload");

        // Update inbound nonce
        self.inbound_nonces
            .insert((src_chain_id, src_address), nonce);

        payload.amount
    }

    // -- Queries -------------------------------------------------------------

    /// Check if a remote is trusted.
    pub fn is_trusted_remote(&self, chain_id: u16, address: &[u8; 32]) -> bool {
        self.trusted_remotes
            .get(&chain_id)
            .map(|a| a == address)
            .unwrap_or(false)
    }

    /// Get the outbound nonce for a destination chain.
    pub fn get_outbound_nonce(&self, chain_id: u16) -> u64 {
        self.outbound_nonces.get(&chain_id).copied().unwrap_or(0)
    }

    /// Get the inbound nonce for a source path.
    pub fn get_inbound_nonce(&self, chain_id: u16, address: &[u8; 32]) -> u64 {
        self.inbound_nonces
            .get(&(chain_id, *address))
            .copied()
            .unwrap_or(0)
    }

    /// Get the number of failed messages.
    pub fn failed_message_count(&self) -> usize {
        self.failed_messages.len()
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    lz_endpoint: [u8; 32],
    endpoint_pubkey: [u8; 32],
    usdc_token: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct SetEndpointPubkeyArgs {
    new_pubkey: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct SetTrustedRemoteArgs {
    chain_id: u16,
    remote_address: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct SetMinDstGasArgs {
    chain_id: u16,
    min_gas: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetAdapterParamsArgs {
    chain_id: u16,
    params: AdapterParams,
}

#[derive(Serialize, Deserialize, Debug)]
struct SendArgs {
    dst_chain_id: u16,
    dst_address: [u8; 32],
    amount: u64,
    adapter_params: Option<AdapterParams>,
}

#[derive(Serialize, Deserialize, Debug)]
struct LzReceiveArgs {
    src_chain_id: u16,
    src_address: [u8; 32],
    nonce: u64,
    payload: OftPayload,
    #[serde(with = "BigArray")]
    endpoint_signature: [u8; 64],
}

#[derive(Serialize, Deserialize, Debug)]
struct RetryMessageArgs {
    src_chain_id: u16,
    src_address: [u8; 32],
    nonce: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct IsTrustedRemoteArgs {
    chain_id: u16,
    address: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct GetNonceArgs {
    chain_id: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetInboundNonceArgs {
    chain_id: u16,
    address: [u8; 32],
}

// ---------------------------------------------------------------------------
// Contract dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<LayerZeroState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "LZ: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("LZ: bad init args");
            *state = Some(LayerZeroState::new(
                caller,
                a.lz_endpoint,
                a.endpoint_pubkey,
                a.usdc_token,
            ));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Admin -----------------------------------------------------------
        "set_trusted_remote" => {
            let s = state.as_mut().expect("LZ: not initialised");
            let a: SetTrustedRemoteArgs = serde_json::from_slice(args).expect("LZ: bad args");
            s.set_trusted_remote(caller, a.chain_id, a.remote_address);
            serde_json::to_vec("ok").unwrap()
        }
        "set_min_dst_gas" => {
            let s = state.as_mut().expect("LZ: not initialised");
            let a: SetMinDstGasArgs = serde_json::from_slice(args).expect("LZ: bad args");
            s.set_min_dst_gas(caller, a.chain_id, a.min_gas);
            serde_json::to_vec("ok").unwrap()
        }
        "set_default_adapter_params" => {
            let s = state.as_mut().expect("LZ: not initialised");
            let a: SetAdapterParamsArgs = serde_json::from_slice(args).expect("LZ: bad args");
            s.set_default_adapter_params(caller, a.chain_id, a.params);
            serde_json::to_vec("ok").unwrap()
        }
        "pause" => {
            let s = state.as_mut().expect("LZ: not initialised");
            s.pause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "unpause" => {
            let s = state.as_mut().expect("LZ: not initialised");
            s.unpause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "set_endpoint_pubkey" => {
            let s = state.as_mut().expect("LZ: not initialised");
            let a: SetEndpointPubkeyArgs = serde_json::from_slice(args).expect("LZ: bad args");
            s.set_endpoint_pubkey(caller, a.new_pubkey);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Core OFT --------------------------------------------------------
        "send" => {
            let s = state.as_mut().expect("LZ: not initialised");
            let a: SendArgs = serde_json::from_slice(args).expect("LZ: bad send args");
            let nonce = s.send(
                caller,
                a.dst_chain_id,
                a.dst_address,
                a.amount,
                a.adapter_params,
            );
            serde_json::to_vec(&nonce).unwrap()
        }
        "lz_receive" => {
            let s = state.as_mut().expect("LZ: not initialised");
            let a: LzReceiveArgs = serde_json::from_slice(args).expect("LZ: bad lz_receive args");
            let amount = s.lz_receive(
                caller,
                a.src_chain_id,
                a.src_address,
                a.nonce,
                a.payload,
                a.endpoint_signature,
            );
            serde_json::to_vec(&amount).unwrap()
        }
        "retry_message" => {
            let s = state.as_mut().expect("LZ: not initialised");
            let a: RetryMessageArgs = serde_json::from_slice(args).expect("LZ: bad args");
            let amount = s.retry_message(caller, a.src_chain_id, a.src_address, a.nonce);
            serde_json::to_vec(&amount).unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "is_trusted_remote" => {
            let s = state.as_ref().expect("LZ: not initialised");
            let a: IsTrustedRemoteArgs = serde_json::from_slice(args).expect("LZ: bad args");
            serde_json::to_vec(&s.is_trusted_remote(a.chain_id, &a.address)).unwrap()
        }
        "get_outbound_nonce" => {
            let s = state.as_ref().expect("LZ: not initialised");
            let a: GetNonceArgs = serde_json::from_slice(args).expect("LZ: bad args");
            serde_json::to_vec(&s.get_outbound_nonce(a.chain_id)).unwrap()
        }
        "get_inbound_nonce" => {
            let s = state.as_ref().expect("LZ: not initialised");
            let a: GetInboundNonceArgs = serde_json::from_slice(args).expect("LZ: bad args");
            serde_json::to_vec(&s.get_inbound_nonce(a.chain_id, &a.address)).unwrap()
        }
        "failed_message_count" => {
            let s = state.as_ref().expect("LZ: not initialised");
            serde_json::to_vec(&s.failed_message_count()).unwrap()
        }

        _ => panic!("LZ: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn owner() -> [u8; 32] {
        [1u8; 32]
    }
    fn usdc() -> [u8; 32] {
        [20u8; 32]
    }
    fn alice() -> [u8; 32] {
        [3u8; 32]
    }
    fn remote_base() -> [u8; 32] {
        [50u8; 32]
    }

    /// Returns (endpoint_address, endpoint_pubkey, signing_key).
    fn endpoint_keypair() -> ([u8; 32], [u8; 32], SigningKey) {
        let signing_key = SigningKey::from_bytes(&[10u8; 32]);
        let pubkey = signing_key.verifying_key().to_bytes();
        let address = [10u8; 32];
        (address, pubkey, signing_key)
    }

    fn sign_receive(
        signing_key: &SigningKey,
        src_chain_id: u16,
        src_address: &[u8; 32],
        nonce: u64,
        payload: &OftPayload,
    ) -> [u8; 64] {
        let hash = LayerZeroState::compute_receive_hash(src_chain_id, src_address, nonce, payload);
        let sig = signing_key.sign(&hash);
        sig.to_bytes()
    }

    fn setup() -> (LayerZeroState, SigningKey) {
        let (ep_addr, ep_pubkey, ep_key) = endpoint_keypair();
        let mut s = LayerZeroState::new(owner(), ep_addr, ep_pubkey, usdc());
        s.set_trusted_remote(owner(), LZ_BASE, remote_base());
        s.set_trusted_remote(owner(), LZ_ETHEREUM, [60u8; 32]);
        (s, ep_key)
    }

    #[test]
    fn test_init() {
        let (s, _) = setup();
        assert_eq!(s.local_chain_id, LZ_DINA);
        assert!(s.is_trusted_remote(LZ_BASE, &remote_base()));
    }

    #[test]
    fn test_send() {
        let (mut s, _) = setup();
        let nonce = s.send(alice(), LZ_BASE, alice(), 1_000_000, None);
        assert_eq!(nonce, 1);
        assert_eq!(s.get_outbound_nonce(LZ_BASE), 1);
    }

    #[test]
    fn test_send_increments_nonce() {
        let (mut s, _) = setup();
        s.send(alice(), LZ_BASE, alice(), 1_000_000, None);
        let nonce = s.send(alice(), LZ_BASE, alice(), 2_000_000, None);
        assert_eq!(nonce, 2);
    }

    #[test]
    #[should_panic(expected = "destination not trusted")]
    fn test_send_to_untrusted_fails() {
        let (mut s, _) = setup();
        s.send(alice(), 9999, alice(), 1_000_000, None);
    }

    #[test]
    fn test_lz_receive() {
        let (mut s, ep_key) = setup();
        let (ep_addr, _, _) = endpoint_keypair();
        let payload = OftPayload {
            to: alice(),
            amount: 500_000,
            compose_msg: None,
        };
        let sig = sign_receive(&ep_key, LZ_BASE, &remote_base(), 1, &payload);
        let amount = s.lz_receive(ep_addr, LZ_BASE, remote_base(), 1, payload, sig);
        assert_eq!(amount, 500_000);
        assert_eq!(s.get_inbound_nonce(LZ_BASE, &remote_base()), 1);
    }

    #[test]
    #[should_panic(expected = "only endpoint")]
    fn test_lz_receive_not_endpoint_fails() {
        let (mut s, ep_key) = setup();
        let payload = OftPayload {
            to: alice(),
            amount: 500_000,
            compose_msg: None,
        };
        let sig = sign_receive(&ep_key, LZ_BASE, &remote_base(), 1, &payload);
        s.lz_receive(alice(), LZ_BASE, remote_base(), 1, payload, sig);
    }

    #[test]
    #[should_panic(expected = "invalid endpoint signature")]
    fn test_lz_receive_bad_signature_fails() {
        let (mut s, _) = setup();
        let (ep_addr, _, _) = endpoint_keypair();
        let payload = OftPayload {
            to: alice(),
            amount: 500_000,
            compose_msg: None,
        };
        let bad_sig = [0u8; 64];
        s.lz_receive(ep_addr, LZ_BASE, remote_base(), 1, payload, bad_sig);
    }

    #[test]
    fn test_out_of_order_nonce_stored_as_failed() {
        let (mut s, ep_key) = setup();
        let (ep_addr, _, _) = endpoint_keypair();
        let payload = OftPayload {
            to: alice(),
            amount: 500_000,
            compose_msg: None,
        };
        // Send nonce 2 before nonce 1
        let sig = sign_receive(&ep_key, LZ_BASE, &remote_base(), 2, &payload);
        let amount = s.lz_receive(ep_addr, LZ_BASE, remote_base(), 2, payload, sig);
        assert_eq!(amount, 0);
        assert_eq!(s.failed_message_count(), 1);
    }

    #[test]
    fn test_retry_message() {
        let (mut s, ep_key) = setup();
        let (ep_addr, _, _) = endpoint_keypair();
        // First receive nonce 1
        let p1 = OftPayload {
            to: alice(),
            amount: 100_000,
            compose_msg: None,
        };
        let sig1 = sign_receive(&ep_key, LZ_BASE, &remote_base(), 1, &p1);
        s.lz_receive(ep_addr, LZ_BASE, remote_base(), 1, p1, sig1);

        // Nonce 3 arrives before nonce 2 — fails
        let p3 = OftPayload {
            to: alice(),
            amount: 300_000,
            compose_msg: None,
        };
        let sig3 = sign_receive(&ep_key, LZ_BASE, &remote_base(), 3, &p3);
        s.lz_receive(ep_addr, LZ_BASE, remote_base(), 3, p3, sig3);
        assert_eq!(s.failed_message_count(), 1);

        // Retry nonce 3 (after nonce 2 was presumably processed)
        let amount = s.retry_message(owner(), LZ_BASE, remote_base(), 3);
        assert_eq!(amount, 300_000);
        assert_eq!(s.failed_message_count(), 0);
    }

    #[test]
    fn test_chain_id_constants() {
        assert_eq!(LZ_ETHEREUM, 30101);
        assert_eq!(LZ_ARBITRUM, 30110);
        assert_eq!(LZ_SOLANA, 30168);
        assert_eq!(LZ_BASE, 30184);
        assert_eq!(LZ_DINA, 30099);
    }

    #[test]
    #[should_panic(expected = "cannot set self as remote")]
    fn test_set_self_as_remote_fails() {
        let (mut s, _) = setup();
        s.set_trusted_remote(owner(), LZ_DINA, alice());
    }
}
