use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DeFi Yield Vault  (ERC-4626 equivalent for Dina Network)
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum VaultStrategy {
    /// Earn yield from a lending pool contract
    LendingPool { pool_address: String },
    /// Earn from validator / transaction-fee rewards
    ValidatorRewards,
    /// Owner manually adds yield (useful for testnet / treasury-backed vaults)
    Manual { apy_bps: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VaultState {
    pub name: String,
    pub symbol: String,
    pub owner: Address,

    // Token accounting
    pub total_assets: u64,
    pub total_shares: u64,
    pub share_balances: BTreeMap<Address, u64>,

    // Yield source
    pub strategy: VaultStrategy,

    // Performance
    pub cumulative_yield: u64,
    pub last_harvest: u64,

    // Limits
    pub deposit_limit: u64,
    pub min_deposit: u64,
    pub paused: bool,
}

impl VaultState {
    pub fn new(
        name: String,
        symbol: String,
        owner: Address,
        strategy: VaultStrategy,
        deposit_limit: u64,
    ) -> Self {
        Self {
            name,
            symbol,
            owner,
            total_assets: 0,
            total_shares: 0,
            share_balances: BTreeMap::new(),
            strategy,
            cumulative_yield: 0,
            last_harvest: 0,
            deposit_limit,
            min_deposit: 1,
            paused: false,
        }
    }

    // -----------------------------------------------------------------------
    // Share conversion helpers
    // -----------------------------------------------------------------------

    /// Convert an asset amount to shares using the current ratio.
    pub fn convert_to_shares(&self, assets: u64) -> u64 {
        if self.total_shares == 0 || self.total_assets == 0 {
            return assets; // 1:1 on first deposit
        }
        (assets as u128 * self.total_shares as u128 / self.total_assets as u128) as u64
    }

    /// Convert a share amount to assets using the current ratio.
    pub fn convert_to_assets(&self, shares: u64) -> u64 {
        if self.total_shares == 0 {
            return 0;
        }
        (shares as u128 * self.total_assets as u128 / self.total_shares as u128) as u64
    }

    /// Preview how many shares a deposit of `amount` would yield.
    pub fn preview_deposit(&self, amount: u64) -> u64 {
        self.convert_to_shares(amount)
    }

    /// Preview how many assets redeeming `shares` would return.
    pub fn preview_withdraw(&self, shares: u64) -> u64 {
        self.convert_to_assets(shares)
    }

    // -----------------------------------------------------------------------
    // Core operations
    // -----------------------------------------------------------------------

    /// Deposit USDC into the vault, receiving proportional shares.
    pub fn deposit(&mut self, caller: Address, amount: u64) -> u64 {
        assert!(!self.paused, "Vault: vault is paused");
        assert!(amount > 0, "Vault: deposit amount must be positive");
        assert!(
            amount >= self.min_deposit,
            "Vault: amount below minimum deposit"
        );
        assert!(
            self.total_assets + amount <= self.deposit_limit,
            "Vault: deposit would exceed limit"
        );

        let shares_minted = self.convert_to_shares(amount);
        assert!(shares_minted > 0, "Vault: zero shares minted");

        self.total_assets += amount;
        self.total_shares += shares_minted;

        let current = self.share_balances.get(&caller).copied().unwrap_or(0);
        self.share_balances.insert(caller, current + shares_minted);

        shares_minted
    }

    /// Withdraw by burning shares, receiving proportional USDC.
    pub fn withdraw(&mut self, caller: Address, shares: u64) -> u64 {
        assert!(!self.paused, "Vault: vault is paused");
        assert!(shares > 0, "Vault: share amount must be positive");

        let user_shares = self.share_balances.get(&caller).copied().unwrap_or(0);
        assert!(
            user_shares >= shares,
            "Vault: insufficient shares ({user_shares} < {shares})"
        );

        let assets_returned = self.convert_to_assets(shares);
        assert!(assets_returned > 0, "Vault: zero assets returned");

        self.share_balances.insert(caller, user_shares - shares);
        self.total_shares -= shares;
        self.total_assets -= assets_returned;

        assets_returned
    }

    /// Harvest yield from strategy — increases total_assets without minting
    /// new shares, so share price goes up.
    pub fn harvest(&mut self, caller: Address, yield_amount: u64, timestamp: u64) {
        assert!(caller == self.owner, "Vault: only owner can harvest");
        assert!(yield_amount > 0, "Vault: yield must be positive");
        self.total_assets += yield_amount;
        self.cumulative_yield += yield_amount;
        self.last_harvest = timestamp;
    }

    /// Add yield manually (for Manual strategy vaults on testnet).
    pub fn add_yield(&mut self, caller: Address, amount: u64) {
        assert!(caller == self.owner, "Vault: only owner can add yield");
        assert!(
            matches!(self.strategy, VaultStrategy::Manual { .. }),
            "Vault: add_yield only for Manual strategy"
        );
        assert!(amount > 0, "Vault: yield amount must be positive");
        self.total_assets += amount;
        self.cumulative_yield += amount;
    }

    // -----------------------------------------------------------------------
    // Read-only queries
    // -----------------------------------------------------------------------

    pub fn get_share_balance(&self, user: &Address) -> u64 {
        self.share_balances.get(user).copied().unwrap_or(0)
    }

    pub fn get_share_value(&self, shares: u64) -> u64 {
        self.convert_to_assets(shares)
    }

    /// Returns vault info as a serialisable struct.
    pub fn get_vault_info(&self) -> VaultInfo {
        VaultInfo {
            name: self.name.clone(),
            symbol: self.symbol.clone(),
            total_assets: self.total_assets,
            total_shares: self.total_shares,
            strategy: self.strategy.clone(),
            cumulative_yield: self.cumulative_yield,
            last_harvest: self.last_harvest,
            deposit_limit: self.deposit_limit,
            min_deposit: self.min_deposit,
            paused: self.paused,
        }
    }

    // -----------------------------------------------------------------------
    // Admin operations
    // -----------------------------------------------------------------------

    pub fn set_deposit_limit(&mut self, caller: Address, limit: u64) {
        assert!(
            caller == self.owner,
            "Vault: only owner can set deposit limit"
        );
        self.deposit_limit = limit;
    }

    pub fn pause(&mut self, caller: Address) {
        assert!(caller == self.owner, "Vault: only owner can pause");
        self.paused = true;
    }

    pub fn unpause(&mut self, caller: Address) {
        assert!(caller == self.owner, "Vault: only owner can unpause");
        self.paused = false;
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VaultInfo {
    pub name: String,
    pub symbol: String,
    pub total_assets: u64,
    pub total_shares: u64,
    pub strategy: VaultStrategy,
    pub cumulative_yield: u64,
    pub last_harvest: u64,
    pub deposit_limit: u64,
    pub min_deposit: u64,
    pub paused: bool,
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateVaultArgs {
    name: String,
    symbol: String,
    strategy: VaultStrategy,
    deposit_limit: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AmountArgs {
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SharesArgs {
    shares: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct HarvestArgs {
    yield_amount: u64,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct LimitArgs {
    limit: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct UserArgs {
    user: Address,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<VaultState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "create_vault" => {
            assert!(state.is_none(), "Vault: already initialised");
            let a: CreateVaultArgs =
                serde_json::from_slice(args).expect("Vault: bad create_vault args");
            *state = Some(VaultState::new(
                a.name,
                a.symbol,
                caller,
                a.strategy,
                a.deposit_limit,
            ));
            serde_json::to_vec("ok").unwrap()
        }

        "deposit" => {
            let s = state.as_mut().expect("Vault: not initialised");
            let a: AmountArgs =
                serde_json::from_slice(args).expect("Vault: bad deposit args");
            let minted = s.deposit(caller, a.amount);
            serde_json::to_vec(&minted).unwrap()
        }

        "withdraw" => {
            let s = state.as_mut().expect("Vault: not initialised");
            let a: SharesArgs =
                serde_json::from_slice(args).expect("Vault: bad withdraw args");
            let assets = s.withdraw(caller, a.shares);
            serde_json::to_vec(&assets).unwrap()
        }

        "harvest" => {
            let s = state.as_mut().expect("Vault: not initialised");
            let a: HarvestArgs =
                serde_json::from_slice(args).expect("Vault: bad harvest args");
            s.harvest(caller, a.yield_amount, a.timestamp);
            serde_json::to_vec("ok").unwrap()
        }

        "add_yield" => {
            let s = state.as_mut().expect("Vault: not initialised");
            let a: AmountArgs =
                serde_json::from_slice(args).expect("Vault: bad add_yield args");
            s.add_yield(caller, a.amount);
            serde_json::to_vec("ok").unwrap()
        }

        "preview_deposit" => {
            let s = state.as_ref().expect("Vault: not initialised");
            let a: AmountArgs =
                serde_json::from_slice(args).expect("Vault: bad preview_deposit args");
            serde_json::to_vec(&s.preview_deposit(a.amount)).unwrap()
        }

        "preview_withdraw" => {
            let s = state.as_ref().expect("Vault: not initialised");
            let a: SharesArgs =
                serde_json::from_slice(args).expect("Vault: bad preview_withdraw args");
            serde_json::to_vec(&s.preview_withdraw(a.shares)).unwrap()
        }

        "get_vault" => {
            let s = state.as_ref().expect("Vault: not initialised");
            serde_json::to_vec(&s.get_vault_info()).unwrap()
        }

        "get_share_balance" => {
            let s = state.as_ref().expect("Vault: not initialised");
            let a: UserArgs =
                serde_json::from_slice(args).expect("Vault: bad get_share_balance args");
            serde_json::to_vec(&s.get_share_balance(&a.user)).unwrap()
        }

        "get_share_value" => {
            let s = state.as_ref().expect("Vault: not initialised");
            let a: SharesArgs =
                serde_json::from_slice(args).expect("Vault: bad get_share_value args");
            serde_json::to_vec(&s.get_share_value(a.shares)).unwrap()
        }

        "set_deposit_limit" => {
            let s = state.as_mut().expect("Vault: not initialised");
            let a: LimitArgs =
                serde_json::from_slice(args).expect("Vault: bad set_deposit_limit args");
            s.set_deposit_limit(caller, a.limit);
            serde_json::to_vec("ok").unwrap()
        }

        "pause" => {
            let s = state.as_mut().expect("Vault: not initialised");
            s.pause(caller);
            serde_json::to_vec("ok").unwrap()
        }

        "unpause" => {
            let s = state.as_mut().expect("Vault: not initialised");
            s.unpause(caller);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("Vault: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(n: u8) -> Address {
        let mut a = [0u8; 32];
        a[0] = n;
        a
    }

    fn make_vault() -> VaultState {
        VaultState::new(
            "USDC Yield Vault".into(),
            "yvUSDC".into(),
            addr(0),
            VaultStrategy::Manual { apy_bps: 500 },
            1_000_000_000, // 1000 USDC (6 decimals)
        )
    }

    #[test]
    fn test_create_vault_and_first_deposit() {
        let mut vault = make_vault();
        let shares = vault.deposit(addr(1), 1_000_000); // 1 USDC
        // First deposit: 1:1 shares
        assert_eq!(shares, 1_000_000);
        assert_eq!(vault.total_assets, 1_000_000);
        assert_eq!(vault.total_shares, 1_000_000);
        assert_eq!(vault.get_share_balance(&addr(1)), 1_000_000);
    }

    #[test]
    fn test_multiple_depositors_proportional_shares() {
        let mut vault = make_vault();
        // Alice deposits 100 USDC
        let alice_shares = vault.deposit(addr(1), 100_000_000);
        assert_eq!(alice_shares, 100_000_000);

        // Bob deposits 50 USDC — should get proportional shares
        let bob_shares = vault.deposit(addr(2), 50_000_000);
        assert_eq!(bob_shares, 50_000_000);

        assert_eq!(vault.total_assets, 150_000_000);
        assert_eq!(vault.total_shares, 150_000_000);
    }

    #[test]
    fn test_harvest_increases_share_value() {
        let mut vault = make_vault();
        vault.deposit(addr(1), 100_000_000); // 100 USDC

        // Harvest 10 USDC yield
        vault.harvest(addr(0), 10_000_000, 1000);

        assert_eq!(vault.total_assets, 110_000_000);
        assert_eq!(vault.total_shares, 100_000_000); // shares unchanged
        assert_eq!(vault.cumulative_yield, 10_000_000);
        assert_eq!(vault.last_harvest, 1000);

        // Each share is now worth 1.1 USDC
        let value = vault.get_share_value(100_000_000);
        assert_eq!(value, 110_000_000);
    }

    #[test]
    fn test_withdraw_after_yield() {
        let mut vault = make_vault();
        vault.deposit(addr(1), 100_000_000); // 100 USDC
        vault.harvest(addr(0), 10_000_000, 1000); // +10 USDC yield

        // Withdraw all shares — should get 110 USDC
        let assets = vault.withdraw(addr(1), 100_000_000);
        assert_eq!(assets, 110_000_000);
        assert_eq!(vault.total_assets, 0);
        assert_eq!(vault.total_shares, 0);
    }

    #[test]
    fn test_preview_matches_actual() {
        let mut vault = make_vault();
        vault.deposit(addr(1), 100_000_000);
        vault.harvest(addr(0), 10_000_000, 1000);

        // Preview deposit
        let preview_shares = vault.preview_deposit(50_000_000);
        let actual_shares = vault.deposit(addr(2), 50_000_000);
        assert_eq!(preview_shares, actual_shares);

        // Preview withdraw
        let preview_assets = vault.preview_withdraw(actual_shares);
        let actual_assets = vault.withdraw(addr(2), actual_shares);
        assert_eq!(preview_assets, actual_assets);
    }

    #[test]
    #[should_panic(expected = "deposit would exceed limit")]
    fn test_deposit_limit_enforced() {
        let mut vault = make_vault(); // limit = 1_000_000_000
        vault.deposit(addr(1), 1_000_000_001); // exceeds limit
    }

    #[test]
    #[should_panic(expected = "vault is paused")]
    fn test_pause_prevents_deposit() {
        let mut vault = make_vault();
        vault.pause(addr(0));
        vault.deposit(addr(1), 1_000_000);
    }

    #[test]
    #[should_panic(expected = "vault is paused")]
    fn test_pause_prevents_withdraw() {
        let mut vault = make_vault();
        vault.deposit(addr(1), 1_000_000);
        vault.pause(addr(0));
        vault.withdraw(addr(1), 1_000_000);
    }

    #[test]
    fn test_manual_add_yield() {
        let mut vault = make_vault();
        vault.deposit(addr(1), 100_000_000);
        vault.add_yield(addr(0), 5_000_000);
        assert_eq!(vault.total_assets, 105_000_000);
        assert_eq!(vault.cumulative_yield, 5_000_000);
    }

    #[test]
    fn test_share_price_increases_with_yield() {
        let mut vault = make_vault();

        // Alice deposits 100 USDC
        vault.deposit(addr(1), 100_000_000);
        let value_before = vault.get_share_value(100_000_000);
        assert_eq!(value_before, 100_000_000);

        // Add 20 USDC yield
        vault.add_yield(addr(0), 20_000_000);

        let value_after = vault.get_share_value(100_000_000);
        assert_eq!(value_after, 120_000_000);

        // Bob deposits 120 USDC — gets 100M shares (same as Alice)
        let bob_shares = vault.deposit(addr(2), 120_000_000);
        assert_eq!(bob_shares, 100_000_000);
    }

    #[test]
    fn test_unpause_restores_operations() {
        let mut vault = make_vault();
        vault.pause(addr(0));
        vault.unpause(addr(0));
        let shares = vault.deposit(addr(1), 1_000_000);
        assert_eq!(shares, 1_000_000);
    }

    #[test]
    #[should_panic(expected = "only owner")]
    fn test_non_owner_cannot_pause() {
        let mut vault = make_vault();
        vault.pause(addr(99));
    }

    #[test]
    fn test_set_deposit_limit() {
        let mut vault = make_vault();
        vault.set_deposit_limit(addr(0), 500_000);
        assert_eq!(vault.deposit_limit, 500_000);
    }

    #[test]
    fn test_dispatch_create_and_deposit() {
        let mut state: Option<VaultState> = None;
        let owner = addr(0);

        let create_args = serde_json::to_vec(&CreateVaultArgs {
            name: "Test Vault".into(),
            symbol: "tvUSDC".into(),
            strategy: VaultStrategy::Manual { apy_bps: 500 },
            deposit_limit: 1_000_000_000,
        })
        .unwrap();
        dispatch(&mut state, "create_vault", &create_args, owner);
        assert!(state.is_some());

        let deposit_args = serde_json::to_vec(&AmountArgs { amount: 1_000_000 }).unwrap();
        let result = dispatch(&mut state, "deposit", &deposit_args, addr(1));
        let shares: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(shares, 1_000_000);
    }

    #[test]
    fn test_get_vault_info() {
        let mut state: Option<VaultState> = None;
        let owner = addr(0);

        let create_args = serde_json::to_vec(&CreateVaultArgs {
            name: "Info Vault".into(),
            symbol: "ivUSDC".into(),
            strategy: VaultStrategy::ValidatorRewards,
            deposit_limit: 500_000_000,
        })
        .unwrap();
        dispatch(&mut state, "create_vault", &create_args, owner);

        let result = dispatch(&mut state, "get_vault", &[], owner);
        let info: VaultInfo = serde_json::from_slice(&result).unwrap();
        assert_eq!(info.name, "Info Vault");
        assert_eq!(info.symbol, "ivUSDC");
        assert_eq!(info.deposit_limit, 500_000_000);
        assert!(!info.paused);
    }
}
