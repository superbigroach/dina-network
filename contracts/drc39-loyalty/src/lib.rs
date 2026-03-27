use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-39  Loyalty Points
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];
pub type ProgramId = u64;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoyaltyProgram {
    pub id: ProgramId,
    pub business: Address,
    pub name: String,
    pub points_per_usdc: u64,
    pub redemption_rate: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoyaltyState {
    pub next_id: ProgramId,
    pub programs: BTreeMap<ProgramId, LoyaltyProgram>,
    pub balances: BTreeMap<(ProgramId, Address), u64>,
}

impl LoyaltyState {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            programs: BTreeMap::new(),
            balances: BTreeMap::new(),
        }
    }

    pub fn create_program(
        &mut self,
        caller: Address,
        name: String,
        points_per_usdc: u64,
        redemption_rate: u64,
    ) -> ProgramId {
        assert!(points_per_usdc > 0, "DRC39: points_per_usdc must be positive");
        assert!(redemption_rate > 0, "DRC39: redemption_rate must be positive");
        let id = self.next_id;
        self.next_id += 1;
        self.programs.insert(
            id,
            LoyaltyProgram {
                id,
                business: caller,
                name,
                points_per_usdc,
                redemption_rate,
            },
        );
        id
    }

    pub fn earn_points(
        &mut self,
        caller: Address,
        program_id: ProgramId,
        user: Address,
        usdc_amount: u64,
    ) {
        let program = self.programs.get(&program_id).expect("DRC39: program not found");
        assert!(program.business == caller, "DRC39: only business can award points");
        let points = usdc_amount * program.points_per_usdc;
        let balance = self.balances.get(&(program_id, user)).copied().unwrap_or(0);
        self.balances.insert((program_id, user), balance + points);
    }

    pub fn redeem_points(
        &mut self,
        caller: Address,
        program_id: ProgramId,
        points: u64,
    ) -> u64 {
        let program = self.programs.get(&program_id).expect("DRC39: program not found");
        assert!(points > 0, "DRC39: must redeem positive points");
        let balance = self.balances.get(&(program_id, caller)).copied().unwrap_or(0);
        assert!(balance >= points, "DRC39: insufficient points ({balance} < {points})");
        self.balances.insert((program_id, caller), balance - points);
        points / program.redemption_rate
    }

    pub fn balance(&self, program_id: ProgramId, user: Address) -> u64 {
        self.balances.get(&(program_id, user)).copied().unwrap_or(0)
    }

    pub fn transfer_points(
        &mut self,
        caller: Address,
        program_id: ProgramId,
        to: Address,
        amount: u64,
    ) {
        assert!(amount > 0, "DRC39: transfer amount must be positive");
        let _ = self.programs.get(&program_id).expect("DRC39: program not found");
        let from_balance = self.balance(program_id, caller);
        assert!(
            from_balance >= amount,
            "DRC39: insufficient points for transfer ({from_balance} < {amount})"
        );
        self.balances.insert((program_id, caller), from_balance - amount);
        let to_balance = self.balance(program_id, to);
        self.balances.insert((program_id, to), to_balance + amount);
    }

    pub fn program_info(&self, program_id: ProgramId) -> &LoyaltyProgram {
        self.programs.get(&program_id).expect("DRC39: program not found")
    }
}

