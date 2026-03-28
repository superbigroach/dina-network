use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{info, warn};

use dina_core::Address;

use crate::attestation;
use crate::types::{BridgeStats, BridgeStatus, BridgeTransfer, ChainId};

// ---------------------------------------------------------------------------
// CCTP domain IDs — Circle's Cross-Chain Transfer Protocol domain identifiers
// ---------------------------------------------------------------------------

/// Ethereum Mainnet CCTP domain.
pub const DOMAIN_ETHEREUM: u32 = 0;
/// Solana CCTP domain.
pub const DOMAIN_SOLANA: u32 = 5;
/// Base CCTP domain.
pub const DOMAIN_BASE: u32 = 6;
/// Dina Network CCTP domain (placeholder — pending Circle registration).
pub const DOMAIN_DINA: u32 = 99;

// ---------------------------------------------------------------------------
// CCTP message types
// ---------------------------------------------------------------------------

/// A CCTP cross-chain message following Circle's V2 specification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CctpMessage {
    /// CCTP protocol version.
    pub version: u32,
    /// Domain identifier of the source chain.
    pub source_domain: u32,
    /// Domain identifier of the destination chain.
    pub destination_domain: u32,
    /// Per-source-domain monotonic nonce.
    pub nonce: u64,
    /// 32-byte padded sender address on the source chain.
    pub sender: [u8; 32],
    /// 32-byte padded recipient address on the destination chain.
    pub recipient: [u8; 32],
    /// 32-byte padded address authorized to call `receiveMessage` on the
    /// destination chain. All zeros means any caller is allowed.
    pub destination_caller: [u8; 32],
    /// ABI-encoded burn/mint body (amount, mint recipient, etc.).
    pub message_body: Vec<u8>,
}

/// A CCTP message paired with Circle's off-chain attestation signature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CctpAttestation {
    /// The original CCTP message.
    pub message: CctpMessage,
    /// Circle's ECDSA signature attesting that the burn event happened.
    pub attestation: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Bridge state machine
// ---------------------------------------------------------------------------

/// Core bridge state that tracks all cross-chain transfers via CCTP.
pub struct CctpBridge {
    /// Mapping from CCTP domain ID to our internal `ChainId`.
    supported_domains: BTreeMap<u32, ChainId>,
    /// In-flight and recently-completed transfers keyed by transfer ID.
    pending_transfers: BTreeMap<[u8; 32], BridgeTransfer>,
    /// IDs of completed transfers (kept for stats / audit trail).
    completed_transfers: Vec<[u8; 32]>,
    /// Monotonically increasing nonce for outbound transfers.
    next_nonce: u64,
    /// Cumulative USDC (micro-units) bridged into Dina.
    total_bridged_in: u64,
    /// Cumulative USDC (micro-units) bridged out of Dina.
    total_bridged_out: u64,
}

/// Errors that can occur during bridge operations.
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("unsupported source domain: {0}")]
    UnsupportedSourceDomain(u32),
    #[error("unsupported destination domain: {0}")]
    UnsupportedDestinationDomain(u32),
    #[error("unsupported chain: {0}")]
    UnsupportedChain(ChainId),
    #[error("invalid attestation for message nonce {0}")]
    InvalidAttestation(u64),
    #[error("transfer not found: {0}")]
    TransferNotFound(String),
    #[error("transfer already exists: {0}")]
    DuplicateTransfer(String),
    #[error("invalid amount: {0}")]
    InvalidAmount(String),
    #[error("invalid recipient: {0}")]
    InvalidRecipient(String),
    #[error("message decode error: {0}")]
    DecodeError(String),
}

impl CctpBridge {
    /// Create a new bridge with the default supported CCTP domain mappings.
    pub fn new() -> Self {
        let mut supported_domains = BTreeMap::new();
        supported_domains.insert(DOMAIN_ETHEREUM, ChainId::EthereumMainnet);
        supported_domains.insert(DOMAIN_BASE, ChainId::BaseMainnet);
        supported_domains.insert(DOMAIN_SOLANA, ChainId::SolanaMainnet);
        supported_domains.insert(DOMAIN_DINA, ChainId::DinaMainnet);

        Self {
            supported_domains,
            pending_transfers: BTreeMap::new(),
            completed_transfers: Vec::new(),
            next_nonce: 0,
            total_bridged_in: 0,
            total_bridged_out: 0,
        }
    }

