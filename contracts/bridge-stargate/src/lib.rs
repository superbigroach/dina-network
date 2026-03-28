use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Stargate / LayerZero OFT V2 — Unified liquidity pool bridging for Dina Network
// ---------------------------------------------------------------------------
//
// Stargate uses LayerZero for cross-chain messaging combined with unified
// liquidity pools on each chain for instant settlement. LP providers deposit
// tokens into pools and earn fees from cross-chain swaps.
//
// Key properties:
//   - Unified liquidity: single pool per asset across all chains
//   - Instant settlement: liquidity pools on each chain, no lock/mint delay
//   - Speed: 1-3 minutes via LayerZero messaging
//   - LP providers earn swap fees
//   - Delta algorithm balances liquidity across chains
//
// LayerZero endpoint chain IDs:
//   Ethereum = 101, Base = 184, Arbitrum = 110, Optimism = 111, Dina = 299
// ---------------------------------------------------------------------------

/// LayerZero endpoint chain IDs (different from EVM chain IDs).
pub const LZ_CHAIN_ETHEREUM: u16 = 101;
pub const LZ_CHAIN_BASE: u16 = 184;
pub const LZ_CHAIN_ARBITRUM: u16 = 110;
pub const LZ_CHAIN_OPTIMISM: u16 = 111;
pub const LZ_CHAIN_DINA: u16 = 299;

/// Standard pool IDs used by Stargate.
pub const POOL_ID_USDC: u16 = 1;
pub const POOL_ID_USDT: u16 = 2;
pub const POOL_ID_ETH: u16 = 13;

/// A liquidity pool holding tokens for cross-chain swaps.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Pool {
    /// Pool ID (e.g. 1 = USDC)
    pub pool_id: u16,
    /// Token address held by this pool
    pub token: [u8; 32],
    /// Total liquidity deposited in the pool (in token's smallest unit)
    pub total_liquidity: u64,
    /// Total LP tokens minted to liquidity providers
    pub total_lp_tokens: u64,
    /// LP token balances per provider address
    pub lp_balances: BTreeMap<[u8; 32], u64>,
    /// Shared decimals for cross-chain amount normalization
    pub shared_decimals: u8,
    /// Local decimals of the token on this chain
    pub local_decimals: u8,
    /// Human-readable pool name
    pub name: String,
    /// Pool token symbol
    pub symbol: String,
    /// Whether the pool is active and accepting swaps
    pub active: bool,
    /// Accumulated swap fees available for LP distribution
    pub accumulated_fees: u64,
}

/// Parameters for LayerZero transaction gas configuration.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LzTxParams {
    /// Gas limit for the destination chain transaction
    pub dst_gas_for_call: u64,
    /// Native token amount to airdrop on destination chain
    pub dst_native_amount: u64,
    /// Address to receive the native token airdrop
    pub dst_native_addr: Vec<u8>,
}

/// A chain path defines a route between two pools on different chains.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChainPath {
    /// Source pool ID on this chain
    pub src_pool_id: u16,
    /// Destination chain ID (LayerZero chain ID)
    pub dst_chain_id: u16,
    /// Destination pool ID on the remote chain
    pub dst_pool_id: u16,
    /// Credit balance: how much this chain is owed by the remote chain
    pub credit: u64,
    /// Whether this path is active
    pub active: bool,
}

/// Record of a completed or pending cross-chain swap.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SwapRecord {
    /// Source chain ID
    pub src_chain_id: u16,
    /// Destination chain ID
    pub dst_chain_id: u16,
    /// Source pool ID
    pub src_pool_id: u16,
    /// Destination pool ID
    pub dst_pool_id: u16,
    /// Sender address
    pub from: [u8; 32],
    /// Recipient address on destination chain
    pub to: Vec<u8>,
    /// Amount sent (in local decimals)
    pub amount_ld: u64,
    /// Minimum amount to receive on destination (slippage protection)
    pub min_amount_ld: u64,
    /// Fee charged for the swap
    pub fee: u64,
    /// LayerZero nonce for ordering
    pub nonce: u64,
}

