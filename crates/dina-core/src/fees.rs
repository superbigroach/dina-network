use serde::{Deserialize, Serialize};
use std::fmt;

use crate::transaction::Transaction;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// 1 USDC = 1_000_000 micro-USDC (six decimals, matching on-chain USDC).
pub const MICRO_USDC_PER_USDC: u64 = 1_000_000;

// ---------------------------------------------------------------------------
// Fee Schedule
// ---------------------------------------------------------------------------

/// Defines the fee parameters for all transaction types on the Dina Network.
///
/// All monetary values are denominated in **micro-USDC** (1 USDC = 1,000,000 micro-USDC).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeSchedule {
    /// Base fee for a simple USDC transfer ($0.0002).
    pub base_transfer_fee: u64,
    /// Base fee for a smart-contract method call ($0.01).
    pub base_contract_call_fee: u64,
    /// Base fee for deploying a new contract ($1.00).
    pub base_deploy_fee: u64,
    /// Base fee for registering a device on-chain ($0.10).
    pub base_register_device_fee: u64,
    /// Additional fee per byte of transaction data payload.
    pub per_byte_fee: u64,
    /// Price per gas unit consumed during WASM execution.
    pub gas_price: u64,
    /// Absolute minimum fee — no transaction may pay less than this.
    pub min_fee: u64,
    /// Absolute maximum fee — no transaction may pay more than this.
    pub max_fee: u64,
}

impl FeeSchedule {
    /// Sensible defaults for the public testnet.
    pub fn default_testnet() -> Self {
        Self {
            base_transfer_fee: 200,            // $0.0002
            base_contract_call_fee: 10_000,    // $0.01
            base_deploy_fee: 1_000_000,        // $1.00
            base_register_device_fee: 100_000, // $0.10
            per_byte_fee: 1,                   // 1 micro-USDC per byte
            gas_price: 1,                      // 1 micro-USDC per gas unit
            min_fee: 100,                      // $0.0001
            max_fee: 10_000_000,               // $10.00
        }
    }

    /// Zero-fee schedule — no fees whatsoever.
    pub fn zero_fee() -> Self {
        Self {
            base_transfer_fee: 0,
            base_contract_call_fee: 0,
            base_deploy_fee: 0,
            base_register_device_fee: 0,
            per_byte_fee: 0,
            gas_price: 0,
            min_fee: 0,
            max_fee: 0,
        }
    }

    /// Mainnet uses the zero-fee schedule (Dina Network mainnet = zero fees).
    pub fn mainnet() -> Self {
        Self::zero_fee()
    }

    /// Low-fee schedule for local development (10x cheaper, higher cap).
    pub fn development() -> Self {
        Self {
            base_transfer_fee: 20,
            base_contract_call_fee: 1_000,
            base_deploy_fee: 100_000,
            base_register_device_fee: 10_000,
            per_byte_fee: 0,
            gas_price: 0,
            min_fee: 10,
            max_fee: 100_000_000,
        }
    }

    // -- Calculation helpers -------------------------------------------------

    /// Calculate the fee for a fully formed [`Transaction`].
    pub fn calculate_fee(&self, tx: &Transaction) -> u64 {
        match tx {
            Transaction::Transfer { memo, .. } => {
                let memo_size = memo.as_ref().map_or(0, |m| m.len());
                self.calculate_transfer_fee(memo_size)
            }
            Transaction::CallContract { method, args, .. } => {
                self.calculate_contract_fee(method, args.len(), 0)
            }
            Transaction::DeployContract { wasm_bytecode, .. } => {
                self.calculate_deploy_fee(wasm_bytecode.len())
            }
            Transaction::RegisterDevice { .. } => self.clamp(self.base_register_device_fee),
        }
    }

    /// Fee for a transfer, factoring in memo byte-length.
    pub fn calculate_transfer_fee(&self, memo_size: usize) -> u64 {
        let raw = self.base_transfer_fee + self.per_byte_fee * memo_size as u64;
        self.clamp(raw)
    }

    /// Fee for a contract call, factoring in argument size and gas consumed.
    pub fn calculate_contract_fee(&self, _method: &str, args_size: usize, gas_used: u64) -> u64 {
        let raw = self.base_contract_call_fee
            + self.per_byte_fee * args_size as u64
            + self.gas_price * gas_used;
        self.clamp(raw)
    }

    /// Fee for deploying a contract — scales linearly with WASM bytecode size.
    pub fn calculate_deploy_fee(&self, wasm_size: usize) -> u64 {
        let raw = self.base_deploy_fee + self.per_byte_fee * wasm_size as u64;
        self.clamp(raw)
    }