    /// Register an additional CCTP domain mapping (e.g., testnets).
    pub fn register_domain(&mut self, domain_id: u32, chain_id: ChainId) {
        self.supported_domains.insert(domain_id, chain_id);
    }

    /// Look up the `ChainId` for a CCTP domain, if supported.
    pub fn chain_for_domain(&self, domain: u32) -> Option<&ChainId> {
        self.supported_domains.get(&domain)
    }

    /// Look up the CCTP domain ID for a `ChainId`, if registered.
    pub fn domain_for_chain(&self, chain: &ChainId) -> Option<u32> {
        self.supported_domains
            .iter()
            .find(|(_, c)| *c == chain)
            .map(|(d, _)| *d)
    }

    // -----------------------------------------------------------------------
    // Bridge out: Dina -> other chain
    // -----------------------------------------------------------------------

    /// Initiate a bridge-out transfer from Dina Network to another chain.
    ///
    /// This creates a pending transfer record and a CCTP burn message. The
    /// actual on-chain burn transaction and Circle attestation flow happen
    /// outside this state machine.
    pub fn initiate_bridge_out(
        &mut self,
        from: Address,
        to_chain: ChainId,
        recipient: Vec<u8>,
        amount: u64,
        current_time: u64,
    ) -> Result<BridgeTransfer, BridgeError> {
        if amount == 0 {
            return Err(BridgeError::InvalidAmount(
                "amount must be greater than zero".to_string(),
            ));
        }

        if recipient.is_empty() {
            return Err(BridgeError::InvalidRecipient(
                "recipient address cannot be empty".to_string(),
            ));
        }

        // Verify destination chain is supported.
        let _dest_domain = self
            .domain_for_chain(&to_chain)
            .ok_or(BridgeError::UnsupportedChain(to_chain))?;

        let nonce = self.next_nonce;
        self.next_nonce += 1;

        // Derive a deterministic transfer ID from the key fields.
        let id = Self::compute_transfer_id(from.as_bytes(), &recipient, amount, nonce);

        if self.pending_transfers.contains_key(&id) {
            return Err(BridgeError::DuplicateTransfer(hex::encode(id)));
        }

        let transfer = BridgeTransfer {
            id,
            from_chain: ChainId::DinaMainnet,
            to_chain,
            sender: from.as_bytes().to_vec(),
            recipient: Self::pad_recipient(&recipient),
            amount,
            nonce,
            status: BridgeStatus::Pending,
            created_at: current_time,
            completed_at: None,
            source_tx_hash: None,
            dina_tx_hash: None,
        };

        info!(
            transfer_id = %hex::encode(id),
            to_chain = %to_chain,
            amount = amount,
            "bridge-out transfer initiated"
        );

        self.pending_transfers.insert(id, transfer.clone());
        self.total_bridged_out += amount;

        Ok(transfer)
    }

    // -----------------------------------------------------------------------
    // Bridge in: other chain -> Dina
    // -----------------------------------------------------------------------

