use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Dina Currency Registry — tracks all deployed currency stablecoins
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurrencyInfo {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub contract_address: Address,
    pub oracle_rate: u64,
    pub yield_rate_bps: u64,
    pub total_supply: u64,
    pub usdc_backing: u64,
    pub active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurrencyRegistry {
    pub admin: Address,
    pub currencies: BTreeMap<String, CurrencyInfo>,
}

impl CurrencyRegistry {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            currencies: BTreeMap::new(),
        }
    }

    /// Register a new currency in the registry (admin only).
    pub fn register_currency(
        &mut self,
        caller: Address,
        symbol: String,
        name: String,
        decimals: u8,
        contract_address: Address,
        oracle_rate: u64,
        yield_rate_bps: u64,
    ) {
        assert!(caller == self.admin, "only admin can register currencies");
        assert!(!symbol.is_empty(), "symbol must not be empty");
        assert!(!name.is_empty(), "name must not be empty");
        assert!(
            !self.currencies.contains_key(&symbol),
            "currency already registered"
        );
        self.currencies.insert(
            symbol.clone(),
            CurrencyInfo {
                symbol,
                name,
                decimals,
                contract_address,
                oracle_rate,
                yield_rate_bps,
                total_supply: 0,
                usdc_backing: 0,
                active: true,
            },
        );
    }

    /// Update a currency's mutable fields (admin only).
    pub fn update_currency(
        &mut self,
        caller: Address,
        symbol: String,
        oracle_rate: Option<u64>,
        yield_rate_bps: Option<u64>,
        total_supply: Option<u64>,
        usdc_backing: Option<u64>,
    ) {
        assert!(caller == self.admin, "only admin can update currencies");
        let info = self
            .currencies
            .get_mut(&symbol)
            .expect("currency not found");
        if let Some(rate) = oracle_rate {
            info.oracle_rate = rate;
        }
        if let Some(yield_bps) = yield_rate_bps {
            info.yield_rate_bps = yield_bps;
        }
        if let Some(supply) = total_supply {
            info.total_supply = supply;
        }
        if let Some(backing) = usdc_backing {
            info.usdc_backing = backing;
        }
    }

    /// Deactivate a currency (admin only). Does not remove it.
    pub fn deactivate_currency(&mut self, caller: Address, symbol: String) {
        assert!(caller == self.admin, "only admin can deactivate currencies");
        let info = self
            .currencies
            .get_mut(&symbol)
            .expect("currency not found");
        info.active = false;
    }

    /// Reactivate a currency (admin only).
    pub fn activate_currency(&mut self, caller: Address, symbol: String) {
        assert!(caller == self.admin, "only admin can activate currencies");
        let info = self
            .currencies
            .get_mut(&symbol)
            .expect("currency not found");
        info.active = true;
    }

    /// Get a single currency's info. Returns None if not found.
    pub fn get_currency(&self, symbol: &str) -> Option<&CurrencyInfo> {
        self.currencies.get(symbol)
    }

    /// List all currencies (active and inactive).
    pub fn list_currencies(&self) -> Vec<&CurrencyInfo> {
        self.currencies.values().collect()
    }

    /// List only active currencies.
    pub fn list_active_currencies(&self) -> Vec<&CurrencyInfo> {
        self.currencies
            .values()
            .filter(|c| c.active)
            .collect()
    }

    /// Get total USDC backing across all active currencies.
    pub fn total_usdc_backing(&self) -> u64 {
        self.currencies
            .values()
            .filter(|c| c.active)
            .map(|c| c.usdc_backing)
            .sum()
    }

    /// Get count of registered currencies.
    pub fn currency_count(&self) -> usize {
        self.currencies.len()
    }

    /// Get count of active currencies.
    pub fn active_count(&self) -> usize {
        self.currencies.values().filter(|c| c.active).count()
    }
}

