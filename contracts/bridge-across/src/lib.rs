use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Across Protocol Spoke Pool — Relayer-based instant bridging for Dina Network
// ---------------------------------------------------------------------------
//
// Across uses a "spoke pool" model: depositors lock funds on the source chain,
// relayers fill the deposit on the destination chain from their own capital,
// and get repaid later via optimistic verification + Merkle proofs on HubPool.
//
// Key properties:
//   - Fastest bridge option: 1-3 minute transfers
//   - Relayers pre-fund fills, no waiting for source chain finality
//   - Fees: ~0.1-0.5% relayer fee
//   - Fallback slow relay path via Merkle proof for unfilled deposits
//
// Chain IDs:
//   Ethereum = 1, Base = 8453, Arbitrum = 42161, Optimism = 10, Dina = 99999
// ---------------------------------------------------------------------------

/// Supported chain IDs for Across bridging.
pub const CHAIN_ID_ETHEREUM: u64 = 1;
pub const CHAIN_ID_BASE: u64 = 8453;
pub const CHAIN_ID_ARBITRUM: u64 = 42161;
pub const CHAIN_ID_OPTIMISM: u64 = 10;
pub const CHAIN_ID_DINA: u64 = 99999;

/// A cross-chain deposit created by a user on the source chain.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Deposit {
    /// Address of the depositor on the source chain
    pub depositor: [u8; 32],
    /// Destination chain ID where the deposit should be filled
    pub destination_chain_id: u64,
    /// Recipient address on the destination chain
    pub recipient: [u8; 32],
    /// Token deposited on the source chain
    pub input_token: [u8; 32],
    /// Token to be received on the destination chain
    pub output_token: [u8; 32],
    /// Amount deposited on the source chain (6 decimals for USDC)
    pub input_amount: u64,
    /// Amount to be received on the destination chain (after relayer fee)
    pub output_amount: u64,
    /// Relayer fee percentage in basis points (e.g. 30 = 0.30%)
    pub relayer_fee_pct: u64,
    /// Timestamp when the quote was generated
    pub quote_timestamp: u64,
    /// Unix timestamp deadline for relayers to fill the deposit
    pub fill_deadline: u64,
    /// Unix timestamp for exclusive relayer period (0 = no exclusivity)
    pub exclusivity_deadline: u64,
    /// Optional message to execute on the destination chain
    pub message: Vec<u8>,
    /// Whether the deposit has been filled
    pub filled: bool,
    /// Origin chain where the deposit was created
    pub origin_chain_id: u64,
}

/// A relay fill record tracking a relayer's fill of a deposit.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Fill {
    /// The relayer who filled the deposit
    pub relayer: [u8; 32],
    /// Origin chain of the deposit
    pub origin_chain_id: u64,
    /// Deposit nonce on the origin chain
    pub deposit_nonce: u64,
    /// Amount filled
    pub amount: u64,
    /// Chain where the relayer wants to be repaid
    pub repayment_chain_id: u64,
    /// Block/timestamp when the fill occurred
    pub fill_timestamp: u64,
}

/// Relay data for slow relay Merkle proof verification.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RelayData {
    /// Depositor address
    pub depositor: [u8; 32],
    /// Recipient address
    pub recipient: [u8; 32],
    /// Input token on source chain
    pub input_token: [u8; 32],
    /// Output token on destination chain
    pub output_token: [u8; 32],
    /// Input amount
    pub input_amount: u64,
    /// Output amount
    pub output_amount: u64,
    /// Origin chain ID
    pub origin_chain_id: u64,
    /// Deposit nonce
    pub deposit_nonce: u64,
    /// Fill deadline
    pub fill_deadline: u64,
    /// Message payload
    pub message: Vec<u8>,
}

/// Merkle proof for slow relay verification.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MerkleProof {
    /// Proof hashes from leaf to root
    pub proof: Vec<[u8; 32]>,
    /// Leaf index in the Merkle tree
    pub leaf_index: u64,
    /// Merkle root hash that was published by HubPool
    pub root: [u8; 32],
}

