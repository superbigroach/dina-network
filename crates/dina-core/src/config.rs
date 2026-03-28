use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

use crate::error::{DinaError, DinaResult};
use crate::fees::{FeeDistribution, FeeSchedule};

// ---------------------------------------------------------------------------
// Network Configuration
// ---------------------------------------------------------------------------

/// Top-level configuration for a Dina Network node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Unique chain identifier (e.g. `"dina-testnet-1"`).
    pub chain_id: String,
    /// Target block production interval in milliseconds.
    pub block_time_ms: u64,
    /// Maximum block payload size in bytes.
    pub max_block_size_bytes: usize,
    /// Maximum number of transactions allowed in a single block.
    pub max_transactions_per_block: usize,
    /// Maximum WASM bytecode size for a deployed contract.
    pub max_contract_size_bytes: usize,
    /// Number of blocks a payment-channel challenge remains open.
    pub channel_challenge_period_blocks: u64,
    /// Maximum lifetime of a payment channel in blocks.
    pub channel_max_duration_blocks: u64,
    /// Fee parameters.
    pub fee_schedule: FeeSchedule,
    /// Fee distribution between validator and treasury.
    pub fee_distribution: FeeDistribution,
    /// Consensus engine parameters.
    pub consensus: ConsensusConfig,
    /// Privacy feature flags.
    pub privacy: PrivacyConfig,
    /// Parallel execution engine configuration.
    pub parallel_execution: ParallelExecutionConfig,
}

impl NetworkConfig {
    /// Configuration for the public testnet.
    pub fn testnet() -> Self {
        Self {
            chain_id: "dina-testnet-1".to_string(),
            block_time_ms: 100,
            max_block_size_bytes: 1_048_576, // 1 MB
            max_transactions_per_block: 1_000,
            max_contract_size_bytes: 512_000,       // 500 KB
            channel_challenge_period_blocks: 1_000, // ~100 seconds at 100ms blocks
            channel_max_duration_blocks: 864_000,   // ~24 hours
            fee_schedule: FeeSchedule::default_testnet(),
            fee_distribution: FeeDistribution::default_split(),
            consensus: ConsensusConfig::default_testnet(),
            privacy: PrivacyConfig::testnet(),
            parallel_execution: ParallelExecutionConfig::default_config(),
        }
    }

    /// Configuration for mainnet (more conservative limits).
    pub fn mainnet() -> Self {
        Self {
            chain_id: "dina-mainnet-1".to_string(),
            block_time_ms: 100,
            max_block_size_bytes: 1_048_576,
            max_transactions_per_block: 1_000,
            max_contract_size_bytes: 512_000,
            channel_challenge_period_blocks: 1_000,
            channel_max_duration_blocks: 864_000,
            fee_schedule: FeeSchedule::default_testnet(), // same fee schedule
            fee_distribution: FeeDistribution::default_split(),
            consensus: ConsensusConfig::default_mainnet(),
            privacy: PrivacyConfig::mainnet(),
            parallel_execution: ParallelExecutionConfig::default_config(),
        }
    }

    /// Minimal configuration for local development: fast blocks, low fees,
    /// single validator, all privacy features enabled for testing.
    pub fn development() -> Self {
        Self {
            chain_id: "dina-dev".to_string(),
            block_time_ms: 50,               // faster blocks
            max_block_size_bytes: 4_194_304, // 4 MB
            max_transactions_per_block: 5_000,
            max_contract_size_bytes: 2_097_152,   // 2 MB
            channel_challenge_period_blocks: 100, // short for testing
            channel_max_duration_blocks: 86_400,  // ~72 min at 50ms
            fee_schedule: FeeSchedule::development(),
            fee_distribution: FeeDistribution::default_split(),
            consensus: ConsensusConfig::development(),
            privacy: PrivacyConfig::development(),
            parallel_execution: ParallelExecutionConfig::default_config(),
        }
    }

    /// Load configuration from a JSON file.
    pub fn from_file(path: &str) -> DinaResult<Self> {
        let contents = std::fs::read_to_string(Path::new(path)).map_err(|e| {
            DinaError::StorageError(format!("failed to read config file {path}: {e}"))
        })?;
        serde_json::from_str(&contents).map_err(|e| {
            DinaError::SerializationError(format!("failed to parse config file {path}: {e}"))
        })
    }

