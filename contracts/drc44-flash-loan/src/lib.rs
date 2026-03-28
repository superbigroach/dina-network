use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-44  Flash Loans  (ERC-3156 equivalent)
// Borrow any amount with zero collateral, repay in same transaction.
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FlashLoanState {
    pub admin: Address,
    /// Pool balances per token address.
    pub pool_balances: BTreeMap<Address, u64>,
    /// Fee in basis points (100 = 1%).
    pub fee_bps: u16,
    pub total_fees_earned: u64,
    /// Active loan tracking (for same-tx repayment verification).
    pub active_loan: Option<FlashLoan>,
    /// Completed loan history.
    pub loan_history: Vec<FlashLoan>,
    /// Tracks the contract's real token balance per token address.
    /// Deposits must increase this before repay is accepted.
    pub contract_balance: BTreeMap<Address, u64>,
    /// Snapshot of contract_balance at loan origination, used to verify
    /// that real tokens were transferred back during repayment.
    pub balance_snapshot_at_loan: Option<(Address, u64)>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FlashLoan {
    pub borrower: Address,
    pub token: Address,
    pub amount: u64,
    pub fee: u64,
    pub repaid: bool,
}

impl FlashLoanState {
    pub fn new(admin: Address, fee_bps: u16) -> Self {
        Self {
            admin,
            pool_balances: BTreeMap::new(),
            fee_bps,
            total_fees_earned: 0,
            active_loan: None,
            loan_history: Vec::new(),
            contract_balance: BTreeMap::new(),
            balance_snapshot_at_loan: None,
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn max_flash_loan(&self, token: &Address) -> u64 {
        self.pool_balances.get(token).copied().unwrap_or(0)
    }

    pub fn flash_fee(&self, _token: &Address, amount: u64) -> u64 {
        (amount as u128 * self.fee_bps as u128 / 10_000) as u64
    }

    // -- Mutations -----------------------------------------------------------

    pub fn deposit_to_pool(&mut self, caller: Address, token: Address, amount: u64) {
        assert!(amount > 0, "DRC44: deposit must be positive");
        let bal = self.pool_balances.get(&token).copied().unwrap_or(0);
        self.pool_balances.insert(token, bal + amount);
        // Track real balance increase (runtime must ensure actual token transfer).
        let real_bal = self.contract_balance.get(&token).copied().unwrap_or(0);
        self.contract_balance.insert(token, real_bal + amount);
        let _ = caller;
    }

    pub fn withdraw_from_pool(&mut self, caller: Address, token: Address, amount: u64) {
        assert!(caller == self.admin, "DRC44: only admin can withdraw");
        assert!(amount > 0, "DRC44: withdraw must be positive");
        let bal = self.pool_balances.get(&token).copied().unwrap_or(0);
        assert!(bal >= amount, "DRC44: insufficient pool balance");
        self.pool_balances.insert(token, bal - amount);
        let real_bal = self.contract_balance.get(&token).copied().unwrap_or(0);
        self.contract_balance
            .insert(token, real_bal.saturating_sub(amount));
    }

    /// Initiate a flash loan. The borrower must call repay_flash_loan
    /// before the transaction ends.
    pub fn flash_loan(&mut self, caller: Address, token: Address, amount: u64, _data: Vec<u8>) {
        assert!(self.active_loan.is_none(), "DRC44: loan already active");
        let pool = self.max_flash_loan(&token);
        assert!(pool >= amount, "DRC44: insufficient pool for loan");
        assert!(amount > 0, "DRC44: loan amount must be positive");

        let fee = self.flash_fee(&token, amount);

        // Debit pool (funds go to borrower)
        self.pool_balances.insert(token, pool - amount);

        // Snapshot the contract's real balance before the loan.
        // The borrower must deposit (amount + fee) back before calling repay.
        let real_bal = self.contract_balance.get(&token).copied().unwrap_or(0);
        // Decrease contract_balance to reflect tokens leaving the contract.
        self.contract_balance
            .insert(token, real_bal.saturating_sub(amount));
        // Record the post-loan balance so repay can verify increase.
        self.balance_snapshot_at_loan = Some((token, real_bal.saturating_sub(amount)));

        self.active_loan = Some(FlashLoan {
            borrower: caller,
            token,
            amount,
            fee,
            repaid: false,
        });
    }

    /// Deposit tokens back into the contract before repaying a flash loan.
    /// The borrower must call this with at least (amount + fee) before calling repay.
    pub fn deposit_repayment(&mut self, caller: Address, token: Address, amount: u64) {
        assert!(amount > 0, "DRC44: deposit amount must be positive");
        let loan = self.active_loan.as_ref().expect("DRC44: no active loan");
        assert!(
            caller == loan.borrower,
            "DRC44: only borrower can deposit repayment"
        );
        assert!(token == loan.token, "DRC44: wrong token for repayment");
        let real_bal = self.contract_balance.get(&token).copied().unwrap_or(0);
        self.contract_balance.insert(token, real_bal + amount);
    }

    /// Repay the active flash loan (amount + fee).
    /// The borrower must have called `deposit_repayment` first to transfer
    /// real tokens back. This function verifies the contract's balance
    /// increased by at least (amount + fee) since loan origination.
    pub fn repay_flash_loan(&mut self, caller: Address) {
        let loan = self.active_loan.as_ref().expect("DRC44: no active loan");
        assert!(caller == loan.borrower, "DRC44: only borrower can repay");

        let repay_amount = loan.amount + loan.fee;
        let token = loan.token;
        let fee = loan.fee;

        // Verify the contract's real balance increased by at least (amount + fee)
        // relative to the snapshot taken when the loan was issued.
        let (snapshot_token, snapshot_balance) = self
            .balance_snapshot_at_loan
            .expect("DRC44: no balance snapshot");
        assert!(snapshot_token == token, "DRC44: snapshot token mismatch");
        let current_real_balance = self.contract_balance.get(&token).copied().unwrap_or(0);
        assert!(
            current_real_balance >= snapshot_balance + repay_amount,
            "DRC44: insufficient repayment — contract balance {} but need {} (snapshot {} + repay {})",
            current_real_balance,
            snapshot_balance + repay_amount,
            snapshot_balance,
            repay_amount
        );

        // Credit pool with repaid amount (original + fee)
        let pool = self.pool_balances.get(&token).copied().unwrap_or(0);
        self.pool_balances.insert(token, pool + repay_amount);
        self.total_fees_earned += fee;

        // Clear snapshot
        self.balance_snapshot_at_loan = None;

        let mut completed = self.active_loan.take().unwrap();
        completed.repaid = true;
        self.loan_history.push(completed);
    }

    /// Verify no active loan remains (called at end of transaction).
    pub fn verify_no_active_loan(&self) {
        assert!(self.active_loan.is_none(), "DRC44: flash loan not repaid");
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct InitArgs {
    fee_bps: u16,
}
#[derive(Serialize, Deserialize, Debug)]
struct DepositArgs {
    token: Address,
    amount: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct WithdrawArgs {
    token: Address,
    amount: u64,
}
#[derive(Serialize, Deserialize, Debug)]
struct FlashLoanArgs {
    token: Address,
    amount: u64,
    data: Vec<u8>,
}
#[derive(Serialize, Deserialize, Debug)]
struct TokenArg {
    token: Address,
}
#[derive(Serialize, Deserialize, Debug)]
struct FlashFeeArgs {
    token: Address,
    amount: u64,
}

pub fn dispatch(
    state: &mut Option<FlashLoanState>,
    method: &str,
    args: &[u8],
    caller: Address,
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC44: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC44: bad init args");
            *state = Some(FlashLoanState::new(caller, a.fee_bps));
            serde_json::to_vec("ok").unwrap()
        }
        "max_flash_loan" => {
            let s = state.as_ref().expect("DRC44: not initialised");
            let a: TokenArg = serde_json::from_slice(args).expect("DRC44: bad args");
            serde_json::to_vec(&s.max_flash_loan(&a.token)).unwrap()
        }
        "flash_fee" => {
            let s = state.as_ref().expect("DRC44: not initialised");
            let a: FlashFeeArgs = serde_json::from_slice(args).expect("DRC44: bad args");
            serde_json::to_vec(&s.flash_fee(&a.token, a.amount)).unwrap()
        }
        "total_fees_earned" => {
            let s = state.as_ref().expect("DRC44: not initialised");
            serde_json::to_vec(&s.total_fees_earned).unwrap()
        }
        "deposit_to_pool" => {
            let s = state.as_mut().expect("DRC44: not initialised");
            let a: DepositArgs = serde_json::from_slice(args).expect("DRC44: bad args");
            s.deposit_to_pool(caller, a.token, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "withdraw_from_pool" => {
            let s = state.as_mut().expect("DRC44: not initialised");
            let a: WithdrawArgs = serde_json::from_slice(args).expect("DRC44: bad args");
            s.withdraw_from_pool(caller, a.token, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "flash_loan" => {
            let s = state.as_mut().expect("DRC44: not initialised");
            let a: FlashLoanArgs = serde_json::from_slice(args).expect("DRC44: bad args");
            s.flash_loan(caller, a.token, a.amount, a.data);
            serde_json::to_vec("ok").unwrap()
        }
        "deposit_repayment" => {
            let s = state.as_mut().expect("DRC44: not initialised");
            let a: DepositArgs = serde_json::from_slice(args).expect("DRC44: bad args");
            s.deposit_repayment(caller, a.token, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "repay_flash_loan" => {
            let s = state.as_mut().expect("DRC44: not initialised");
            s.repay_flash_loan(caller);
            serde_json::to_vec("ok").unwrap()
        }
        "verify_no_active_loan" => {
            let s = state.as_ref().expect("DRC44: not initialised");
            s.verify_no_active_loan();
            serde_json::to_vec("ok").unwrap()
        }
        _ => panic!("DRC44: unknown method '{method}'"),
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
    fn token_a() -> Address {
        [100; 32]
    }

    fn setup() -> Option<FlashLoanState> {
        let mut state = None;
        let args = serde_json::to_vec(&InitArgs { fee_bps: 50 }).unwrap(); // 0.5%
        dispatch(&mut state, "init", &args, addr(1));
        // Deposit 10000 of token_a
        let dep = serde_json::to_vec(&DepositArgs {
            token: token_a(),
            amount: 10_000,
        })
        .unwrap();
        dispatch(&mut state, "deposit_to_pool", &dep, addr(1));
        state
    }

    #[test]
    fn test_flash_loan_and_repay() {
        let mut state = setup();
        let loan_args = serde_json::to_vec(&FlashLoanArgs {
            token: token_a(),
            amount: 5000,
            data: vec![],
        })
        .unwrap();
        dispatch(&mut state, "flash_loan", &loan_args, addr(2));
        let s = state.as_ref().unwrap();
        assert_eq!(s.max_flash_loan(&token_a()), 5000); // 10000 - 5000
        assert!(s.active_loan.is_some());

        // Borrower must deposit repayment (amount + fee = 5000 + 25 = 5025)
        let dep = serde_json::to_vec(&DepositArgs {
            token: token_a(),
            amount: 5025,
        })
        .unwrap();
        dispatch(&mut state, "deposit_repayment", &dep, addr(2));

        dispatch(&mut state, "repay_flash_loan", b"", addr(2));
        let s = state.as_ref().unwrap();
        assert!(s.active_loan.is_none());
        // Pool gets original + fee: 5000 + 5000 + 25 = 10025
        assert_eq!(s.max_flash_loan(&token_a()), 10025);
        assert_eq!(s.total_fees_earned, 25);
    }

    #[test]
    #[should_panic(expected = "insufficient repayment")]
    fn test_flash_loan_repay_without_deposit_fails() {
        let mut state = setup();
        let loan_args = serde_json::to_vec(&FlashLoanArgs {
            token: token_a(),
            amount: 5000,
            data: vec![],
        })
        .unwrap();
        dispatch(&mut state, "flash_loan", &loan_args, addr(2));
        // Attempt to repay without depositing tokens back — must fail
        dispatch(&mut state, "repay_flash_loan", b"", addr(2));
    }

    #[test]
    fn test_flash_fee_calculation() {
        let state = setup();
        let s = state.as_ref().unwrap();
        assert_eq!(s.flash_fee(&token_a(), 10_000), 50); // 0.5% of 10000
        assert_eq!(s.flash_fee(&token_a(), 200), 1);
    }

    #[test]
    #[should_panic(expected = "insufficient pool for loan")]
    fn test_flash_loan_exceeds_pool() {
        let mut state = setup();
        let args = serde_json::to_vec(&FlashLoanArgs {
            token: token_a(),
            amount: 99_999,
            data: vec![],
        })
        .unwrap();
        dispatch(&mut state, "flash_loan", &args, addr(2));
    }

    #[test]
    #[should_panic(expected = "flash loan not repaid")]
    fn test_verify_fails_with_active_loan() {
        let mut state = setup();
        let args = serde_json::to_vec(&FlashLoanArgs {
            token: token_a(),
            amount: 100,
            data: vec![],
        })
        .unwrap();
        dispatch(&mut state, "flash_loan", &args, addr(2));
        dispatch(&mut state, "verify_no_active_loan", b"", addr(2));
    }

    #[test]
    fn test_withdraw_from_pool() {
        let mut state = setup();
        let args = serde_json::to_vec(&WithdrawArgs {
            token: token_a(),
            amount: 3000,
        })
        .unwrap();
        dispatch(&mut state, "withdraw_from_pool", &args, addr(1));
        let s = state.as_ref().unwrap();
        assert_eq!(s.max_flash_loan(&token_a()), 7000);
    }

    #[test]
    #[should_panic(expected = "only admin can withdraw")]
    fn test_withdraw_non_admin() {
        let mut state = setup();
        let args = serde_json::to_vec(&WithdrawArgs {
            token: token_a(),
            amount: 100,
        })
        .unwrap();
        dispatch(&mut state, "withdraw_from_pool", &args, addr(99));
    }
}
