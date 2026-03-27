use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Bridged USDC Token — Circle Bridged USDC Standard for Dina Network
// ---------------------------------------------------------------------------
//
// This contract implements the Circle "Bridged USDC Standard" which allows
// new chains to get USDC bridging. When the chain proves itself, Circle can
// upgrade this to native USDC by transferring ownership to Circle's master
// minter contract.
//
// Key properties:
//   - ERC-20 compatible (DRC-1 style on Dina)
//   - Only the designated bridge contract can mint/burn
//   - Owner can set the bridge address (one-time lock for upgrade path)
//   - Circle can later upgrade by becoming the owner and setting their
//     own minter/bridge address
// ---------------------------------------------------------------------------

/// Represents the full on-chain state for the Bridged USDC token.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BridgedUsdcState {
    /// Token name (always "Bridged USDC (Dina)")
    pub name: String,
    /// Token symbol (always "USDC.e")
    pub symbol: String,
    /// Decimal places (always 6, matching USDC on all chains)
    pub decimals: u8,
    /// Total minted supply currently in circulation
    pub total_supply: u64,
    /// Contract owner — can transfer ownership to Circle for native upgrade
    pub owner: [u8; 32],
    /// The bridge contract address authorized to mint and burn
    /// Once set, only the owner can change it (for Circle upgrade path)
    pub bridge_address: Option<[u8; 32]>,
    /// Whether the bridge address has been permanently locked
    /// When true, not even the owner can change the bridge address
    pub bridge_locked: bool,
    /// Account balances: address -> amount
    pub balances: BTreeMap<[u8; 32], u64>,
    /// Spending allowances: (owner, spender) -> amount
    pub allowances: BTreeMap<([u8; 32], [u8; 32]), u64>,
    /// Paused state — owner or Circle can pause in emergencies
    pub paused: bool,
    /// Blacklisted addresses (compliance requirement for USDC)
    pub blacklisted: BTreeMap<[u8; 32], bool>,
}

impl BridgedUsdcState {
    /// Create a new Bridged USDC token with the given owner.
    pub fn new(owner: [u8; 32]) -> Self {
        Self {
            name: "Bridged USDC (Dina)".to_string(),
            symbol: "USDC.e".to_string(),
            decimals: 6,
            total_supply: 0,
            owner,
            bridge_address: None,
            bridge_locked: false,
            balances: BTreeMap::new(),
            allowances: BTreeMap::new(),
            paused: false,
            blacklisted: BTreeMap::new(),
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn decimals(&self) -> u8 {
        self.decimals
    }

    pub fn total_supply(&self) -> u64 {
        self.total_supply
    }

    pub fn balance_of(&self, account: &[u8; 32]) -> u64 {
        self.balances.get(account).copied().unwrap_or(0)
    }

    pub fn allowance(&self, owner: &[u8; 32], spender: &[u8; 32]) -> u64 {
        self.allowances
            .get(&(*owner, *spender))
            .copied()
            .unwrap_or(0)
    }

    pub fn is_blacklisted(&self, account: &[u8; 32]) -> bool {
        self.blacklisted.get(account).copied().unwrap_or(false)
    }

    // -- Owner functions -----------------------------------------------------

    /// Set the bridge address. Can only be called by owner.
    /// Once locked, this cannot be changed.
    pub fn set_bridge_address(&mut self, caller: [u8; 32], bridge: [u8; 32]) {
        assert!(caller == self.owner, "USDC.e: only owner");
        assert!(!self.bridge_locked, "USDC.e: bridge address is locked");
        self.bridge_address = Some(bridge);
    }

    /// Permanently lock the bridge address. Used when Circle upgrades
    /// to native USDC — they set their minter then lock it.
    pub fn lock_bridge_address(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "USDC.e: only owner");
        assert!(self.bridge_address.is_some(), "USDC.e: no bridge set");
        self.bridge_locked = true;
    }

    /// Transfer ownership to a new address. Used for Circle upgrade path:
    /// current owner transfers to Circle's master minter contract.
    pub fn transfer_ownership(&mut self, caller: [u8; 32], new_owner: [u8; 32]) {
        assert!(caller == self.owner, "USDC.e: only owner");
        self.owner = new_owner;
    }

    /// Pause all transfers. Emergency function.
    pub fn pause(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "USDC.e: only owner");
        self.paused = true;
    }