    /// Persist configuration to a JSON file.
    pub fn to_file(&self, path: &str) -> DinaResult<()> {
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            DinaError::SerializationError(format!("failed to serialize config: {e}"))
        })?;
        std::fs::write(Path::new(path), json).map_err(|e| {
            DinaError::StorageError(format!("failed to write config file {path}: {e}"))
        })
    }

    /// Validate internal consistency of the configuration.
    pub fn validate(&self) -> DinaResult<()> {
        if self.chain_id.is_empty() {
            return Err(DinaError::Custom("chain_id must not be empty".into()));
        }
        if self.block_time_ms == 0 {
            return Err(DinaError::Custom("block_time_ms must be > 0".into()));
        }
        if self.max_block_size_bytes == 0 {
            return Err(DinaError::Custom("max_block_size_bytes must be > 0".into()));
        }
        if self.max_transactions_per_block == 0 {
            return Err(DinaError::Custom(
                "max_transactions_per_block must be > 0".into(),
            ));
        }
        if !self.fee_distribution.is_valid() {
            return Err(DinaError::Custom(
                "fee_distribution shares must sum to 10,000 bps".into(),
            ));
        }
        self.consensus.validate()?;
        Ok(())
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self::testnet()
    }
}

impl fmt::Display for NetworkConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "NetworkConfig {{")?;
        writeln!(f, "  chain_id              : {}", self.chain_id)?;
        writeln!(f, "  block_time            : {}ms", self.block_time_ms)?;
        writeln!(
            f,
            "  max_block_size        : {} bytes",
            self.max_block_size_bytes
        )?;
        writeln!(
            f,
            "  max_txns_per_block    : {}",
            self.max_transactions_per_block
        )?;
        writeln!(
            f,
            "  max_contract_size     : {} bytes",
            self.max_contract_size_bytes
        )?;
        writeln!(
            f,
            "  channel_challenge     : {} blocks",
            self.channel_challenge_period_blocks
        )?;
        writeln!(
            f,
            "  channel_max_duration  : {} blocks",
            self.channel_max_duration_blocks
        )?;
        writeln!(f, "  consensus             : {}", self.consensus)?;
        writeln!(f, "  privacy               : {}", self.privacy)?;
        writeln!(f, "  parallel_execution    : {}", self.parallel_execution)?;
        write!(f, "}}")
    }
}

// ---------------------------------------------------------------------------
// Consensus Configuration
// ---------------------------------------------------------------------------

/// Parameters governing the BFT consensus engine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Minimum number of validators required for the network to operate.
    pub min_validators: usize,
    /// Maximum number of validators in the active set.
    pub max_validators: usize,
    /// Timeout for consensus rounds in milliseconds.
    pub timeout_ms: u64,
    /// Whether HotStuff-style pipelining is enabled.
    pub pipelining_enabled: bool,
}

impl ConsensusConfig {
    pub fn default_testnet() -> Self {
        Self {
            min_validators: 3,
            max_validators: 7,
            timeout_ms: 500,
            pipelining_enabled: true,
        }
    }

    pub fn default_mainnet() -> Self {
        Self {
            min_validators: 4,
            max_validators: 7,
            timeout_ms: 500,
            pipelining_enabled: true,
        }
    }

    pub fn development() -> Self {
        Self {
            min_validators: 1,
            max_validators: 1,
            timeout_ms: 200,
            pipelining_enabled: false,
        }
    }

    /// Validate consensus parameters.
    pub fn validate(&self) -> DinaResult<()> {
        if self.min_validators == 0 {
            return Err(DinaError::Custom("min_validators must be > 0".into()));
        }
        if self.max_validators < self.min_validators {
            return Err(DinaError::Custom(
                "max_validators must be >= min_validators".into(),
            ));
        }
        if self.timeout_ms == 0 {
            return Err(DinaError::Custom("consensus timeout_ms must be > 0".into()));
        }
        Ok(())
    }
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self::default_testnet()
    }
}

impl fmt::Display for ConsensusConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Consensus {{ validators: {}-{}, timeout: {}ms, pipelining: {} }}",
            self.min_validators,
            self.max_validators,
            self.timeout_ms,
            if self.pipelining_enabled { "on" } else { "off" },
        )
    }
}

// ---------------------------------------------------------------------------
// Privacy Configuration
// ---------------------------------------------------------------------------

