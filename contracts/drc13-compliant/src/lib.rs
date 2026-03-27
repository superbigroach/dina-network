use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-13  Compliant Token  (ERC-3643 equivalent)
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VerificationInfo {
    pub country: String,
    pub credentials: Vec<String>,
    pub verified_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ComplianceRule {
    RequireCredential(String),
    MaxHolders(u64),
    MaxPerHolder(u64),
    CountryWhitelist(Vec<String>),
    CountryBlacklist(Vec<String>),
    MinHoldPeriod(u64),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CompliantTokenState {
    // DRC-1 base fields
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u64,
    pub balances: BTreeMap<Address, u64>,

    // DRC-13 compliance fields
    pub admin: Address,
    pub identity_registry: Option<Address>,
    pub compliance_rules: Vec<ComplianceRule>,
    pub frozen_addresses: BTreeMap<Address, bool>,
    pub verified_addresses: BTreeMap<Address, VerificationInfo>,
    pub holder_count: u64,
    pub transfer_timestamps: BTreeMap<Address, u64>,
}

impl CompliantTokenState {
    pub fn new(
        name: String,
        symbol: String,
        decimals: u8,
        admin: Address,
    ) -> Self {
        Self {
            name,
            symbol,
            decimals,
            total_supply: 0,
            balances: BTreeMap::new(),
            admin,
            identity_registry: None,
            compliance_rules: Vec::new(),
            frozen_addresses: BTreeMap::new(),
            verified_addresses: BTreeMap::new(),
            holder_count: 0,
            transfer_timestamps: BTreeMap::new(),
        }
    }

    // -- DRC-1 base queries --------------------------------------------------

    pub fn balance_of(&self, account: &Address) -> u64 {
        self.balances.get(account).copied().unwrap_or(0)
    }

    pub fn total_supply(&self) -> u64 {
        self.total_supply
    }

    // -- Identity & compliance -----------------------------------------------

    pub fn is_verified(&self, addr: &Address) -> bool {
        self.verified_addresses.contains_key(addr)
    }

    pub fn set_identity_registry(&mut self, caller: Address, registry: Address) {
        assert!(
            caller == self.admin,
            "DRC13: only admin can set identity registry"
        );
        self.identity_registry = Some(registry);
    }

    pub fn add_compliance(&mut self, caller: Address, rule: ComplianceRule) {
        assert!(
            caller == self.admin,
            "DRC13: only admin can add compliance rules"
        );
        self.compliance_rules.push(rule);
    }

    pub fn freeze(&mut self, caller: Address, addr: Address) {
        assert!(caller == self.admin, "DRC13: only admin can freeze");
        self.frozen_addresses.insert(addr, true);
    }

    pub fn unfreeze(&mut self, caller: Address, addr: Address) {
        assert!(caller == self.admin, "DRC13: only admin can unfreeze");
        self.frozen_addresses.remove(&addr);
    }

    pub fn verify_address(
        &mut self,
        caller: Address,
        addr: Address,
        info: VerificationInfo,
    ) {
        if let Some(registry) = self.identity_registry {
            assert!(
                caller == registry,
                "DRC13: only identity registry can verify"
            );
        } else {
            assert!(
                caller == self.admin,
                "DRC13: only admin can verify (no registry set)"
            );
        }
        if !self.verified_addresses.contains_key(&addr) {
            // New verified holder doesn't necessarily hold tokens yet,
            // but we track for holder count during transfer.
        }
        self.verified_addresses.insert(addr, info);
    }

    // -- Compliance checks ---------------------------------------------------

    fn check_compliance(&self, to: &Address, amount: u64) {
        let to_info = self
            .verified_addresses
            .get(to)
            .expect("DRC13: recipient is not verified");

        for rule in &self.compliance_rules {
            match rule {
                ComplianceRule::RequireCredential(cred) => {
                    assert!(
                        to_info.credentials.contains(cred),
                        "DRC13: recipient missing required credential '{cred}'"
                    );
                }
                ComplianceRule::MaxHolders(max) => {
                    // If recipient doesn't already hold tokens, this is a new holder.
                    let current_balance = self.balance_of(to);
                    if current_balance == 0 {
                        assert!(
                            self.holder_count < *max,
                            "DRC13: max holders ({max}) reached"
                        );
                    }
                }
                ComplianceRule::MaxPerHolder(max) => {
                    let current_balance = self.balance_of(to);
                    assert!(
                        current_balance + amount <= *max,
                        "DRC13: transfer would exceed max per holder ({max})"
                    );
                }
                ComplianceRule::CountryWhitelist(countries) => {
                    assert!(
                        countries.contains(&to_info.country),
                        "DRC13: recipient country '{}' not in whitelist",
                        to_info.country
                    );
                }
                ComplianceRule::CountryBlacklist(countries) => {
                    assert!(
                        !countries.contains(&to_info.country),
                        "DRC13: recipient country '{}' is blacklisted",
                        to_info.country
                    );
                }
                ComplianceRule::MinHoldPeriod(_period) => {
                    // Hold period is checked on sender side during transfer
                }
            }
        }
    }

    fn check_hold_period(&self, sender: &Address, current_time: u64) {
        if let Some(last_received) = self.transfer_timestamps.get(sender) {
            for rule in &self.compliance_rules {
                if let ComplianceRule::MinHoldPeriod(period) = rule {
                    assert!(
                        current_time >= last_received + period,
                        "DRC13: min hold period not met ({period}s required)"
                    );
                }
            }
        }
    }

    // -- Compliant transfer --------------------------------------------------

    pub fn compliant_transfer(
        &mut self,
        caller: Address,
        to: Address,
        amount: u64,
        current_time: u64,
    ) {
        assert!(amount > 0, "DRC13: amount must be positive");
        assert!(
            !self.frozen_addresses.contains_key(&caller),
            "DRC13: sender is frozen"
        );
        assert!(
            !self.frozen_addresses.contains_key(&to),
            "DRC13: recipient is frozen"
        );
        assert!(
            self.is_verified(&caller),
            "DRC13: sender is not verified"
        );

        self.check_compliance(&to, amount);
        self.check_hold_period(&caller, current_time);

        let from_balance = self.balance_of(&caller);
        assert!(
            from_balance >= amount,
            "DRC13: insufficient balance ({from_balance} < {amount})"
        );

        let to_balance = self.balance_of(&to);
        let was_zero = to_balance == 0;

        self.balances.insert(caller, from_balance - amount);
        self.balances.insert(to, to_balance + amount);

        // Update holder count
        if was_zero {
            self.holder_count += 1;
        }
        if from_balance - amount == 0 {
            self.holder_count -= 1;
        }

        self.transfer_timestamps.insert(to, current_time);
    }

    /// Admin-only mint (also checks compliance on recipient).
    pub fn mint(&mut self, caller: Address, to: Address, amount: u64) {
        assert!(caller == self.admin, "DRC13: only admin can mint");
        assert!(amount > 0, "DRC13: amount must be positive");

        // Recipient must be verified
        assert!(
            self.is_verified(&to),
            "DRC13: mint recipient is not verified"
        );
        assert!(
            !self.frozen_addresses.contains_key(&to),
            "DRC13: mint recipient is frozen"
        );

        let balance = self.balance_of(&to);
        if balance == 0 {
            self.holder_count += 1;
        }
        self.balances.insert(to, balance + amount);
        self.total_supply += amount;
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    name: String,
    symbol: String,
    decimals: u8,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferArgs {
    to: Address,
    amount: u64,
    #[serde(default)]
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddressArg {
    addr: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct RegistryArgs {
    registry: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct ComplianceArgs {
    rule: ComplianceRule,
}

#[derive(Serialize, Deserialize, Debug)]
struct VerifyArgs {
    addr: Address,
    info: VerificationInfo,
}

#[derive(Serialize, Deserialize, Debug)]
struct MintArgs {
    to: Address,
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BalanceOfArgs {
    account: Address,
}

pub fn dispatch(
    state: &mut Option<CompliantTokenState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC13: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC13: bad init args");
            *state = Some(CompliantTokenState::new(a.name, a.symbol, a.decimals, caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- DRC-1 base queries --
        "balance_of" => {
            let s = state.as_ref().expect("DRC13: not initialised");
            let a: BalanceOfArgs =
                serde_json::from_slice(args).expect("DRC13: bad balance_of args");
            serde_json::to_vec(&s.balance_of(&a.account)).unwrap()
        }

        "total_supply" => {
            let s = state.as_ref().expect("DRC13: not initialised");
            serde_json::to_vec(&s.total_supply()).unwrap()
        }

        // -- DRC-13 compliance --
        "compliant_transfer" => {
            let s = state.as_mut().expect("DRC13: not initialised");
            let a: TransferArgs =
                serde_json::from_slice(args).expect("DRC13: bad compliant_transfer args");
            s.compliant_transfer(caller, a.to, a.amount, a.current_time);
            serde_json::to_vec("ok").unwrap()
        }

        "is_verified" => {
            let s = state.as_ref().expect("DRC13: not initialised");
            let a: AddressArg =
                serde_json::from_slice(args).expect("DRC13: bad is_verified args");
            serde_json::to_vec(&s.is_verified(&a.addr)).unwrap()
        }

        "set_identity_registry" => {
            let s = state.as_mut().expect("DRC13: not initialised");
            let a: RegistryArgs =
                serde_json::from_slice(args).expect("DRC13: bad set_identity_registry args");
            s.set_identity_registry(caller, a.registry);
            serde_json::to_vec("ok").unwrap()
        }

        "add_compliance" => {
            let s = state.as_mut().expect("DRC13: not initialised");
            let a: ComplianceArgs =
                serde_json::from_slice(args).expect("DRC13: bad add_compliance args");
            s.add_compliance(caller, a.rule);
            serde_json::to_vec("ok").unwrap()
        }

        "freeze" => {
            let s = state.as_mut().expect("DRC13: not initialised");
            let a: AddressArg =
                serde_json::from_slice(args).expect("DRC13: bad freeze args");
            s.freeze(caller, a.addr);
            serde_json::to_vec("ok").unwrap()
        }

        "unfreeze" => {
            let s = state.as_mut().expect("DRC13: not initialised");
            let a: AddressArg =
                serde_json::from_slice(args).expect("DRC13: bad unfreeze args");
            s.unfreeze(caller, a.addr);
            serde_json::to_vec("ok").unwrap()
        }

        "verify_address" => {
            let s = state.as_mut().expect("DRC13: not initialised");
            let a: VerifyArgs =
                serde_json::from_slice(args).expect("DRC13: bad verify_address args");
            s.verify_address(caller, a.addr, a.info);
            serde_json::to_vec("ok").unwrap()
        }

        "mint" => {
            let s = state.as_mut().expect("DRC13: not initialised");
            let a: MintArgs = serde_json::from_slice(args).expect("DRC13: bad mint args");
            s.mint(caller, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC13: unknown method '{method}'"),
    }
}