/// Full on-chain state for the Stargate Router contract on Dina.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StargateRouterState {
    /// Contract owner
    pub owner: [u8; 32],
    /// This chain's LayerZero chain ID (Dina = 299)
    pub chain_id: u16,
    /// LayerZero endpoint address on this chain
    pub lz_endpoint: [u8; 32],
    /// All pools indexed by pool ID
    pub pools: BTreeMap<u16, Pool>,
    /// Chain paths defining valid cross-chain routes
    pub chain_paths: Vec<ChainPath>,
    /// Outbound nonce per destination chain for ordering
    pub outbound_nonce: BTreeMap<u16, u64>,
    /// Swap history for auditing
    pub swap_records: Vec<SwapRecord>,
    /// Whether the contract is paused
    pub paused: bool,
    /// Swap fee rate in basis points (e.g. 6 = 0.06%)
    pub swap_fee_bps: u64,
    /// Protocol fee portion of swap fee in basis points
    pub protocol_fee_bps: u64,
    /// Accumulated protocol fees per token
    pub protocol_fees: BTreeMap<[u8; 32], u64>,
}

impl StargateRouterState {
    /// Create a new Stargate Router with the given owner.
    pub fn new(owner: [u8; 32]) -> Self {
        Self {
            owner,
            chain_id: LZ_CHAIN_DINA,
            lz_endpoint: [0u8; 32],
            pools: BTreeMap::new(),
            chain_paths: Vec::new(),
            outbound_nonce: BTreeMap::new(),
            swap_records: Vec::new(),
            paused: false,
            swap_fee_bps: 6,     // 0.06% default fee
            protocol_fee_bps: 1, // ~17% of swap fee goes to protocol
            protocol_fees: BTreeMap::new(),
        }
    }

    // -- Queries -------------------------------------------------------------

    /// Get a pool by its ID.
    pub fn get_pool(&self, pool_id: u16) -> Option<&Pool> {
        self.pools.get(&pool_id)
    }

    /// Get all active pool IDs.
    pub fn get_pool_ids(&self) -> Vec<u16> {
        self.pools.keys().copied().collect()
    }

    /// Get the LP balance of a provider in a specific pool.
    pub fn lp_balance_of(&self, pool_id: u16, provider: &[u8; 32]) -> u64 {
        self.pools
            .get(&pool_id)
            .and_then(|p| p.lp_balances.get(provider))
            .copied()
            .unwrap_or(0)
    }

    /// Get the current outbound nonce for a destination chain.
    pub fn get_nonce(&self, dst_chain_id: u16) -> u64 {
        self.outbound_nonce.get(&dst_chain_id).copied().unwrap_or(0)
    }

    // -- Pool management (owner) ---------------------------------------------

    /// Create a new liquidity pool for a token.
    pub fn create_pool(
        &mut self,
        caller: [u8; 32],
        token_address: [u8; 32],
        shared_decimals: u8,
        local_decimals: u8,
        name: String,
        symbol: String,
    ) -> u16 {
        assert!(caller == self.owner, "Stargate: only owner");
        assert!(
            shared_decimals <= local_decimals,
            "Stargate: shared decimals must be <= local decimals"
        );

        let pool_id = (self.pools.len() as u16) + 1;
        assert!(
            !self.pools.contains_key(&pool_id),
            "Stargate: pool ID collision"
        );

        let pool = Pool {
            pool_id,
            token: token_address,
            total_liquidity: 0,
            total_lp_tokens: 0,
            lp_balances: BTreeMap::new(),
            shared_decimals,
            local_decimals,
            name,
            symbol,
            active: true,
            accumulated_fees: 0,
        };

        self.pools.insert(pool_id, pool);
        pool_id
    }

