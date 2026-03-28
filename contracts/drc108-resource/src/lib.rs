use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DRC-108  Resource Budget
// ---------------------------------------------------------------------------

type Address = [u8; 32];
type DeviceId = [u8; 32];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResourceAllocation {
    pub resource_type: String,
    pub amount: u64,
    pub used: u64,
    pub expires_at: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResourceRegistry {
    pub admin: Address,
    pub allocations: BTreeMap<(DeviceId, String), ResourceAllocation>,
    pub prices: BTreeMap<String, u64>,
    pub revenue: u64,
}

impl ResourceRegistry {
    pub fn new(admin: Address) -> Self {
        Self {
            admin,
            allocations: BTreeMap::new(),
            prices: BTreeMap::new(),
            revenue: 0,
        }
    }

    // -- Queries -------------------------------------------------------------

    pub fn balance(&self, device_id: &DeviceId, resource_type: &str) -> u64 {
        self.allocations
            .get(&(*device_id, resource_type.to_string()))
            .map(|a| a.amount.saturating_sub(a.used))
            .unwrap_or(0)
    }

    // -- Mutations -----------------------------------------------------------

    pub fn allocate(
        &mut self,
        caller: Address,
        device_id: DeviceId,
        resource_type: String,
        amount: u64,
        expires_at: Option<u64>,
    ) {
        assert!(
            caller == self.admin,
            "DRC108: only admin can allocate resources"
        );
        assert!(amount > 0, "DRC108: allocation amount must be positive");

        let key = (device_id, resource_type.clone());
        let alloc = self
            .allocations
            .entry(key)
            .or_insert_with(|| ResourceAllocation {
                resource_type: resource_type.clone(),
                amount: 0,
                used: 0,
                expires_at: None,
            });
        alloc.amount += amount;
        if expires_at.is_some() {
            alloc.expires_at = expires_at;
        }
    }

    pub fn transfer_resource(
        &mut self,
        caller: Address,
        from_device: DeviceId,
        to_device: DeviceId,
        resource_type: String,
        amount: u64,
    ) {
        assert!(amount > 0, "DRC108: transfer amount must be positive");

        // Only the admin or the device itself (represented by matching caller)
        // can transfer resources. We check admin here since devices don't have
        // addresses in the same sense. In production this would check device
        // ownership.
        assert!(
            caller == self.admin || caller == from_device,
            "DRC108: not authorised to transfer"
        );

        let from_key = (from_device, resource_type.clone());
        let from_alloc = self
            .allocations
            .get_mut(&from_key)
            .expect("DRC108: source has no allocation");
        let available = from_alloc.amount.saturating_sub(from_alloc.used);
        assert!(
            available >= amount,
            "DRC108: insufficient resource balance ({available} < {amount})"
        );
        from_alloc.amount -= amount;

        let to_key = (to_device, resource_type.clone());
        let to_alloc = self
            .allocations
            .entry(to_key)
            .or_insert_with(|| ResourceAllocation {
                resource_type: resource_type.clone(),
                amount: 0,
                used: 0,
                expires_at: None,
            });
        to_alloc.amount += amount;
    }

    pub fn purchase_resource(&mut self, caller: Address, resource_type: String, amount: u64) {
        assert!(amount > 0, "DRC108: purchase amount must be positive");
        let price_per_unit = self
            .prices
            .get(&resource_type)
            .expect("DRC108: resource type has no price set");
        let total_cost = price_per_unit
            .checked_mul(amount)
            .expect("DRC108: cost overflow");

        // In production, this would deduct USDC from caller's balance via
        // cross-contract call. Here we just track revenue.
        self.revenue += total_cost;

        // Credit the resource to the caller's device (using caller as device ID)
        let key = (caller, resource_type.clone());
        let alloc = self
            .allocations
            .entry(key)
            .or_insert_with(|| ResourceAllocation {
                resource_type,
                amount: 0,
                used: 0,
                expires_at: None,
            });
        alloc.amount += amount;
    }

    pub fn report_usage(
        &mut self,
        caller: Address,
        device_id: DeviceId,
        resource_type: String,
        used: u64,
    ) {
        // Only admin or the device itself can report usage
        assert!(
            caller == self.admin || caller == device_id,
            "DRC108: not authorised to report usage"
        );

        let key = (device_id, resource_type);
        let alloc = self
            .allocations
            .get_mut(&key)
            .expect("DRC108: no allocation for device/resource");
        alloc.used += used;
        assert!(
            alloc.used <= alloc.amount,
            "DRC108: usage exceeds allocation"
        );
    }

    pub fn set_price(&mut self, caller: Address, resource_type: String, price_per_unit: u64) {
        assert!(caller == self.admin, "DRC108: only admin can set prices");
        self.prices.insert(resource_type, price_per_unit);
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct AllocateArgs {
    device_id: DeviceId,
    resource_type: String,
    amount: u64,
    expires_at: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransferResourceArgs {
    from_device: DeviceId,
    to_device: DeviceId,
    resource_type: String,
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BalanceArgs {
    device_id: DeviceId,
    resource_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PurchaseResourceArgs {
    resource_type: String,
    amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReportUsageArgs {
    device_id: DeviceId,
    resource_type: String,
    used: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SetPriceArgs {
    resource_type: String,
    price_per_unit: u64,
}

pub fn dispatch(
    state: &mut Option<ResourceRegistry>,
    method: &str,
    args: &[u8],
    caller: [u8; 32],
) -> Vec<u8> {
    match method {
        "init" => {
            assert!(state.is_none(), "DRC108: already initialised");
            *state = Some(ResourceRegistry::new(caller));
            serde_json::to_vec("ok").unwrap()
        }

        // -- Queries ---------------------------------------------------------
        "balance" => {
            let s = state.as_ref().expect("DRC108: not initialised");
            let a: BalanceArgs = serde_json::from_slice(args).expect("DRC108: bad balance args");
            serde_json::to_vec(&s.balance(&a.device_id, &a.resource_type)).unwrap()
        }

        // -- Mutations -------------------------------------------------------
        "allocate" => {
            let s = state.as_mut().expect("DRC108: not initialised");
            let a: AllocateArgs = serde_json::from_slice(args).expect("DRC108: bad allocate args");
            s.allocate(caller, a.device_id, a.resource_type, a.amount, a.expires_at);
            serde_json::to_vec("ok").unwrap()
        }
        "transfer_resource" => {
            let s = state.as_mut().expect("DRC108: not initialised");
            let a: TransferResourceArgs =
                serde_json::from_slice(args).expect("DRC108: bad transfer_resource args");
            s.transfer_resource(
                caller,
                a.from_device,
                a.to_device,
                a.resource_type,
                a.amount,
            );
            serde_json::to_vec("ok").unwrap()
        }
        "purchase_resource" => {
            let s = state.as_mut().expect("DRC108: not initialised");
            let a: PurchaseResourceArgs =
                serde_json::from_slice(args).expect("DRC108: bad purchase_resource args");
            s.purchase_resource(caller, a.resource_type, a.amount);
            serde_json::to_vec("ok").unwrap()
        }
        "report_usage" => {
            let s = state.as_mut().expect("DRC108: not initialised");
            let a: ReportUsageArgs =
                serde_json::from_slice(args).expect("DRC108: bad report_usage args");
            s.report_usage(caller, a.device_id, a.resource_type, a.used);
            serde_json::to_vec("ok").unwrap()
        }
        "set_price" => {
            let s = state.as_mut().expect("DRC108: not initialised");
            let a: SetPriceArgs = serde_json::from_slice(args).expect("DRC108: bad set_price args");
            s.set_price(caller, a.resource_type, a.price_per_unit);
            serde_json::to_vec("ok").unwrap()
        }

        _ => panic!("DRC108: unknown method '{method}'"),
    }
}
