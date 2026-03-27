use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Axelar Integration — ITS (Interchain Token Service) for Dina Network
// ---------------------------------------------------------------------------
//
// Implements an Axelar-compatible bridge for Dina Network using the
// Interchain Token Service (ITS) pattern. Axelar provides general-purpose
// cross-chain messaging through a decentralized validator network and
// the Axelar Gateway contract.
//
// Axelar has two main bridging patterns:
//   1. Gateway + GMP (General Message Passing) — arbitrary cross-chain calls
//   2. ITS (Interchain Token Service) — standardized token bridging
//
// This contract implements both patterns:
//   - send_to_chain() for token transfers (ITS style)
//   - execute() for receiving general messages from Axelar
//   - execute_with_token() for receiving messages that include tokens
//
// Key concepts:
//   - Gateway: the Axelar gateway contract that validates cross-chain messages
//   - Command ID: unique identifier for each cross-chain command
//   - Chains are identified by string names (e.g., "ethereum", "base")
// ---------------------------------------------------------------------------

/// An outbound cross-chain token transfer.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OutboundTransfer {
    /// Destination chain name
    pub destination_chain: String,
    /// Destination address (as a string, since different chains have
    /// different address formats)
    pub destination_address: String,
    /// Amount of tokens transferred
    pub amount: u64,
    /// Sender on Dina
    pub sender: [u8; 32],
    /// Token symbol
    pub token_symbol: String,
}

/// A processed inbound command.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProcessedCommand {
    /// The command ID
    pub command_id: [u8; 32],
    /// Source chain
    pub source_chain: String,
    /// Source address
    pub source_address: String,
    /// Payload hash
    pub payload_hash: [u8; 32],
    /// Amount (if token transfer)
    pub amount: Option<u64>,
}

/// Full on-chain state for the Axelar bridge contract on Dina.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AxelarState {
    /// Contract owner
    pub owner: [u8; 32],
    /// The Axelar gateway address (validates cross-chain messages)
    pub gateway_address: [u8; 32],
    /// The ITS (Interchain Token Service) address
    pub its_address: [u8; 32],
    /// The bridged USDC token address on Dina
    pub usdc_token: [u8; 32],
    /// Processed command IDs (prevents replay)
    pub processed_commands: BTreeMap<[u8; 32], bool>,
    /// Processed command details (for auditing)
    pub command_log: Vec<ProcessedCommand>,
    /// Outbound transfer log
    pub outbound_log: Vec<OutboundTransfer>,
    /// Trusted source chains and their contract addresses
    /// Maps chain_name -> trusted contract address (as string)
    pub trusted_sources: BTreeMap<String, String>,
    /// Supported token symbols and their Dina token addresses
    pub token_registry: BTreeMap<String, [u8; 32]>,
    /// Whether the bridge is paused
    pub paused: bool,
    /// Chain name for this deployment
    pub chain_name: String,
}

impl AxelarState {
    /// Initialize a new Axelar bridge contract.
    pub fn new(
        owner: [u8; 32],
        gateway_address: [u8; 32],
        its_address: [u8; 32],
        usdc_token: [u8; 32],
    ) -> Self {
        let mut token_registry = BTreeMap::new();
        token_registry.insert("USDC".to_string(), usdc_token);

        Self {
            owner,
            gateway_address,
            its_address,
            usdc_token,
            processed_commands: BTreeMap::new(),
            command_log: Vec::new(),
            outbound_log: Vec::new(),
            trusted_sources: BTreeMap::new(),
            token_registry,
            paused: false,
            chain_name: "dina".to_string(),
        }
    }

    // -- Admin ---------------------------------------------------------------

    /// Set a trusted source for a given chain. Only messages from trusted
    /// sources are accepted.
    pub fn set_trusted_source(
        &mut self,
        caller: [u8; 32],
        chain_name: String,
        source_address: String,
    ) {
        assert!(caller == self.owner, "Axelar: only owner");
        self.trusted_sources.insert(chain_name, source_address);
    }