    /// Discounted aggregate fee for a batch of `tx_count` transactions.
    ///
    /// Discount schedule:
    /// - 1-9 transactions : no discount (100%)
    /// - 10-49            : 10% off
    /// - 50-99            : 20% off
    /// - 100+             : 30% off
    ///
    /// The returned value is the *per-transaction* fee after the discount.
    /// Multiply by `tx_count` for the total batch fee.
    pub fn calculate_batch_fee(&self, tx_count: usize) -> u64 {
        let base_total = self.base_transfer_fee * tx_count as u64;
        let discount_bps: u64 = match tx_count {
            0..=9 => 0,
            10..=49 => 1_000, // 10%
            50..=99 => 2_000, // 20%
            _ => 3_000,       // 30%
        };
        let discounted = base_total * (10_000 - discount_bps) / 10_000;
        // Enforce that each tx still pays at least min_fee
        let floor = self.min_fee * tx_count as u64;
        discounted.max(floor)
    }

    /// Convert a micro-USDC value to a human-readable USDC `f64`.
    pub fn fee_in_usdc(micro_usdc: u64) -> f64 {
        micro_usdc as f64 / MICRO_USDC_PER_USDC as f64
    }

    /// Pretty-print a fee as a dollar string, e.g. `"$0.0002"`.
    pub fn format_fee(micro_usdc: u64) -> String {
        let usdc = Self::fee_in_usdc(micro_usdc);
        if usdc >= 1.0 {
            format!("${:.2}", usdc)
        } else if usdc >= 0.01 {
            format!("${:.4}", usdc)
        } else {
            // Show enough decimals so the smallest fees are visible
            format!("${:.6}", usdc)
        }
    }

    // -- Internal -----------------------------------------------------------

    /// Clamp a raw fee to the `[min_fee, max_fee]` range.
    fn clamp(&self, raw: u64) -> u64 {
        raw.clamp(self.min_fee, self.max_fee)
    }
}

impl Default for FeeSchedule {
    fn default() -> Self {
        Self::default_testnet()
    }
}

impl fmt::Display for FeeSchedule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "FeeSchedule {{")?;
        writeln!(
            f,
            "  transfer    : {}",
            Self::format_fee(self.base_transfer_fee)
        )?;
        writeln!(
            f,
            "  contract    : {}",
            Self::format_fee(self.base_contract_call_fee)
        )?;
        writeln!(
            f,
            "  deploy      : {}",
            Self::format_fee(self.base_deploy_fee)
        )?;
        writeln!(
            f,
            "  register    : {}",
            Self::format_fee(self.base_register_device_fee)
        )?;
        writeln!(f, "  per_byte    : {} micro-USDC", self.per_byte_fee)?;
        writeln!(f, "  gas_price   : {} micro-USDC", self.gas_price)?;
        writeln!(f, "  min         : {}", Self::format_fee(self.min_fee))?;
        writeln!(f, "  max         : {}", Self::format_fee(self.max_fee))?;
        write!(f, "}}")
    }
}

// ---------------------------------------------------------------------------
// Fee Distribution
// ---------------------------------------------------------------------------

/// How collected fees are split between network participants.
///
/// Values are in basis points (bps): 10_000 bps = 100%.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeDistribution {
    /// Share paid to the block-producing validator (default 80%).
    pub validator_share_bps: u16,
    /// Share sent to the network treasury (default 20%).
    pub treasury_share_bps: u16,
}

impl FeeDistribution {
    /// Standard 80/20 split.
    pub fn default_split() -> Self {
        Self {
            validator_share_bps: 8_000,
            treasury_share_bps: 2_000,
        }
    }

    /// Split a `fee` into `(validator_share, treasury_share)`.
    ///
    /// Any remainder from integer division goes to the validator to avoid
    /// micro-USDC being silently lost.
    pub fn distribute(&self, fee: u64) -> (u64, u64) {
        let treasury = fee * self.treasury_share_bps as u64 / 10_000;
        let validator = fee - treasury; // remainder stays with validator
        (validator, treasury)
    }

    /// Validate that shares sum to 10,000 bps (100%).
    pub fn is_valid(&self) -> bool {
        self.validator_share_bps as u32 + self.treasury_share_bps as u32 == 10_000
    }
}

impl Default for FeeDistribution {
    fn default() -> Self {
        Self::default_split()
    }
}

