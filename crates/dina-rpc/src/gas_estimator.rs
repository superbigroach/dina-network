use serde::Serialize;

use dina_core::fees::FeeSchedule;

// ---------------------------------------------------------------------------
// Gas estimation types
// ---------------------------------------------------------------------------

/// Breakdown of how a fee was calculated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GasBreakdown {
    /// Base fee for the transaction type (micro-USDC).
    pub base_fee: u64,
    /// Fee for the data payload (memo, args, bytecode) (micro-USDC).
    pub data_fee: u64,
    /// Fee for WASM execution gas consumed (micro-USDC).
    pub execution_fee: u64,
    /// Sum of the above before clamping.
    pub total: u64,
}

/// A complete gas estimate for a transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GasEstimate {
    /// Estimated gas units consumed.
    pub gas_units: u64,
    /// Total fee in micro-USDC.
    pub fee_usdc: u64,
    /// Human-readable fee string (e.g. "$0.0002").
    pub fee_formatted: String,
    /// Detailed fee breakdown.
    pub breakdown: GasBreakdown,
}

// ---------------------------------------------------------------------------
// Gas price info (returned by dina_gasPrice)
// ---------------------------------------------------------------------------

/// Current gas price information.
#[derive(Clone, Debug, Serialize)]
pub struct GasPriceInfo {
    /// Price per gas unit in micro-USDC.
    pub gas_price: u64,
    /// Price per byte of data payload in micro-USDC.
    pub per_byte_fee: u64,
    /// Minimum fee any transaction must pay (micro-USDC).
    pub min_fee: u64,
    /// Maximum fee any transaction will be charged (micro-USDC).
    pub max_fee: u64,
}

// ---------------------------------------------------------------------------
// Estimator
// ---------------------------------------------------------------------------

/// Provides gas and fee estimates for all Dina Network transaction types.
///
/// Uses the active [`FeeSchedule`] to produce deterministic estimates that
/// match what the validator will charge.
pub struct GasEstimator {
    fee_schedule: FeeSchedule,
}

impl GasEstimator {
    /// Create a new estimator backed by the given fee schedule.
    pub fn new(fee_schedule: FeeSchedule) -> Self {
        Self { fee_schedule }
    }

    /// Return the current gas price info.
    pub fn gas_price_info(&self) -> GasPriceInfo {
        GasPriceInfo {
            gas_price: self.fee_schedule.gas_price,
            per_byte_fee: self.fee_schedule.per_byte_fee,
            min_fee: self.fee_schedule.min_fee,
            max_fee: self.fee_schedule.max_fee,
        }
    }

    /// Estimate the fee for a simple USDC transfer.
    ///
    /// * `_amount` — transfer amount (not used for fee calculation, but
    ///   accepted for future percentage-based fee models).
    /// * `memo_size` — length of the optional memo in bytes.
    pub fn estimate_transfer(&self, _amount: u64, memo_size: usize) -> GasEstimate {
        let base_fee = self.fee_schedule.base_transfer_fee;
        let data_fee = self.fee_schedule.per_byte_fee * memo_size as u64;
        let execution_fee = 0;

        self.build_estimate(base_fee, data_fee, execution_fee, memo_size as u64)
    }

    /// Estimate the fee for calling a smart-contract method.
    ///
    /// * `_method` — method name (reserved for method-specific pricing).
    /// * `args_size` — serialized argument size in bytes.
    pub fn estimate_contract_call(&self, _method: &str, args_size: usize) -> GasEstimate {
        let base_fee = self.fee_schedule.base_contract_call_fee;
        let data_fee = self.fee_schedule.per_byte_fee * args_size as u64;
        // Estimate execution gas as a function of args size (heuristic).
        let estimated_gas_units = 1000 + (args_size as u64 * 10);
        let execution_fee = self.fee_schedule.gas_price * estimated_gas_units;

        self.build_estimate(base_fee, data_fee, execution_fee, estimated_gas_units)
    }

    /// Estimate the fee for deploying a new WASM contract.
    ///
    /// * `wasm_size` — bytecode size in bytes.
    pub fn estimate_deploy(&self, wasm_size: usize) -> GasEstimate {
        let base_fee = self.fee_schedule.base_deploy_fee;
        let data_fee = self.fee_schedule.per_byte_fee * wasm_size as u64;
        // Deploy gas scales with bytecode size (compilation cost).
        let estimated_gas_units = 10_000 + (wasm_size as u64 * 5);
        let execution_fee = self.fee_schedule.gas_price * estimated_gas_units;

        self.build_estimate(base_fee, data_fee, execution_fee, estimated_gas_units)
    }

    /// Estimate the fee for registering a device on-chain.
    pub fn estimate_device_registration(&self) -> GasEstimate {
        let base_fee = self.fee_schedule.base_register_device_fee;
        let data_fee = 0;
        let execution_fee = 0;

        self.build_estimate(base_fee, data_fee, execution_fee, 0)
    }

    /// Estimate the per-transaction fee for a batch of `tx_count` transfers.
    ///
    /// The returned estimate reflects the **total** batch fee, not per-tx.
    pub fn estimate_batch(&self, tx_count: usize) -> GasEstimate {
        let total_fee = self.fee_schedule.calculate_batch_fee(tx_count);
        let per_tx = if tx_count > 0 {
            total_fee / tx_count as u64
        } else {
            0
        };

        GasEstimate {
            gas_units: tx_count as u64,
            fee_usdc: total_fee,
            fee_formatted: FeeSchedule::format_fee(total_fee),
            breakdown: GasBreakdown {
                base_fee: per_tx * tx_count as u64,
                data_fee: 0,
                execution_fee: 0,
                total: total_fee,
            },
        }
    }