    /// Unpause transfers.
    pub fn unpause(&mut self, caller: [u8; 32]) {
        assert!(caller == self.owner, "USDC.e: only owner");
        self.paused = false;
    }

    /// Add an address to the blacklist (USDC compliance).
    pub fn blacklist(&mut self, caller: [u8; 32], account: [u8; 32]) {
        assert!(caller == self.owner, "USDC.e: only owner");
        self.blacklisted.insert(account, true);
    }

    /// Remove an address from the blacklist.
    pub fn unblacklist(&mut self, caller: [u8; 32], account: [u8; 32]) {
        assert!(caller == self.owner, "USDC.e: only owner");
        self.blacklisted.remove(&account);
    }

    // -- Bridge functions (mint/burn) ----------------------------------------

    /// Mint new USDC.e tokens. Only callable by the bridge contract.
    /// This is called when USDC is locked/burned on the source chain
    /// and needs to be minted on Dina.
    pub fn mint(&mut self, caller: [u8; 32], to: [u8; 32], amount: u64) {
        assert!(!self.paused, "USDC.e: paused");
        let bridge = self.bridge_address.expect("USDC.e: no bridge set");
        assert!(caller == bridge, "USDC.e: only bridge can mint");
        assert!(!self.is_blacklisted(&to), "USDC.e: recipient blacklisted");
        assert!(amount > 0, "USDC.e: mint amount must be positive");

        let balance = self.balance_of(&to);
        self.balances.insert(to, balance + amount);
        self.total_supply += amount;
    }

    /// Burn USDC.e tokens. Callable by the bridge contract or the token
    /// holder themselves (for direct burns / withdrawals).
    pub fn burn(&mut self, caller: [u8; 32], from: [u8; 32], amount: u64) {
        assert!(!self.paused, "USDC.e: paused");
        let bridge = self.bridge_address.expect("USDC.e: no bridge set");
        assert!(
            caller == bridge || caller == from,
            "USDC.e: only bridge or token holder can burn"
        );
        assert!(amount > 0, "USDC.e: burn amount must be positive");

        let balance = self.balance_of(&from);
        assert!(
            balance >= amount,
            "USDC.e: insufficient balance ({balance} < {amount})"
        );
        self.balances.insert(from, balance - amount);
        self.total_supply -= amount;
    }

    // -- ERC-20 / DRC-1 functions --------------------------------------------

    /// Transfer tokens from caller to recipient.
    pub fn transfer(&mut self, caller: [u8; 32], to: [u8; 32], amount: u64) {
        assert!(!self.paused, "USDC.e: paused");
        assert!(
            !self.is_blacklisted(&caller),
            "USDC.e: sender blacklisted"
        );
        assert!(
            !self.is_blacklisted(&to),
            "USDC.e: recipient blacklisted"
        );
        assert!(amount > 0, "USDC.e: transfer amount must be positive");

        let from_balance = self.balance_of(&caller);
        assert!(
            from_balance >= amount,
            "USDC.e: insufficient balance ({from_balance} < {amount})"
        );
        self.balances.insert(caller, from_balance - amount);
        let to_balance = self.balance_of(&to);
        self.balances.insert(to, to_balance + amount);
    }

    /// Approve a spender to spend up to `amount` of the caller's tokens.
    pub fn approve(&mut self, caller: [u8; 32], spender: [u8; 32], amount: u64) {
        assert!(!self.paused, "USDC.e: paused");
        assert!(
            !self.is_blacklisted(&caller),
            "USDC.e: owner blacklisted"
        );
        self.allowances.insert((caller, spender), amount);
    }