    /// Process an inbound bridge transfer from another chain into Dina Network.
    ///
    /// Validates the CCTP message structure and Circle attestation, then
    /// creates a transfer record that will be fulfilled by minting USDC on
    /// the Dina side.
    pub fn receive_bridge_in(
        &mut self,
        cctp_msg: CctpMessage,
        attestation_bytes: Vec<u8>,
        current_time: u64,
    ) -> Result<BridgeTransfer, BridgeError> {
        // Verify the source domain is one we recognize.
        let from_chain = self
            .supported_domains
            .get(&cctp_msg.source_domain)
            .copied()
            .ok_or(BridgeError::UnsupportedSourceDomain(cctp_msg.source_domain))?;

        // Verify the destination domain is Dina.
        if self.supported_domains.get(&cctp_msg.destination_domain) != Some(&ChainId::DinaMainnet)
            && self.supported_domains.get(&cctp_msg.destination_domain)
                != Some(&ChainId::DinaTestnet)
        {
            return Err(BridgeError::UnsupportedDestinationDomain(
                cctp_msg.destination_domain,
            ));
        }

        // Verify the attestation from Circle.
        if !attestation::verify_cctp_attestation(&cctp_msg, &attestation_bytes) {
            warn!(
                nonce = cctp_msg.nonce,
                source_domain = cctp_msg.source_domain,
                "invalid CCTP attestation"
            );
            return Err(BridgeError::InvalidAttestation(cctp_msg.nonce));
        }

        // Extract the USDC amount from the message body.
        let amount = Self::extract_amount_from_body(&cctp_msg.message_body);

        let id = Self::compute_transfer_id(
            &cctp_msg.sender,
            &cctp_msg.recipient,
            amount,
            cctp_msg.nonce,
        );

        if self.pending_transfers.contains_key(&id) {
            return Err(BridgeError::DuplicateTransfer(hex::encode(id)));
        }

        let transfer = BridgeTransfer {
            id,
            from_chain,
            to_chain: ChainId::DinaMainnet,
            sender: cctp_msg.sender.to_vec(),
            recipient: cctp_msg.recipient,
            amount,
            nonce: cctp_msg.nonce,
            status: BridgeStatus::AttestationReceived,
            created_at: current_time,
            completed_at: None,
            source_tx_hash: None,
            dina_tx_hash: None,
        };

        info!(
            transfer_id = %hex::encode(id),
            from_chain = %from_chain,
            amount = amount,
            "bridge-in transfer received with valid attestation"
        );

        self.pending_transfers.insert(id, transfer.clone());
        self.total_bridged_in += amount;

        Ok(transfer)
    }

    // -----------------------------------------------------------------------
    // Transfer lifecycle management
    // -----------------------------------------------------------------------

    /// Update a transfer's status to `SourceConfirmed` and record the source
    /// chain transaction hash.
    pub fn confirm_source(
        &mut self,
        id: [u8; 32],
        source_tx_hash: Vec<u8>,
    ) -> Result<(), BridgeError> {
        let transfer = self
            .pending_transfers
            .get_mut(&id)
            .ok_or_else(|| BridgeError::TransferNotFound(hex::encode(id)))?;

        transfer.status = BridgeStatus::SourceConfirmed;
        transfer.source_tx_hash = Some(source_tx_hash);
        Ok(())
    }

    /// Update a transfer's status to `DinaConfirmed` and record the Dina
    /// transaction hash.
    pub fn confirm_dina(
        &mut self,
        id: [u8; 32],
        dina_tx_hash: [u8; 32],
    ) -> Result<(), BridgeError> {
        let transfer = self
            .pending_transfers
            .get_mut(&id)
            .ok_or_else(|| BridgeError::TransferNotFound(hex::encode(id)))?;

        transfer.status = BridgeStatus::DinaConfirmed;
        transfer.dina_tx_hash = Some(dina_tx_hash);
        Ok(())
    }

    /// Mark a transfer as fully completed.
    pub fn complete_transfer(
        &mut self,
        id: [u8; 32],
        completed_at: u64,
    ) -> Result<(), BridgeError> {
        let transfer = self
            .pending_transfers
            .get_mut(&id)
            .ok_or_else(|| BridgeError::TransferNotFound(hex::encode(id)))?;

        transfer.status = BridgeStatus::Completed;
        transfer.completed_at = Some(completed_at);
        self.completed_transfers.push(id);

        info!(
            transfer_id = %hex::encode(id),
            "bridge transfer completed"
        );

        Ok(())
    }