/// Feature flags for the privacy subsystem.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Allow encrypted memos on transfers.
    pub encrypted_memos_enabled: bool,
    /// Allow stealth-address derivation (DRC-111).
    pub stealth_addresses_enabled: bool,
    /// Allow view-key disclosure (DRC-112).
    pub view_keys_enabled: bool,
    /// Enable ZK-proof verification (Phase 5 — disabled by default).
    pub zk_proofs_enabled: bool,
}

impl PrivacyConfig {
    pub fn testnet() -> Self {
        Self {
            encrypted_memos_enabled: true,
            stealth_addresses_enabled: true,
            view_keys_enabled: true,
            zk_proofs_enabled: false,
        }
    }

    pub fn mainnet() -> Self {
        Self {
            encrypted_memos_enabled: true,
            stealth_addresses_enabled: true,
            view_keys_enabled: true,
            zk_proofs_enabled: false, // Phase 5
        }
    }

    pub fn development() -> Self {
        Self {
            encrypted_memos_enabled: true,
            stealth_addresses_enabled: true,
            view_keys_enabled: true,
            zk_proofs_enabled: true, // enable everything for testing
        }
    }
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self::testnet()
    }
}

impl fmt::Display for PrivacyConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let flags: Vec<&str> = [
            self.encrypted_memos_enabled.then_some("encrypted-memos"),
            self.stealth_addresses_enabled.then_some("stealth-addr"),
            self.view_keys_enabled.then_some("view-keys"),
            self.zk_proofs_enabled.then_some("zk-proofs"),
        ]
        .into_iter()
        .flatten()
        .collect();
        write!(f, "Privacy {{ {} }}", flags.join(", "))
    }
}

// ---------------------------------------------------------------------------
// Parallel Execution Configuration
// ---------------------------------------------------------------------------

/// Configuration for the parallel transaction execution engine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParallelExecutionConfig {
    /// Whether parallel execution is enabled. When false, the sequential
    /// `BlockExecutor` is used instead.
    pub enabled: bool,
    /// Maximum number of execution lanes. 0 means auto-detect from the
    /// number of available CPU cores.
    pub max_lanes: usize,
    /// Minimum number of transactions in a block before the parallel
    /// engine is engaged. Blocks smaller than this threshold are executed
    /// sequentially to avoid thread-spawn overhead.
    pub min_txs_for_parallel: usize,
}

impl ParallelExecutionConfig {
    /// Sensible defaults: enabled, auto-detect lanes, require at least 4 txs.
    pub fn default_config() -> Self {
        Self {
            enabled: true,
            max_lanes: 0,
            min_txs_for_parallel: 4,
        }
    }

    /// Disabled configuration (sequential execution only).
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            max_lanes: 1,
            min_txs_for_parallel: usize::MAX,
        }
    }
}

impl Default for ParallelExecutionConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