// ---------------------------------------------------------------------------
// Dispatch args
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateProgramArgs {
    name: String,
    points_per_usdc: u64,
    redemption_rate: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct EarnPointsArgs {
    program_id: ProgramId,
    user: Address,
    usdc_amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct RedeemPointsArgs {
    program_id: ProgramId,
    points: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BalanceArgs {
    program_id: ProgramId,
    user: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferPointsArgs {
    program_id: ProgramId,
    to: Address,
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProgramInfoArgs {
    program_id: ProgramId,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(
    state: &mut Option<LoyaltyState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC39: already initialised");
            *state = Some(LoyaltyState::new());
            serde_json::to_vec("ok").unwrap()
        }

        "create_program" => {
            let s = state.as_mut().expect("DRC39: not initialised");
            let a: CreateProgramArgs =
                serde_json::from_slice(args).expect("DRC39: bad create_program args");
            let id = s.create_program(caller, a.name, a.points_per_usdc, a.redemption_rate);
            serde_json::to_vec(&id).unwrap()
        }

        "earn_points" => {
            let s = state.as_mut().expect("DRC39: not initialised");
            let a: EarnPointsArgs =
                serde_json::from_slice(args).expect("DRC39: bad earn_points args");
            s.earn_points(caller, a.program_id, a.user, a.usdc_amount);
            serde_json::to_vec("ok").unwrap()
        }

        "redeem_points" => {
            let s = state.as_mut().expect("DRC39: not initialised");
            let a: RedeemPointsArgs =
                serde_json::from_slice(args).expect("DRC39: bad redeem_points args");
            let usdc = s.redeem_points(caller, a.program_id, a.points);
            serde_json::to_vec(&usdc).unwrap()
        }

        "balance" => {
            let s = state.as_ref().expect("DRC39: not initialised");
            let a: BalanceArgs =
                serde_json::from_slice(args).expect("DRC39: bad balance args");
            serde_json::to_vec(&s.balance(a.program_id, a.user)).unwrap()
        }

        "transfer_points" => {
            let s = state.as_mut().expect("DRC39: not initialised");
            let a: TransferPointsArgs =
                serde_json::from_slice(args).expect("DRC39: bad transfer_points args");
            s.transfer_points(caller, a.program_id, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }

        "program_info" => {
            let s = state.as_ref().expect("DRC39: not initialised");
            let a: ProgramInfoArgs =
                serde_json::from_slice(args).expect("DRC39: bad program_info args");
            let info = s.program_info(a.program_id);
            serde_json::to_vec(info).unwrap()
        }

        _ => panic!("DRC39: unknown method '{method}'"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const BUSINESS: Address = [1u8; 32];
    const ALICE: Address = [2u8; 32];
    const BOB: Address = [3u8; 32];

    fn init() -> Option<LoyaltyState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", BUSINESS);
        state
    }

    fn create(state: &mut Option<LoyaltyState>, name: &str, ppu: u64, rr: u64) -> ProgramId {
        let args = serde_json::to_vec(&CreateProgramArgs {
            name: name.to_string(),
            points_per_usdc: ppu,
            redemption_rate: rr,
        })
        .unwrap();
        let result = dispatch(state, "create_program", &args, BUSINESS);
        serde_json::from_slice(&result).unwrap()
    }

    #[test]
    fn test_create_program_and_info() {
        let mut state = init();
        let id = create(&mut state, "CoffeeRewards", 10, 100);
        assert_eq!(id, 1);

        let args = serde_json::to_vec(&ProgramInfoArgs { program_id: id }).unwrap();
        let result = dispatch(&mut state, "program_info", &args, BUSINESS);
        let info: LoyaltyProgram = serde_json::from_slice(&result).unwrap();
        assert_eq!(info.name, "CoffeeRewards");
        assert_eq!(info.points_per_usdc, 10);
        assert_eq!(info.redemption_rate, 100);
    }

    #[test]
    fn test_earn_and_check_balance() {
        let mut state = init();
        let id = create(&mut state, "Shop", 5, 50);

        let earn = serde_json::to_vec(&EarnPointsArgs {
            program_id: id,
            user: ALICE,
            usdc_amount: 20,
        })
        .unwrap();
        dispatch(&mut state, "earn_points", &earn, BUSINESS);

        let bal_args = serde_json::to_vec(&BalanceArgs {
            program_id: id,
            user: ALICE,
        })
        .unwrap();
        let result = dispatch(&mut state, "balance", &bal_args, BUSINESS);
        let balance: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(balance, 100); // 20 * 5
    }

    #[test]
    fn test_redeem_points() {
        let mut state = init();
        let id = create(&mut state, "Gas", 10, 100);

        let earn = serde_json::to_vec(&EarnPointsArgs {
            program_id: id,
            user: ALICE,
            usdc_amount: 100,
        })
        .unwrap();
        dispatch(&mut state, "earn_points", &earn, BUSINESS);

        // Redeem 500 points => 500/100 = 5 USDC
        let redeem = serde_json::to_vec(&RedeemPointsArgs {
            program_id: id,
            points: 500,
        })
        .unwrap();
        let result = dispatch(&mut state, "redeem_points", &redeem, ALICE);
        let usdc: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(usdc, 5);
        assert_eq!(state.as_ref().unwrap().balance(id, ALICE), 500);
    }

    #[test]
    fn test_transfer_points() {
        let mut state = init();
        let id = create(&mut state, "Air", 1, 10);

        let earn = serde_json::to_vec(&EarnPointsArgs {
            program_id: id,
            user: ALICE,
            usdc_amount: 200,
        })
        .unwrap();
        dispatch(&mut state, "earn_points", &earn, BUSINESS);

        let xfer = serde_json::to_vec(&TransferPointsArgs {
            program_id: id,
            to: BOB,
            amount: 80,
        })
        .unwrap();
        dispatch(&mut state, "transfer_points", &xfer, ALICE);

        let s = state.as_ref().unwrap();
        assert_eq!(s.balance(id, ALICE), 120);
        assert_eq!(s.balance(id, BOB), 80);
    }

    #[test]
    #[should_panic(expected = "insufficient points")]
    fn test_redeem_insufficient() {
        let mut state = init();
        let id = create(&mut state, "X", 1, 10);

        let redeem = serde_json::to_vec(&RedeemPointsArgs {
            program_id: id,
            points: 999,
        })
        .unwrap();
        dispatch(&mut state, "redeem_points", &redeem, ALICE);
    }

    #[test]
    #[should_panic(expected = "only business")]
    fn test_earn_unauthorized() {
        let mut state = init();
        let id = create(&mut state, "X", 1, 10);

        let earn = serde_json::to_vec(&EarnPointsArgs {
            program_id: id,
            user: ALICE,
            usdc_amount: 10,
        })
        .unwrap();
        dispatch(&mut state, "earn_points", &earn, ALICE);
    }
}