// ---------------------------------------------------------------------------
// Dispatch argument structs
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct RegisterCurrencyArgs {
    symbol: String,
    name: String,
    decimals: u8,
    contract_address: Address,
    oracle_rate: u64,
    yield_rate_bps: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateCurrencyArgs {
    symbol: String,
    oracle_rate: Option<u64>,
    yield_rate_bps: Option<u64>,
    total_supply: Option<u64>,
    usdc_backing: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SymbolArgs {
    symbol: String,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<CurrencyRegistry>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "already initialised");
            *state = Some(CurrencyRegistry::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "register_currency" => {
            let s = state.as_mut().expect("not initialised");
            let a: RegisterCurrencyArgs =
                serde_json::from_slice(args).expect("bad register_currency args");
            s.register_currency(
                caller,
                a.symbol,
                a.name,
                a.decimals,
                a.contract_address,
                a.oracle_rate,
                a.yield_rate_bps,
            );
            serde_json::to_vec("ok").unwrap()
        }
        "update_currency" => {
            let s = state.as_mut().expect("not initialised");
            let a: UpdateCurrencyArgs =
                serde_json::from_slice(args).expect("bad update_currency args");
            s.update_currency(
                caller,
                a.symbol,
                a.oracle_rate,
                a.yield_rate_bps,
                a.total_supply,
                a.usdc_backing,
            );
            serde_json::to_vec("ok").unwrap()
        }
        "deactivate_currency" => {
            let s = state.as_mut().expect("not initialised");
            let a: SymbolArgs =
                serde_json::from_slice(args).expect("bad deactivate_currency args");
            s.deactivate_currency(caller, a.symbol);
            serde_json::to_vec("ok").unwrap()
        }
        "activate_currency" => {
            let s = state.as_mut().expect("not initialised");
            let a: SymbolArgs =
                serde_json::from_slice(args).expect("bad activate_currency args");
            s.activate_currency(caller, a.symbol);
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "get_currency" => {
            let s = state.as_ref().expect("not initialised");
            let a: SymbolArgs =
                serde_json::from_slice(args).expect("bad get_currency args");
            let info = s.get_currency(&a.symbol);
            serde_json::to_vec(&info).unwrap()
        }
        "list_currencies" => {
            let s = state.as_ref().expect("not initialised");
            let list = s.list_currencies();
            serde_json::to_vec(&list).unwrap()
        }
        "list_active_currencies" => {
            let s = state.as_ref().expect("not initialised");
            let list = s.list_active_currencies();
            serde_json::to_vec(&list).unwrap()
        }
        "currency_count" => {
            let s = state.as_ref().expect("not initialised");
            serde_json::to_vec(&s.currency_count()).unwrap()
        }
        "active_count" => {
            let s = state.as_ref().expect("not initialised");
            serde_json::to_vec(&s.active_count()).unwrap()
        }
        "total_usdc_backing" => {
            let s = state.as_ref().expect("not initialised");
            serde_json::to_vec(&s.total_usdc_backing()).unwrap()
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
    const USER: Address = [2u8; 32];
    const EURC_ADDR: Address = [10u8; 32];
    const GBPC_ADDR: Address = [11u8; 32];
    const CADC_ADDR: Address = [12u8; 32];

    fn setup_registry() -> CurrencyRegistry {
        let mut r = CurrencyRegistry::new(ADMIN);
        r.register_currency(
            ADMIN,
            "EURC".into(),
            "Dina Euro".into(),
            6,
            EURC_ADDR,
            930_000,
            400,
        );
        r.register_currency(
            ADMIN,
            "GBPC".into(),
            "Dina British Pound".into(),
            6,
            GBPC_ADDR,
            790_000,
            350,
        );
        r
    }

    // -- Registration --------------------------------------------------------

    #[test]
    fn test_register_currency() {
        let r = setup_registry();
        assert_eq!(r.currency_count(), 2);
        let eurc = r.get_currency("EURC").unwrap();
        assert_eq!(eurc.name, "Dina Euro");
        assert_eq!(eurc.symbol, "EURC");
        assert_eq!(eurc.decimals, 6);
        assert_eq!(eurc.oracle_rate, 930_000);
        assert_eq!(eurc.yield_rate_bps, 400);
        assert!(eurc.active);
        assert_eq!(eurc.total_supply, 0);
        assert_eq!(eurc.usdc_backing, 0);
    }

    #[test]
    #[should_panic(expected = "only admin can register")]
    fn test_register_non_admin() {
        let mut r = CurrencyRegistry::new(ADMIN);
        r.register_currency(USER, "X".into(), "X Coin".into(), 6, [0u8; 32], 1_000_000, 0);
    }

    #[test]
    #[should_panic(expected = "currency already registered")]
    fn test_register_duplicate() {
        let mut r = setup_registry();
        r.register_currency(ADMIN, "EURC".into(), "Euro Again".into(), 6, [0u8; 32], 930_000, 400);
    }

    #[test]
    #[should_panic(expected = "symbol must not be empty")]
    fn test_register_empty_symbol() {
        let mut r = CurrencyRegistry::new(ADMIN);
        r.register_currency(ADMIN, "".into(), "No Symbol".into(), 6, [0u8; 32], 1_000_000, 0);
    }

    #[test]
    #[should_panic(expected = "name must not be empty")]
    fn test_register_empty_name() {
        let mut r = CurrencyRegistry::new(ADMIN);
        r.register_currency(ADMIN, "X".into(), "".into(), 6, [0u8; 32], 1_000_000, 0);
    }

    // -- Update --------------------------------------------------------------

    #[test]
    fn test_update_currency() {
        let mut r = setup_registry();
        r.update_currency(
            ADMIN,
            "EURC".into(),
            Some(940_000),
            Some(450),
            Some(100_000_000),
            Some(107_526_882),
        );
        let eurc = r.get_currency("EURC").unwrap();
        assert_eq!(eurc.oracle_rate, 940_000);
        assert_eq!(eurc.yield_rate_bps, 450);
        assert_eq!(eurc.total_supply, 100_000_000);
        assert_eq!(eurc.usdc_backing, 107_526_882);
    }

    #[test]
    fn test_update_currency_partial() {
        let mut r = setup_registry();
        r.update_currency(ADMIN, "EURC".into(), Some(950_000), None, None, None);
        let eurc = r.get_currency("EURC").unwrap();
        assert_eq!(eurc.oracle_rate, 950_000);
        assert_eq!(eurc.yield_rate_bps, 400); // unchanged
    }

    #[test]
    #[should_panic(expected = "only admin can update")]
    fn test_update_non_admin() {
        let mut r = setup_registry();
        r.update_currency(USER, "EURC".into(), Some(950_000), None, None, None);
    }

    #[test]
    #[should_panic(expected = "currency not found")]
    fn test_update_nonexistent() {
        let mut r = setup_registry();
        r.update_currency(ADMIN, "XYZ".into(), Some(1_000_000), None, None, None);
    }

    // -- Deactivate/activate -------------------------------------------------

    #[test]
    fn test_deactivate_currency() {
        let mut r = setup_registry();
        r.deactivate_currency(ADMIN, "EURC".into());
        let eurc = r.get_currency("EURC").unwrap();
        assert!(!eurc.active);
        assert_eq!(r.active_count(), 1);
        assert_eq!(r.currency_count(), 2);
    }

    #[test]
    fn test_activate_currency() {
        let mut r = setup_registry();
        r.deactivate_currency(ADMIN, "EURC".into());
        r.activate_currency(ADMIN, "EURC".into());
        let eurc = r.get_currency("EURC").unwrap();
        assert!(eurc.active);
        assert_eq!(r.active_count(), 2);
    }

    #[test]
    #[should_panic(expected = "only admin can deactivate")]
    fn test_deactivate_non_admin() {
        let mut r = setup_registry();
        r.deactivate_currency(USER, "EURC".into());
    }

    // -- Queries -------------------------------------------------------------

    #[test]
    fn test_get_currency_not_found() {
        let r = setup_registry();
        assert!(r.get_currency("XYZ").is_none());
    }

    #[test]
    fn test_list_currencies() {
        let r = setup_registry();
        let list = r.list_currencies();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_list_active_currencies() {
        let mut r = setup_registry();
        r.deactivate_currency(ADMIN, "GBPC".into());
        let active = r.list_active_currencies();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].symbol, "EURC");
    }

    #[test]
    fn test_total_usdc_backing() {
        let mut r = setup_registry();
        r.update_currency(ADMIN, "EURC".into(), None, None, None, Some(100_000_000));
        r.update_currency(ADMIN, "GBPC".into(), None, None, None, Some(50_000_000));
        assert_eq!(r.total_usdc_backing(), 150_000_000);

        // Deactivated currencies excluded
        r.deactivate_currency(ADMIN, "GBPC".into());
        assert_eq!(r.total_usdc_backing(), 100_000_000);
    }

    // -- Dispatch tests ------------------------------------------------------

    #[test]
    fn test_dispatch_init_and_register() {
        let mut state: Option<CurrencyRegistry> = None;
        dispatch(&mut state, "init", &[], ADMIN);
        assert!(state.is_some());

        let reg_args = serde_json::to_vec(&serde_json::json!({
            "symbol": "EURC",
            "name": "Dina Euro",
            "decimals": 6,
            "contract_address": EURC_ADDR,
            "oracle_rate": 930_000u64,
            "yield_rate_bps": 400u64,
        }))
        .unwrap();
        dispatch(&mut state, "register_currency", &reg_args, ADMIN);

        let get_args = serde_json::to_vec(&serde_json::json!({
            "symbol": "EURC",
        }))
        .unwrap();
        let result = dispatch(&mut state, "get_currency", &get_args, USER);
        let info: Option<CurrencyInfo> = serde_json::from_slice(&result).unwrap();
        assert!(info.is_some());
        assert_eq!(info.unwrap().name, "Dina Euro");
    }

    #[test]
    fn test_dispatch_list_and_count() {
        let mut state: Option<CurrencyRegistry> = None;
        dispatch(&mut state, "init", &[], ADMIN);

        let reg1 = serde_json::to_vec(&serde_json::json!({
            "symbol": "EURC", "name": "Dina Euro", "decimals": 6,
            "contract_address": EURC_ADDR, "oracle_rate": 930_000u64, "yield_rate_bps": 400u64,
        }))
        .unwrap();
        dispatch(&mut state, "register_currency", &reg1, ADMIN);

        let reg2 = serde_json::to_vec(&serde_json::json!({
            "symbol": "CADC", "name": "Dina Canadian Dollar", "decimals": 6,
            "contract_address": CADC_ADDR, "oracle_rate": 1_370_000u64, "yield_rate_bps": 350u64,
        }))
        .unwrap();
        dispatch(&mut state, "register_currency", &reg2, ADMIN);

        let result = dispatch(&mut state, "currency_count", &[], USER);
        let count: usize = serde_json::from_slice(&result).unwrap();
        assert_eq!(count, 2);

        let result = dispatch(&mut state, "active_count", &[], USER);
        let count: usize = serde_json::from_slice(&result).unwrap();
        assert_eq!(count, 2);

        // Deactivate one
        let deact_args = serde_json::to_vec(&serde_json::json!({ "symbol": "CADC" })).unwrap();
        dispatch(&mut state, "deactivate_currency", &deact_args, ADMIN);

        let result = dispatch(&mut state, "active_count", &[], USER);
        let count: usize = serde_json::from_slice(&result).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_dispatch_update_currency() {
        let mut state: Option<CurrencyRegistry> = None;
        dispatch(&mut state, "init", &[], ADMIN);

        let reg = serde_json::to_vec(&serde_json::json!({
            "symbol": "EURC", "name": "Dina Euro", "decimals": 6,
            "contract_address": EURC_ADDR, "oracle_rate": 930_000u64, "yield_rate_bps": 400u64,
        }))
        .unwrap();
        dispatch(&mut state, "register_currency", &reg, ADMIN);

        let upd = serde_json::to_vec(&serde_json::json!({
            "symbol": "EURC",
            "oracle_rate": 940_000u64,
            "yield_rate_bps": null,
            "total_supply": 500_000_000u64,
            "usdc_backing": 537_634_408u64,
        }))
        .unwrap();
        dispatch(&mut state, "update_currency", &upd, ADMIN);

        let s = state.as_ref().unwrap();
        let eurc = s.get_currency("EURC").unwrap();
        assert_eq!(eurc.oracle_rate, 940_000);
        assert_eq!(eurc.yield_rate_bps, 400); // unchanged
        assert_eq!(eurc.total_supply, 500_000_000);
        assert_eq!(eurc.usdc_backing, 537_634_408);
    }

    #[test]
    #[should_panic(expected = "already initialised")]
    fn test_dispatch_double_init() {
        let mut state: Option<CurrencyRegistry> = None;
        dispatch(&mut state, "init", &[], ADMIN);
        dispatch(&mut state, "init", &[], ADMIN);
    }

    #[test]
    #[should_panic(expected = "unknown method")]
    fn test_dispatch_unknown_method() {
        let mut state: Option<CurrencyRegistry> = None;
        dispatch(&mut state, "init", &[], ADMIN);
        dispatch(&mut state, "foo", &[], ADMIN);
    }
}