    /// Mark a transfer as failed with a reason.
    pub fn fail_transfer(&mut self, id: [u8; 32], reason: String) -> Result<(), BridgeError> {
        let transfer = self
            .pending_transfers
            .get_mut(&id)
            .ok_or_else(|| BridgeError::TransferNotFound(hex::encode(id)))?;

        warn!(
            transfer_id = %hex::encode(id),
            reason = %reason,
            "bridge transfer failed"
        );

        transfer.status = BridgeStatus::Failed(reason);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Look up a transfer by its ID.
    pub fn get_transfer(&self, id: [u8; 32]) -> Option<&BridgeTransfer> {
        self.pending_transfers.get(&id)
    }

    /// Return all transfers that have not yet completed or failed.
    pub fn pending_transfers(&self) -> Vec<&BridgeTransfer> {
        self.pending_transfers
            .values()
            .filter(|t| {
                matches!(
                    t.status,
                    BridgeStatus::Pending
                        | BridgeStatus::SourceConfirmed
                        | BridgeStatus::AttestationReceived
                        | BridgeStatus::DinaConfirmed
                )
            })
            .collect()
    }

    /// Aggregate bridge statistics.
    pub fn stats(&self) -> BridgeStats {
        let mut pending_count = 0;
        let mut failed_count = 0;

        for transfer in self.pending_transfers.values() {
            match &transfer.status {
                BridgeStatus::Completed => {}
                BridgeStatus::Failed(_) => failed_count += 1,
                _ => pending_count += 1,
            }
        }

        BridgeStats {
            total_bridged_in: self.total_bridged_in,
            total_bridged_out: self.total_bridged_out,
            pending_count,
            completed_count: self.completed_transfers.len(),
            failed_count,
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Compute a deterministic transfer ID by hashing the key fields.
    fn compute_transfer_id(sender: &[u8], recipient: &[u8], amount: u64, nonce: u64) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(sender);
        hasher.update(recipient);
        hasher.update(amount.to_le_bytes());
        hasher.update(nonce.to_le_bytes());
        let result = hasher.finalize();
        let mut id = [0u8; 32];
        id.copy_from_slice(&result);
        id
    }

    /// Pad a variable-length recipient address to 32 bytes (right-aligned,
    /// matching CCTP's address encoding for EVM chains).
    fn pad_recipient(recipient: &[u8]) -> [u8; 32] {
        let mut padded = [0u8; 32];
        if recipient.len() >= 32 {
            padded.copy_from_slice(&recipient[..32]);
        } else {
            // Right-align (EVM-style zero padding on the left).
            let offset = 32 - recipient.len();
            padded[offset..].copy_from_slice(recipient);
        }
        padded
    }

    /// Extract the USDC amount from a CCTP message body.
    ///
    /// The CCTP V2 burn message body is ABI-encoded with the amount as the
    /// first 32-byte word (big-endian uint256). We read the last 8 bytes
    /// of that word since USDC amounts fit in u64.
    fn extract_amount_from_body(body: &[u8]) -> u64 {
        if body.len() < 32 {
            return 0;
        }
        // Amount is in the first 32-byte slot, big-endian.
        // USDC uses 6 decimals so the value always fits in u64.
        let mut amount_bytes = [0u8; 8];
        amount_bytes.copy_from_slice(&body[24..32]);
        u64::from_be_bytes(amount_bytes)
    }
}

impl Default for CctpBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_address() -> Address {
        Address([0xaa; 32])
    }

    #[test]
    fn new_bridge_has_default_domains() {
        let bridge = CctpBridge::new();
        assert_eq!(
            bridge.chain_for_domain(DOMAIN_ETHEREUM),
            Some(&ChainId::EthereumMainnet)
        );
        assert_eq!(
            bridge.chain_for_domain(DOMAIN_BASE),
            Some(&ChainId::BaseMainnet)
        );
        assert_eq!(
            bridge.chain_for_domain(DOMAIN_SOLANA),
            Some(&ChainId::SolanaMainnet)
        );
        assert_eq!(
            bridge.chain_for_domain(DOMAIN_DINA),
            Some(&ChainId::DinaMainnet)
        );
    }

    #[test]
    fn register_custom_domain() {
        let mut bridge = CctpBridge::new();
        bridge.register_domain(100, ChainId::ArcMainnet);
        assert_eq!(bridge.chain_for_domain(100), Some(&ChainId::ArcMainnet));
    }

    #[test]
    fn domain_for_chain_roundtrip() {
        let bridge = CctpBridge::new();
        assert_eq!(
            bridge.domain_for_chain(&ChainId::EthereumMainnet),
            Some(DOMAIN_ETHEREUM)
        );
        assert_eq!(
            bridge.domain_for_chain(&ChainId::BaseMainnet),
            Some(DOMAIN_BASE)
        );
        assert_eq!(bridge.domain_for_chain(&ChainId::ArcTestnet), None);
    }

    #[test]
    fn initiate_bridge_out_success() {
        let mut bridge = CctpBridge::new();
        let from = test_address();
        let recipient = vec![0xbb; 20]; // EVM address

        let transfer = bridge
            .initiate_bridge_out(
                from,
                ChainId::BaseMainnet,
                recipient.clone(),
                1_000_000,
                1700000000,
            )
            .unwrap();

        assert_eq!(transfer.from_chain, ChainId::DinaMainnet);
        assert_eq!(transfer.to_chain, ChainId::BaseMainnet);
        assert_eq!(transfer.amount, 1_000_000);
        assert_eq!(transfer.status, BridgeStatus::Pending);
        assert_eq!(transfer.nonce, 0);
    }

    #[test]
    fn initiate_bridge_out_zero_amount_fails() {
        let mut bridge = CctpBridge::new();
        let result = bridge.initiate_bridge_out(
            test_address(),
            ChainId::BaseMainnet,
            vec![0xbb; 20],
            0,
            1700000000,
        );
        assert!(matches!(result, Err(BridgeError::InvalidAmount(_))));
    }

    #[test]
    fn initiate_bridge_out_empty_recipient_fails() {
        let mut bridge = CctpBridge::new();
        let result = bridge.initiate_bridge_out(
            test_address(),
            ChainId::BaseMainnet,
            vec![],
            1_000_000,
            1700000000,
        );
        assert!(matches!(result, Err(BridgeError::InvalidRecipient(_))));
    }

    #[test]
    fn initiate_bridge_out_unsupported_chain_fails() {
        let mut bridge = CctpBridge::new();
        let result = bridge.initiate_bridge_out(
            test_address(),
            ChainId::ArcTestnet,
            vec![0xbb; 20],
            1_000_000,
            1700000000,
        );
        assert!(matches!(result, Err(BridgeError::UnsupportedChain(_))));
    }

    #[test]
    fn nonce_increments_on_successive_bridge_outs() {
        let mut bridge = CctpBridge::new();
        let from = test_address();

        let t1 = bridge
            .initiate_bridge_out(from, ChainId::BaseMainnet, vec![0xbb; 20], 100, 1700000000)
            .unwrap();
        let t2 = bridge
            .initiate_bridge_out(from, ChainId::BaseMainnet, vec![0xcc; 20], 200, 1700000001)
            .unwrap();

        assert_eq!(t1.nonce, 0);
        assert_eq!(t2.nonce, 1);
    }

    #[test]
    fn transfer_lifecycle() {
        let mut bridge = CctpBridge::new();
        let transfer = bridge
            .initiate_bridge_out(
                test_address(),
                ChainId::EthereumMainnet,
                vec![0xdd; 20],
                5_000_000,
                1700000000,
            )
            .unwrap();
        let id = transfer.id;

        // Confirm source.
        bridge.confirm_source(id, vec![0xee; 32]).unwrap();
        assert_eq!(
            bridge.get_transfer(id).unwrap().status,
            BridgeStatus::SourceConfirmed
        );

        // Confirm on Dina side.
        bridge.confirm_dina(id, [0xff; 32]).unwrap();
        assert_eq!(
            bridge.get_transfer(id).unwrap().status,
            BridgeStatus::DinaConfirmed
        );

        // Complete.
        bridge.complete_transfer(id, 1700001000).unwrap();
        let completed = bridge.get_transfer(id).unwrap();
        assert_eq!(completed.status, BridgeStatus::Completed);
        assert_eq!(completed.completed_at, Some(1700001000));
    }

    #[test]
    fn fail_transfer_records_reason() {
        let mut bridge = CctpBridge::new();
        let transfer = bridge
            .initiate_bridge_out(
                test_address(),
                ChainId::BaseMainnet,
                vec![0xaa; 20],
                1_000,
                1700000000,
            )
            .unwrap();

        bridge
            .fail_transfer(transfer.id, "attestation timeout".to_string())
            .unwrap();

        let failed = bridge.get_transfer(transfer.id).unwrap();
        assert_eq!(
            failed.status,
            BridgeStatus::Failed("attestation timeout".to_string())
        );
    }

    #[test]
    fn pending_transfers_filters_correctly() {
        let mut bridge = CctpBridge::new();
        let t1 = bridge
            .initiate_bridge_out(
                test_address(),
                ChainId::BaseMainnet,
                vec![0x01; 20],
                100,
                1700000000,
            )
            .unwrap();
        let t2 = bridge
            .initiate_bridge_out(
                test_address(),
                ChainId::BaseMainnet,
                vec![0x02; 20],
                200,
                1700000001,
            )
            .unwrap();

        // Complete one of them.
        bridge.complete_transfer(t1.id, 1700001000).unwrap();

        let pending = bridge.pending_transfers();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, t2.id);
    }

