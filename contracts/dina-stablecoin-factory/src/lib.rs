use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Dina Stablecoin Factory — DRC-1 compatible yield-bearing stablecoins
// ---------------------------------------------------------------------------

type Address = [u8; 32];

const SECONDS_PER_YEAR: u64 = 31_536_000;
const BPS_DENOMINATOR: u64 = 10_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StablecoinState {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub admin: Address,
    pub total_supply: u64,
    pub balances: BTreeMap<Address, u64>,
    pub allowances: BTreeMap<(Address, Address), u64>,
    /// Total USDC backing this stablecoin (in micro-USDC, 6 decimals)
    pub usdc_backing: u64,
    /// Exchange rate: micro-units of this currency per 1 USDC (6 decimals)
    /// e.g., EUR at 0.93 = 930_000
    pub rate_per_usdc: u64,
    /// Yield rate in basis points (from USDC yield, passed through)
    pub yield_rate_bps: u64,
    /// Per-account yield tracking: last update timestamp
    pub yield_last_update: BTreeMap<Address, u64>,
    pub paused: bool,
}

impl StablecoinState {
    pub fn new(
        name: String,
        symbol: String,
        decimals: u8,
        admin: Address,
        rate_per_usdc: u64,
        yield_rate_bps: u64,
    ) -> Self {
        Self {
            name,
            symbol,
            decimals,
            admin,
            total_supply: 0,
            balances: BTreeMap::new(),
            allowances: BTreeMap::new(),
            usdc_backing: 0,
            rate_per_usdc,
            yield_rate_bps,
            yield_last_update: BTreeMap::new(),
            paused: false,
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

    pub fn balance_of(&self, addr: &Address) -> u64 {
        *self.balances.get(addr).unwrap_or(&0)
    }

    pub fn allowance(&self, owner: &Address, spender: &Address) -> u64 {
        self.allowances
            .get(&(*owner, *spender))
            .copied()
            .unwrap_or(0)
    }

    // -- Yield ---------------------------------------------------------------

    fn settle_yield(&mut self, addr: &Address, current_time: u64) {
        let balance = self.balance_of(addr);
        if balance == 0 {
            return;
        }
        let last = *self.yield_last_update.get(addr).unwrap_or(&current_time);
        if current_time <= last {
            return;
        }
        let elapsed = current_time - last;
        // accrued = balance * yield_rate_bps * elapsed / (BPS_DENOMINATOR * SECONDS_PER_YEAR)
        let accrued = (balance as u128)
            .checked_mul(self.yield_rate_bps as u128)
            .unwrap_or(0)
            .checked_mul(elapsed as u128)
            .unwrap_or(0)
            / (BPS_DENOMINATOR as u128 * SECONDS_PER_YEAR as u128);
        let accrued = accrued as u64;
        if accrued > 0 {
            let new_bal = balance.saturating_add(accrued);
            self.balances.insert(*addr, new_bal);
            self.total_supply = self.total_supply.saturating_add(accrued);
        }
        self.yield_last_update.insert(*addr, current_time);
    }

    // -- Mutations -----------------------------------------------------------

    /// Mint new tokens (admin only). Called when USDC is deposited as backing.
    pub fn mint(
        &mut self,
        caller: Address,
        to: Address,
        amount: u64,
        usdc_backing_amount: u64,
        current_time: u64,
    ) {
        assert!(caller == self.admin, "only admin can mint");
        assert!(!self.paused, "contract paused");
        assert!(amount > 0, "mint amount must be positive");
        self.settle_yield(&to, current_time);
        let bal = self.balance_of(&to);
        self.balances
            .insert(to, bal.checked_add(amount).expect("balance overflow"));
        self.total_supply = self
            .total_supply
            .checked_add(amount)
            .expect("supply overflow");
        self.usdc_backing = self
            .usdc_backing
            .checked_add(usdc_backing_amount)
            .expect("backing overflow");
        self.yield_last_update.entry(to).or_insert(current_time);
    }

    /// Burn tokens (admin only). Called when redeeming back to USDC.
    pub fn burn(
        &mut self,
        caller: Address,
        from: Address,
        amount: u64,
        usdc_backing_amount: u64,
        current_time: u64,
    ) {
        assert!(caller == self.admin, "only admin can burn");
        assert!(amount > 0, "burn amount must be positive");
        self.settle_yield(&from, current_time);
        let bal = self.balance_of(&from);
        assert!(bal >= amount, "insufficient balance to burn");
        self.balances.insert(from, bal - amount);
        self.total_supply = self
            .total_supply
            .checked_sub(amount)
            .expect("supply underflow");
        self.usdc_backing = self.usdc_backing.saturating_sub(usdc_backing_amount);
    }

    pub fn transfer(&mut self, caller: Address, to: Address, amount: u64, current_time: u64) {
        assert!(!self.paused, "contract paused");
        assert!(amount > 0, "transfer amount must be positive");
        assert!(caller != to, "cannot transfer to self");
        self.settle_yield(&caller, current_time);
        self.settle_yield(&to, current_time);
        let from_bal = self.balance_of(&caller);
        assert!(from_bal >= amount, "insufficient balance");
        let to_bal = self.balance_of(&to);
        self.balances.insert(caller, from_bal - amount);
        self.balances
            .insert(to, to_bal.checked_add(amount).expect("balance overflow"));
        self.yield_last_update.entry(to).or_insert(current_time);
    }

    pub fn approve(&mut self, caller: Address, spender: Address, amount: u64) {
        self.allowances.insert((caller, spender), amount);
    }

    pub fn transfer_from(
        &mut self,
        caller: Address,
        from: Address,
        to: Address,
        amount: u64,
        current_time: u64,
    ) {
        assert!(!self.paused, "contract paused");
        assert!(amount > 0, "transfer amount must be positive");
        let allowed = self.allowance(&from, &caller);
        assert!(allowed >= amount, "insufficient allowance");
        self.settle_yield(&from, current_time);
        self.settle_yield(&to, current_time);
        let from_bal = self.balance_of(&from);
        assert!(from_bal >= amount, "insufficient balance");
        let to_bal = self.balance_of(&to);
        self.balances.insert(from, from_bal - amount);
        self.balances
            .insert(to, to_bal.checked_add(amount).expect("balance overflow"));
        self.allowances.insert((from, caller), allowed - amount);
    }

    /// Update the exchange rate (admin, from oracle)
    pub fn update_rate(&mut self, caller: Address, new_rate: u64) {
        assert!(caller == self.admin, "only admin can update rate");
        assert!(new_rate > 0, "rate must be positive");
        self.rate_per_usdc = new_rate;
    }

    /// Update the yield rate (admin)
    pub fn update_yield_rate(&mut self, caller: Address, new_yield_bps: u64) {
        assert!(caller == self.admin, "only admin can update yield rate");
        self.yield_rate_bps = new_yield_bps;
    }

    /// Get proof of reserves: (usdc_backing, supply_value_in_usdc, is_fully_backed)
    pub fn proof_of_reserves(&self) -> (u64, u64, bool) {
        let supply_in_usdc = if self.rate_per_usdc > 0 {
            (self.total_supply as u128 * 1_000_000 / self.rate_per_usdc as u128) as u64
        } else {
            0
        };
        let is_fully_backed = self.usdc_backing >= supply_in_usdc;
        (self.usdc_backing, supply_in_usdc, is_fully_backed)
    }

    pub fn pause(&mut self, caller: Address) {
        assert!(caller == self.admin, "only admin");
        self.paused = true;
    }

    pub fn unpause(&mut self, caller: Address) {
        assert!(caller == self.admin, "only admin");
        self.paused = false;
    }
}

// ---------------------------------------------------------------------------
// Dispatch argument structs
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    name: String,
    symbol: String,
    decimals: u8,
    rate_per_usdc: u64,
    yield_rate_bps: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct MintArgs {
    to: Address,
    amount: u64,
    usdc_backing_amount: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BurnArgs {
    from: Address,
    amount: u64,
    usdc_backing_amount: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferArgs {
    to: Address,
    amount: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApproveArgs {
    spender: Address,
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferFromArgs {
    from: Address,
    to: Address,
    amount: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateRateArgs {
    new_rate: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateYieldRateArgs {
    new_yield_bps: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BalanceOfArgs {
    account: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct AllowanceArgs {
    owner: Address,
    spender: Address,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Contract-level dispatch. `state` is None on first call (init).
pub fn dispatch(
    state: &mut Option<StablecoinState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        // -- Init ------------------------------------------------------------
        "init" => {
            assert!(state.is_none(), "already initialised");
            let a: InitArgs =
                serde_json::from_slice(args).expect("bad init args");
            *state = Some(StablecoinState::new(
                a.name,
                a.symbol,
                a.decimals,
                caller,
                a.rate_per_usdc,
                a.yield_rate_bps,
            ));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "name" => {
            let s = state.as_ref().expect("not initialised");
            serde_json::to_vec(s.name()).unwrap()
        }
        "symbol" => {
            let s = state.as_ref().expect("not initialised");
            serde_json::to_vec(s.symbol()).unwrap()
        }
        "decimals" => {
            let s = state.as_ref().expect("not initialised");
            serde_json::to_vec(&s.decimals()).unwrap()
        }
        "total_supply" => {
            let s = state.as_ref().expect("not initialised");
            serde_json::to_vec(&s.total_supply()).unwrap()
        }
        "balance_of" => {
            let s = state.as_ref().expect("not initialised");
            let a: BalanceOfArgs =
                serde_json::from_slice(args).expect("bad balance_of args");
            serde_json::to_vec(&s.balance_of(&a.account)).unwrap()
        }
        "allowance" => {
            let s = state.as_ref().expect("not initialised");
            let a: AllowanceArgs =
                serde_json::from_slice(args).expect("bad allowance args");
            serde_json::to_vec(&s.allowance(&a.owner, &a.spender)).unwrap()
        }
        "proof_of_reserves" => {
            let s = state.as_ref().expect("not initialised");
            let (backing, value, backed) = s.proof_of_reserves();
            serde_json::to_vec(&(backing, value, backed)).unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "mint" => {
            let s = state.as_mut().expect("not initialised");
            let a: MintArgs =
                serde_json::from_slice(args).expect("bad mint args");
            s.mint(caller, a.to, a.amount, a.usdc_backing_amount, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }
        "burn" => {
            let s = state.as_mut().expect("not initialised");
            let a: BurnArgs =
                serde_json::from_slice(args).expect("bad burn args");
            s.burn(caller, a.from, a.amount, a.usdc_backing_amount, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer" => {
            let s = state.as_mut().expect("not initialised");
            let a: TransferArgs =
                serde_json::from_slice(args).expect("bad transfer args");
            s.transfer(caller, a.to, a.amount, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }
        "approve" => {
            let s = state.as_mut().expect("not initialised");
            let a: ApproveArgs =
                serde_json::from_slice(args).expect("bad approve args");
            s.approve(caller, a.spender, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer_from" => {
            let s = state.as_mut().expect("not initialised");
            let a: TransferFromArgs =
                serde_json::from_slice(args).expect("bad transfer_from args");
            s.transfer_from(caller, a.from, a.to, a.amount, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }
        "update_rate" => {
            let s = state.as_mut().expect("not initialised");
            let a: UpdateRateArgs =
                serde_json::from_slice(args).expect("bad update_rate args");
            s.update_rate(caller, a.new_rate);
            serde_json::to_vec("ok").unwrap()
        }
        "update_yield_rate" => {
            let s = state.as_mut().expect("not initialised");
            let a: UpdateYieldRateArgs =
                serde_json::from_slice(args).expect("bad update_yield_rate args");
            s.update_yield_rate(caller, a.new_yield_bps);
            serde_json::to_vec("ok").unwrap()
        }
        "pause" => {
            let s = state.as_mut().expect("not initialised");
            s.pause(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "unpause" => {
            let s = state.as_mut().expect("not initialised");
            s.unpause(caller);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ADMIN: Address = [1u8; 32];
    const ALICE: Address = [2u8; 32];
    const BOB: Address = [3u8; 32];
    const T0: u64 = 1_700_000_000;

    fn new_eurc() -> StablecoinState {
        StablecoinState::new(
            "Dina Euro".into(),
            "EURC".into(),
            6,
            ADMIN,
            930_000,  // 0.93 EUR per USDC
            400,      // 4% APY
        )
    }

    // -- Init & queries -------------------------------------------------------

    #[test]
    fn test_init_state() {
        let s = new_eurc();
        assert_eq!(s.name(), "Dina Euro");
        assert_eq!(s.symbol(), "EURC");
        assert_eq!(s.decimals(), 6);
        assert_eq!(s.total_supply(), 0);
        assert_eq!(s.rate_per_usdc, 930_000);
        assert_eq!(s.yield_rate_bps, 400);
        assert!(!s.paused);
    }

    // -- Mint -----------------------------------------------------------------

    #[test]
    fn test_mint() {
        let mut s = new_eurc();
        // Mint 93 EURC backed by 100 USDC
        s.mint(ADMIN, ALICE, 93_000_000, 100_000_000, T0);
        assert_eq!(s.balance_of(&ALICE), 93_000_000);
        assert_eq!(s.total_supply(), 93_000_000);
        assert_eq!(s.usdc_backing, 100_000_000);
    }

    #[test]
    #[should_panic(expected = "only admin can mint")]
    fn test_mint_non_admin() {
        let mut s = new_eurc();
        s.mint(ALICE, ALICE, 100, 100, T0);
    }

    #[test]
    #[should_panic(expected = "contract paused")]
    fn test_mint_while_paused() {
        let mut s = new_eurc();
        s.pause(ADMIN);
        s.mint(ADMIN, ALICE, 100, 100, T0);
    }

    #[test]
    #[should_panic(expected = "mint amount must be positive")]
    fn test_mint_zero() {
        let mut s = new_eurc();
        s.mint(ADMIN, ALICE, 0, 0, T0);
    }

    // -- Burn -----------------------------------------------------------------

    #[test]
    fn test_burn() {
        let mut s = new_eurc();
        s.mint(ADMIN, ALICE, 93_000_000, 100_000_000, T0);
        s.burn(ADMIN, ALICE, 50_000_000, 53_763_441, T0);
        assert_eq!(s.balance_of(&ALICE), 43_000_000);
        assert_eq!(s.total_supply(), 43_000_000);
        assert_eq!(s.usdc_backing, 100_000_000 - 53_763_441);
    }

    #[test]
    #[should_panic(expected = "only admin can burn")]
    fn test_burn_non_admin() {
        let mut s = new_eurc();
        s.mint(ADMIN, ALICE, 100, 100, T0);
        s.burn(ALICE, ALICE, 50, 50, T0);
    }

    #[test]
    #[should_panic(expected = "insufficient balance to burn")]
    fn test_burn_insufficient() {
        let mut s = new_eurc();
        s.mint(ADMIN, ALICE, 100, 100, T0);
        s.burn(ADMIN, ALICE, 200, 200, T0);
    }

    // -- Transfer -------------------------------------------------------------

    #[test]
    fn test_transfer() {
        let mut s = new_eurc();
        s.mint(ADMIN, ALICE, 1_000_000, 1_075_269, T0);
        s.transfer(ALICE, BOB, 400_000, T0);
        assert_eq!(s.balance_of(&ALICE), 600_000);
        assert_eq!(s.balance_of(&BOB), 400_000);
    }

    #[test]
    #[should_panic(expected = "insufficient balance")]
    fn test_transfer_insufficient() {
        let mut s = new_eurc();
        s.mint(ADMIN, ALICE, 100, 108, T0);
        s.transfer(ALICE, BOB, 200, T0);
    }

    #[test]
    #[should_panic(expected = "cannot transfer to self")]
    fn test_transfer_self() {
        let mut s = new_eurc();
        s.mint(ADMIN, ALICE, 100, 108, T0);
        s.transfer(ALICE, ALICE, 50, T0);
    }

    #[test]
    #[should_panic(expected = "contract paused")]
    fn test_transfer_while_paused() {
        let mut s = new_eurc();
        s.mint(ADMIN, ALICE, 100, 108, T0);
        s.pause(ADMIN);
        s.transfer(ALICE, BOB, 50, T0);
    }

    // -- Approve & TransferFrom -----------------------------------------------

    #[test]
    fn test_approve_and_transfer_from() {
        let mut s = new_eurc();
        s.mint(ADMIN, ALICE, 1_000_000, 1_075_269, T0);
        s.approve(ALICE, BOB, 500_000);
        assert_eq!(s.allowance(&ALICE, &BOB), 500_000);
        s.transfer_from(BOB, ALICE, BOB, 300_000, T0);
        assert_eq!(s.balance_of(&ALICE), 700_000);
        assert_eq!(s.balance_of(&BOB), 300_000);
        assert_eq!(s.allowance(&ALICE, &BOB), 200_000);
    }

    #[test]
    #[should_panic(expected = "insufficient allowance")]
    fn test_transfer_from_no_allowance() {
        let mut s = new_eurc();
        s.mint(ADMIN, ALICE, 1_000_000, 1_075_269, T0);
        s.transfer_from(BOB, ALICE, BOB, 100, T0);
    }

    // -- Yield ----------------------------------------------------------------

    #[test]
    fn test_yield_accrual() {
        let mut s = new_eurc();
        // 4% APY on 1,000,000 micro-units
        s.mint(ADMIN, ALICE, 1_000_000, 1_075_269, T0);
        // Advance 1 year, mint more to trigger yield settlement
        let t1 = T0 + SECONDS_PER_YEAR;
        s.mint(ADMIN, ALICE, 1, 1, t1);
        // Expected yield: 1_000_000 * 400 / 10_000 = 40_000 (4%)
        // Balance = 1_000_000 + 40_000 + 1 = 1_040_001
        assert_eq!(s.balance_of(&ALICE), 1_040_001);
    }

    #[test]
    fn test_yield_accrual_on_transfer() {
        let mut s = new_eurc();
        // 4% APY on 10,000,000 micro-units (10 EURC)
        s.mint(ADMIN, ALICE, 10_000_000, 10_752_688, T0);

        // Advance half a year
        let t1 = T0 + SECONDS_PER_YEAR / 2;
        // Transfer triggers yield settlement
        s.transfer(ALICE, BOB, 1_000_000, t1);

        // Expected yield: 10_000_000 * 400 * (SECONDS_PER_YEAR/2) / (10000 * SECONDS_PER_YEAR)
        // = 10_000_000 * 400 / 20_000 = 200_000
        let alice_bal = s.balance_of(&ALICE);
        let bob_bal = s.balance_of(&BOB);
        // Alice had 10M, got 200K yield, then sent 1M => 9.2M
        assert_eq!(alice_bal, 9_200_000);
        assert_eq!(bob_bal, 1_000_000);
        // Total supply increased by yield
        assert_eq!(s.total_supply(), 10_000_000 + 200_000);
    }

    #[test]
    fn test_yield_accrual_full_year() {
        let mut s = new_eurc();
        s.mint(ADMIN, ALICE, 1_000_000, 1_075_269, T0);
        let t1 = T0 + SECONDS_PER_YEAR;
        // Trigger yield via a transfer
        s.transfer(ALICE, BOB, 100_000, t1);
        // Expected yield: 1_000_000 * 400 / 10_000 = 40_000 (4%)
        // Alice: 1_000_000 + 40_000 - 100_000 = 940_000
        assert_eq!(s.balance_of(&ALICE), 940_000);
        assert_eq!(s.balance_of(&BOB), 100_000);
    }

    // -- Rate & reserves ------------------------------------------------------

    #[test]
    fn test_update_rate() {
        let mut s = new_eurc();
        s.update_rate(ADMIN, 950_000);
        assert_eq!(s.rate_per_usdc, 950_000);
    }

    #[test]
    #[should_panic(expected = "only admin can update rate")]
    fn test_update_rate_non_admin() {
        let mut s = new_eurc();
        s.update_rate(ALICE, 950_000);
    }

    #[test]
    #[should_panic(expected = "rate must be positive")]
    fn test_update_rate_zero() {
        let mut s = new_eurc();
        s.update_rate(ADMIN, 0);
    }

    #[test]
    fn test_proof_of_reserves() {
        let mut s = new_eurc();
        // Mint 93 EURC backed by 100 USDC
        s.mint(ADMIN, ALICE, 93_000_000, 100_000_000, T0);
        let (backing, value, backed) = s.proof_of_reserves();
        assert_eq!(backing, 100_000_000);
        // value = 93_000_000 * 1_000_000 / 930_000 = 100_000_000
        assert_eq!(value, 100_000_000);
        assert!(backed);
    }

    #[test]
    fn test_proof_of_reserves_underbacked() {
        let mut s = new_eurc();
        s.mint(ADMIN, ALICE, 93_000_000, 90_000_000, T0);
        let (backing, value, backed) = s.proof_of_reserves();
        assert_eq!(backing, 90_000_000);
        assert_eq!(value, 100_000_000);
        assert!(!backed);
    }

    // -- Pause/unpause --------------------------------------------------------

    #[test]
    fn test_pause_unpause() {
        let mut s = new_eurc();
        s.pause(ADMIN);
        assert!(s.paused);
        s.unpause(ADMIN);
        assert!(!s.paused);
    }

    #[test]
    #[should_panic(expected = "only admin")]
    fn test_pause_non_admin() {
        let mut s = new_eurc();
        s.pause(ALICE);
    }

    // -- Dispatch -------------------------------------------------------------

    #[test]
    fn test_dispatch_init() {
        let mut state: Option<StablecoinState> = None;
        let args = serde_json::to_vec(&serde_json::json!({
            "name": "Dina Euro",
            "symbol": "EURC",
            "decimals": 6,
            "rate_per_usdc": 930_000,
            "yield_rate_bps": 400
        }))
        .unwrap();
        let result = dispatch(&mut state, "init", &args, ADMIN);
        let ok: String = serde_json::from_slice(&result).unwrap();
        assert_eq!(ok, "ok");
        assert!(state.is_some());
        assert_eq!(state.as_ref().unwrap().symbol(), "EURC");
    }

    #[test]
    fn test_dispatch_mint_and_balance() {
        let mut state: Option<StablecoinState> = None;
        let init_args = serde_json::to_vec(&serde_json::json!({
            "name": "Dina Euro",
            "symbol": "EURC",
            "decimals": 6,
            "rate_per_usdc": 930_000,
            "yield_rate_bps": 400
        }))
        .unwrap();
        dispatch(&mut state, "init", &init_args, ADMIN);

        let mint_args = serde_json::to_vec(&serde_json::json!({
            "to": ALICE,
            "amount": 93_000_000u64,
            "usdc_backing_amount": 100_000_000u64,
            "current_time": T0,
        }))
        .unwrap();
        dispatch(&mut state, "mint", &mint_args, ADMIN);

        let bal_args = serde_json::to_vec(&serde_json::json!({
            "account": ALICE,
        }))
        .unwrap();
        let result = dispatch(&mut state, "balance_of", &bal_args, ALICE);
        let bal: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(bal, 93_000_000);
    }

    #[test]
    fn test_dispatch_transfer() {
        let mut state: Option<StablecoinState> = None;
        let init_args = serde_json::to_vec(&serde_json::json!({
            "name": "Dina Euro",
            "symbol": "EURC",
            "decimals": 6,
            "rate_per_usdc": 930_000,
            "yield_rate_bps": 0
        }))
        .unwrap();
        dispatch(&mut state, "init", &init_args, ADMIN);

        let mint_args = serde_json::to_vec(&serde_json::json!({
            "to": ALICE,
            "amount": 1_000_000u64,
            "usdc_backing_amount": 1_075_269u64,
            "current_time": T0,
        }))
        .unwrap();
        dispatch(&mut state, "mint", &mint_args, ADMIN);

        let xfer_args = serde_json::to_vec(&serde_json::json!({
            "to": BOB,
            "amount": 400_000u64,
            "current_time": T0,
        }))
        .unwrap();
        dispatch(&mut state, "transfer", &xfer_args, ALICE);

        let s = state.as_ref().unwrap();
        assert_eq!(s.balance_of(&ALICE), 600_000);
        assert_eq!(s.balance_of(&BOB), 400_000);
    }

    #[test]
    fn test_dispatch_proof_of_reserves() {
        let mut state: Option<StablecoinState> = None;
        let init_args = serde_json::to_vec(&serde_json::json!({
            "name": "Dina Euro",
            "symbol": "EURC",
            "decimals": 6,
            "rate_per_usdc": 930_000,
            "yield_rate_bps": 0
        }))
        .unwrap();
        dispatch(&mut state, "init", &init_args, ADMIN);

        let mint_args = serde_json::to_vec(&serde_json::json!({
            "to": ALICE,
            "amount": 93_000_000u64,
            "usdc_backing_amount": 100_000_000u64,
            "current_time": T0,
        }))
        .unwrap();
        dispatch(&mut state, "mint", &mint_args, ADMIN);

        let result = dispatch(&mut state, "proof_of_reserves", &[], ALICE);
        let (backing, value, backed): (u64, u64, bool) =
            serde_json::from_slice(&result).unwrap();
        assert_eq!(backing, 100_000_000);
        assert_eq!(value, 100_000_000);
        assert!(backed);
    }

    #[test]
    #[should_panic(expected = "already initialised")]
    fn test_dispatch_double_init() {
        let mut state: Option<StablecoinState> = None;
        let args = serde_json::to_vec(&serde_json::json!({
            "name": "X", "symbol": "X", "decimals": 6,
            "rate_per_usdc": 1_000_000, "yield_rate_bps": 0
        }))
        .unwrap();
        dispatch(&mut state, "init", &args, ADMIN);
        dispatch(&mut state, "init", &args, ADMIN);
    }

    #[test]
    #[should_panic(expected = "unknown method")]
    fn test_dispatch_unknown_method() {
        let mut state: Option<StablecoinState> = None;
        let args = serde_json::to_vec(&serde_json::json!({
            "name": "X", "symbol": "X", "decimals": 6,
            "rate_per_usdc": 1_000_000, "yield_rate_bps": 0
        }))
        .unwrap();
        dispatch(&mut state, "init", &args, ADMIN);
        dispatch(&mut state, "foo", &[], ADMIN);
    }

    #[test]
    fn test_update_yield_rate() {
        let mut s = new_eurc();
        s.update_yield_rate(ADMIN, 500);
        assert_eq!(s.yield_rate_bps, 500);
    }
}