impl fmt::Display for FeeDistribution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FeeDistribution {{ validator: {:.1}%, treasury: {:.1}% }}",
            self.validator_share_bps as f64 / 100.0,
            self.treasury_share_bps as f64 / 100.0,
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::Sig64;
    use crate::types::{Address, Hash};

    fn schedule() -> FeeSchedule {
        FeeSchedule::default_testnet()
    }

    // -- Transfer fees ------------------------------------------------------

    #[test]
    fn transfer_fee_no_memo() {
        let s = schedule();
        let fee = s.calculate_transfer_fee(0);
        assert_eq!(fee, 200); // base_transfer_fee
    }

    #[test]
    fn transfer_fee_with_memo() {
        let s = schedule();
        let fee = s.calculate_transfer_fee(100);
        // 200 base + 1 * 100 bytes = 300
        assert_eq!(fee, 300);
    }

    #[test]
    fn transfer_fee_via_transaction() {
        let s = schedule();
        let tx = Transaction::Transfer {
            from: Address::ZERO,
            to: Address([0x01; 32]),
            amount: 5_000_000,
            memo: Some(vec![0u8; 50]),
            device_witness: None,
            nonce: 0,
            fee: 0,
            pub_key: [0u8; 32],
            signature: Sig64([0u8; 64]),
        };
        let fee = s.calculate_fee(&tx);
        assert_eq!(fee, 250); // 200 + 50 bytes
    }

    // -- Contract call fees -------------------------------------------------

    #[test]
    fn contract_call_fee_no_gas() {
        let s = schedule();
        let fee = s.calculate_contract_fee("transfer", 0, 0);
        assert_eq!(fee, 10_000);
    }

    #[test]
    fn contract_call_fee_with_gas() {
        let s = schedule();
        let fee = s.calculate_contract_fee("transfer", 256, 5_000);
        // 10_000 base + 256 bytes + 5_000 gas = 15_256
        assert_eq!(fee, 15_256);
    }

    #[test]
    fn contract_call_via_transaction() {
        let s = schedule();
        let tx = Transaction::CallContract {
            from: Address::ZERO,
            contract: Address([0x02; 32]),
            method: "swap".to_string(),
            args: vec![0u8; 128],
            usdc_attached: 0,
            nonce: 0,
            fee: 0,
            pub_key: [0u8; 32],
            signature: Sig64([0u8; 64]),
        };
        let fee = s.calculate_fee(&tx);
        // 10_000 + 128 bytes + 0 gas = 10_128
        assert_eq!(fee, 10_128);
    }

    // -- Deploy fees --------------------------------------------------------

    #[test]
    fn deploy_fee_small_contract() {
        let s = schedule();
        let fee = s.calculate_deploy_fee(1_000);
        // 1_000_000 + 1_000 = 1_001_000
        assert_eq!(fee, 1_001_000);
    }

    #[test]
    fn deploy_fee_scales_with_wasm_size() {
        let s = schedule();
        let small = s.calculate_deploy_fee(1_000);
        let large = s.calculate_deploy_fee(100_000);
        assert!(large > small);
        assert_eq!(large - small, 99_000); // 99_000 extra bytes * 1 micro-USDC
    }

    #[test]
    fn deploy_fee_via_transaction() {
        let s = schedule();
        let tx = Transaction::DeployContract {
            from: Address::ZERO,
            wasm_bytecode: vec![0u8; 50_000],
            init_args: vec![],
            nonce: 0,
            fee: 0,
            pub_key: [0u8; 32],
            signature: Sig64([0u8; 64]),
        };
        let fee = s.calculate_fee(&tx);
        assert_eq!(fee, 1_050_000); // 1_000_000 + 50_000
    }

    // -- Register device fees -----------------------------------------------

    #[test]
    fn register_device_fee() {
        let s = schedule();
        let tx = Transaction::RegisterDevice {
            device_pubkey: [0xAA; 32],
            owner: Address::ZERO,
            attestation: crate::transaction::DeviceAttestation {
                pubkey: [0xAA; 32],
                firmware_hash: Hash::ZERO,
                witness_root: Hash::ZERO,
                timestamp: 0,
                signature: Sig64([0u8; 64]),
            },
            nonce: 0,
            fee: 0,
            pub_key: [0u8; 32],
            signature: Sig64([0u8; 64]),
        };
        let fee = s.calculate_fee(&tx);
        assert_eq!(fee, 100_000);
    }

    // -- Batch discount -----------------------------------------------------

    #[test]
    fn batch_no_discount_under_10() {
        let s = schedule();
        let batch = s.calculate_batch_fee(5);
        assert_eq!(batch, 200 * 5); // no discount
    }

    #[test]
    fn batch_10_percent_discount() {
        let s = schedule();
        let batch = s.calculate_batch_fee(10);
        // 200 * 10 = 2000, minus 10% = 1800
        assert_eq!(batch, 1_800);
    }

    #[test]
    fn batch_20_percent_discount() {
        let s = schedule();
        let batch = s.calculate_batch_fee(50);
        // 200 * 50 = 10_000, minus 20% = 8_000
        assert_eq!(batch, 8_000);
    }

    #[test]
    fn batch_30_percent_discount() {
        let s = schedule();
        let batch = s.calculate_batch_fee(100);
        // 200 * 100 = 20_000, minus 30% = 14_000
        assert_eq!(batch, 14_000);
    }

    #[test]
    fn batch_zero_transactions() {
        let s = schedule();
        assert_eq!(s.calculate_batch_fee(0), 0);
    }

    // -- Min / Max clamping -------------------------------------------------

    #[test]
    fn fee_clamped_to_minimum() {
        let s = FeeSchedule {
            base_transfer_fee: 10, // below min_fee
            min_fee: 100,
            ..FeeSchedule::default_testnet()
        };
        let fee = s.calculate_transfer_fee(0);
        assert_eq!(fee, 100); // clamped up
    }

    #[test]
    fn fee_clamped_to_maximum() {
        let s = FeeSchedule {
            base_deploy_fee: 50_000_000, // way above max_fee
            max_fee: 10_000_000,
            ..FeeSchedule::default_testnet()
        };
        let fee = s.calculate_deploy_fee(0);
        assert_eq!(fee, 10_000_000); // clamped down
    }

    // -- Distribution -------------------------------------------------------

    #[test]
    fn distribution_80_20() {
        let dist = FeeDistribution::default_split();
        let (validator, treasury) = dist.distribute(10_000);
        assert_eq!(validator, 8_000);
        assert_eq!(treasury, 2_000);
    }

    #[test]
    fn distribution_remainder_goes_to_validator() {
        let dist = FeeDistribution::default_split();
        // 10_001 * 2000 / 10000 = 2000 (integer), validator gets 8001
        let (validator, treasury) = dist.distribute(10_001);
        assert_eq!(validator + treasury, 10_001);
        assert_eq!(treasury, 2_000);
        assert_eq!(validator, 8_001);
    }

    #[test]
    fn distribution_zero_fee() {
        let dist = FeeDistribution::default_split();
        let (v, t) = dist.distribute(0);
        assert_eq!(v, 0);
        assert_eq!(t, 0);
    }

    #[test]
    fn distribution_is_valid() {
        let dist = FeeDistribution::default_split();
        assert!(dist.is_valid());
    }

    #[test]
    fn distribution_invalid_shares() {
        let dist = FeeDistribution {
            validator_share_bps: 5_000,
            treasury_share_bps: 3_000,
        };
        assert!(!dist.is_valid());
    }

    // -- Formatting ---------------------------------------------------------

    #[test]
    fn format_fee_small() {
        assert_eq!(FeeSchedule::format_fee(200), "$0.000200");
    }

    #[test]
    fn format_fee_medium() {
        assert_eq!(FeeSchedule::format_fee(10_000), "$0.0100");
    }

    #[test]
    fn format_fee_dollar() {
        assert_eq!(FeeSchedule::format_fee(1_000_000), "$1.00");
    }

    #[test]
    fn format_fee_ten_dollars() {
        assert_eq!(FeeSchedule::format_fee(10_000_000), "$10.00");
    }

    #[test]
    fn fee_in_usdc_conversion() {
        assert!((FeeSchedule::fee_in_usdc(500_000) - 0.5).abs() < f64::EPSILON);
    }

    // -- Display ------------------------------------------------------------

    #[test]
    fn fee_schedule_display() {
        let s = FeeSchedule::default_testnet();
        let display = format!("{s}");
        assert!(display.contains("transfer"));
        assert!(display.contains("deploy"));
    }

    #[test]
    fn fee_distribution_display() {
        let d = FeeDistribution::default_split();
        let display = format!("{d}");
        assert!(display.contains("80.0%"));
        assert!(display.contains("20.0%"));
    }

    // -- Serde roundtrip ----------------------------------------------------

    #[test]
    fn fee_schedule_serde_roundtrip() {
        let s = FeeSchedule::default_testnet();
        let json = serde_json::to_string(&s).unwrap();
        let parsed: FeeSchedule = serde_json::from_str(&json).unwrap();
        assert_eq!(s, parsed);
    }

    #[test]
    fn fee_distribution_serde_roundtrip() {
        let d = FeeDistribution::default_split();
        let json = serde_json::to_string(&d).unwrap();
        let parsed: FeeDistribution = serde_json::from_str(&json).unwrap();
        assert_eq!(d, parsed);
    }

    // -- Development schedule -----------------------------------------------

    #[test]
    fn development_schedule_cheaper() {
        let testnet = FeeSchedule::default_testnet();
        let dev = FeeSchedule::development();
        assert!(dev.base_transfer_fee < testnet.base_transfer_fee);
        assert!(dev.base_deploy_fee < testnet.base_deploy_fee);
    }
}