    #[test]
    fn stats_are_accurate() {
        let mut bridge = CctpBridge::new();

        let t1 = bridge
            .initiate_bridge_out(
                test_address(),
                ChainId::BaseMainnet,
                vec![0x01; 20],
                1_000_000,
                1700000000,
            )
            .unwrap();
        bridge
            .initiate_bridge_out(
                test_address(),
                ChainId::BaseMainnet,
                vec![0x02; 20],
                2_000_000,
                1700000001,
            )
            .unwrap();

        bridge.complete_transfer(t1.id, 1700001000).unwrap();

        let stats = bridge.stats();
        assert_eq!(stats.total_bridged_out, 3_000_000);
        assert_eq!(stats.completed_count, 1);
        assert_eq!(stats.pending_count, 1);
    }

    #[test]
    fn pad_recipient_evm_address() {
        let evm_addr = vec![0xab; 20];
        let padded = CctpBridge::pad_recipient(&evm_addr);
        // First 12 bytes should be zero, last 20 should be the address.
        assert_eq!(&padded[..12], &[0u8; 12]);
        assert_eq!(&padded[12..], &[0xab; 20]);
    }

    #[test]
    fn pad_recipient_32_byte_address() {
        let full_addr = vec![0xcd; 32];
        let padded = CctpBridge::pad_recipient(&full_addr);
        assert_eq!(&padded, &[0xcd; 32]);
    }

    #[test]
    fn extract_amount_from_body_valid() {
        // Construct a 32-byte ABI-encoded amount: 1_000_000 in big-endian.
        let mut body = vec![0u8; 32];
        let amount_bytes = 1_000_000u64.to_be_bytes();
        body[24..32].copy_from_slice(&amount_bytes);

        let amount = CctpBridge::extract_amount_from_body(&body);
        assert_eq!(amount, 1_000_000);
    }

    #[test]
    fn extract_amount_from_body_too_short() {
        let body = vec![0u8; 16];
        let amount = CctpBridge::extract_amount_from_body(&body);
        assert_eq!(amount, 0);
    }

    #[test]
    fn get_nonexistent_transfer_returns_none() {
        let bridge = CctpBridge::new();
        assert!(bridge.get_transfer([0xff; 32]).is_none());
    }

    #[test]
    fn confirm_nonexistent_transfer_fails() {
        let mut bridge = CctpBridge::new();
        let result = bridge.confirm_source([0xff; 32], vec![0x00; 32]);
        assert!(matches!(result, Err(BridgeError::TransferNotFound(_))));
    }
}