    // -- internal -----------------------------------------------------------

    fn build_estimate(
        &self,
        base_fee: u64,
        data_fee: u64,
        execution_fee: u64,
        gas_units: u64,
    ) -> GasEstimate {
        let raw_total = base_fee + data_fee + execution_fee;
        let clamped = raw_total.clamp(self.fee_schedule.min_fee, self.fee_schedule.max_fee);

        GasEstimate {
            gas_units,
            fee_usdc: clamped,
            fee_formatted: FeeSchedule::format_fee(clamped),
            breakdown: GasBreakdown {
                base_fee,
                data_fee,
                execution_fee,
                total: clamped,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn estimator() -> GasEstimator {
        GasEstimator::new(FeeSchedule::default_testnet())
    }

    // -- Transfer estimates -------------------------------------------------

    #[test]
    fn transfer_no_memo() {
        let est = estimator().estimate_transfer(1_000_000, 0);
        assert_eq!(est.fee_usdc, 200); // base_transfer_fee
        assert_eq!(est.breakdown.base_fee, 200);
        assert_eq!(est.breakdown.data_fee, 0);
        assert_eq!(est.breakdown.execution_fee, 0);
    }

    #[test]
    fn transfer_with_memo() {
        let est = estimator().estimate_transfer(1_000_000, 100);
        // 200 base + 100 bytes * 1 per_byte_fee = 300
        assert_eq!(est.fee_usdc, 300);
        assert_eq!(est.breakdown.data_fee, 100);
    }

    #[test]
    fn transfer_fee_formatted() {
        let est = estimator().estimate_transfer(1_000_000, 0);
        assert_eq!(est.fee_formatted, "$0.000200");
    }

    // -- Contract call estimates -------------------------------------------

    #[test]
    fn contract_call_estimate() {
        let est = estimator().estimate_contract_call("swap", 256);
        assert_eq!(est.breakdown.base_fee, 10_000);
        assert_eq!(est.breakdown.data_fee, 256); // 256 * 1
        // execution: gas_price(1) * (1000 + 256*10) = 3560
        assert_eq!(est.breakdown.execution_fee, 3_560);
        assert_eq!(est.fee_usdc, 10_000 + 256 + 3_560);
    }

    #[test]
    fn contract_call_zero_args() {
        let est = estimator().estimate_contract_call("ping", 0);
        // base + 0 data + 1000 gas units
        assert_eq!(est.fee_usdc, 10_000 + 1_000);
    }

    // -- Deploy estimates ---------------------------------------------------

    #[test]
    fn deploy_small_contract() {
        let est = estimator().estimate_deploy(1_000);
        assert_eq!(est.breakdown.base_fee, 1_000_000);
        assert_eq!(est.breakdown.data_fee, 1_000); // 1000 * 1
        // execution: 1 * (10_000 + 1000*5) = 15_000
        assert_eq!(est.breakdown.execution_fee, 15_000);
        assert_eq!(est.fee_usdc, 1_000_000 + 1_000 + 15_000);
    }

    #[test]
    fn deploy_fee_clamped_to_max() {
        let schedule = FeeSchedule {
            base_deploy_fee: 9_000_000,
            max_fee: 10_000_000,
            ..FeeSchedule::default_testnet()
        };
        let est = GasEstimator::new(schedule).estimate_deploy(500_000);
        // raw would exceed max_fee, so clamped
        assert_eq!(est.fee_usdc, 10_000_000);
    }

    // -- Device registration -----------------------------------------------

    #[test]
    fn device_registration_estimate() {
        let est = estimator().estimate_device_registration();
        assert_eq!(est.fee_usdc, 100_000);
        assert_eq!(est.breakdown.base_fee, 100_000);
        assert_eq!(est.breakdown.data_fee, 0);
        assert_eq!(est.breakdown.execution_fee, 0);
    }

    // -- Batch estimates ---------------------------------------------------

    #[test]
    fn batch_no_discount() {
        let est = estimator().estimate_batch(5);
        // 200 * 5 = 1000, no discount
        assert_eq!(est.fee_usdc, 1_000);
    }

    #[test]
    fn batch_with_discount() {
        let est = estimator().estimate_batch(10);
        // 200 * 10 = 2000, 10% off = 1800
        assert_eq!(est.fee_usdc, 1_800);
    }

    #[test]
    fn batch_zero() {
        let est = estimator().estimate_batch(0);
        assert_eq!(est.fee_usdc, 0);
    }

    // -- Gas price info ----------------------------------------------------

    #[test]
    fn gas_price_info_matches_schedule() {
        let schedule = FeeSchedule::default_testnet();
        let est = GasEstimator::new(schedule.clone());
        let info = est.gas_price_info();
        assert_eq!(info.gas_price, schedule.gas_price);
        assert_eq!(info.per_byte_fee, schedule.per_byte_fee);
        assert_eq!(info.min_fee, schedule.min_fee);
        assert_eq!(info.max_fee, schedule.max_fee);
    }

    // -- Min fee clamping --------------------------------------------------

    #[test]
    fn transfer_clamped_to_min() {
        let schedule = FeeSchedule {
            base_transfer_fee: 10,
            per_byte_fee: 0,
            min_fee: 100,
            ..FeeSchedule::default_testnet()
        };
        let est = GasEstimator::new(schedule).estimate_transfer(1_000, 0);
        assert_eq!(est.fee_usdc, 100);
    }
}