    /// Add a chain path (cross-chain route) between pools.
    pub fn add_chain_path(
        &mut self,
        caller: [u8; 32],
        src_pool_id: u16,
        dst_chain_id: u16,
        dst_pool_id: u16,
    ) {
        assert!(caller == self.owner, "Stargate: only owner");
        assert!(
            self.pools.contains_key(&src_pool_id),
            "Stargate: source pool does not exist"
        );

        // Check for duplicate path
        let exists = self.chain_paths.iter().any(|p| {
            p.src_pool_id == src_pool_id
                && p.dst_chain_id == dst_chain_id
                && p.dst_pool_id == dst_pool_id
        });
        assert!(!exists, "Stargate: chain path already exists");

        self.chain_paths.push(ChainPath {
            src_pool_id,
            dst_chain_id,
            dst_pool_id,
            credit: 0,
            active: true,
        });
    }

    // -- Liquidity -----------------------------------------------------------

    /// Minimum LP tokens burned on first deposit to prevent first-depositor
    /// share-price manipulation (same pattern as DinaDEX).
    const MINIMUM_LIQUIDITY: u64 = 1000;

    /// Add liquidity to a pool. The provider deposits tokens and receives
    /// LP tokens proportional to their share of the pool.
    pub fn add_liquidity(&mut self, caller: [u8; 32], pool_id: u16, amount: u64) -> u64 {
        assert!(!self.paused, "Stargate: contract is paused");
        assert!(amount > 0, "Stargate: amount must be positive");

        let pool = self
            .pools
            .get_mut(&pool_id)
            .expect("Stargate: pool does not exist");
        assert!(pool.active, "Stargate: pool is not active");

        // Calculate LP tokens to mint
        let lp_tokens = if pool.total_liquidity == 0 {
            // First deposit: mint 1:1 but burn MINIMUM_LIQUIDITY to the zero
            // address to prevent first-depositor share-price manipulation.
            assert!(
                amount > Self::MINIMUM_LIQUIDITY,
                "Stargate: first deposit must exceed MINIMUM_LIQUIDITY ({})",
                Self::MINIMUM_LIQUIDITY
            );
            let minted = amount;
            // Burn MINIMUM_LIQUIDITY to zero address (permanently locked)
            let zero_addr = [0u8; 32];
            pool.total_liquidity += amount;
            pool.total_lp_tokens += minted;
            pool.lp_balances.insert(zero_addr, Self::MINIMUM_LIQUIDITY);
            let caller_lp = minted - Self::MINIMUM_LIQUIDITY;
            pool.lp_balances.insert(caller, caller_lp);
            return caller_lp;
        } else {
            // Proportional to existing pool
            (amount as u128 * pool.total_lp_tokens as u128 / pool.total_liquidity as u128) as u64
        };
        assert!(lp_tokens > 0, "Stargate: LP tokens would be zero");

        pool.total_liquidity += amount;
        pool.total_lp_tokens += lp_tokens;

        let current_lp = pool.lp_balances.get(&caller).copied().unwrap_or(0);
        pool.lp_balances.insert(caller, current_lp + lp_tokens);

        lp_tokens
    }

    /// Remove liquidity from a pool. Burns LP tokens and returns the
    /// proportional share of the pool's tokens.
    pub fn remove_liquidity(&mut self, caller: [u8; 32], pool_id: u16, lp_amount: u64) -> u64 {
        assert!(!self.paused, "Stargate: contract is paused");
        assert!(lp_amount > 0, "Stargate: LP amount must be positive");

        let pool = self
            .pools
            .get_mut(&pool_id)
            .expect("Stargate: pool does not exist");

        let caller_lp = pool.lp_balances.get(&caller).copied().unwrap_or(0);
        assert!(
            caller_lp >= lp_amount,
            "Stargate: insufficient LP balance ({caller_lp} < {lp_amount})"
        );

        // Calculate tokens to return (proportional to LP share)
        let token_amount = (lp_amount as u128 * pool.total_liquidity as u128
            / pool.total_lp_tokens as u128) as u64;

        pool.total_liquidity -= token_amount;
        pool.total_lp_tokens -= lp_amount;
        pool.lp_balances.insert(caller, caller_lp - lp_amount);

        // Clean up zero balances
        if caller_lp - lp_amount == 0 {
            pool.lp_balances.remove(&caller);
        }

        token_amount
    }

