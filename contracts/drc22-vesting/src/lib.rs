use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-22  Token Vesting
// ---------------------------------------------------------------------------

pub type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VestingSchedule {
    pub beneficiary: Address,
    pub total_amount: u64,
    pub released: u64,
    pub start_time: u64,
    pub duration: u64,
    pub cliff_duration: u64,
    pub revocable: bool,
    pub revoked: bool,
}

impl VestingSchedule {
    /// Amount vested at a given timestamp.
    pub fn vested_amount(&self, current_time: u64) -> u64 {
        if self.revoked {
            return self.released;
        }
        if current_time < self.start_time + self.cliff_duration {
            return 0;
        }
        let elapsed = current_time - self.start_time;
        if elapsed >= self.duration {
            self.total_amount
        } else {
            (self.total_amount as u128 * elapsed as u128 / self.duration as u128) as u64
        }
    }

    /// Amount that can be released right now.
    pub fn releasable_amount(&self, current_time: u64) -> u64 {
        self.vested_amount(current_time).saturating_sub(self.released)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VestingState {
    pub admin: Address,
    pub vestings: BTreeMap<u64, VestingSchedule>,
    pub next_id: u64,
}

impl VestingState {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            vestings: BTreeMap::new(),
            next_id: 1,
        }
    }

    /// Create a new vesting schedule. Returns the vesting id.
    #[allow(clippy::too_many_arguments)]
    pub fn create_vesting(
        &mut self,
        caller: Address,
        beneficiary: Address,
        total_amount: u64,
        start_time: u64,
        duration: u64,
        cliff_duration: u64,
        revocable: bool,
    ) -> u64 {
        assert!(caller == self.admin, "DRC22: only admin can create vesting");
        assert!(total_amount > 0, "DRC22: total amount must be positive");
        assert!(duration > 0, "DRC22: duration must be positive");
        assert!(
            cliff_duration <= duration,
            "DRC22: cliff ({cliff_duration}) exceeds duration ({duration})"
        );

        let id = self.next_id;
        self.next_id += 1;

        let schedule = VestingSchedule {
            beneficiary,
            total_amount,
            released: 0,
            start_time,
            duration,
            cliff_duration,
            revocable,
            revoked: false,
        };
        self.vestings.insert(id, schedule);
        id
    }

    /// Release vested tokens for a schedule. Caller must be the beneficiary.
    pub fn release(&mut self, caller: Address, id: u64, current_time: u64) -> u64 {
        let schedule = self
            .vestings
            .get_mut(&id)
            .expect("DRC22: vesting not found");
        assert!(
            caller == schedule.beneficiary,
            "DRC22: only beneficiary can release"
        );
        assert!(!schedule.revoked, "DRC22: vesting has been revoked");

        let releasable = schedule.releasable_amount(current_time);
        assert!(releasable > 0, "DRC22: nothing to release");
        schedule.released += releasable;
        releasable
    }

    /// Revoke a vesting schedule. Only admin, only if revocable.
    pub fn revoke(&mut self, caller: Address, id: u64) {
        assert!(caller == self.admin, "DRC22: only admin can revoke");
        let schedule = self
            .vestings
            .get_mut(&id)
            .expect("DRC22: vesting not found");
        assert!(schedule.revocable, "DRC22: vesting is not revocable");
        assert!(!schedule.revoked, "DRC22: vesting already revoked");
        schedule.revoked = true;
    }

    pub fn vested_amount(&self, id: u64, current_time: u64) -> u64 {
        let schedule = self
            .vestings
            .get(&id)
            .expect("DRC22: vesting not found");
        schedule.vested_amount(current_time)
    }

