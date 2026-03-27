use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// DRC-23  Price Oracle
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PriceFeed {
    pub pair: String,
    pub price: u64,       // 8 decimal places (1.00 = 100_000_000)
    pub updated_at: u64,
    pub reporter: Address,
    pub confidence: u64,  // 8 decimal places
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OracleState {
    pub owner: Address,
    pub feeds: BTreeMap<String, PriceFeed>,
    pub authorized_reporters: BTreeSet<Address>,
}

impl OracleState {
    pub fn new(owner: Address) -> Self {
        let mut authorized_reporters = BTreeSet::new();
        authorized_reporters.insert(owner);
        Self {
            owner,
            feeds: BTreeMap::new(),
            authorized_reporters,
        }
    }

    pub fn add_reporter(&mut self, caller: Address, reporter: Address) {
        assert!(caller == self.owner, "DRC23: only owner can add reporters");
        self.authorized_reporters.insert(reporter);
    }

    pub fn remove_reporter(&mut self, caller: Address, reporter: Address) {
        assert!(caller == self.owner, "DRC23: only owner can remove reporters");
        assert!(reporter != self.owner, "DRC23: cannot remove owner as reporter");
        self.authorized_reporters.remove(&reporter);
    }

    pub fn update_price(
        &mut self,
        caller: Address,
        pair: String,
        price: u64,
        timestamp: u64,
        confidence: u64,
    ) {
        assert!(
            self.authorized_reporters.contains(&caller),
            "DRC23: caller is not an authorized reporter"
        );
        assert!(price > 0, "DRC23: price must be positive");
        assert!(confidence > 0, "DRC23: confidence must be positive");

        // Reject stale updates
        if let Some(existing) = self.feeds.get(&pair) {
            assert!(
                timestamp >= existing.updated_at,
                "DRC23: cannot submit stale price (existing timestamp is newer)"
            );
        }

        self.feeds.insert(
            pair.clone(),
            PriceFeed {
                pair,
                price,
                updated_at: timestamp,
                reporter: caller,
                confidence,
            },
        );
    }

    pub fn get_price(&self, pair: &str) -> Option<&PriceFeed> {
        self.feeds.get(pair)
    }

    pub fn get_latest_prices(&self) -> Vec<&PriceFeed> {
        self.feeds.values().collect()
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct UpdatePriceArgs {
    pair: String,
    price: u64,
    timestamp: u64,
    confidence: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetPriceArgs {
    pair: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReporterArgs {
    reporter: Address,
}

/// Contract-level dispatch.
pub fn dispatch(
    state: &mut Option<OracleState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC23: already initialised");
            *state = Some(OracleState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "update_price" => {
            let s = state.as_mut().expect("DRC23: not initialised");
            let a: UpdatePriceArgs =
                serde_json::from_slice(args).expect("DRC23: bad update_price args");
            s.update_price(caller, a.pair, a.price, a.timestamp, a.confidence);
            serde_json::to_vec("ok").unwrap()
        }

        "get_price" => {
            let s = state.as_ref().expect("DRC23: not initialised");
            let a: GetPriceArgs =
                serde_json::from_slice(args).expect("DRC23: bad get_price args");
            match s.get_price(&a.pair) {
                Some(feed) => serde_json::to_vec(feed).unwrap(),
                None => serde_json::to_vec(&Option::<PriceFeed>::None).unwrap(),
            }
        }

        "get_latest_prices" => {
            let s = state.as_ref().expect("DRC23: not initialised");
            let prices: Vec<&PriceFeed> = s.get_latest_prices();
            serde_json::to_vec(&prices).unwrap()
        }

        "add_reporter" => {
            let s = state.as_mut().expect("DRC23: not initialised");
            let a: ReporterArgs =
                serde_json::from_slice(args).expect("DRC23: bad add_reporter args");
            s.add_reporter(caller, a.reporter);
            serde_json::to_vec("ok").unwrap()
        }

        "remove_reporter" => {
            let s = state.as_mut().expect("DRC23: not initialised");
            let a: ReporterArgs =
                serde_json::from_slice(args).expect("DRC23: bad remove_reporter args");
            s.remove_reporter(caller, a.reporter);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC23: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: Address = [1u8; 32];
    const REPORTER: Address = [2u8; 32];
    const NOBODY: Address = [3u8; 32];

    fn init_oracle() -> Option<OracleState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", OWNER);
        state
    }

    #[test]
    fn test_init_and_owner_can_report() {
        let mut state = init_oracle();
        let args = serde_json::to_vec(&UpdatePriceArgs {
            pair: "BTC/USD".into(),
            price: 5_000_000_000_000, // $50,000
            timestamp: 1000,
            confidence: 99_000_000,
        })
        .unwrap();
        dispatch(&mut state, "update_price", &args, OWNER);

        let query = serde_json::to_vec(&GetPriceArgs {
            pair: "BTC/USD".into(),
        })
        .unwrap();
        let result = dispatch(&mut state, "get_price", &query, NOBODY);
        let feed: PriceFeed = serde_json::from_slice(&result).unwrap();
        assert_eq!(feed.price, 5_000_000_000_000);
        assert_eq!(feed.reporter, OWNER);
    }

    #[test]
    fn test_add_reporter_and_report() {
        let mut state = init_oracle();
        let add_args = serde_json::to_vec(&ReporterArgs { reporter: REPORTER }).unwrap();
        dispatch(&mut state, "add_reporter", &add_args, OWNER);

        let price_args = serde_json::to_vec(&UpdatePriceArgs {
            pair: "ETH/USD".into(),
            price: 300_000_000_000, // $3,000
            timestamp: 2000,
            confidence: 95_000_000,
        })
        .unwrap();
        dispatch(&mut state, "update_price", &price_args, REPORTER);

        let query = serde_json::to_vec(&GetPriceArgs {
            pair: "ETH/USD".into(),
        })
        .unwrap();
        let result = dispatch(&mut state, "get_price", &query, NOBODY);
        let feed: PriceFeed = serde_json::from_slice(&result).unwrap();
        assert_eq!(feed.reporter, REPORTER);
    }

    #[test]
    #[should_panic(expected = "caller is not an authorized reporter")]
    fn test_unauthorized_reporter_rejected() {
        let mut state = init_oracle();
        let args = serde_json::to_vec(&UpdatePriceArgs {
            pair: "BTC/USD".into(),
            price: 5_000_000_000_000,
            timestamp: 1000,
            confidence: 99_000_000,
        })
        .unwrap();
        dispatch(&mut state, "update_price", &args, NOBODY);
    }

    #[test]
    fn test_remove_reporter() {
        let mut state = init_oracle();
        let add_args = serde_json::to_vec(&ReporterArgs { reporter: REPORTER }).unwrap();
        dispatch(&mut state, "add_reporter", &add_args, OWNER);

        let remove_args = serde_json::to_vec(&ReporterArgs { reporter: REPORTER }).unwrap();
        dispatch(&mut state, "remove_reporter", &remove_args, OWNER);

        let s = state.as_ref().unwrap();
        assert!(!s.authorized_reporters.contains(&REPORTER));
    }

    #[test]
    fn test_get_latest_prices_multiple_feeds() {
        let mut state = init_oracle();

        for (pair, price, ts) in [
            ("BTC/USD", 5_000_000_000_000u64, 1000u64),
            ("ETH/USD", 300_000_000_000, 1001),
            ("DINA/USD", 1_000_000, 1002),
        ] {
            let args = serde_json::to_vec(&UpdatePriceArgs {
                pair: pair.into(),
                price,
                timestamp: ts,
                confidence: 99_000_000,
            })
            .unwrap();
            dispatch(&mut state, "update_price", &args, OWNER);
        }

        let result = dispatch(&mut state, "get_latest_prices", b"", NOBODY);
        let feeds: Vec<PriceFeed> = serde_json::from_slice(&result).unwrap();
        assert_eq!(feeds.len(), 3);
    }

    #[test]
    #[should_panic(expected = "cannot submit stale price")]
    fn test_stale_price_rejected() {
        let mut state = init_oracle();
        let args1 = serde_json::to_vec(&UpdatePriceArgs {
            pair: "BTC/USD".into(),
            price: 5_000_000_000_000,
            timestamp: 2000,
            confidence: 99_000_000,
        })
        .unwrap();
        dispatch(&mut state, "update_price", &args1, OWNER);

        let args2 = serde_json::to_vec(&UpdatePriceArgs {
            pair: "BTC/USD".into(),
            price: 4_900_000_000_000,
            timestamp: 1000, // older timestamp
            confidence: 99_000_000,
        })
        .unwrap();
        dispatch(&mut state, "update_price", &args2, OWNER);
    }
}