    /// Register a token symbol with its Dina contract address.
    pub fn register_token(
        &mut self,
        caller: [u8; 32],
        symbol: String,
        token_address: [u8; 32],
    ) {
        assert!(caller == self.owner, "Axelar: only owner");
        self.token_registry.insert(symbol, token_address);
    }

    /// Update the gateway address. Only callable by owner.
    pub fn set_gateway(
        &mut self,
        caller: [u8; 32],
        new_gateway: [u8; 32],
    ) {
        assert!(caller == self.owner, "Axelar: only owner");
        self.gateway_address = new_gateway;
    }

    /// Update the ITS address. Only callable by owner.
    pub fn set_its(
        &mut self,
        caller: [u8; 32],
        new_its: [u8; 32],
    ) {
        assert!(caller == self.owner, "Axelar: only owner");
        self.its_address = new_its;
    }

    /// Pause the bridge. Only callable by owner.
    pub fn pause(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "Axelar: only owner");
        self.paused = true;
    }

    /// Unpause. Only callable by owner.
    pub fn unpause(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "Axelar: only owner");
        self.paused = false;
    }

    // -- ITS: Send tokens ----------------------------------------------------

    /// Send tokens from Dina to another chain via Axelar ITS.
    ///
    /// Burns the tokens on Dina and emits a cross-chain transfer request
    /// that the Axelar network will relay to the destination chain.
    pub fn send_to_chain(
        &mut self,
        caller: [u8; 32],
        destination_chain: String,
        destination_address: String,
        amount: u64,
    ) {
        assert!(!self.paused, "Axelar: paused");
        assert!(amount > 0, "Axelar: amount must be positive");
        assert!(
            self.trusted_sources.contains_key(&destination_chain),
            "Axelar: untrusted destination chain"
        );

        self.outbound_log.push(OutboundTransfer {
            destination_chain,
            destination_address,
            amount,
            sender: caller,
            token_symbol: "USDC".to_string(),
        });

        // In production: burn tokens from caller via USDC.e token contract,
        // then call gateway.call_contract() to initiate the cross-chain message
    }

    // -- GMP: Receive messages -----------------------------------------------

    /// Execute a cross-chain message from Axelar (without tokens).
    ///
    /// This is called by the Axelar gateway after the validator network
    /// has confirmed the message. The command_id is unique and prevents
    /// replay attacks.
    pub fn execute(
        &mut self,
        caller: [u8; 32],
        command_id: [u8; 32],
        source_chain: String,
        source_address: String,
        payload: Vec<u8>,
    ) {
        assert!(!self.paused, "Axelar: paused");
        assert!(
            caller == self.gateway_address,
            "Axelar: only gateway can execute"
        );
        assert!(
            !self.processed_commands.contains_key(&command_id),
            "Axelar: command already processed"
        );

        // Verify the source is trusted
        let trusted = self.trusted_sources.get(&source_chain);
        assert!(
            trusted.is_some() && *trusted.unwrap() == source_address,
            "Axelar: untrusted source"
        );

        // Compute payload hash for logging
        let mut payload_hash = [0u8; 32];
        if payload.len() >= 32 {
            payload_hash.copy_from_slice(&payload[..32]);
        } else {
            payload_hash[..payload.len()].copy_from_slice(&payload);
        }

        // Mark command as processed
        self.processed_commands.insert(command_id, true);
        self.command_log.push(ProcessedCommand {
            command_id,
            source_chain,
            source_address,
            payload_hash,
            amount: None,
        });

        // In production: decode and execute the payload
        // (e.g., mint tokens, update state, trigger other contracts)
    }

    /// Execute a cross-chain message that includes tokens.
    ///
    /// This is the most common Axelar receive pattern — the gateway has
    /// already validated the message and released the tokens to this
    /// contract. We just need to process the payload and forward tokens.
    pub fn execute_with_token(
        &mut self,
        caller: [u8; 32],
        command_id: [u8; 32],
        source_chain: String,
        source_address: String,
        payload: Vec<u8>,
        token_symbol: String,
        amount: u64,
    ) {
        assert!(!self.paused, "Axelar: paused");
        assert!(
            caller == self.gateway_address,
            "Axelar: only gateway can execute"
        );
        assert!(
            !self.processed_commands.contains_key(&command_id),
            "Axelar: command already processed"
        );

        // Verify the source is trusted
        let trusted = self.trusted_sources.get(&source_chain);
        assert!(
            trusted.is_some() && *trusted.unwrap() == source_address,
            "Axelar: untrusted source"
        );

        // Verify the token is registered
        assert!(
            self.token_registry.contains_key(&token_symbol),
            "Axelar: unregistered token {token_symbol}"
        );

        // Compute payload hash
        let mut payload_hash = [0u8; 32];
        if payload.len() >= 32 {
            payload_hash.copy_from_slice(&payload[..32]);
        } else {
            payload_hash[..payload.len()].copy_from_slice(&payload);
        }

        // Mark command as processed
        self.processed_commands.insert(command_id, true);
        self.command_log.push(ProcessedCommand {
            command_id,
            source_chain,
            source_address,
            payload_hash,
            amount: Some(amount),
        });

        // In production: decode payload to get recipient address,
        // then mint/transfer tokens via the registered token contract
    }

    // -- Queries -------------------------------------------------------------

    /// Check if a command has been processed.
    pub fn is_command_processed(&self, command_id: &[u8; 32]) -> bool {
        self.processed_commands
            .get(command_id)
            .copied()
            .unwrap_or(false)
    }

    /// Get the number of outbound transfers.
    pub fn outbound_count(&self) -> usize {
        self.outbound_log.len()
    }

    /// Get the number of processed commands.
    pub fn processed_command_count(&self) -> usize {
        self.command_log.len()
    }

    /// Check if a token symbol is registered.
    pub fn is_token_registered(&self, symbol: &str) -> bool {
        self.token_registry.contains_key(symbol)
    }

    /// Check if a chain is trusted.
    pub fn is_chain_trusted(&self, chain_name: &str) -> bool {
        self.trusted_sources.contains_key(chain_name)
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    gateway_address: [u8; 32],
    its_address: [u8; 32],
    usdc_token: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct SetTrustedSourceArgs {
    chain_name: String,
    source_address: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RegisterTokenArgs {
    symbol: String,
    token_address: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct SetGatewayArgs {
    new_gateway: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct SetItsArgs {
    new_its: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct SendToChainArgs {
    destination_chain: String,
    destination_address: String,
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteArgs {
    command_id: [u8; 32],
    source_chain: String,
    source_address: String,
    payload: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExecuteWithTokenArgs {
    command_id: [u8; 32],
    source_chain: String,
    source_address: String,
    payload: Vec<u8>,
    token_symbol: String,
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct IsCommandProcessedArgs {
    command_id: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct IsTokenRegisteredArgs {
    symbol: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct IsChainTrustedArgs {
    chain_name: String,
}

// ---------------------------------------------------------------------------
// Contract dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<AxelarState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "Axelar: already initialised");
            let a: InitArgs =
                serde_json::from_slice(args).expect("Axelar: bad init args");
            *state = Some(AxelarState::new(
                caller,
                a.gateway_address,
                a.its_address,
                a.usdc_token,
            ));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Admin -----------------------------------------------------------
        "set_trusted_source" => {
            let s = state.as_mut().expect("Axelar: not initialised");
            let a: SetTrustedSourceArgs =
                serde_json::from_slice(args).expect("Axelar: bad args");
            s.set_trusted_source(caller, a.chain_name, a.source_address);
            serde_json::to_vec("ok").unwrap()
        }
        "register_token" => {
            let s = state.as_mut().expect("Axelar: not initialised");
            let a: RegisterTokenArgs =
                serde_json::from_slice(args).expect("Axelar: bad args");
            s.register_token(caller, a.symbol, a.token_address);
            serde_json::to_vec("ok").unwrap()
        }
        "set_gateway" => {
            let s = state.as_mut().expect("Axelar: not initialised");
            let a: SetGatewayArgs =
                serde_json::from_slice(args).expect("Axelar: bad args");
            s.set_gateway(caller, a.new_gateway);
            serde_json::to_vec("ok").unwrap()
        }
        "set_its" => {
            let s = state.as_mut().expect("Axelar: not initialised");
            let a: SetItsArgs =
                serde_json::from_slice(args).expect("Axelar: bad args");
            s.set_its(caller, a.new_its);
            serde_json::to_vec("ok").unwrap()
        }
        "pause" => {
            let s = state.as_mut().expect("Axelar: not initialised");
            s.pause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "unpause" => {
            let s = state.as_mut().expect("Axelar: not initialised");
            s.unpause(caller);
            serde_json::to_vec("ok").unwrap()
        }

        // -- ITS: Token transfers --------------------------------------------
        "send_to_chain" => {
            let s = state.as_mut().expect("Axelar: not initialised");
            let a: SendToChainArgs =
                serde_json::from_slice(args).expect("Axelar: bad args");
            s.send_to_chain(caller, a.destination_chain, a.destination_address, a.amount);
            serde_json::to_vec("ok").unwrap()
        }

        // -- GMP: Receive messages -------------------------------------------
        "execute" => {
            let s = state.as_mut().expect("Axelar: not initialised");
            let a: ExecuteArgs =
                serde_json::from_slice(args).expect("Axelar: bad args");
            s.execute(caller, a.command_id, a.source_chain, a.source_address, a.payload);
            serde_json::to_vec("ok").unwrap()
        }
        "execute_with_token" => {
            let s = state.as_mut().expect("Axelar: not initialised");
            let a: ExecuteWithTokenArgs =
                serde_json::from_slice(args).expect("Axelar: bad args");
            s.execute_with_token(
                caller,
                a.command_id,
                a.source_chain,
                a.source_address,
                a.payload,
                a.token_symbol,
                a.amount,
            );
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "is_command_processed" => {
            let s = state.as_ref().expect("Axelar: not initialised");
            let a: IsCommandProcessedArgs =
                serde_json::from_slice(args).expect("Axelar: bad args");
            serde_json::to_vec(&s.is_command_processed(&a.command_id)).unwrap()
        }
        "outbound_count" => {
            let s = state.as_ref().expect("Axelar: not initialised");
            serde_json::to_vec(&s.outbound_count()).unwrap()
        }
        "processed_command_count" => {
            let s = state.as_ref().expect("Axelar: not initialised");
            serde_json::to_vec(&s.processed_command_count()).unwrap()
        }
        "is_token_registered" => {
            let s = state.as_ref().expect("Axelar: not initialised");
            let a: IsTokenRegisteredArgs =
                serde_json::from_slice(args).expect("Axelar: bad args");
            serde_json::to_vec(&s.is_token_registered(&a.symbol)).unwrap()
        }
        "is_chain_trusted" => {
            let s = state.as_ref().expect("Axelar: not initialised");
            let a: IsChainTrustedArgs =
                serde_json::from_slice(args).expect("Axelar: bad args");
            serde_json::to_vec(&s.is_chain_trusted(&a.chain_name)).unwrap()
        }

        _ => panic!("Axelar: unknown method '{method}'"),
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
    fn gateway() -> [u8; 32] {
        [10u8; 32]
    }
    fn its() -> [u8; 32] {
        [11u8; 32]
    }
    fn usdc() -> [u8; 32] {
        [20u8; 32]
    }
    fn alice() -> [u8; 32] {
        [3u8; 32]
    }

    fn setup() -> AxelarState {
        let mut s = AxelarState::new(owner(), gateway(), its(), usdc());
        s.set_trusted_source(owner(), "ethereum".to_string(), "0xABC".to_string());
        s.set_trusted_source(owner(), "base".to_string(), "0xDEF".to_string());
        s
    }

    #[test]
    fn test_init() {
        let s = setup();
        assert_eq!(s.chain_name, "dina");
        assert!(s.is_chain_trusted("ethereum"));
        assert!(s.is_chain_trusted("base"));
        assert!(s.is_token_registered("USDC"));
    }

    #[test]
    fn test_send_to_chain() {
        let mut s = setup();
        s.send_to_chain(
            alice(),
            "ethereum".to_string(),
            "0x1234".to_string(),
            1_000_000,
        );
        assert_eq!(s.outbound_count(), 1);
    }

    #[test]
    #[should_panic(expected = "untrusted destination")]
    fn test_send_to_untrusted_fails() {
        let mut s = setup();
        s.send_to_chain(
            alice(),
            "solana".to_string(),
            "abc".to_string(),
            1_000_000,
        );
    }

    #[test]
    fn test_execute() {
        let mut s = setup();
        let cmd_id = [42u8; 32];
        s.execute(
            gateway(),
            cmd_id,
            "ethereum".to_string(),
            "0xABC".to_string(),
            vec![1, 2, 3, 4],
        );
        assert!(s.is_command_processed(&cmd_id));
        assert_eq!(s.processed_command_count(), 1);
    }

    #[test]
    #[should_panic(expected = "only gateway")]
    fn test_execute_not_gateway_fails() {
        let mut s = setup();
        s.execute(
            alice(),
            [42u8; 32],
            "ethereum".to_string(),
            "0xABC".to_string(),
            vec![],
        );
    }

    #[test]
    #[should_panic(expected = "command already processed")]
    fn test_replay_prevention() {
        let mut s = setup();
        let cmd_id = [42u8; 32];
        s.execute(
            gateway(),
            cmd_id,
            "ethereum".to_string(),
            "0xABC".to_string(),
            vec![],
        );
        s.execute(
            gateway(),
            cmd_id,
            "ethereum".to_string(),
            "0xABC".to_string(),
            vec![],
        );
    }

    #[test]
    fn test_execute_with_token() {
        let mut s = setup();
        let cmd_id = [43u8; 32];
        s.execute_with_token(
            gateway(),
            cmd_id,
            "base".to_string(),
            "0xDEF".to_string(),
            vec![5, 6, 7],
            "USDC".to_string(),
            2_000_000,
        );
        assert!(s.is_command_processed(&cmd_id));
    }

    #[test]
    #[should_panic(expected = "unregistered token")]
    fn test_execute_with_unknown_token_fails() {
        let mut s = setup();
        s.execute_with_token(
            gateway(),
            [44u8; 32],
            "ethereum".to_string(),
            "0xABC".to_string(),
            vec![],
            "WBTC".to_string(),
            100,
        );
    }

    #[test]
    #[should_panic(expected = "untrusted source")]
    fn test_execute_from_untrusted_source_fails() {
        let mut s = setup();
        s.execute(
            gateway(),
            [45u8; 32],
            "ethereum".to_string(),
            "0xUNKNOWN".to_string(),
            vec![],
        );
    }

    #[test]
    fn test_register_token() {
        let mut s = setup();
        s.register_token(owner(), "WETH".to_string(), [99u8; 32]);
        assert!(s.is_token_registered("WETH"));
    }

    #[test]
    #[should_panic(expected = "paused")]
    fn test_paused_blocks_send() {
        let mut s = setup();
        s.pause(owner());
        s.send_to_chain(
            alice(),
            "ethereum".to_string(),
            "0x1234".to_string(),
            1_000_000,
        );
    }
}
