use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Output format for CLI display.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Pretty,
    Json,
    Table,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Pretty => write!(f, "pretty"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Table => write!(f, "table"),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "pretty" => Ok(OutputFormat::Pretty),
            "json" => Ok(OutputFormat::Json),
            "table" => Ok(OutputFormat::Table),
            _ => anyhow::bail!(
                "invalid output format '{}', expected: pretty, json, table",
                s
            ),
        }
    }
}

/// CLI configuration persisted at `~/.dina/config.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CliConfig {
    pub rpc_url: String,
    pub rest_url: String,
    pub default_wallet: Option<String>,
    pub chain_id: String,
    pub output_format: OutputFormat,
    pub wallet_dir: String,
}

impl Default for CliConfig {
    fn default() -> Self {
        let home = dirs::home_dir()
            .map(|h| h.join(".dina").join("wallets"))
            .unwrap_or_else(|| PathBuf::from(".dina/wallets"));

        Self {
            rpc_url: "http://localhost:8545".to_string(),
            rest_url: "http://localhost:8080".to_string(),
            default_wallet: None,
            chain_id: "dina-testnet-1".to_string(),
            output_format: OutputFormat::Pretty,
            wallet_dir: home.to_string_lossy().to_string(),
        }
    }
}

impl CliConfig {
    /// Path to the config file.
    fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("cannot determine home directory")?;
        Ok(home.join(".dina").join("config.json"))
    }

    /// Load config from disk, falling back to defaults if the file does not exist.
    pub fn load() -> Self {
        let path = match Self::config_path() {
            Ok(p) => p,
            Err(_) => return Self::default(),
        };

        if !path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save config to disk.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create config dir {:?}", parent))?;
        }

        let json = serde_json::to_string_pretty(self).context("failed to serialize config")?;
        std::fs::write(&path, json)
            .with_context(|| format!("failed to write config to {:?}", path))?;
        Ok(())
    }

    /// Set a config value by key name.
    pub fn set_value(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "rpc-url" | "rpc_url" => self.rpc_url = value.to_string(),
            "rest-url" | "rest_url" => self.rest_url = value.to_string(),
            "chain-id" | "chain_id" => self.chain_id = value.to_string(),
            "format" | "output-format" | "output_format" => {
                self.output_format = value.parse()?;
            }
            "default-wallet" | "default_wallet" => {
                if value == "none" || value.is_empty() {
                    self.default_wallet = None;
                } else {
                    self.default_wallet = Some(value.to_string());
                }
            }
            "wallet-dir" | "wallet_dir" => self.wallet_dir = value.to_string(),
            _ => anyhow::bail!(
                "unknown config key '{key}'. Valid keys: rpc-url, rest-url, chain-id, format, default-wallet, wallet-dir"
            ),
        }
        Ok(())
    }

    /// Determine effective output format, considering --json flag override.
    pub fn effective_format(&self, json_flag: bool) -> OutputFormat {
        if json_flag {
            OutputFormat::Json
        } else {
            self.output_format.clone()
        }
    }
}