impl fmt::Display for ParallelExecutionConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.enabled {
            let lanes = if self.max_lanes == 0 {
                "auto".to_string()
            } else {
                self.max_lanes.to_string()
            };
            write!(
                f,
                "ParallelExec {{ lanes: {}, min_txs: {} }}",
                lanes, self.min_txs_for_parallel
            )
        } else {
            write!(f, "ParallelExec {{ disabled }}")
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Preset configs -----------------------------------------------------

    #[test]
    fn testnet_config_valid() {
        let cfg = NetworkConfig::testnet();
        assert!(cfg.validate().is_ok());
        assert_eq!(cfg.chain_id, "dina-testnet-1");
        assert_eq!(cfg.block_time_ms, 100);
    }

    #[test]
    fn mainnet_config_valid() {
        let cfg = NetworkConfig::mainnet();
        assert!(cfg.validate().is_ok());
        assert_eq!(cfg.chain_id, "dina-mainnet-1");
    }

    #[test]
    fn development_config_valid() {
        let cfg = NetworkConfig::development();
        assert!(cfg.validate().is_ok());
        assert_eq!(cfg.consensus.min_validators, 1);
        assert_eq!(cfg.consensus.max_validators, 1);
        assert!(cfg.privacy.zk_proofs_enabled); // all features on for dev
    }

    // -- Testnet parameters -------------------------------------------------

    #[test]
    fn testnet_block_limits() {
        let cfg = NetworkConfig::testnet();
        assert_eq!(cfg.max_block_size_bytes, 1_048_576);
        assert_eq!(cfg.max_transactions_per_block, 1_000);
        assert_eq!(cfg.max_contract_size_bytes, 512_000);
    }

    #[test]
    fn testnet_channel_params() {
        let cfg = NetworkConfig::testnet();
        assert_eq!(cfg.channel_challenge_period_blocks, 1_000);
        assert_eq!(cfg.channel_max_duration_blocks, 864_000);
    }

    // -- Consensus validation -----------------------------------------------

    #[test]
    fn consensus_min_validators_zero_invalid() {
        let mut cfg = NetworkConfig::testnet();
        cfg.consensus.min_validators = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn consensus_max_less_than_min_invalid() {
        let mut cfg = NetworkConfig::testnet();
        cfg.consensus.min_validators = 5;
        cfg.consensus.max_validators = 3;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn consensus_timeout_zero_invalid() {
        let mut cfg = NetworkConfig::testnet();
        cfg.consensus.timeout_ms = 0;
        assert!(cfg.validate().is_err());
    }

    // -- Config validation --------------------------------------------------

    #[test]
    fn empty_chain_id_invalid() {
        let mut cfg = NetworkConfig::testnet();
        cfg.chain_id = String::new();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn zero_block_time_invalid() {
        let mut cfg = NetworkConfig::testnet();
        cfg.block_time_ms = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn bad_fee_distribution_invalid() {
        let mut cfg = NetworkConfig::testnet();
        cfg.fee_distribution.treasury_share_bps = 5_000; // 8000 + 5000 != 10000
        assert!(cfg.validate().is_err());
    }

    // -- Serde roundtrip ----------------------------------------------------

    #[test]
    fn config_serde_roundtrip() {
        let cfg = NetworkConfig::testnet();
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let parsed: NetworkConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, parsed);
    }

    // -- File I/O -----------------------------------------------------------

    #[test]
    fn config_file_roundtrip() {
        let cfg = NetworkConfig::testnet();
        let dir = std::env::temp_dir();
        let path = dir.join("dina_test_config.json");
        let path_str = path.to_str().unwrap();

        cfg.to_file(path_str).unwrap();
        let loaded = NetworkConfig::from_file(path_str).unwrap();
        assert_eq!(cfg, loaded);

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn config_from_missing_file_errors() {
        let result = NetworkConfig::from_file("/nonexistent/path/config.json");
        assert!(result.is_err());
    }

    #[test]
    fn config_from_invalid_json_errors() {
        let dir = std::env::temp_dir();
        let path = dir.join("dina_bad_config.json");
        std::fs::write(&path, "not valid json {{{").unwrap();
        let result = NetworkConfig::from_file(path.to_str().unwrap());
        assert!(result.is_err());
        let _ = std::fs::remove_file(&path);
    }

    // -- Display ------------------------------------------------------------

    #[test]
    fn network_config_display() {
        let cfg = NetworkConfig::testnet();
        let display = format!("{cfg}");
        assert!(display.contains("dina-testnet-1"));
        assert!(display.contains("100ms"));
    }

    #[test]
    fn consensus_config_display() {
        let c = ConsensusConfig::default_testnet();
        let display = format!("{c}");
        assert!(display.contains("3-7"));
        assert!(display.contains("pipelining: on"));
    }

    #[test]
    fn privacy_config_display() {
        let p = PrivacyConfig::testnet();
        let display = format!("{p}");
        assert!(display.contains("encrypted-memos"));
        assert!(display.contains("view-keys"));
        assert!(!display.contains("zk-proofs")); // disabled on testnet
    }

    #[test]
    fn privacy_dev_has_zk() {
        let p = PrivacyConfig::development();
        let display = format!("{p}");
        assert!(display.contains("zk-proofs"));
    }

    // -- Default impls ------------------------------------------------------

    #[test]
    fn default_network_config_is_testnet() {
        assert_eq!(NetworkConfig::default(), NetworkConfig::testnet());
    }

    #[test]
    fn default_consensus_is_testnet() {
        assert_eq!(
            ConsensusConfig::default(),
            ConsensusConfig::default_testnet()
        );
    }

    #[test]
    fn default_privacy_is_testnet() {
        assert_eq!(PrivacyConfig::default(), PrivacyConfig::testnet());
    }
}