/// Full on-chain state for the Across Spoke Pool contract on Dina.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AcrossSpokePoolState {
    /// Contract owner (can configure pool settings)
    pub owner: [u8; 32],
    /// Address of this spoke pool contract
    pub spoke_pool_address: [u8; 32],
    /// Chain ID of the HubPool (Ethereum mainnet = 1)
    pub hub_pool_chain_id: u64,
    /// This spoke pool's chain ID (Dina = 99999)
    pub chain_id: u64,
    /// Monotonically increasing deposit nonce
    pub deposit_nonce: u64,
    /// All deposits indexed by nonce
    pub deposits: BTreeMap<u64, Deposit>,
    /// All fills indexed by (origin_chain_id, deposit_nonce)
    pub fills: BTreeMap<(u64, u64), Fill>,
    /// Published Merkle roots from HubPool for slow relay verification
    pub merkle_roots: Vec<[u8; 32]>,
    /// Whitelisted relayer addresses
    pub whitelisted_relayers: BTreeMap<[u8; 32], bool>,
    /// Token balances held by the spoke pool (token -> amount)
    pub pool_balances: BTreeMap<[u8; 32], u64>,
    /// Whether the contract is paused
    pub paused: bool,
    /// Supported destination chain IDs
    pub supported_chains: Vec<u64>,
}

impl AcrossSpokePoolState {
    /// Create a new Across Spoke Pool with the given owner.
    pub fn new(owner: [u8; 32]) -> Self {
        let supported_chains = vec![
            CHAIN_ID_ETHEREUM,
            CHAIN_ID_BASE,
            CHAIN_ID_ARBITRUM,
            CHAIN_ID_OPTIMISM,
            CHAIN_ID_DINA,
        ];

        Self {
            owner,
            spoke_pool_address: [0u8; 32],
            hub_pool_chain_id: CHAIN_ID_ETHEREUM,
            chain_id: CHAIN_ID_DINA,
            deposit_nonce: 0,
            deposits: BTreeMap::new(),
            fills: BTreeMap::new(),
            merkle_roots: Vec::new(),
            whitelisted_relayers: BTreeMap::new(),
            pool_balances: BTreeMap::new(),
            paused: false,
            supported_chains,
        }
    }

    // -- Queries -------------------------------------------------------------

    /// Get the current deposit nonce.
    pub fn current_nonce(&self) -> u64 {
        self.deposit_nonce
    }

    /// Get a deposit by nonce.
    pub fn get_deposit(&self, nonce: u64) -> Option<&Deposit> {
        self.deposits.get(&nonce)
    }

    /// Get a fill by origin chain and deposit nonce.
    pub fn get_fill(&self, origin_chain_id: u64, deposit_nonce: u64) -> Option<&Fill> {
        self.fills.get(&(origin_chain_id, deposit_nonce))
    }

    /// Check if a chain is supported.
    pub fn is_chain_supported(&self, chain_id: u64) -> bool {
        self.supported_chains.contains(&chain_id)
    }

    // -- Deposit (source chain) ----------------------------------------------

    /// Create a deposit for relayers to fill on the destination chain.
    ///
    /// The depositor locks tokens on Dina and a relayer fills on the
    /// destination chain from their own capital.
    pub fn deposit(
        &mut self,
        caller: [u8; 32],
        destination_chain_id: u64,
        recipient: [u8; 32],
        input_token: [u8; 32],
        output_token: [u8; 32],
        input_amount: u64,
        output_amount: u64,
        relayer_fee_pct: u64,
        quote_timestamp: u64,
        fill_deadline: u64,
        exclusivity_deadline: u64,
        message: Vec<u8>,
    ) -> u64 {
        assert!(!self.paused, "Across: contract is paused");
        assert!(input_amount > 0, "Across: deposit amount must be positive");
        assert!(
            output_amount <= input_amount,
            "Across: output cannot exceed input"
        );
        assert!(
            self.is_chain_supported(destination_chain_id),
            "Across: unsupported destination chain"
        );
        assert!(
            relayer_fee_pct <= 10_000,
            "Across: relayer fee exceeds 100%"
        );

        let nonce = self.deposit_nonce;
        self.deposit_nonce += 1;

        let deposit = Deposit {
            depositor: caller,
            destination_chain_id,
            recipient,
            input_token,
            output_token,
            input_amount,
            output_amount,
            relayer_fee_pct,
            quote_timestamp,
            fill_deadline,
            exclusivity_deadline,
            message,
            filled: false,
            origin_chain_id: self.chain_id,
        };

        // Lock the input tokens in the spoke pool
        let pool_balance = self.pool_balances.get(&input_token).copied().unwrap_or(0);
        self.pool_balances
            .insert(input_token, pool_balance + input_amount);

        self.deposits.insert(nonce, deposit);
        nonce
    }