    // -- Cross-chain swap ----------------------------------------------------

    /// Swap tokens cross-chain via Stargate's liquidity pools.
    ///
    /// Tokens are removed from the source pool on Dina, a LayerZero message
    /// is sent to the destination chain, and the destination pool releases
    /// tokens to the recipient.
    pub fn swap(
        &mut self,
        caller: [u8; 32],
        dst_chain_id: u16,
        src_pool_id: u16,
        dst_pool_id: u16,
        refund_address: [u8; 32],
        amount_ld: u64,
        min_amount_ld: u64,
        lz_tx_params: LzTxParams,
        to: Vec<u8>,
        payload: Vec<u8>,
    ) -> u64 {
        assert!(!self.paused, "Stargate: contract is paused");
        assert!(amount_ld > 0, "Stargate: swap amount must be positive");

        // Verify the chain path exists and is active
        let path_exists = self.chain_paths.iter().any(|p| {
            p.src_pool_id == src_pool_id
                && p.dst_chain_id == dst_chain_id
                && p.dst_pool_id == dst_pool_id
                && p.active
        });
        assert!(path_exists, "Stargate: chain path not found or inactive");

        let pool = self
            .pools
            .get_mut(&src_pool_id)
            .expect("Stargate: source pool does not exist");
        assert!(pool.active, "Stargate: source pool is not active");
        assert!(
            pool.total_liquidity >= amount_ld,
            "Stargate: insufficient pool liquidity"
        );

        // Calculate fee
        let fee = (amount_ld as u128 * self.swap_fee_bps as u128 / 10_000) as u64;
        let amount_after_fee = amount_ld - fee;
        assert!(
            amount_after_fee >= min_amount_ld,
            "Stargate: slippage exceeded (got {amount_after_fee}, min {min_amount_ld})"
        );

        // Deduct from pool liquidity
        pool.total_liquidity -= amount_after_fee;
        pool.accumulated_fees += fee;

        // Protocol fee split
        let protocol_fee =
            (fee as u128 * self.protocol_fee_bps as u128 / self.swap_fee_bps as u128) as u64;
        let current_protocol = self.protocol_fees.get(&pool.token).copied().unwrap_or(0);
        self.protocol_fees
            .insert(pool.token, current_protocol + protocol_fee);

        // Assign nonce for LayerZero message ordering
        let nonce = self.outbound_nonce.get(&dst_chain_id).copied().unwrap_or(0);
        self.outbound_nonce.insert(dst_chain_id, nonce + 1);

        // Suppress unused variable warnings
        let _ = refund_address;
        let _ = lz_tx_params;
        let _ = payload;

        // Record the swap
        self.swap_records.push(SwapRecord {
            src_chain_id: self.chain_id,
            dst_chain_id,
            src_pool_id,
            dst_pool_id,
            from: caller,
            to,
            amount_ld,
            min_amount_ld,
            fee,
            nonce,
        });

        nonce
    }

    // -- LayerZero receive ---------------------------------------------------

    /// Receive a payload from LayerZero (called by the LZ endpoint).
    ///
    /// When a swap is initiated on a remote chain, the LayerZero endpoint
    /// on Dina calls this function to release tokens from the local pool.
    pub fn receive_payload(
        &mut self,
        src_chain_id: u16,
        src_address: Vec<u8>,
        nonce: u64,
        payload: Vec<u8>,
    ) {
        assert!(!self.paused, "Stargate: contract is paused");

        // In production, verify the caller is the LZ endpoint
        // and the src_address is the trusted remote Stargate contract.
        let _ = src_address;

        // Decode the payload to determine pool_id, recipient, and amount.
        // Simplified: in production this would be ABI-decoded from the payload.
        if payload.len() >= 42 {
            let pool_id = u16::from_be_bytes([payload[0], payload[1]]);
            let amount = u64::from_be_bytes([
                payload[2], payload[3], payload[4], payload[5], payload[6], payload[7], payload[8],
                payload[9],
            ]);
            let mut recipient = [0u8; 32];
            recipient.copy_from_slice(&payload[10..42]);

            if let Some(pool) = self.pools.get_mut(&pool_id) {
                pool.total_liquidity += amount;
            }

            // Update credit for the chain path
            for path in &mut self.chain_paths {
                if path.dst_chain_id == src_chain_id && path.src_pool_id == pool_id {
                    path.credit += amount;
                }
            }

            let _ = nonce;
        }
    }

