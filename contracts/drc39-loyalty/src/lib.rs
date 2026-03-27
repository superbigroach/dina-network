use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-39  Loyalty Points Program
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Program {
    pub id: u64,
    pub business: Address,
    pub name: String,
    pub points_per_usdc: u64, // points earned per 1 USDC spent
    pub redemption_rate: u64, // how many points = 1 USDC of value (e.g., 100)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoyaltyState {
    pub admin: Address,
    pub programs: BTreeMap<u64, Program>,
    pub balances: BTreeMap<(u64, Address), u64>, // (program_id, customer) -> points
    pub next_id: u64,
}

impl LoyaltyState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            programs: BTreeMap::new(),
            balances: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn create_program(
        &mut self,
        caller: Address,
        name: String,
        points_per_usdc: u64,
        redemption_rate: u64,
    ) -> u64 {
        assert!(!name.is_empty(), "DRC39: name cannot be empty");
        assert!(points_per_usdc > 0, "DRC39: points_per_usdc must be positive");
        assert!(
            redemption_rate > 0,
            "DRC39: redemption_rate must be positive"
        );
        let id = self.next_id;
        self.next_id += 1;
        let program = Program {
            id,
            business: caller,
            name,
            points_per_usdc,
            redemption_rate,
        };
        self.programs.insert(id, program);
        id
    }

    pub fn earn_points(
        &mut self,
        caller: Address,
        program_id: u64,
        customer: Address,
        spend_amount: u64,
    ) {
        let program = self
            .programs
            .get(&program_id)
            .expect("DRC39: program not found");
        assert!(
            program.business == caller,
            "DRC39: only business can issue points"
        );
        let points = spend_amount * program.points_per_usdc;
        let balance = self
            .balances
            .entry((program_id, customer))
            .or_insert(0);
        *balance += points;
    }

    pub fn redeem_points(
        &mut self,
        caller: Address,
        program_id: u64,
        points: u64,
    ) -> u64 {
        assert!(points > 0, "DRC39: points must be positive");
        let _program = self
            .programs
            .get(&program_id)
            .expect("DRC39: program not found");
        let balance = self
            .balances
            .get_mut(&(program_id, caller))
            .expect("DRC39: no balance");
        assert!(*balance >= points, "DRC39: insufficient points");
        *balance -= points;
        let usdc_value = points / _program.redemption_rate;
        usdc_value
    }

    pub fn balance(&self, program_id: u64, customer: &Address) -> u64 {
        self.balances
            .get(&(program_id, *customer))
            .copied()
            .unwrap_or(0)
    }

    pub fn transfer_points(
        &mut self,
        caller: Address,
        program_id: u64,
        to: Address,
        points: u64,
    ) {
        assert!(points > 0, "DRC39: points must be positive");
        let _ = self
            .programs
            .get(&program_id)
            .expect("DRC39: program not found");
        let from_bal = self
            .balances
            .get_mut(&(program_id, caller))
            .expect("DRC39: no balance");
        assert!(*from_bal >= points, "DRC39: insufficient points");
        *from_bal -= points;
        let to_bal = self
            .balances
            .entry((program_id, to))
            .or_insert(0);
        *to_bal += points;
    }

    pub fn program_info(&self, id: u64) -> Option<&Program> {
        self.programs.get(&id)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateProgramArgs {
    name: String,
    points_per_usdc: u64,
    redemption_rate: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct EarnPointsArgs {
    program_id: u64,
    customer: Address,
    spend_amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct RedeemPointsArgs {
    program_id: u64,
    points: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BalanceArgs {
    program_id: u64,
    customer: Address,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferPointsArgs {
    program_id: u64,
    to: Address,
    points: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProgramInfoArgs {
    id: u64,
}

pub fn dispatch(
    state: &mut Option<LoyaltyState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC39: already initialised");
            *state = Some(LoyaltyState::new(caller));
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
            s.earn_points(caller, a.program_id, a.customer, a.spend_amount);
            serde_json::to_vec("ok").unwrap()
        }
        "redeem_points" => {
            let s = state.as_mut().expect("DRC39: not initialised");
            let a: RedeemPointsArgs =
                serde_json::from_slice(args).expect("DRC39: bad redeem_points args");
            let value = s.redeem_points(caller, a.program_id, a.points);
            serde_json::to_vec(&value).unwrap()
        }
        "balance" => {
            let s = state.as_ref().expect("DRC39: not initialised");
            let a: BalanceArgs =
                serde_json::from_slice(args).expect("DRC39: bad balance args");
            serde_json::to_vec(&s.balance(a.program_id, &a.customer)).unwrap()
        }
        "transfer_points" => {
            let s = state.as_mut().expect("DRC39: not initialised");
            let a: TransferPointsArgs =
                serde_json::from_slice(args).expect("DRC39: bad transfer_points args");
            s.transfer_points(caller, a.program_id, a.to, a.points);
            serde_json::to_vec("ok").unwrap()
        }
        "program_info" => {
            let s = state.as_ref().expect("DRC39: not initialised");
            let a: ProgramInfoArgs =
                serde_json::from_slice(args).expect("DRC39: bad program_info args");
            serde_json::to_vec(&s.program_info(a.id)).unwrap()
        }
        _ => panic!("DRC39: unknown method '{method}'"),
    }
}