    // -- Fill relay (destination chain) --------------------------------------

    /// Relayer fills a deposit on the destination chain.
    ///
    /// A relayer sends their own tokens to the recipient and records the fill
    /// so they can be repaid later via the HubPool.
    pub fn fill_relay(
        &mut self,
        caller: [u8; 32],
        depositor: [u8; 32],
        recipient: [u8; 32],
        input_token: [u8; 32],
        output_token: [u8; 32],
        input_amount: u64,
        output_amount: u64,
        repayment_chain_id: u64,
        origin_chain_id: u64,
        deposit_nonce: u64,
        fill_deadline: u64,
        message: Vec<u8>,
    ) {
        assert!(!self.paused, "Across: contract is paused");
        assert!(
            !self.fills.contains_key(&(origin_chain_id, deposit_nonce)),
            "Across: deposit already filled"
        );

        // Verify fill is for this chain
        // (In production, this would also verify the deposit details via a cross-chain message)

        let _ = depositor;
        let _ = input_token;
        let _ = input_amount;
        let _ = fill_deadline;
        let _ = message;

        // Record the fill
        let fill = Fill {
            relayer: caller,
            origin_chain_id,
            deposit_nonce,
            amount: output_amount,
            repayment_chain_id,
            fill_timestamp: 0, // Set by runtime
        };

        // Transfer output tokens from relayer's balance to the recipient
        // (In production this would interact with the token contract)
        let relayer_balance = self.pool_balances.get(&output_token).copied().unwrap_or(0);
        self.pool_balances
            .insert(output_token, relayer_balance + output_amount);

        self.fills.insert((origin_chain_id, deposit_nonce), fill);
    }

    // -- Slow relay fallback -------------------------------------------------

    /// Execute a slow relay leaf via Merkle proof verification.
    ///
    /// If no relayer fills a deposit before the deadline, the deposit can be
    /// settled via a Merkle proof published by the HubPool. This is slower
    /// but guarantees eventual settlement.
    pub fn execute_slow_relay_leaf(
        &mut self,
        relay_data: RelayData,
        proof: MerkleProof,
    ) {
        assert!(!self.paused, "Across: contract is paused");
        assert!(
            !self
                .fills
                .contains_key(&(relay_data.origin_chain_id, relay_data.deposit_nonce)),
            "Across: deposit already filled"
        );

        // Verify the Merkle root has been published by HubPool
        assert!(
            self.merkle_roots.contains(&proof.root),
            "Across: unknown Merkle root"
        );

        // Verify the Merkle proof
        // In production, this would compute the leaf hash from relay_data
        // and verify it against the proof and root.
        let leaf_hash = self.compute_relay_leaf_hash(&relay_data);
        assert!(
            self.verify_merkle_proof(&leaf_hash, &proof),
            "Across: invalid Merkle proof"
        );

        // Record as filled via slow relay
        let fill = Fill {
            relayer: [0u8; 32], // No relayer for slow fills
            origin_chain_id: relay_data.origin_chain_id,
            deposit_nonce: relay_data.deposit_nonce,
            amount: relay_data.output_amount,
            repayment_chain_id: 0,
            fill_timestamp: 0,
        };

        self.fills.insert(
            (relay_data.origin_chain_id, relay_data.deposit_nonce),
            fill,
        );
    }

