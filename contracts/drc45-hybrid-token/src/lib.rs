use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-45  Hybrid Fungible+NFT Token  (ERC-404 equivalent)
// Token that's both fungible AND NFT. When you hold enough fungible units,
// you automatically get an NFT. Transfers auto-mint/burn NFTs.
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HybridTokenState {
    pub name: String,
    pub symbol: String,
    pub owner: Address,
    pub total_supply: u64,
    /// Fungible balances per address.
    pub balances: BTreeMap<Address, u64>,
    /// How many fungible units = 1 NFT.
    pub units_per_nft: u64,
    /// NFT id -> owner.
    pub nft_owners: BTreeMap<u64, Address>,
    /// Owner -> list of NFT ids.
    pub nft_of_owner: BTreeMap<Address, Vec<u64>>,
    pub next_nft_id: u64,
}

impl HybridTokenState {
    pub fn new(name: String, symbol: String, units_per_nft: u64, owner: Address) -> Self {
        assert!(units_per_nft > 0, "DRC45: units_per_nft must be > 0");
        Self {
            name,
            symbol,
            owner,
            total_supply: 0,
            balances: BTreeMap::new(),
            units_per_nft,
            nft_owners: BTreeMap::new(),
            nft_of_owner: BTreeMap::new(),
            next_nft_id: 1,
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn balance_of(&self, account: &Address) -> u64 {
        self.balances.get(account).copied().unwrap_or(0)
    }

    pub fn owner_of_nft(&self, nft_id: u64) -> Option<&Address> {
        self.nft_owners.get(&nft_id)
    }

    pub fn nfts_of(&self, owner: &Address) -> Vec<u64> {
        self.nft_of_owner.get(owner).cloned().unwrap_or_default()
    }

    pub fn total_nfts(&self) -> usize {
        self.nft_owners.len()
    }

    /// How many NFTs this address should have based on fungible balance.
    fn expected_nft_count(&self, addr: &Address) -> u64 {
        self.balance_of(addr) / self.units_per_nft
    }

    // -- Internal NFT sync ---------------------------------------------------

    /// Sync NFTs for an address: mint or burn as needed to match fungible balance.
    fn sync_nfts(&mut self, addr: Address) {
        let expected = self.expected_nft_count(&addr) as usize;
        let current = self.nft_of_owner.get(&addr).map_or(0, |v| v.len());

        if expected > current {
            // Mint new NFTs
            let to_mint = expected - current;
            for _ in 0..to_mint {
                let id = self.next_nft_id;
                self.next_nft_id += 1;
                self.nft_owners.insert(id, addr);
                self.nft_of_owner.entry(addr).or_default().push(id);
            }
        } else if expected < current {
            // Burn excess NFTs (from the end)
            let to_burn = current - expected;
            for _ in 0..to_burn {
                let ids = self.nft_of_owner.get_mut(&addr).unwrap();
                let nft_id = ids.pop().unwrap();
                self.nft_owners.remove(&nft_id);
            }
            // Clean up empty vec
            if self.nft_of_owner.get(&addr).map_or(true, |v| v.is_empty()) {
                self.nft_of_owner.remove(&addr);
            }
        }
    }

    // -- Mutations -----------------------------------------------------------

    pub fn mint(&mut self, caller: Address, to: Address, amount: u64) {
        assert!(caller == self.owner, "DRC45: only owner can mint");
        assert!(amount > 0, "DRC45: mint amount must be positive");
        let bal = self.balance_of(&to);
        self.balances.insert(to, bal + amount);
        self.total_supply += amount;
        self.sync_nfts(to);
    }

    pub fn transfer(&mut self, caller: Address, to: Address, amount: u64) {
        assert!(amount > 0, "DRC45: transfer amount must be positive");
        let from_bal = self.balance_of(&caller);
        assert!(from_bal >= amount, "DRC45: insufficient balance");
        self.balances.insert(caller, from_bal - amount);
        let to_bal = self.balance_of(&to);
        self.balances.insert(to, to_bal + amount);
        self.sync_nfts(caller);
        self.sync_nfts(to);
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    name: String,
    symbol: String,
    units_per_nft: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct TransferArgs {
    to: Address,
    amount: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct MintArgs {
    to: Address,
    amount: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct AddrArg {
    account: Address,
}
#[derive(Serialize, Deserialize, Debug)]
struct NftIdArg {
    nft_id: u64,
}

pub fn dispatch(
    state: &mut Option<HybridTokenState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC45: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC45: bad init args");
            *state = Some(HybridTokenState::new(
                a.name,
                a.symbol,
                a.units_per_nft,
                caller,
            ));
            serde_json::to_vec("ok").unwrap()
        }
        "balance_of" => {
            let s = state.as_ref().expect("DRC45: not initialised");
            let a: AddrArg = serde_json::from_slice(args).expect("DRC45: bad args");
            serde_json::to_vec(&s.balance_of(&a.account)).unwrap()
        }
        "owner_of_nft" => {
            let s = state.as_ref().expect("DRC45: not initialised");
            let a: NftIdArg = serde_json::from_slice(args).expect("DRC45: bad args");
            serde_json::to_vec(&s.owner_of_nft(a.nft_id)).unwrap()
        }
        "nfts_of" => {
            let s = state.as_ref().expect("DRC45: not initialised");
            let a: AddrArg = serde_json::from_slice(args).expect("DRC45: bad args");
            serde_json::to_vec(&s.nfts_of(&a.account)).unwrap()
        }
        "total_nfts" => {
            let s = state.as_ref().expect("DRC45: not initialised");
            serde_json::to_vec(&s.total_nfts()).unwrap()
        }
        "total_supply" => {
            let s = state.as_ref().expect("DRC45: not initialised");
            serde_json::to_vec(&s.total_supply).unwrap()
        }
        "mint" => {
            let s = state.as_mut().expect("DRC45: not initialised");
            let a: MintArgs = serde_json::from_slice(args).expect("DRC45: bad args");
            s.mint(caller, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer" => {
            let s = state.as_mut().expect("DRC45: not initialised");
            let a: TransferArgs = serde_json::from_slice(args).expect("DRC45: bad args");
            s.transfer(caller, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        _ => panic!("DRC45: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(n: u8) -> Address {
        [n; 32]
    }

    fn setup() -> Option<HybridTokenState> {
        let mut state = None;
        let args = serde_json::to_vec(&InitArgs {
            name: "HybridCoin".into(),
            symbol: "HYB".into(),
            units_per_nft: 100,
        })
        .unwrap();
        dispatch(&mut state, "init", &args, addr(1));
        state
    }

    #[test]
    fn test_mint_auto_creates_nfts() {
        let mut state = setup();
        // Mint 250 units => 2 NFTs (250 / 100 = 2)
        let args = serde_json::to_vec(&MintArgs {
            to: addr(2),
            amount: 250,
        })
        .unwrap();
        dispatch(&mut state, "mint", &args, addr(1));
        let s = state.as_ref().unwrap();
        assert_eq!(s.balance_of(&addr(2)), 250);
        assert_eq!(s.nfts_of(&addr(2)).len(), 2);
        assert_eq!(s.total_nfts(), 2);
    }

    #[test]
    fn test_transfer_burns_and_mints_nfts() {
        let mut state = setup();
        // Give addr(1) 300 tokens => 3 NFTs
        let mint = serde_json::to_vec(&MintArgs {
            to: addr(1),
            amount: 300,
        })
        .unwrap();
        dispatch(&mut state, "mint", &mint, addr(1));
        assert_eq!(state.as_ref().unwrap().nfts_of(&addr(1)).len(), 3);

        // Transfer 150 to addr(2) => addr(1) has 150 (1 NFT), addr(2) has 150 (1 NFT)
        let xfer = serde_json::to_vec(&TransferArgs {
            to: addr(2),
            amount: 150,
        })
        .unwrap();
        dispatch(&mut state, "transfer", &xfer, addr(1));
        let s = state.as_ref().unwrap();
        assert_eq!(s.balance_of(&addr(1)), 150);
        assert_eq!(s.nfts_of(&addr(1)).len(), 1);
        assert_eq!(s.balance_of(&addr(2)), 150);
        assert_eq!(s.nfts_of(&addr(2)).len(), 1);
        assert_eq!(s.total_nfts(), 2);
    }

    #[test]
    fn test_sub_unit_no_nft() {
        let mut state = setup();
        // Mint 50 (less than units_per_nft=100) => 0 NFTs
        let args = serde_json::to_vec(&MintArgs {
            to: addr(3),
            amount: 50,
        })
        .unwrap();
        dispatch(&mut state, "mint", &args, addr(1));
        let s = state.as_ref().unwrap();
        assert_eq!(s.nfts_of(&addr(3)).len(), 0);
    }

    #[test]
    fn test_nft_ownership_tracking() {
        let mut state = setup();
        let args = serde_json::to_vec(&MintArgs {
            to: addr(4),
            amount: 100,
        })
        .unwrap();
        dispatch(&mut state, "mint", &args, addr(1));
        let s = state.as_ref().unwrap();
        let nfts = s.nfts_of(&addr(4));
        assert_eq!(nfts.len(), 1);
        let nft_id = nfts[0];
        assert_eq!(s.owner_of_nft(nft_id), Some(&addr(4)));
    }

    #[test]
    #[should_panic(expected = "insufficient balance")]
    fn test_transfer_insufficient() {
        let mut state = setup();
        let mint = serde_json::to_vec(&MintArgs {
            to: addr(1),
            amount: 50,
        })
        .unwrap();
        dispatch(&mut state, "mint", &mint, addr(1));
        let xfer = serde_json::to_vec(&TransferArgs {
            to: addr(2),
            amount: 100,
        })
        .unwrap();
        dispatch(&mut state, "transfer", &xfer, addr(1));
    }
}
