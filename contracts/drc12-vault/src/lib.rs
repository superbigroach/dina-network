use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-12  Tokenized Vault  (ERC-4626 equivalent)
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VaultState {
    pub total_shares: u64,
    pub total_assets: u64,
    pub shares: BTreeMap<Address, u64>,
    pub deposit_timestamps: BTreeMap<Address, u64>,
    pub admin: Address,
}

impl VaultState {
    pub fn new(admin: Address) -> Self {
        Self {
            total_shares: 0,
            total_assets: 0,
            shares: BTreeMap::new(),
            deposit_timestamps: BTreeMap::new(),
            admin,
        }
    }

    /// Convert an asset amount to shares using current ratio.
    pub fn convert_to_shares(&self, assets: u64) -> u64 {
        if self.total_shares == 0 || self.total_assets == 0 {
            return assets; // 1:1 on first deposit
        }
        (assets as u128 * self.total_shares as u128 / self.total_assets as u128) as u64
    }

    /// Convert a share amount to assets using current ratio.
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
    pub fn preview_redeem(&self, shares: u64) -> u64 {
        self.convert_to_assets(shares)
    }

    pub fn total_assets(&self) -> u64 {
        self.total_assets
    }

    pub fn balance_of(&self, owner: &Address) -> u64 {
        self.shares.get(owner).copied().unwrap_or(0)
    }

    /// Deposit assets and mint shares to the receiver.
    pub fn deposit(&mut self, amount: u64, receiver: Address, timestamp: u64) -> u64 {
        assert!(amount > 0, "DRC12: deposit amount must be positive");
        let shares_minted = self.convert_to_shares(amount);
        assert!(shares_minted > 0, "DRC12: zero shares minted");

        self.total_assets += amount;
        self.total_shares += shares_minted;

        let current = self.balance_of(&receiver);
        self.shares.insert(receiver, current + shares_minted);
        self.deposit_timestamps.insert(receiver, timestamp);

        shares_minted
    }

    /// Withdraw assets by burning shares from the owner.
    pub fn withdraw(
        &mut self,
        amount: u64,
        receiver: Address,
        owner: Address,
        caller: Address,
    ) -> u64 {
        assert!(amount > 0, "DRC12: withdraw amount must be positive");
        assert!(
            caller == owner,
            "DRC12: only owner can withdraw their shares"
        );

        let shares_needed = self.convert_to_shares(amount);
        assert!(shares_needed > 0, "DRC12: zero shares to burn");

        let owner_shares = self.balance_of(&owner);
        assert!(
            owner_shares >= shares_needed,
            "DRC12: insufficient shares ({owner_shares} < {shares_needed})"
        );

        self.shares.insert(owner, owner_shares - shares_needed);
        self.total_shares -= shares_needed;
        self.total_assets -= amount;

        // In a real vault the assets would be transferred to `receiver` here.
        let _ = receiver;

        shares_needed
    }

    /// Admin-only: add yield to the vault, increasing total_assets without
    /// minting new shares. This makes each share worth more.
    pub fn add_yield(&mut self, caller: Address, amount: u64) {
        assert!(caller == self.admin, "DRC12: only admin can add yield");
        assert!(amount > 0, "DRC12: yield amount must be positive");
        self.total_assets += amount;
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs {
    amount: u64,
    receiver: Address,
    #[serde(default)]
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct WithdrawArgs {
    amount: u64,
    receiver: Address,
    owner: Address,
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
struct OwnerArgs {
    owner: Address,
}

pub fn dispatch(
    state: &mut Option<VaultState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC12: already initialised");
            *state = Some(VaultState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "deposit" => {
            let s = state.as_mut().expect("DRC12: not initialised");
            let a: DepositArgs =
                serde_json::from_slice(args).expect("DRC12: bad deposit args");
            let minted = s.deposit(a.amount, a.receiver, a.timestamp);
            serde_json::to_vec(&minted).unwrap()
        }

        "withdraw" => {
            let s = state.as_mut().expect("DRC12: not initialised");
            let a: WithdrawArgs =
                serde_json::from_slice(args).expect("DRC12: bad withdraw args");
            let burned = s.withdraw(a.amount, a.receiver, a.owner, caller);
            serde_json::to_vec(&burned).unwrap()
        }

        "preview_deposit" => {
            let s = state.as_ref().expect("DRC12: not initialised");
            let a: AmountArgs =
                serde_json::from_slice(args).expect("DRC12: bad preview_deposit args");
            serde_json::to_vec(&s.preview_deposit(a.amount)).unwrap()
        }

        "preview_redeem" => {
            let s = state.as_ref().expect("DRC12: not initialised");
            let a: SharesArgs =
                serde_json::from_slice(args).expect("DRC12: bad preview_redeem args");
            serde_json::to_vec(&s.preview_redeem(a.shares)).unwrap()
        }

        "total_assets" => {
            let s = state.as_ref().expect("DRC12: not initialised");
            serde_json::to_vec(&s.total_assets()).unwrap()
        }

        "convert_to_shares" => {
            let s = state.as_ref().expect("DRC12: not initialised");
            let a: AmountArgs =
                serde_json::from_slice(args).expect("DRC12: bad convert_to_shares args");
            serde_json::to_vec(&s.convert_to_shares(a.amount)).unwrap()
        }

        "convert_to_assets" => {
            let s = state.as_ref().expect("DRC12: not initialised");
            let a: SharesArgs =
                serde_json::from_slice(args).expect("DRC12: bad convert_to_assets args");
            serde_json::to_vec(&s.convert_to_assets(a.shares)).unwrap()
        }

        "balance_of" => {
            let s = state.as_ref().expect("DRC12: not initialised");
            let a: OwnerArgs =
                serde_json::from_slice(args).expect("DRC12: bad balance_of args");
            serde_json::to_vec(&s.balance_of(&a.owner)).unwrap()
        }

        "add_yield" => {
            let s = state.as_mut().expect("DRC12: not initialised");
            let a: AmountArgs =
                serde_json::from_slice(args).expect("DRC12: bad add_yield args");
            s.add_yield(caller, a.amount);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC12: unknown method '{method}'"),
    }
}