    // -- Owner functions -----------------------------------------------------

    /// Set the LayerZero endpoint address.
    pub fn set_lz_endpoint(&mut self, caller: [u8; 32], endpoint: [u8; 32]) {
        assert!(caller == self.owner, "Stargate: only owner");
        self.lz_endpoint = endpoint;
    }

    /// Set the swap fee rate in basis points.
    pub fn set_swap_fee(&mut self, caller: [u8; 32], fee_bps: u64) {
        assert!(caller == self.owner, "Stargate: only owner");
        assert!(fee_bps <= 1000, "Stargate: fee too high (max 10%)");
        self.swap_fee_bps = fee_bps;
    }

    /// Pause the contract.
    pub fn pause(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "Stargate: only owner");
        self.paused = true;
    }

    /// Unpause the contract.
    pub fn unpause(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "Stargate: only owner");
        self.paused = false;
    }

    /// Transfer ownership to a new address.
    pub fn transfer_ownership(&mut self, caller: [u8; 32], new_owner: [u8; 32]) {
        assert!(caller == self.owner, "Stargate: only owner");
        self.owner = new_owner;
    }

    /// Withdraw accumulated protocol fees.
    pub fn withdraw_protocol_fees(&mut self, caller: [u8; 32], token: [u8; 32]) -> u64 {
        assert!(caller == self.owner, "Stargate: only owner");
        let amount = self.protocol_fees.get(&token).copied().unwrap_or(0);
        self.protocol_fees.insert(token, 0);
        amount
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreatePoolArgs {
    token_address: [u8; 32],
    shared_decimals: u8,
    local_decimals: u8,
    name: String,
    symbol: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddChainPathArgs {
    src_pool_id: u16,
    dst_chain_id: u16,
    dst_pool_id: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct LiquidityArgs {
    pool_id: u16,
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct RemoveLiquidityArgs {
    pool_id: u16,
    lp_amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SwapArgs {
    dst_chain_id: u16,
    src_pool_id: u16,
    dst_pool_id: u16,
    refund_address: [u8; 32],
    amount_ld: u64,
    min_amount_ld: u64,
    lz_tx_params: LzTxParams,
    to: Vec<u8>,
    payload: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReceivePayloadArgs {
    src_chain_id: u16,
    src_address: Vec<u8>,
    nonce: u64,
    payload: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PoolIdArgs {
    pool_id: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct LpBalanceArgs {
    pool_id: u16,
    provider: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct ChainIdArgs {
    chain_id: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct EndpointArgs {
    endpoint: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct FeeArgs {
    fee_bps: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct TokenArgs {
    token: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferOwnershipArgs {
    new_owner: [u8; 32],
}

// ---------------------------------------------------------------------------
// Contract dispatch
// ---------------------------------------------------------------------------

/// Entry point for the Stargate Router contract. Routes method calls to the
/// appropriate handler on `StargateRouterState`.
pub fn dispatch(
    state: &mut Option<StargateRouterState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "Stargate: already initialised");
            *state = Some(StargateRouterState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "get_pool" => {
            let s = state.as_ref().expect("Stargate: not initialised");
            let a: PoolIdArgs = serde_json::from_slice(args).expect("Stargate: bad get_pool args");
            serde_json::to_vec(&s.get_pool(a.pool_id)).unwrap()
        }
        "get_pool_ids" => {
            let s = state.as_ref().expect("Stargate: not initialised");
            serde_json::to_vec(&s.get_pool_ids()).unwrap()
        }
        "lp_balance_of" => {
            let s = state.as_ref().expect("Stargate: not initialised");
            let a: LpBalanceArgs =
                serde_json::from_slice(args).expect("Stargate: bad lp_balance_of args");
            serde_json::to_vec(&s.lp_balance_of(a.pool_id, &a.provider)).unwrap()
        }
        "get_nonce" => {
            let s = state.as_ref().expect("Stargate: not initialised");
            let a: ChainIdArgs =
                serde_json::from_slice(args).expect("Stargate: bad get_nonce args");
            serde_json::to_vec(&s.get_nonce(a.chain_id)).unwrap()
        }
        "chain_id" => {
            let s = state.as_ref().expect("Stargate: not initialised");
            serde_json::to_vec(&s.chain_id).unwrap()
        }

        // -- Pool management -------------------------------------------------
        "create_pool" => {
            let s = state.as_mut().expect("Stargate: not initialised");
            let a: CreatePoolArgs =
                serde_json::from_slice(args).expect("Stargate: bad create_pool args");
            let id = s.create_pool(
                caller,
                a.token_address,
                a.shared_decimals,
                a.local_decimals,
                a.name,
                a.symbol,
            );
            serde_json::to_vec(&id).unwrap()
        }
        "add_chain_path" => {
            let s = state.as_mut().expect("Stargate: not initialised");
            let a: AddChainPathArgs =
                serde_json::from_slice(args).expect("Stargate: bad add_chain_path args");
            s.add_chain_path(caller, a.src_pool_id, a.dst_chain_id, a.dst_pool_id);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Liquidity -------------------------------------------------------
        "add_liquidity" => {
            let s = state.as_mut().expect("Stargate: not initialised");
            let a: LiquidityArgs =
                serde_json::from_slice(args).expect("Stargate: bad add_liquidity args");
            let lp = s.add_liquidity(caller, a.pool_id, a.amount);
            serde_json::to_vec(&lp).unwrap()
        }
        "remove_liquidity" => {
            let s = state.as_mut().expect("Stargate: not initialised");
            let a: RemoveLiquidityArgs =
                serde_json::from_slice(args).expect("Stargate: bad remove_liquidity args");
            let tokens = s.remove_liquidity(caller, a.pool_id, a.lp_amount);
            serde_json::to_vec(&tokens).unwrap()
        }

        // -- Swap ------------------------------------------------------------
        "swap" => {
            let s = state.as_mut().expect("Stargate: not initialised");
            let a: SwapArgs = serde_json::from_slice(args).expect("Stargate: bad swap args");
            let nonce = s.swap(
                caller,
                a.dst_chain_id,
                a.src_pool_id,
                a.dst_pool_id,
                a.refund_address,
                a.amount_ld,
                a.min_amount_ld,
                a.lz_tx_params,
                a.to,
                a.payload,
            );
            serde_json::to_vec(&nonce).unwrap()
        }

        // -- LayerZero receive -----------------------------------------------
        "receive_payload" => {
            let s = state.as_mut().expect("Stargate: not initialised");
            let a: ReceivePayloadArgs =
                serde_json::from_slice(args).expect("Stargate: bad receive_payload args");
            s.receive_payload(a.src_chain_id, a.src_address, a.nonce, a.payload);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Owner functions -------------------------------------------------
        "set_lz_endpoint" => {
            let s = state.as_mut().expect("Stargate: not initialised");
            let a: EndpointArgs =
                serde_json::from_slice(args).expect("Stargate: bad set_lz_endpoint args");
            s.set_lz_endpoint(caller, a.endpoint);
            serde_json::to_vec("ok").unwrap()
        }
        "set_swap_fee" => {
            let s = state.as_mut().expect("Stargate: not initialised");
            let a: FeeArgs = serde_json::from_slice(args).expect("Stargate: bad set_swap_fee args");
            s.set_swap_fee(caller, a.fee_bps);
            serde_json::to_vec("ok").unwrap()
        }
        "pause" => {
            let s = state.as_mut().expect("Stargate: not initialised");
            s.pause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "unpause" => {
            let s = state.as_mut().expect("Stargate: not initialised");
            s.unpause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer_ownership" => {
            let s = state.as_mut().expect("Stargate: not initialised");
            let a: TransferOwnershipArgs =
                serde_json::from_slice(args).expect("Stargate: bad transfer_ownership args");
            s.transfer_ownership(caller, a.new_owner);
            serde_json::to_vec("ok").unwrap()
        }
        "withdraw_protocol_fees" => {
            let s = state.as_mut().expect("Stargate: not initialised");
            let a: TokenArgs =
                serde_json::from_slice(args).expect("Stargate: bad withdraw_protocol_fees args");
            let amount = s.withdraw_protocol_fees(caller, a.token);
            serde_json::to_vec(&amount).unwrap()
        }

        _ => panic!("Stargate: unknown method '{method}'"),
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
    fn alice() -> [u8; 32] {
        [3u8; 32]
    }
    fn bob() -> [u8; 32] {
        [4u8; 32]
    }
    fn usdc_token() -> [u8; 32] {
        [10u8; 32]
    }

    fn setup() -> StargateRouterState {
        let mut s = StargateRouterState::new(owner());
        s.create_pool(owner(), usdc_token(), 6, 6, "USDC".into(), "USDC".into());
        s.add_chain_path(owner(), 1, LZ_CHAIN_BASE, 1);
        s.add_chain_path(owner(), 1, LZ_CHAIN_ETHEREUM, 1);
        s
    }

    #[test]
    fn test_init() {
        let s = StargateRouterState::new(owner());
        assert_eq!(s.chain_id, LZ_CHAIN_DINA);
        assert!(s.pools.is_empty());
        assert!(!s.paused);
    }

    #[test]
    fn test_create_pool() {
        let mut s = StargateRouterState::new(owner());
        let id = s.create_pool(owner(), usdc_token(), 6, 6, "USDC".into(), "USDC".into());
        assert_eq!(id, 1);
        let pool = s.get_pool(1).unwrap();
        assert_eq!(pool.token, usdc_token());
        assert_eq!(pool.total_liquidity, 0);
        assert_eq!(pool.name, "USDC");
    }

    #[test]
    fn test_add_liquidity() {
        let mut s = setup();
        let lp = s.add_liquidity(alice(), 1, 1_000_000);
        // First deposit: 1:1 minus MINIMUM_LIQUIDITY burned to zero address
        assert_eq!(lp, 1_000_000 - StargateRouterState::MINIMUM_LIQUIDITY);

        let pool = s.get_pool(1).unwrap();
        assert_eq!(pool.total_liquidity, 1_000_000);
        assert_eq!(pool.total_lp_tokens, 1_000_000);
        assert_eq!(
            s.lp_balance_of(1, &alice()),
            1_000_000 - StargateRouterState::MINIMUM_LIQUIDITY
        );
        // MINIMUM_LIQUIDITY is locked at the zero address
        assert_eq!(
            s.lp_balance_of(1, &[0u8; 32]),
            StargateRouterState::MINIMUM_LIQUIDITY
        );
    }

    #[test]
    fn test_add_liquidity_proportional() {
        let mut s = setup();
        s.add_liquidity(alice(), 1, 1_000_000);
        let lp = s.add_liquidity(bob(), 1, 500_000);
        assert_eq!(lp, 500_000); // Proportional to pool

        assert_eq!(
            s.lp_balance_of(1, &alice()),
            1_000_000 - StargateRouterState::MINIMUM_LIQUIDITY
        );
        assert_eq!(s.lp_balance_of(1, &bob()), 500_000);
    }

    #[test]
    fn test_remove_liquidity() {
        let mut s = setup();
        let alice_lp = s.add_liquidity(alice(), 1, 1_000_000);
        let remove_amount = alice_lp / 2;
        let tokens = s.remove_liquidity(alice(), 1, remove_amount);
        // tokens returned proportional to share of total pool
        assert!(tokens > 0);

        let pool = s.get_pool(1).unwrap();
        assert_eq!(pool.total_liquidity, 1_000_000 - tokens);
        assert_eq!(s.lp_balance_of(1, &alice()), alice_lp - remove_amount);
    }

    #[test]
    #[should_panic(expected = "insufficient LP balance")]
    fn test_remove_excess_liquidity_fails() {
        let mut s = setup();
        let alice_lp = s.add_liquidity(alice(), 1, 1_000_000);
        s.remove_liquidity(alice(), 1, alice_lp + 1);
    }

    #[test]
    fn test_swap() {
        let mut s = setup();
        s.add_liquidity(alice(), 1, 10_000_000); // 10 USDC in pool

        let nonce = s.swap(
            bob(),
            LZ_CHAIN_BASE,
            1,
            1,
            bob(),
            1_000_000,
            990_000, // accept up to 1% slippage
            LzTxParams {
                dst_gas_for_call: 200_000,
                dst_native_amount: 0,
                dst_native_addr: vec![],
            },
            bob().to_vec(),
            vec![],
        );
        assert_eq!(nonce, 0);

        // Fee: 1_000_000 * 6 / 10_000 = 600
        let pool = s.get_pool(1).unwrap();
        assert_eq!(pool.total_liquidity, 10_000_000 - (1_000_000 - 600));
        assert!(pool.accumulated_fees > 0);
    }

    #[test]
    #[should_panic(expected = "slippage exceeded")]
    fn test_swap_slippage_protection() {
        let mut s = setup();
        s.add_liquidity(alice(), 1, 10_000_000);
        // min_amount_ld too high
        s.swap(
            bob(),
            LZ_CHAIN_BASE,
            1,
            1,
            bob(),
            1_000_000,
            1_000_000, // want full amount, but fee makes it less
            LzTxParams {
                dst_gas_for_call: 0,
                dst_native_amount: 0,
                dst_native_addr: vec![],
            },
            bob().to_vec(),
            vec![],
        );
    }

    #[test]
    #[should_panic(expected = "chain path not found")]
    fn test_swap_invalid_path() {
        let mut s = setup();
        s.add_liquidity(alice(), 1, 10_000_000);
        s.swap(
            bob(),
            999,
            1,
            1,
            bob(), // invalid dst chain
            1_000_000,
            0,
            LzTxParams {
                dst_gas_for_call: 0,
                dst_native_amount: 0,
                dst_native_addr: vec![],
            },
            bob().to_vec(),
            vec![],
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
    fn test_swap_while_paused() {
        let mut s = setup();
        s.add_liquidity(alice(), 1, 10_000_000);
        s.pause(owner());
        s.swap(
            bob(),
            LZ_CHAIN_BASE,
            1,
            1,
            bob(),
            1_000_000,
            0,
            LzTxParams {
                dst_gas_for_call: 0,
                dst_native_amount: 0,
                dst_native_addr: vec![],
            },
            bob().to_vec(),
            vec![],
        );
    }

    #[test]
    fn test_ownership_transfer() {
        let mut s = StargateRouterState::new(owner());
        let new_owner = [99u8; 32];
        s.transfer_ownership(owner(), new_owner);
        assert_eq!(s.owner, new_owner);
    }

    #[test]
    fn test_dispatch_init_and_create_pool() {
        let mut state: Option<StargateRouterState> = None;
        dispatch(&mut state, "init", b"{}", owner());
        assert!(state.is_some());

        let args = serde_json::to_vec(&CreatePoolArgs {
            token_address: usdc_token(),
            shared_decimals: 6,
            local_decimals: 6,
            name: "USDC".into(),
            symbol: "USDC".into(),
        })
        .unwrap();
        let result = dispatch(&mut state, "create_pool", &args, owner());
        let pool_id: u16 = serde_json::from_slice(&result).unwrap();
        assert_eq!(pool_id, 1);
    }
}
