use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-17  Advanced Token Hooks
// ---------------------------------------------------------------------------

type Address = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HookCall {
    pub from: Address,
    pub to: Address,
    pub amount: u64,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HookRegistry {
    pub receive_hooks: BTreeMap<Address, Address>,
    pub send_hooks: BTreeMap<Address, Address>,
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            receive_hooks: BTreeMap::new(),
            send_hooks: BTreeMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TokenWithHooks {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u64,
    pub owner: Address,
    pub balances: BTreeMap<Address, u64>,
    pub allowances: BTreeMap<(Address, Address), u64>,
    pub hooks: HookRegistry,
    /// Populated after a send() call so the runtime can dispatch hook calls.
    pub pending_hook_calls: Vec<HookCall>,
}

impl TokenWithHooks {
    pub fn new(name: String, symbol: String, decimals: u8, owner: Address) -> Self {
        Self {
            name,
            symbol,
            decimals,
            total_supply: 0,
            owner,
            balances: BTreeMap::new(),
            allowances: BTreeMap::new(),
            hooks: HookRegistry::new(),
            pending_hook_calls: Vec::new(),
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn balance_of(&self, account: &Address) -> u64 {
        self.balances.get(account).copied().unwrap_or(0)
    }

    pub fn allowance(&self, owner: &Address, spender: &Address) -> u64 {
        self.allowances
            .get(&(*owner, *spender))
            .copied()
            .unwrap_or(0)
    }

    // -- DRC-1 basic methods -------------------------------------------------

    pub fn transfer(&mut self, caller: Address, to: Address, amount: u64) {
        assert!(amount > 0, "DRC17: transfer amount must be positive");
        let from_bal = self.balance_of(&caller);
        assert!(
            from_bal >= amount,
            "DRC17: insufficient balance ({from_bal} < {amount})"
        );
        self.balances.insert(caller, from_bal - amount);
        let to_bal = self.balance_of(&to);
        self.balances.insert(to, to_bal + amount);
    }

    pub fn approve(&mut self, caller: Address, spender: Address, amount: u64) {
        self.allowances.insert((caller, spender), amount);
    }

    // -- Hook registration ---------------------------------------------------

    pub fn register_receive_hook(&mut self, caller: Address, hook_contract: Address) {
        self.hooks.receive_hooks.insert(caller, hook_contract);
    }

    pub fn register_send_hook(&mut self, caller: Address, hook_contract: Address) {
        self.hooks.send_hooks.insert(caller, hook_contract);
    }

    // -- Hook-aware send -----------------------------------------------------

    /// Transfers tokens and records hook calls for the runtime to dispatch.
    /// 1) If sender has a send_hook, a HookCall is queued for it.
    /// 2) If receiver has a receive_hook, a HookCall is queued for it.
    ///    The actual token transfer is performed immediately.
    pub fn send(&mut self, caller: Address, to: Address, amount: u64, data: Vec<u8>) {
        assert!(amount > 0, "DRC17: send amount must be positive");
        let from_bal = self.balance_of(&caller);
        assert!(
            from_bal >= amount,
            "DRC17: insufficient balance ({from_bal} < {amount})"
        );

        self.pending_hook_calls.clear();

        let hook_call = HookCall {
            from: caller,
            to,
            amount,
            data: data.clone(),
        };

        // Sender hook (called first)
        if let Some(send_hook) = self.hooks.send_hooks.get(&caller) {
            let mut call = hook_call.clone();
            // The `to` field in send hook context points to the hook contract
            // but we keep the original data so the hook knows the real recipient.
            let _ = send_hook; // hook address available for runtime dispatch
            self.pending_hook_calls.push(HookCall {
                from: caller,
                to: *send_hook,
                amount,
                data: serde_json::to_vec(&call).unwrap_or_default(),
            });
            call.data = data.clone(); // reset
        }

        // Receiver hook (called second)
        if let Some(recv_hook) = self.hooks.receive_hooks.get(&to) {
            self.pending_hook_calls.push(HookCall {
                from: caller,
                to: *recv_hook,
                amount,
                data: serde_json::to_vec(&hook_call).unwrap_or_default(),
            });
        }

        // Perform the transfer
        self.balances.insert(caller, from_bal - amount);
        let to_bal = self.balance_of(&to);
        self.balances.insert(to, to_bal + amount);
    }

    pub fn mint(&mut self, caller: Address, to: Address, amount: u64) {
        assert!(caller == self.owner, "DRC17: only owner can mint");
        assert!(amount > 0, "DRC17: mint amount must be positive");
        let balance = self.balance_of(&to);
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
}

#[derive(Serialize, Deserialize, Debug)]
struct ApproveArgs {
    spender: Address,
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SendArgs {
    to: Address,
    amount: u64,
    data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RegisterHookArgs {
    hook_contract: Address,
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

#[derive(Serialize, Deserialize, Debug)]
struct MintArgs {
    to: Address,
    amount: u64,
}

pub fn dispatch(
    state: &mut Option<TokenWithHooks>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC17: already initialised");
            let a: InitArgs = serde_json::from_slice(args).expect("DRC17: bad init args");
            *state = Some(TokenWithHooks::new(a.name, a.symbol, a.decimals, caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "balance_of" => {
            let s = state.as_ref().expect("DRC17: not initialised");
            let a: BalanceOfArgs =
                serde_json::from_slice(args).expect("DRC17: bad balance_of args");
            serde_json::to_vec(&s.balance_of(&a.account)).unwrap()
        }
        "allowance" => {
            let s = state.as_ref().expect("DRC17: not initialised");
            let a: AllowanceArgs = serde_json::from_slice(args).expect("DRC17: bad allowance args");
            serde_json::to_vec(&s.allowance(&a.owner, &a.spender)).unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "transfer" => {
            let s = state.as_mut().expect("DRC17: not initialised");
            let a: TransferArgs = serde_json::from_slice(args).expect("DRC17: bad transfer args");
            s.transfer(caller, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "approve" => {
            let s = state.as_mut().expect("DRC17: not initialised");
            let a: ApproveArgs = serde_json::from_slice(args).expect("DRC17: bad approve args");
            s.approve(caller, a.spender, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "send" => {
            let s = state.as_mut().expect("DRC17: not initialised");
            let a: SendArgs = serde_json::from_slice(args).expect("DRC17: bad send args");
            s.send(caller, a.to, a.amount, a.data);
            // Return pending hook calls so the runtime can dispatch them
            let hooks = s.pending_hook_calls.clone();
            serde_json::to_vec(&hooks).unwrap()
        }
        "register_receive_hook" => {
            let s = state.as_mut().expect("DRC17: not initialised");
            let a: RegisterHookArgs =
                serde_json::from_slice(args).expect("DRC17: bad register_receive_hook args");
            s.register_receive_hook(caller, a.hook_contract);
            serde_json::to_vec("ok").unwrap()
        }
        "register_send_hook" => {
            let s = state.as_mut().expect("DRC17: not initialised");
            let a: RegisterHookArgs =
                serde_json::from_slice(args).expect("DRC17: bad register_send_hook args");
            s.register_send_hook(caller, a.hook_contract);
            serde_json::to_vec("ok").unwrap()
        }
        "mint" => {
            let s = state.as_mut().expect("DRC17: not initialised");
            let a: MintArgs = serde_json::from_slice(args).expect("DRC17: bad mint args");
            s.mint(caller, a.to, a.amount);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC17: unknown method '{method}'"),
    }
}