    /// Transfer tokens using an allowance (delegated transfer).
    pub fn transfer_from(
        &mut self,
        caller: [u8; 32],
        from: [u8; 32],
        to: [u8; 32],
        amount: u64,
    ) {
        assert!(!self.paused, "USDC.e: paused");
        assert!(
            !self.is_blacklisted(&caller),
            "USDC.e: spender blacklisted"
        );
        assert!(
            !self.is_blacklisted(&from),
            "USDC.e: sender blacklisted"
        );
        assert!(
            !self.is_blacklisted(&to),
            "USDC.e: recipient blacklisted"
        );
        assert!(amount > 0, "USDC.e: transfer amount must be positive");

        let allowed = self.allowance(&from, &caller);
        assert!(
            allowed >= amount,
            "USDC.e: allowance exceeded ({allowed} < {amount})"
        );
        let from_balance = self.balance_of(&from);
        assert!(
            from_balance >= amount,
            "USDC.e: insufficient balance ({from_balance} < {amount})"
        );

        self.allowances.insert((from, caller), allowed - amount);
        self.balances.insert(from, from_balance - amount);
        let to_balance = self.balance_of(&to);
        self.balances.insert(to, to_balance + amount);
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct TransferArgs {
    to: [u8; 32],
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApproveArgs {
    spender: [u8; 32],
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferFromArgs {
    from: [u8; 32],
    to: [u8; 32],
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct MintArgs {
    to: [u8; 32],
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BurnArgs {
    from: [u8; 32],
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BalanceOfArgs {
    account: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct AllowanceArgs {
    owner: [u8; 32],
    spender: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct SetBridgeArgs {
    bridge: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferOwnershipArgs {
    new_owner: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct BlacklistArgs {
    account: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
struct IsBlacklistedArgs {
    account: [u8; 32],
}

// ---------------------------------------------------------------------------
// Contract dispatch
// ---------------------------------------------------------------------------

/// Entry point for the Bridged USDC contract. Routes method calls to the
/// appropriate handler on `BridgedUsdcState`.
pub fn dispatch(
    state: &mut Option<BridgedUsdcState>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "USDC.e: already initialised");
            *state = Some(BridgedUsdcState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "name" => {
            let s = state.as_ref().expect("USDC.e: not initialised");
            serde_json::to_vec(s.name()).unwrap()
        }
        "symbol" => {
            let s = state.as_ref().expect("USDC.e: not initialised");
            serde_json::to_vec(s.symbol()).unwrap()
        }
        "decimals" => {
            let s = state.as_ref().expect("USDC.e: not initialised");
            serde_json::to_vec(&s.decimals()).unwrap()
        }
        "total_supply" => {
            let s = state.as_ref().expect("USDC.e: not initialised");
            serde_json::to_vec(&s.total_supply()).unwrap()
        }
        "balance_of" => {
            let s = state.as_ref().expect("USDC.e: not initialised");
            let a: BalanceOfArgs =
                serde_json::from_slice(args).expect("USDC.e: bad balance_of args");
            serde_json::to_vec(&s.balance_of(&a.account)).unwrap()
        }
        "allowance" => {
            let s = state.as_ref().expect("USDC.e: not initialised");
            let a: AllowanceArgs =
                serde_json::from_slice(args).expect("USDC.e: bad allowance args");
            serde_json::to_vec(&s.allowance(&a.owner, &a.spender)).unwrap()
        }
        "is_blacklisted" => {
            let s = state.as_ref().expect("USDC.e: not initialised");
            let a: IsBlacklistedArgs =
                serde_json::from_slice(args).expect("USDC.e: bad is_blacklisted args");
            serde_json::to_vec(&s.is_blacklisted(&a.account)).unwrap()
        }

        // -- Owner functions -------------------------------------------------
        "set_bridge_address" => {
            let s = state.as_mut().expect("USDC.e: not initialised");
            let a: SetBridgeArgs =
                serde_json::from_slice(args).expect("USDC.e: bad set_bridge args");
            s.set_bridge_address(caller, a.bridge);
            serde_json::to_vec("ok").unwrap()
        }
        "lock_bridge_address" => {
            let s = state.as_mut().expect("USDC.e: not initialised");
            s.lock_bridge_address(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer_ownership" => {
            let s = state.as_mut().expect("USDC.e: not initialised");
            let a: TransferOwnershipArgs =
                serde_json::from_slice(args).expect("USDC.e: bad transfer_ownership args");
            s.transfer_ownership(caller, a.new_owner);
            serde_json::to_vec("ok").unwrap()
        }
        "pause" => {
            let s = state.as_mut().expect("USDC.e: not initialised");
            s.pause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "unpause" => {
            let s = state.as_mut().expect("USDC.e: not initialised");
            s.unpause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "blacklist" => {
            let s = state.as_mut().expect("USDC.e: not initialised");
            let a: BlacklistArgs =
                serde_json::from_slice(args).expect("USDC.e: bad blacklist args");
            s.blacklist(caller, a.account);
            serde_json::to_vec("ok").unwrap()
        }
        "unblacklist" => {
            let s = state.as_mut().expect("USDC.e: not initialised");
            let a: BlacklistArgs =
                serde_json::from_slice(args).expect("USDC.e: bad unblacklist args");
            s.unblacklist(caller, a.account);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Bridge functions ------------------------------------------------
        "mint" => {
            let s = state.as_mut().expect("USDC.e: not initialised");
            let a: MintArgs = serde_json::from_slice(args).expect("USDC.e: bad mint args");
            s.mint(caller, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "burn" => {
            let s = state.as_mut().expect("USDC.e: not initialised");
            let a: BurnArgs = serde_json::from_slice(args).expect("USDC.e: bad burn args");
            s.burn(caller, a.from, a.amount);
            serde_json::to_vec("ok").unwrap()
        }

        // -- ERC-20 / DRC-1 functions ----------------------------------------
        "transfer" => {
            let s = state.as_mut().expect("USDC.e: not initialised");
            let a: TransferArgs =
                serde_json::from_slice(args).expect("USDC.e: bad transfer args");
            s.transfer(caller, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "approve" => {
            let s = state.as_mut().expect("USDC.e: not initialised");
            let a: ApproveArgs =
                serde_json::from_slice(args).expect("USDC.e: bad approve args");
            s.approve(caller, a.spender, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer_from" => {
            let s = state.as_mut().expect("USDC.e: not initialised");
            let a: TransferFromArgs =
                serde_json::from_slice(args).expect("USDC.e: bad transfer_from args");
            s.transfer_from(caller, a.from, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("USDC.e: unknown method '{method}'"),
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
    fn bridge() -> [u8; 32] {
        [2u8; 32]
    }
    fn alice() -> [u8; 32] {
        [3u8; 32]
    }
    fn bob() -> [u8; 32] {
        [4u8; 32]
    }

    fn setup() -> BridgedUsdcState {
        let mut s = BridgedUsdcState::new(owner());
        s.set_bridge_address(owner(), bridge());
        s
    }

    #[test]
    fn test_init_and_metadata() {
        let s = BridgedUsdcState::new(owner());
        assert_eq!(s.name(), "Bridged USDC (Dina)");
        assert_eq!(s.symbol(), "USDC.e");
        assert_eq!(s.decimals(), 6);
        assert_eq!(s.total_supply(), 0);
    }

    #[test]
    fn test_mint_by_bridge() {
        let mut s = setup();
        s.mint(bridge(), alice(), 1_000_000);
        assert_eq!(s.balance_of(&alice()), 1_000_000);
        assert_eq!(s.total_supply(), 1_000_000);
    }

    #[test]
    #[should_panic(expected = "only bridge can mint")]
    fn test_mint_by_non_bridge_fails() {
        let mut s = setup();
        s.mint(alice(), alice(), 1_000_000);
    }

    #[test]
    fn test_burn_by_bridge() {
        let mut s = setup();
        s.mint(bridge(), alice(), 1_000_000);
        s.burn(bridge(), alice(), 500_000);
        assert_eq!(s.balance_of(&alice()), 500_000);
        assert_eq!(s.total_supply(), 500_000);
    }

    #[test]
    fn test_burn_by_holder() {
        let mut s = setup();
        s.mint(bridge(), alice(), 1_000_000);
        s.burn(alice(), alice(), 300_000);
        assert_eq!(s.balance_of(&alice()), 700_000);
    }

    #[test]
    fn test_transfer() {
        let mut s = setup();
        s.mint(bridge(), alice(), 1_000_000);
        s.transfer(alice(), bob(), 400_000);
        assert_eq!(s.balance_of(&alice()), 600_000);
        assert_eq!(s.balance_of(&bob()), 400_000);
    }

    #[test]
    fn test_approve_and_transfer_from() {
        let mut s = setup();
        s.mint(bridge(), alice(), 1_000_000);
        s.approve(alice(), bob(), 500_000);
        assert_eq!(s.allowance(&alice(), &bob()), 500_000);
        s.transfer_from(bob(), alice(), bob(), 300_000);
        assert_eq!(s.balance_of(&alice()), 700_000);
        assert_eq!(s.balance_of(&bob()), 300_000);
        assert_eq!(s.allowance(&alice(), &bob()), 200_000);
    }

    #[test]
    #[should_panic(expected = "paused")]
    fn test_pause_blocks_transfer() {
        let mut s = setup();
        s.mint(bridge(), alice(), 1_000_000);
        s.pause(owner());
        s.transfer(alice(), bob(), 100_000);
    }

    #[test]
    fn test_blacklist() {
        let mut s = setup();
        s.mint(bridge(), alice(), 1_000_000);
        s.blacklist(owner(), alice());
        assert!(s.is_blacklisted(&alice()));
    }

    #[test]
    #[should_panic(expected = "sender blacklisted")]
    fn test_blacklisted_sender_cannot_transfer() {
        let mut s = setup();
        s.mint(bridge(), alice(), 1_000_000);
        s.blacklist(owner(), alice());
        s.transfer(alice(), bob(), 100_000);
    }

    #[test]
    fn test_ownership_transfer() {
        let mut s = BridgedUsdcState::new(owner());
        let new_owner = [99u8; 32];
        s.transfer_ownership(owner(), new_owner);
        assert_eq!(s.owner, new_owner);
        // New owner can set bridge
        s.set_bridge_address(new_owner, bridge());
        assert_eq!(s.bridge_address, Some(bridge()));
    }

    #[test]
    fn test_lock_bridge() {
        let mut s = setup();
        s.lock_bridge_address(owner());
        assert!(s.bridge_locked);
    }

    #[test]
    #[should_panic(expected = "bridge address is locked")]
    fn test_cannot_change_locked_bridge() {
        let mut s = setup();
        s.lock_bridge_address(owner());
        s.set_bridge_address(owner(), [99u8; 32]);
    }

    #[test]
    fn test_dispatch_init_and_mint() {
        let mut state: Option<BridgedUsdcState> = None;
        dispatch(&mut state, "init", b"{}", owner());
        assert!(state.is_some());

        // Set bridge
        let args = serde_json::to_vec(&SetBridgeArgs { bridge: bridge() }).unwrap();
        dispatch(&mut state, "set_bridge_address", &args, owner());

        // Mint via bridge
        let args = serde_json::to_vec(&MintArgs {
            to: alice(),
            amount: 500_000,
        })
        .unwrap();
        dispatch(&mut state, "mint", &args, bridge());

        assert_eq!(state.as_ref().unwrap().balance_of(&alice()), 500_000);
    }
}