    pub fn releasable_amount(&self, id: u64, current_time: u64) -> u64 {
        let schedule = self
            .vestings
            .get(&id)
            .expect("DRC22: vesting not found");
        schedule.releasable_amount(current_time)
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct CreateVestingArgs {
    beneficiary: Address,
    total_amount: u64,
    start_time: u64,
    duration: u64,
    cliff_duration: u64,
    revocable: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReleaseArgs {
    id: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct RevokeArgs {
    id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct VestedAmountArgs {
    id: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReleasableAmountArgs {
    id: u64,
    current_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetVestingArgs {
    id: u64,
}

pub fn dispatch(
    state: &mut Option<VestingState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC22: already initialised");
            *state = Some(VestingState::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        "create_vesting" => {
            let s = state.as_mut().expect("DRC22: not initialised");
            let a: CreateVestingArgs =
                serde_json::from_slice(args).expect("DRC22: bad create_vesting args");
            let id = s.create_vesting(
                caller,
                a.beneficiary,
                a.total_amount,
                a.start_time,
                a.duration,
                a.cliff_duration,
                a.revocable,
            );
            serde_json::to_vec(&id).unwrap()
        }

        "release" => {
            let s = state.as_mut().expect("DRC22: not initialised");
            let a: ReleaseArgs =
                serde_json::from_slice(args).expect("DRC22: bad release args");
            let amount = s.release(caller, a.id, a.current_time);
            serde_json::to_vec(&amount).unwrap()
        }

        "revoke" => {
            let s = state.as_mut().expect("DRC22: not initialised");
            let a: RevokeArgs =
                serde_json::from_slice(args).expect("DRC22: bad revoke args");
            s.revoke(caller, a.id);
            serde_json::to_vec("ok").unwrap()
        }

        "vested_amount" => {
            let s = state.as_ref().expect("DRC22: not initialised");
            let a: VestedAmountArgs =
                serde_json::from_slice(args).expect("DRC22: bad vested_amount args");
            serde_json::to_vec(&s.vested_amount(a.id, a.current_time)).unwrap()
        }

        "releasable_amount" => {
            let s = state.as_ref().expect("DRC22: not initialised");
            let a: ReleasableAmountArgs =
                serde_json::from_slice(args).expect("DRC22: bad releasable_amount args");
            serde_json::to_vec(&s.releasable_amount(a.id, a.current_time)).unwrap()
        }

        "get_vesting" => {
            let s = state.as_ref().expect("DRC22: not initialised");
            let a: GetVestingArgs =
                serde_json::from_slice(args).expect("DRC22: bad get_vesting args");
            serde_json::to_vec(&s.vestings.get(&a.id)).unwrap()
        }

        _ => panic!("DRC22: unknown method '{method}'"),
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
    const OUTSIDER: Address = [99u8; 32];

    fn init_state() -> Option<VestingState> {
        let mut state = None;
        dispatch(&mut state, "init", b"", ADMIN);
        state
    }

    fn create_standard_vesting(state: &mut Option<VestingState>) -> u64 {
        // 10000 tokens, starts at t=1000, duration 1000, cliff 200, revocable
        let args = serde_json::to_vec(&CreateVestingArgs {
            beneficiary: ALICE,
            total_amount: 10000,
            start_time: 1000,
            duration: 1000,
            cliff_duration: 200,
            revocable: true,
        })
        .unwrap();
        let result = dispatch(state, "create_vesting", &args, ADMIN);
        serde_json::from_slice(&result).unwrap()
    }

    #[test]
    fn test_vesting_before_cliff_is_zero() {
        let mut state = init_state();
        let id = create_standard_vesting(&mut state);

        // Before cliff (t=1000+200=1200), vested should be 0
        let args = serde_json::to_vec(&VestedAmountArgs {
            id,
            current_time: 1100,
        })
        .unwrap();
        let result = dispatch(&mut state, "vested_amount", &args, ADMIN);
        let vested: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(vested, 0);
    }

    #[test]
    fn test_vesting_linear_after_cliff() {
        let mut state = init_state();
        let id = create_standard_vesting(&mut state);

        // At t=1500 (500/1000 elapsed) => 50% vested = 5000
        let args = serde_json::to_vec(&VestedAmountArgs {
            id,
            current_time: 1500,
        })
        .unwrap();
        let result = dispatch(&mut state, "vested_amount", &args, ADMIN);
        let vested: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(vested, 5000);
    }

    #[test]
    fn test_full_vest_after_duration() {
        let mut state = init_state();
        let id = create_standard_vesting(&mut state);

        // At t=2000 (duration complete) => 100%
        let args = serde_json::to_vec(&VestedAmountArgs {
            id,
            current_time: 2000,
        })
        .unwrap();
        let result = dispatch(&mut state, "vested_amount", &args, ADMIN);
        let vested: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(vested, 10000);

        // Also fully vested well after
        let args2 = serde_json::to_vec(&VestedAmountArgs {
            id,
            current_time: 9999,
        })
        .unwrap();
        let result2 = dispatch(&mut state, "vested_amount", &args2, ADMIN);
        let vested2: u64 = serde_json::from_slice(&result2).unwrap();
        assert_eq!(vested2, 10000);
    }

    #[test]
    fn test_release_and_releasable() {
        let mut state = init_state();
        let id = create_standard_vesting(&mut state);

        // At t=1500, 5000 releasable
        let rel_args = serde_json::to_vec(&ReleasableAmountArgs {
            id,
            current_time: 1500,
        })
        .unwrap();
        let result = dispatch(&mut state, "releasable_amount", &rel_args, ADMIN);
        let releasable: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(releasable, 5000);

        // Release 5000
        let release_args = serde_json::to_vec(&ReleaseArgs {
            id,
            current_time: 1500,
        })
        .unwrap();
        let result = dispatch(&mut state, "release", &release_args, ALICE);
        let released: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(released, 5000);

        // Releasable is now 0 at same time
        let result2 = dispatch(&mut state, "releasable_amount", &rel_args, ADMIN);
        let releasable2: u64 = serde_json::from_slice(&result2).unwrap();
        assert_eq!(releasable2, 0);

        // At t=2000, remaining 5000 releasable
        let rel_args2 = serde_json::to_vec(&ReleasableAmountArgs {
            id,
            current_time: 2000,
        })
        .unwrap();
        let result3 = dispatch(&mut state, "releasable_amount", &rel_args2, ADMIN);
        let releasable3: u64 = serde_json::from_slice(&result3).unwrap();
        assert_eq!(releasable3, 5000);
    }

    #[test]
    fn test_revoke_stops_vesting() {
        let mut state = init_state();
        let id = create_standard_vesting(&mut state);

        // Release partial first at t=1500
        let release_args = serde_json::to_vec(&ReleaseArgs {
            id,
            current_time: 1500,
        })
        .unwrap();
        dispatch(&mut state, "release", &release_args, ALICE);

        // Revoke
        let revoke_args = serde_json::to_vec(&RevokeArgs { id }).unwrap();
        dispatch(&mut state, "revoke", &revoke_args, ADMIN);

        // Vested amount after revoke is frozen to released amount
        let vested_args = serde_json::to_vec(&VestedAmountArgs {
            id,
            current_time: 2000,
        })
        .unwrap();
        let result = dispatch(&mut state, "vested_amount", &vested_args, ADMIN);
        let vested: u64 = serde_json::from_slice(&result).unwrap();
        assert_eq!(vested, 5000); // frozen at what was released
    }

    #[test]
    #[should_panic(expected = "DRC22: only beneficiary can release")]
    fn test_non_beneficiary_cannot_release() {
        let mut state = init_state();
        let id = create_standard_vesting(&mut state);
        let release_args = serde_json::to_vec(&ReleaseArgs {
            id,
            current_time: 1500,
        })
        .unwrap();
        dispatch(&mut state, "release", &release_args, OUTSIDER);
    }

    #[test]
    #[should_panic(expected = "DRC22: vesting is not revocable")]
    fn test_cannot_revoke_irrevocable() {
        let mut state = init_state();
        let args = serde_json::to_vec(&CreateVestingArgs {
            beneficiary: BOB,
            total_amount: 5000,
            start_time: 0,
            duration: 1000,
            cliff_duration: 0,
            revocable: false,
        })
        .unwrap();
        let result = dispatch(&mut state, "create_vesting", &args, ADMIN);
        let id: u64 = serde_json::from_slice(&result).unwrap();

        let revoke_args = serde_json::to_vec(&RevokeArgs { id }).unwrap();
        dispatch(&mut state, "revoke", &revoke_args, ADMIN);
    }

    #[test]
    #[should_panic(expected = "DRC22: cliff (500) exceeds duration (100)")]
    fn test_cliff_exceeds_duration_fails() {
        let mut state = init_state();
        let args = serde_json::to_vec(&CreateVestingArgs {
            beneficiary: ALICE,
            total_amount: 1000,
            start_time: 0,
            duration: 100,
            cliff_duration: 500,
            revocable: true,
        })
        .unwrap();
        dispatch(&mut state, "create_vesting", &args, ADMIN);
    }
}