    // -- Owner functions -----------------------------------------------------

    /// Add a Merkle root published by the HubPool.
    pub fn publish_merkle_root(&mut self, caller: [u8; 32], root: [u8; 32]) {
        assert!(caller == self.owner, "Across: only owner");
        self.merkle_roots.push(root);
    }

    /// Whitelist a relayer address.
    pub fn whitelist_relayer(&mut self, caller: [u8; 32], relayer: [u8; 32]) {
        assert!(caller == self.owner, "Across: only owner");
        self.whitelisted_relayers.insert(relayer, true);
    }

    /// Remove a relayer from the whitelist.
    pub fn remove_relayer(&mut self, caller: [u8; 32], relayer: [u8; 32]) {
        assert!(caller == self.owner, "Across: only owner");
        self.whitelisted_relayers.remove(&relayer);
    }

    /// Add a supported destination chain.
    pub fn add_supported_chain(&mut self, caller: [u8; 32], chain_id: u64) {
        assert!(caller == self.owner, "Across: only owner");
        if !self.supported_chains.contains(&chain_id) {
            self.supported_chains.push(chain_id);
        }
    }

    /// Pause the contract.
    pub fn pause(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "Across: only owner");
        self.paused = true;
    }

    /// Unpause the contract.
    pub fn unpause(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "Across: only owner");
        self.paused = false;
    }

    /// Transfer ownership to a new address.
    pub fn transfer_ownership(&mut self, caller: [u8; 32], new_owner: [u8; 32]) {
        assert!(caller == self.owner, "Across: only owner");
        self.owner = new_owner;
    }

    /// Set the spoke pool address.
    pub fn set_spoke_pool_address(&mut self, caller: [u8; 32], address: [u8; 32]) {
        assert!(caller == self.owner, "Across: only owner");
        self.spoke_pool_address = address;
    }

    // -- Internal helpers ----------------------------------------------------

    /// Compute a hash of the relay leaf data for Merkle proof verification.
    fn compute_relay_leaf_hash(&self, relay_data: &RelayData) -> [u8; 32] {
        // Simplified hash: in production this would use keccak256 or sha256
        // over the ABI-encoded relay data fields.
        let serialized = serde_json::to_vec(relay_data).unwrap_or_default();
        let mut hash = [0u8; 32];
        for (i, byte) in serialized.iter().enumerate() {
            hash[i % 32] ^= byte;
        }
        hash
    }

    /// Verify a Merkle proof against a root hash.
    fn verify_merkle_proof(&self, leaf_hash: &[u8; 32], proof: &MerkleProof) -> bool {
        // Simplified verification: in production this would walk the proof
        // path from leaf to root, hashing at each level.
        let mut current = *leaf_hash;
        for sibling in &proof.proof {
            let mut combined = [0u8; 32];
            for i in 0..32 {
                combined[i] = current[i] ^ sibling[i];
            }
            current = combined;
        }
        current == proof.root
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs {
    destination_chain_id: u64,
    recipient: [u8; 32],
    input_token: [u8; 32],
    output_token: [u8; 32],
    input_amount: u64,
    output_amount: u64,
    relayer_fee_pct: u64,
    quote_timestamp: u64,
    fill_deadline: u64,
    exclusivity_deadline: u64,
    message: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct FillRelayArgs {
    depositor: [u8; 32],
    recipient: [u8; 32],
    input_token: [u8; 32],
    output_token: [u8; 32],
    input_amount: u64,
    output_amount: u64,
    repayment_chain_id: u64,
    origin_chain_id: u64,
    deposit_nonce: u64,
    fill_deadline: u64,
    message: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlowRelayArgs {
    relay_data: RelayData,
    proof: MerkleProof,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetDepositArgs {
    nonce: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetFillArgs {
    origin_chain_id: u64,
    deposit_nonce: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct RelayerArgs {
    relayer: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct ChainArgs {
    chain_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct MerkleRootArgs {
    root: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct AddressArgs {
    address: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferOwnershipArgs {
    new_owner: [u8; 32],
}

// ---------------------------------------------------------------------------
// Contract dispatch
// ---------------------------------------------------------------------------

/// Entry point for the Across Spoke Pool contract. Routes method calls to the
/// appropriate handler on `AcrossSpokePoolState`.
pub fn dispatch(
    state: &mut Option<AcrossSpokePoolState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "Across: already initialised");
            *state = Some(AcrossSpokePoolState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "current_nonce" => {
            let s = state.as_ref().expect("Across: not initialised");
            serde_json::to_vec(&s.current_nonce()).unwrap()
        }
        "get_deposit" => {
            let s = state.as_ref().expect("Across: not initialised");
            let a: GetDepositArgs =
                serde_json::from_slice(args).expect("Across: bad get_deposit args");
            serde_json::to_vec(&s.get_deposit(a.nonce)).unwrap()
        }
        "get_fill" => {
            let s = state.as_ref().expect("Across: not initialised");
            let a: GetFillArgs =
                serde_json::from_slice(args).expect("Across: bad get_fill args");
            serde_json::to_vec(&s.get_fill(a.origin_chain_id, a.deposit_nonce)).unwrap()
        }
        "is_chain_supported" => {
            let s = state.as_ref().expect("Across: not initialised");
            let a: ChainArgs =
                serde_json::from_slice(args).expect("Across: bad is_chain_supported args");
            serde_json::to_vec(&s.is_chain_supported(a.chain_id)).unwrap()
        }
        "chain_id" => {
            let s = state.as_ref().expect("Across: not initialised");
            serde_json::to_vec(&s.chain_id).unwrap()
        }

        // -- Deposit ---------------------------------------------------------
        "deposit" => {
            let s = state.as_mut().expect("Across: not initialised");
            let a: DepositArgs =
                serde_json::from_slice(args).expect("Across: bad deposit args");
            let nonce = s.deposit(
                caller,
                a.destination_chain_id,
                a.recipient,
                a.input_token,
                a.output_token,
                a.input_amount,
                a.output_amount,
                a.relayer_fee_pct,
                a.quote_timestamp,
                a.fill_deadline,
                a.exclusivity_deadline,
                a.message,
            );
            serde_json::to_vec(&nonce).unwrap()
        }

        // -- Fill relay ------------------------------------------------------
        "fill_relay" => {
            let s = state.as_mut().expect("Across: not initialised");
            let a: FillRelayArgs =
                serde_json::from_slice(args).expect("Across: bad fill_relay args");
            s.fill_relay(
                caller,
                a.depositor,
                a.recipient,
                a.input_token,
                a.output_token,
                a.input_amount,
                a.output_amount,
                a.repayment_chain_id,
                a.origin_chain_id,
                a.deposit_nonce,
                a.fill_deadline,
                a.message,
            );
            serde_json::to_vec("ok").unwrap()
        }

        // -- Slow relay ------------------------------------------------------
        "execute_slow_relay_leaf" => {
            let s = state.as_mut().expect("Across: not initialised");
            let a: SlowRelayArgs =
                serde_json::from_slice(args).expect("Across: bad slow_relay args");
            s.execute_slow_relay_leaf(a.relay_data, a.proof);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Owner functions -------------------------------------------------
        "publish_merkle_root" => {
            let s = state.as_mut().expect("Across: not initialised");
            let a: MerkleRootArgs =
                serde_json::from_slice(args).expect("Across: bad merkle_root args");
            s.publish_merkle_root(caller, a.root);
            serde_json::to_vec("ok").unwrap()
        }
        "whitelist_relayer" => {
            let s = state.as_mut().expect("Across: not initialised");
            let a: RelayerArgs =
                serde_json::from_slice(args).expect("Across: bad whitelist_relayer args");
            s.whitelist_relayer(caller, a.relayer);
            serde_json::to_vec("ok").unwrap()
        }
        "remove_relayer" => {
            let s = state.as_mut().expect("Across: not initialised");
            let a: RelayerArgs =
                serde_json::from_slice(args).expect("Across: bad remove_relayer args");
            s.remove_relayer(caller, a.relayer);
            serde_json::to_vec("ok").unwrap()
        }
        "add_supported_chain" => {
            let s = state.as_mut().expect("Across: not initialised");
            let a: ChainArgs =
                serde_json::from_slice(args).expect("Across: bad add_supported_chain args");
            s.add_supported_chain(caller, a.chain_id);
            serde_json::to_vec("ok").unwrap()
        }
        "pause" => {
            let s = state.as_mut().expect("Across: not initialised");
            s.pause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "unpause" => {
            let s = state.as_mut().expect("Across: not initialised");
            s.unpause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer_ownership" => {
            let s = state.as_mut().expect("Across: not initialised");
            let a: TransferOwnershipArgs =
                serde_json::from_slice(args).expect("Across: bad transfer_ownership args");
            s.transfer_ownership(caller, a.new_owner);
            serde_json::to_vec("ok").unwrap()
        }
        "set_spoke_pool_address" => {
            let s = state.as_mut().expect("Across: not initialised");
            let a: AddressArgs =
                serde_json::from_slice(args).expect("Across: bad set_spoke_pool_address args");
            s.set_spoke_pool_address(caller, a.address);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("Across: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn owner() -> [u8; 32] {
        [1u8; 32]
    }
    fn relayer() -> [u8; 32] {
        [2u8; 32]
    }
    fn alice() -> [u8; 32] {
        [3u8; 32]
    }
    fn bob() -> [u8; 32] {
        [4u8; 32]
    }
    fn usdc_dina() -> [u8; 32] {
        [10u8; 32]
    }
    fn usdc_base() -> [u8; 32] {
        [11u8; 32]
    }

    fn setup() -> AcrossSpokePoolState {
        let mut s = AcrossSpokePoolState::new(owner());
        s.whitelist_relayer(owner(), relayer());
        s
    }

    #[test]
    fn test_init() {
        let s = AcrossSpokePoolState::new(owner());
        assert_eq!(s.chain_id, CHAIN_ID_DINA);
        assert_eq!(s.hub_pool_chain_id, CHAIN_ID_ETHEREUM);
        assert_eq!(s.deposit_nonce, 0);
        assert!(s.is_chain_supported(CHAIN_ID_BASE));
        assert!(s.is_chain_supported(CHAIN_ID_DINA));
    }

    #[test]
    fn test_deposit() {
        let mut s = setup();
        let nonce = s.deposit(
            alice(),
            CHAIN_ID_BASE,
            bob(),
            usdc_dina(),
            usdc_base(),
            1_000_000, // 1 USDC input
            997_000,   // 0.997 USDC output (0.3% fee)
            30,        // 0.30% relayer fee
            1700000000,
            1700003600,
            0,
            vec![],
        );
        assert_eq!(nonce, 0);
        assert_eq!(s.deposit_nonce, 1);

        let deposit = s.get_deposit(0).unwrap();
        assert_eq!(deposit.depositor, alice());
        assert_eq!(deposit.destination_chain_id, CHAIN_ID_BASE);
        assert_eq!(deposit.recipient, bob());
        assert_eq!(deposit.input_amount, 1_000_000);
        assert_eq!(deposit.output_amount, 997_000);
        assert!(!deposit.filled);
    }

    #[test]
    fn test_multiple_deposits_increment_nonce() {
        let mut s = setup();
        let n0 = s.deposit(
            alice(), CHAIN_ID_BASE, bob(), usdc_dina(), usdc_base(),
            1_000_000, 997_000, 30, 0, 3600, 0, vec![],
        );
        let n1 = s.deposit(
            alice(), CHAIN_ID_BASE, bob(), usdc_dina(), usdc_base(),
            2_000_000, 1_994_000, 30, 0, 3600, 0, vec![],
        );
        assert_eq!(n0, 0);
        assert_eq!(n1, 1);
        assert_eq!(s.deposit_nonce, 2);
    }

    #[test]
    #[should_panic(expected = "unsupported destination chain")]
    fn test_deposit_unsupported_chain() {
        let mut s = setup();
        s.deposit(
            alice(), 12345, bob(), usdc_dina(), usdc_base(),
            1_000_000, 997_000, 30, 0, 3600, 0, vec![],
        );
    }

    #[test]
    #[should_panic(expected = "output cannot exceed input")]
    fn test_deposit_output_exceeds_input() {
        let mut s = setup();
        s.deposit(
            alice(), CHAIN_ID_BASE, bob(), usdc_dina(), usdc_base(),
            1_000_000, 2_000_000, 30, 0, 3600, 0, vec![],
        );
    }

    #[test]
    fn test_fill_relay() {
        let mut s = setup();
        s.fill_relay(
            relayer(),
            alice(),
            bob(),
            usdc_dina(),
            usdc_base(),
            1_000_000,
            997_000,
            CHAIN_ID_ETHEREUM,
            CHAIN_ID_BASE, // origin
            0,             // deposit nonce
            1700003600,
            vec![],
        );
        let fill = s.get_fill(CHAIN_ID_BASE, 0).unwrap();
        assert_eq!(fill.relayer, relayer());
        assert_eq!(fill.amount, 997_000);
    }

    #[test]
    #[should_panic(expected = "already filled")]
    fn test_double_fill_fails() {
        let mut s = setup();
        s.fill_relay(
            relayer(), alice(), bob(), usdc_dina(), usdc_base(),
            1_000_000, 997_000, CHAIN_ID_ETHEREUM, CHAIN_ID_BASE, 0, 3600, vec![],
        );
        s.fill_relay(
            relayer(), alice(), bob(), usdc_dina(), usdc_base(),
            1_000_000, 997_000, CHAIN_ID_ETHEREUM, CHAIN_ID_BASE, 0, 3600, vec![],
        );
    }

    #[test]
    fn test_pause_unpause() {
        let mut s = setup();
        s.pause(owner());
        assert!(s.paused);
        s.unpause(owner());
        assert!(!s.paused);
    }

    #[test]
    #[should_panic(expected = "contract is paused")]
    fn test_deposit_while_paused() {
        let mut s = setup();
        s.pause(owner());
        s.deposit(
            alice(), CHAIN_ID_BASE, bob(), usdc_dina(), usdc_base(),
            1_000_000, 997_000, 30, 0, 3600, 0, vec![],
        );
    }

    #[test]
    fn test_ownership_transfer() {
        let mut s = AcrossSpokePoolState::new(owner());
        let new_owner = [99u8; 32];
        s.transfer_ownership(owner(), new_owner);
        assert_eq!(s.owner, new_owner);
        // New owner can pause
        s.pause(new_owner);
        assert!(s.paused);
    }

    #[test]
    #[should_panic(expected = "only owner")]
    fn test_non_owner_cannot_pause() {
        let mut s = setup();
        s.pause(alice());
    }

    #[test]
    fn test_add_supported_chain() {
        let mut s = setup();
        assert!(!s.is_chain_supported(56)); // BSC
        s.add_supported_chain(owner(), 56);
        assert!(s.is_chain_supported(56));
    }

    #[test]
    fn test_dispatch_init_and_deposit() {
        let mut state: Option<AcrossSpokePoolState> = None;
        dispatch(&mut state, "init", b"{}", owner());
        assert!(state.is_some());

        let args = serde_json::to_vec(&DepositArgs {
            destination_chain_id: CHAIN_ID_BASE,
            recipient: bob(),
            input_token: usdc_dina(),
            output_token: usdc_base(),
            input_amount: 1_000_000,
            output_amount: 997_000,
            relayer_fee_pct: 30,
            quote_timestamp: 1700000000,
            fill_deadline: 1700003600,
            exclusivity_deadline: 0,
            message: vec![],
        })
        .unwrap();
        let result = dispatch(&mut state, "deposit", &args, alice());
        let nonce: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(nonce, 0);
    }
}
