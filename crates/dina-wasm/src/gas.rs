/// Gas cost table for metering WASM contract execution.
///
/// Each host operation has a fixed fuel cost. Wasmtime's fuel mechanism is
/// used under the hood: one unit of fuel equals one unit of gas.
#[derive(Debug, Clone)]
pub struct GasCosts {
    /// Cost of a basic WASM instruction (added by wasmtime fuel).
    pub base_cost: u64,
    /// Cost to read from WASM linear memory in a host call.
    pub memory_read: u64,
    /// Cost to write into WASM linear memory in a host call.
    pub memory_write: u64,
    /// Cost to read a key from contract persistent storage.
    pub storage_read: u64,
    /// Cost to write a key-value pair into contract persistent storage.
    pub storage_write: u64,
    /// Cost to execute a USDC transfer.
    pub transfer: u64,
    /// Cost to invoke another contract (cross-contract call).
    pub cross_contract_call: u64,
    /// Cost to compute a SHA-256 hash.
    pub sha256: u64,
    /// Cost to verify an Ed25519 signature.
    pub ed25519_verify: u64,
    /// Cost to emit a contract event.
    pub emit_event: u64,
}

/// Default gas costs used throughout the network.
pub const DEFAULT_GAS_COSTS: GasCosts = GasCosts {
    base_cost: 1,
    memory_read: 5,
    memory_write: 5,
    storage_read: 100,
    storage_write: 500,
    transfer: 200,
    cross_contract_call: 1000,
    sha256: 50,
    ed25519_verify: 300,
    emit_event: 100,
};

impl Default for GasCosts {
    fn default() -> Self {
        DEFAULT_GAS_COSTS
    }
}

/// Convert gas units to USDC micro-units.
///
/// 1 gas = 0.000001 USDC = 1 micro-USDC unit.
/// Since USDC has 6 decimals, 1 gas = 1 base unit.
pub fn gas_to_usdc(gas: u64) -> u64 {
    gas
}

/// Convert USDC micro-units to gas units.
///
/// 1 micro-USDC unit = 1 gas.
pub fn usdc_to_gas(usdc: u64) -> u64 {
    usdc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gas_usdc_roundtrip() {
        assert_eq!(gas_to_usdc(1_000_000), 1_000_000);
        assert_eq!(usdc_to_gas(500), 500);
        assert_eq!(gas_to_usdc(usdc_to_gas(42)), 42);
    }

    #[test]
    fn default_costs_match_constants() {
        let costs = GasCosts::default();
        assert_eq!(costs.base_cost, 1);
        assert_eq!(costs.storage_write, 500);
        assert_eq!(costs.ed25519_verify, 300);
    }
}
