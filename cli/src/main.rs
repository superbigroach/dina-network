mod config_manager;
mod rpc_client;
mod wallet_manager;

use std::str::FromStr;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

use dina_core::transaction::Sig64;
use dina_core::types::Address;

use crate::config_manager::{CliConfig, OutputFormat};
use crate::rpc_client::RpcClient;
use crate::wallet_manager::WalletManager;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Format micro-USDC (u64) as a human-readable string like "100.500000 USDC".
fn format_usdc(micro: u64) -> String {
    let whole = micro / 1_000_000;
    let frac = micro % 1_000_000;
    format!("{whole}.{frac:06} USDC")
}

/// Parse a human-friendly USDC amount string (e.g. "100.50") into micro-USDC.
/// Also accepts plain integers which are treated as micro-USDC directly.
fn parse_usdc_amount(s: &str) -> Result<u64> {
    if let Ok(v) = s.parse::<u64>() {
        return Ok(v);
    }
    if let Some((whole_s, frac_s)) = s.split_once('.') {
        let whole: u64 = whole_s.parse().context("invalid USDC amount")?;
        let padded = format!("{:0<6}", frac_s);
        if padded.len() > 6 {
            anyhow::bail!("USDC amounts support at most 6 decimal places");
        }
        let frac: u64 = padded.parse().context("invalid USDC fraction")?;
        Ok(whole * 1_000_000 + frac)
    } else {
        anyhow::bail!("invalid USDC amount: '{s}'")
    }
}

/// Prompt for a password from stdin (no echo when possible).
fn prompt_password(prompt: &str) -> Result<String> {
    eprint!("{prompt}");
    let mut password = String::new();
    std::io::stdin()
        .read_line(&mut password)
        .context("failed to read password")?;
    Ok(password.trim().to_string())
}

/// Load a signing key from a raw 32-byte key file.
fn load_signing_key(path: &str) -> Result<SigningKey> {
    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read key file '{path}'"))?;

    if bytes.len() != 32 {
        anyhow::bail!(
            "key file has invalid length: expected 32 bytes, got {}",
            bytes.len()
        );
    }

    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&bytes);
    Ok(SigningKey::from_bytes(&key_bytes))
}

/// Resolve a signing key: use --wallet name (prompting for password) or --key file.
fn resolve_signing_key(
    wallet_name: &Option<String>,
    key_path: &str,
    config: &CliConfig,
) -> Result<SigningKey> {
    // Explicit --wallet takes priority.
    if let Some(name) = wallet_name {
        let mgr = WalletManager::default_path()?;
        let password = prompt_password(&format!("Password for wallet '{name}': "))?;
        return mgr.signing_key(name, &password);
    }

    // If key_path is the default ("dina_key") and a default wallet exists, prefer it.
    if key_path == "dina_key" {
        if let Some(ref default_name) = config.default_wallet {
            let mgr = WalletManager::default_path()?;
            if mgr.wallet_dir().join(format!("{default_name}.json")).exists() {
                let password =
                    prompt_password(&format!("Password for wallet '{default_name}': "))?;
                return mgr.signing_key(default_name, &password);
            }
        }
    }

    // Fall back to raw key file.
    load_signing_key(key_path)
}

/// Pretty-print a serde_json::Value with coloring, or print raw JSON.
fn print_value(val: &serde_json::Value, format: &OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(val).unwrap_or_default());
        }
        _ => {
            println!(
                "{}",
                serde_json::to_string_pretty(val).unwrap_or_default()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

/// Dina CLI -- interact with the Dina Network.
#[derive(Parser, Debug)]
#[command(name = "dina", version, about = "Dina Network CLI tool")]
struct Cli {
    /// JSON-RPC endpoint URL.
    #[arg(long, global = true)]
    rpc_url: Option<String>,

    /// Output as raw JSON.
    #[arg(long, global = true, default_value_t = false)]
    json: bool,

    /// Named wallet to use for signing (prompts for password).
    #[arg(long, global = true)]
    wallet: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a new Ed25519 keypair.
    Keygen {
        /// Output file path for the private key (default: ./dina_key).
        #[arg(short, long, default_value = "dina_key")]
        output: String,
    },

    /// Check the USDC balance of an address.
    Balance {
        /// The address to query (hex, with or without 0x prefix).
        address: String,
    },

    /// Send USDC to another address.
    Transfer {
        /// Recipient address (hex).
        to: String,
        /// Amount in micro-USDC (6 decimals), or decimal like "100.50".
        amount: String,
        /// Path to the sender's private key file.
        #[arg(short, long, default_value = "dina_key")]
        key: String,
        /// Optional memo (UTF-8 text).
        #[arg(short, long)]
        memo: Option<String>,
    },

    /// Deploy a smart contract from a WASM file.
    Deploy {
        /// Path to the WASM binary.
        wasm_file: String,
        /// Path to the deployer's private key file.
        #[arg(short, long, default_value = "dina_key")]
        key: String,
        /// Hex-encoded init arguments (optional).
        #[arg(long, default_value = "")]
        init_args: String,
    },

    /// Call a method on a deployed smart contract.
    Call {
        /// Contract address (hex).
        contract: String,
        /// Method name to invoke.
        method: String,
        /// Hex-encoded call arguments (optional).
        #[arg(long, default_value = "")]
        args: String,
        /// USDC to attach to the call (micro-USDC).
        #[arg(long, default_value_t = 0)]
        value: u64,
        /// Path to the caller's private key file.
        #[arg(short, long, default_value = "dina_key")]
        key: String,
    },

    /// Device management commands.
    Device {
        #[command(subcommand)]
        action: DeviceCommands,
    },

    /// Get block information.
    Block {
        /// Block height or "latest".
        height: String,
    },

    /// Get transaction information by hash.
    Tx {
        /// Transaction hash (hex).
        hash: String,
    },

    /// Show node status and network information.
    Status,

    // ------ New subcommand groups ------
    /// Wallet management commands.
    Wallet {
        #[command(subcommand)]
        action: WalletCommands,
    },

    /// CLI configuration commands.
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },

    /// Smart contract commands.
    Contract {
        #[command(subcommand)]
        action: ContractCommands,
    },

    /// Payment channel commands.
    Channel {
        #[command(subcommand)]
        action: ChannelCommands,
    },

    /// Request testnet USDC from the faucet.
    Faucet {
        /// The address to fund.
        address: String,
    },

    /// Validator commands.
    Validators {
        #[command(subcommand)]
        action: ValidatorCommands,
    },

    /// Block explorer commands.
    Explorer {
        #[command(subcommand)]
        action: ExplorerCommands,
    },
}

// ---------------------------------------------------------------------------
// Subcommand enums
// ---------------------------------------------------------------------------

#[derive(Subcommand, Debug)]
enum DeviceCommands {
    /// Register a new Cognitum device on-chain.
    Register {
        /// Path to the device's private key file.
        #[arg(short, long, default_value = "dina_key")]
        key: String,
    },

    /// Get information about a registered device.
    Info {
        /// Device public key (hex).
        pubkey: String,
    },
}

#[derive(Subcommand, Debug)]
enum WalletCommands {
    /// Create a new wallet.
    Create {
        /// Wallet name.
        name: String,
    },

    /// List all wallets.
    List,

    /// Delete a wallet.
    Delete {
        /// Wallet name.
        name: String,
    },

    /// Export private key (hex).
    Export {
        /// Wallet name.
        name: String,
    },

    /// Import a wallet from a hex private key.
    Import {
        /// Wallet name.
        name: String,
        /// Hex-encoded private key.
        key: String,
    },

    /// Set the default wallet.
    SetDefault {
        /// Wallet name.
        name: String,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    /// Show current configuration.
    Show,

    /// Set a configuration value.
    Set {
        /// Config key (rpc-url, rest-url, chain-id, format, default-wallet, wallet-dir).
        key: String,
        /// New value.
        value: String,
    },

    /// Reset configuration to defaults.
    Reset,
}

#[derive(Subcommand, Debug)]
enum ContractCommands {
    /// Deploy a smart contract (with gas estimate).
    Deploy {
        /// Path to the WASM binary.
        wasm: String,
        /// Path to the deployer's private key file.
        #[arg(short, long, default_value = "dina_key")]
        key: String,
        /// Hex-encoded init arguments (optional).
        #[arg(long, default_value = "")]
        init_args: String,
    },

    /// Call a method on a deployed smart contract.
    Call {
        /// Contract address (hex).
        addr: String,
        /// Method name.
        method: String,
        /// Hex-encoded call arguments (optional).
        #[arg(long, default_value = "")]
        args: String,
        /// USDC to attach (micro-USDC).
        #[arg(long, default_value_t = 0)]
        value: u64,
        /// Path to the caller's private key file.
        #[arg(short, long, default_value = "dina_key")]
        key: String,
    },

    /// Get contract information.
    Info {
        /// Contract address (hex).
        addr: String,
    },
}

#[derive(Subcommand, Debug)]
enum ChannelCommands {
    /// Open a payment channel with a peer.
    Open {
        /// Peer address (hex).
        peer: String,
        /// Amount to deposit (micro-USDC or decimal like "100.50").
        amount: String,
        /// Path to the sender's private key file.
        #[arg(short, long, default_value = "dina_key")]
        key: String,
    },

    /// Send a payment through an open channel.
    Pay {
        /// Channel ID (hex hash).
        channel_id: String,
        /// Amount to pay (micro-USDC or decimal like "1.50").
        amount: String,
        /// Path to the sender's private key file.
        #[arg(short, long, default_value = "dina_key")]
        key: String,
    },

    /// Close and settle a payment channel.
    Close {
        /// Channel ID (hex hash).
        channel_id: String,
        /// Path to the sender's private key file.
        #[arg(short, long, default_value = "dina_key")]
        key: String,
    },

    /// List open payment channels.
    List,
}

#[derive(Subcommand, Debug)]
enum ValidatorCommands {
    /// List active validators.
    List,

    /// Show details for a specific validator.
    Info {
        /// Validator address (hex).
        addr: String,
    },
}

#[derive(Subcommand, Debug)]
enum ExplorerCommands {
    /// Show recent blocks.
    Blocks {
        /// Number of blocks to show.
        #[arg(long, default_value_t = 10)]
        limit: u64,
    },

    /// Show transaction details by hash.
    Tx {
        /// Transaction hash (hex).
        hash: String,
    },

    /// Search blocks and transactions.
    Search {
        /// Search query (address, hash, or block height).
        query: String,
    },
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = CliConfig::load();

    let rpc_url = cli
        .rpc_url
        .as_deref()
        .unwrap_or(&config.rpc_url);
    let client = RpcClient::new(rpc_url);
    let format = config.effective_format(cli.json);

    match cli.command {
        // ---- Original commands (preserved) ----
        Commands::Keygen { output } => cmd_keygen(&output, &format)?,
        Commands::Balance { address } => cmd_balance(&client, &address, &format).await?,
        Commands::Transfer {
            to,
            amount,
            key,
            memo,
        } => {
            let micro = parse_usdc_amount(&amount)?;
            let signer = resolve_signing_key(&cli.wallet, &key, &config)?;
            cmd_transfer(&client, &to, micro, &signer, memo, &format).await?;
        }
        Commands::Deploy {
            wasm_file,
            key,
            init_args,
        } => {
            let signer = resolve_signing_key(&cli.wallet, &key, &config)?;
            cmd_deploy(&client, &wasm_file, &signer, &init_args, &format).await?;
        }
        Commands::Call {
            contract,
            method,
            args,
            value,
            key,
        } => {
            let signer = resolve_signing_key(&cli.wallet, &key, &config)?;
            cmd_call(&client, &contract, &method, &args, value, &signer, &format).await?;
        }
        Commands::Device { action } => match action {
            DeviceCommands::Register { key } => {
                let signer = resolve_signing_key(&cli.wallet, &key, &config)?;
                cmd_device_register(&client, &signer, &format).await?;
            }
            DeviceCommands::Info { pubkey } => {
                cmd_device_info(&client, &pubkey, &format).await?;
            }
        },
        Commands::Block { height } => cmd_block(&client, &height, &format).await?,
        Commands::Tx { hash } => cmd_tx(&client, &hash, &format).await?,
        Commands::Status => cmd_status(&client, &format).await?,

        // ---- Wallet commands ----
        Commands::Wallet { action } => cmd_wallet(action, &config, &format)?,

        // ---- Config commands ----
        Commands::Config { action } => cmd_config(action, &format)?,

        // ---- Contract commands ----
        Commands::Contract { action } => {
            cmd_contract(action, &client, &cli.wallet, &config, &format).await?;
        }

        // ---- Channel commands ----
        Commands::Channel { action } => {
            cmd_channel(action, &client, &cli.wallet, &config, &format).await?;
        }

        // ---- Faucet ----
        Commands::Faucet { address } => cmd_faucet(&client, &address, &config, &format).await?,

        // ---- Validators ----
        Commands::Validators { action } => {
            cmd_validators(action, &client, &format).await?;
        }

        // ---- Explorer ----
        Commands::Explorer { action } => {
            cmd_explorer(action, &client, &format).await?;
        }
    }

    Ok(())
}

// ===========================================================================
// Original command implementations (updated for format support)
// ===========================================================================

/// Generate a new Ed25519 keypair, save the private key, and print the address.
fn cmd_keygen(output: &str, format: &OutputFormat) -> Result<()> {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    let address = Address::from_pubkey(&verifying_key);

    std::fs::write(output, signing_key.as_bytes())
        .with_context(|| format!("failed to write key to '{output}'"))?;

    let pubkey_path = format!("{output}.pub");
    std::fs::write(&pubkey_path, verifying_key.as_bytes())
        .with_context(|| format!("failed to write public key to '{pubkey_path}'"))?;

    if *format == OutputFormat::Json {
        let val = serde_json::json!({
            "private_key_file": output,
            "public_key_file": pubkey_path,
            "address": address.to_string(),
            "public_key": format!("0x{}", hex::encode(verifying_key.as_bytes())),
        });
        println!("{}", serde_json::to_string(&val)?);
    } else {
        println!("{}", "Key pair generated successfully.".green());
        println!("  Private key: {output}");
        println!("  Public key:  {pubkey_path}");
        println!("  Address:     {}", address.to_string().cyan());
        println!(
            "  Public key:  0x{}",
            hex::encode(verifying_key.as_bytes())
        );
    }

    Ok(())
}

/// Query the balance of an address via JSON-RPC.
async fn cmd_balance(client: &RpcClient, address: &str, format: &OutputFormat) -> Result<()> {
    let addr =
        Address::from_str(address).map_err(|e| anyhow::anyhow!("invalid address: {e}"))?;

    let balance = client.get_balance(&addr.to_string()).await?;

    if *format == OutputFormat::Json {
        let val = serde_json::json!({
            "address": addr.to_string(),
            "balance_micro_usdc": balance,
            "balance_usdc": format_usdc(balance),
        });
        println!("{}", serde_json::to_string(&val)?);
    } else {
        println!("Address: {}", addr.to_string().cyan());
        println!("Balance: {}", format_usdc(balance).green());
    }

    Ok(())
}

/// Send a USDC transfer transaction.
async fn cmd_transfer(
    client: &RpcClient,
    to: &str,
    amount: u64,
    signing_key: &SigningKey,
    memo: Option<String>,
    format: &OutputFormat,
) -> Result<()> {
    let from = Address::from_pubkey(&signing_key.verifying_key());
    let to_addr =
        Address::from_str(to).map_err(|e| anyhow::anyhow!("invalid recipient address: {e}"))?;

    let memo_bytes = memo.map(|m| m.into_bytes());

    let mut tx = dina_core::Transaction::Transfer {
        from,
        to: to_addr,
        amount,
        memo: memo_bytes,
        device_witness: None,
        nonce: 0,
        fee: 100,
        signature: Sig64([0u8; 64]),
    };

    let msg = tx.signing_bytes();
    let sig = dina_core::crypto::sign(signing_key, &msg);

    if let dina_core::Transaction::Transfer {
        ref mut signature, ..
    } = tx
    {
        *signature = Sig64(sig);
    }

    let tx_bytes = serde_json::to_vec(&tx).context("failed to serialize transaction")?;
    let tx_hex = format!("0x{}", hex::encode(&tx_bytes));
    let tx_hash = client.send_transaction(&tx_hex).await?;

    if *format == OutputFormat::Json {
        let val = serde_json::json!({
            "from": from.to_string(),
            "to": to_addr.to_string(),
            "amount_micro_usdc": amount,
            "amount_usdc": format_usdc(amount),
            "tx_hash": tx_hash,
        });
        println!("{}", serde_json::to_string(&val)?);
    } else {
        println!("{}", "Transfer submitted successfully.".green());
        println!("  From:    {}", from.to_string().cyan());
        println!("  To:      {}", to_addr.to_string().cyan());
        println!("  Amount:  {}", format_usdc(amount).yellow());
        println!("  Tx Hash: {}", tx_hash.cyan());
    }

    Ok(())
}

/// Deploy a smart contract.
async fn cmd_deploy(
    client: &RpcClient,
    wasm_file: &str,
    signing_key: &SigningKey,
    init_args_hex: &str,
    format: &OutputFormat,
) -> Result<()> {
    let from = Address::from_pubkey(&signing_key.verifying_key());

    let wasm_bytes = std::fs::read(wasm_file)
        .with_context(|| format!("failed to read WASM file '{wasm_file}'"))?;

    let wasm_size = wasm_bytes.len();
    // Estimate fee: base 1000 + 1 micro-USDC per 100 bytes of WASM.
    let estimated_fee: u64 = 1000 + (wasm_size as u64 / 100);

    let init_args = if init_args_hex.is_empty() {
        Vec::new()
    } else {
        let raw = init_args_hex.strip_prefix("0x").unwrap_or(init_args_hex);
        hex::decode(raw).context("invalid hex in init_args")?
    };

    let mut tx = dina_core::Transaction::DeployContract {
        from,
        wasm_bytecode: wasm_bytes,
        init_args,
        nonce: 0,
        fee: estimated_fee,
        signature: Sig64([0u8; 64]),
    };

    let msg = tx.signing_bytes();
    let sig = dina_core::crypto::sign(signing_key, &msg);

    if let dina_core::Transaction::DeployContract {
        ref mut signature, ..
    } = tx
    {
        *signature = Sig64(sig);
    }

    let tx_bytes = serde_json::to_vec(&tx).context("failed to serialize transaction")?;
    let tx_hex = format!("0x{}", hex::encode(&tx_bytes));
    let tx_hash = client.send_transaction(&tx_hex).await?;

    if *format == OutputFormat::Json {
        let val = serde_json::json!({
            "deployer": from.to_string(),
            "wasm_file": wasm_file,
            "wasm_size_bytes": wasm_size,
            "estimated_fee_micro_usdc": estimated_fee,
            "tx_hash": tx_hash,
        });
        println!("{}", serde_json::to_string(&val)?);
    } else {
        println!("{}", "Contract deployment submitted.".green());
        println!("  Deployer:       {}", from.to_string().cyan());
        println!("  WASM:           {wasm_file} ({wasm_size} bytes)");
        println!("  Estimated fee:  {}", format_usdc(estimated_fee).yellow());
        println!("  Tx Hash:        {}", tx_hash.cyan());
    }

    Ok(())
}

/// Call a method on a deployed smart contract.
async fn cmd_call(
    client: &RpcClient,
    contract: &str,
    method: &str,
    args_hex: &str,
    value: u64,
    signing_key: &SigningKey,
    format: &OutputFormat,
) -> Result<()> {
    let from = Address::from_pubkey(&signing_key.verifying_key());
    let contract_addr = Address::from_str(contract)
        .map_err(|e| anyhow::anyhow!("invalid contract address: {e}"))?;

    let args = if args_hex.is_empty() {
        Vec::new()
    } else {
        let raw = args_hex.strip_prefix("0x").unwrap_or(args_hex);
        hex::decode(raw).context("invalid hex in args")?
    };

    let mut tx = dina_core::Transaction::CallContract {
        from,
        contract: contract_addr,
        method: method.to_string(),
        args,
        usdc_attached: value,
        nonce: 0,
        fee: 500,
        signature: Sig64([0u8; 64]),
    };

    let msg = tx.signing_bytes();
    let sig = dina_core::crypto::sign(signing_key, &msg);

    if let dina_core::Transaction::CallContract {
        ref mut signature, ..
    } = tx
    {
        *signature = Sig64(sig);
    }

    let tx_bytes = serde_json::to_vec(&tx).context("failed to serialize transaction")?;
    let tx_hex = format!("0x{}", hex::encode(&tx_bytes));
    let tx_hash = client.send_transaction(&tx_hex).await?;

    if *format == OutputFormat::Json {
        let val = serde_json::json!({
            "from": from.to_string(),
            "contract": contract_addr.to_string(),
            "method": method,
            "value_micro_usdc": value,
            "tx_hash": tx_hash,
        });
        println!("{}", serde_json::to_string(&val)?);
    } else {
        println!("{}", "Contract call submitted.".green());
        println!("  From:     {}", from.to_string().cyan());
        println!("  Contract: {}", contract_addr.to_string().cyan());
        println!("  Method:   {method}");
        println!("  Value:    {}", format_usdc(value).yellow());
        println!("  Tx Hash:  {}", tx_hash.cyan());
    }

    Ok(())
}

/// Register a device on-chain.
async fn cmd_device_register(
    client: &RpcClient,
    signing_key: &SigningKey,
    format: &OutputFormat,
) -> Result<()> {
    let owner = Address::from_pubkey(&signing_key.verifying_key());
    let device_pubkey = signing_key.verifying_key().to_bytes();

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let attestation = dina_core::transaction::DeviceAttestation {
        pubkey: device_pubkey,
        firmware_hash: dina_core::types::Hash::ZERO,
        witness_root: dina_core::types::Hash::ZERO,
        timestamp: now_secs,
        signature: Sig64([0u8; 64]),
    };

    let mut tx = dina_core::Transaction::RegisterDevice {
        device_pubkey,
        owner,
        attestation,
        nonce: 0,
        fee: 200,
        signature: Sig64([0u8; 64]),
    };

    let msg = tx.signing_bytes();
    let sig = dina_core::crypto::sign(signing_key, &msg);

    if let dina_core::Transaction::RegisterDevice {
        ref mut signature, ..
    } = tx
    {
        *signature = Sig64(sig);
    }

    let tx_bytes = serde_json::to_vec(&tx).context("failed to serialize transaction")?;
    let tx_hex = format!("0x{}", hex::encode(&tx_bytes));
    let tx_hash = client.send_transaction(&tx_hex).await?;

    if *format == OutputFormat::Json {
        let val = serde_json::json!({
            "owner": owner.to_string(),
            "device_pubkey": format!("0x{}", hex::encode(device_pubkey)),
            "tx_hash": tx_hash,
        });
        println!("{}", serde_json::to_string(&val)?);
    } else {
        println!("{}", "Device registration submitted.".green());
        println!("  Owner:      {}", owner.to_string().cyan());
        println!("  Device key: 0x{}", hex::encode(device_pubkey));
        println!("  Tx Hash:    {}", tx_hash.cyan());
    }

    Ok(())
}

/// Get device info by public key.
async fn cmd_device_info(
    client: &RpcClient,
    pubkey: &str,
    format: &OutputFormat,
) -> Result<()> {
    let info = client.get_device(pubkey).await?;
    print_value(&info, format);
    Ok(())
}

/// Get block information by height or "latest".
async fn cmd_block(client: &RpcClient, height: &str, format: &OutputFormat) -> Result<()> {
    let info = if height == "latest" {
        client.get_latest_block().await?
    } else {
        let h: u64 = height
            .parse()
            .with_context(|| format!("invalid block height: '{height}'"))?;
        client.get_block(h).await?
    };

    print_value(&info, format);
    Ok(())
}

/// Get transaction information by hash.
async fn cmd_tx(client: &RpcClient, hash: &str, format: &OutputFormat) -> Result<()> {
    let info = client.get_transaction(hash).await?;
    print_value(&info, format);
    Ok(())
}

/// Show node status and network information.
async fn cmd_status(client: &RpcClient, format: &OutputFormat) -> Result<()> {
    let info = client.network_info().await?;

    if *format == OutputFormat::Json {
        println!("{}", serde_json::to_string(&info)?);
        return Ok(());
    }

    println!("{}", "Dina Network Node Status".bold());
    println!("{}", "========================".bold());
    println!(
        "  Chain ID:         {}",
        info.get("chain_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .cyan()
    );
    println!(
        "  Block Height:     {}",
        info.get("block_height")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
    );
    println!(
        "  Peer Count:       {}",
        info.get("peer_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
    );
    println!(
        "  Version:          {}",
        info.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
    );
    println!(
        "  Protocol Version: {}",
        info.get("protocol_version")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
    );

    Ok(())
}

// ===========================================================================
// Wallet commands
// ===========================================================================

fn cmd_wallet(action: WalletCommands, _config: &CliConfig, format: &OutputFormat) -> Result<()> {
    let mgr = WalletManager::default_path()?;
    let default_name = mgr.default_wallet()?;

    match action {
        WalletCommands::Create { name } => {
            let password = prompt_password("Set password: ")?;
            if password.is_empty() {
                anyhow::bail!("password cannot be empty");
            }
            let confirm = prompt_password("Confirm password: ")?;
            if password != confirm {
                anyhow::bail!("passwords do not match");
            }

            let wallet = mgr.create_wallet(&name, &password)?;

            if *format == OutputFormat::Json {
                let val = serde_json::json!({
                    "name": wallet.name,
                    "address": wallet.address,
                    "pubkey": wallet.pubkey_hex,
                    "created_at": wallet.created_at,
                });
                println!("{}", serde_json::to_string(&val)?);
            } else {
                println!("{}", "Wallet created successfully.".green());
                println!("  Name:    {}", wallet.name.cyan());
                println!("  Address: {}", wallet.address.cyan());
                println!("  Pubkey:  0x{}", wallet.pubkey_hex);
            }
        }

        WalletCommands::List => {
            let wallets = mgr.list_wallets()?;

            if *format == OutputFormat::Json {
                let items: Vec<_> = wallets
                    .iter()
                    .map(|w| {
                        serde_json::json!({
                            "name": w.name,
                            "address": w.address,
                            "pubkey": w.pubkey_hex,
                            "created_at": w.created_at,
                            "is_default": default_name.as_deref() == Some(&w.name),
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string(&items)?);
            } else if wallets.is_empty() {
                println!("No wallets found. Create one with: dina wallet create <name>");
            } else {
                println!("{}", "Wallets".bold());
                println!("{}", "-------".bold());
                for w in &wallets {
                    let marker = if default_name.as_deref() == Some(&w.name) {
                        " (default)".green().to_string()
                    } else {
                        String::new()
                    };
                    println!(
                        "  {} {}{marker}",
                        w.name.cyan(),
                        w.address.dimmed()
                    );
                }
            }
        }

        WalletCommands::Delete { name } => {
            mgr.delete_wallet(&name)?;

            if *format == OutputFormat::Json {
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({"deleted": name}))?
                );
            } else {
                println!("{}", format!("Wallet '{name}' deleted.").yellow());
            }
        }

        WalletCommands::Export { name } => {
            let password = prompt_password(&format!("Password for wallet '{name}': "))?;
            let hex_key = mgr.export_wallet(&name, &password)?;

            if *format == OutputFormat::Json {
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({"name": name, "private_key": hex_key}))?
                );
            } else {
                println!("{}", "WARNING: Never share your private key!".red().bold());
                println!("  Private key: 0x{hex_key}");
            }
        }

        WalletCommands::Import { name, key } => {
            let password = prompt_password("Set password: ")?;
            if password.is_empty() {
                anyhow::bail!("password cannot be empty");
            }

            let wallet = mgr.import_wallet(&name, &password, &key)?;

            if *format == OutputFormat::Json {
                let val = serde_json::json!({
                    "name": wallet.name,
                    "address": wallet.address,
                    "pubkey": wallet.pubkey_hex,
                });
                println!("{}", serde_json::to_string(&val)?);
            } else {
                println!("{}", "Wallet imported successfully.".green());
                println!("  Name:    {}", wallet.name.cyan());
                println!("  Address: {}", wallet.address.cyan());
            }
        }

        WalletCommands::SetDefault { name } => {
            mgr.set_default(&name)?;

            if *format == OutputFormat::Json {
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({"default_wallet": name}))?
                );
            } else {
                println!(
                    "{}",
                    format!("Default wallet set to '{name}'.").green()
                );
            }
        }
    }

    Ok(())
}

// ===========================================================================
// Config commands
// ===========================================================================

fn cmd_config(action: ConfigCommands, format: &OutputFormat) -> Result<()> {
    match action {
        ConfigCommands::Show => {
            let config = CliConfig::load();

            if *format == OutputFormat::Json {
                println!("{}", serde_json::to_string(&config)?);
            } else {
                println!("{}", "Dina CLI Configuration".bold());
                println!("{}", "======================".bold());
                println!("  rpc-url:        {}", config.rpc_url.cyan());
                println!("  rest-url:       {}", config.rest_url.cyan());
                println!("  chain-id:       {}", config.chain_id.cyan());
                println!("  format:         {}", config.output_format.to_string().cyan());
                println!(
                    "  default-wallet: {}",
                    config
                        .default_wallet
                        .as_deref()
                        .unwrap_or("(none)")
                        .cyan()
                );
                println!("  wallet-dir:     {}", config.wallet_dir.dimmed());
            }
        }

        ConfigCommands::Set { key, value } => {
            let mut config = CliConfig::load();
            config.set_value(&key, &value)?;
            config.save()?;

            if *format == OutputFormat::Json {
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({"key": key, "value": value}))?
                );
            } else {
                println!("{}", format!("Config '{key}' set to '{value}'.").green());
            }
        }

        ConfigCommands::Reset => {
            let config = CliConfig::default();
            config.save()?;

            if *format == OutputFormat::Json {
                println!("{}", serde_json::to_string(&config)?);
            } else {
                println!("{}", "Configuration reset to defaults.".green());
            }
        }
    }

    Ok(())
}

// ===========================================================================
// Contract commands (enhanced wrappers)
// ===========================================================================

async fn cmd_contract(
    action: ContractCommands,
    client: &RpcClient,
    wallet: &Option<String>,
    config: &CliConfig,
    format: &OutputFormat,
) -> Result<()> {
    match action {
        ContractCommands::Deploy {
            wasm,
            key,
            init_args,
        } => {
            let signer = resolve_signing_key(wallet, &key, config)?;
            cmd_deploy(client, &wasm, &signer, &init_args, format).await?;
        }

        ContractCommands::Call {
            addr,
            method,
            args,
            value,
            key,
        } => {
            let signer = resolve_signing_key(wallet, &key, config)?;
            cmd_call(client, &addr, &method, &args, value, &signer, format).await?;
        }

        ContractCommands::Info { addr } => {
            let info = client.get_contract_info(&addr).await?;
            print_value(&info, format);
        }
    }

    Ok(())
}

// ===========================================================================
// Channel commands
// ===========================================================================

async fn cmd_channel(
    action: ChannelCommands,
    client: &RpcClient,
    wallet: &Option<String>,
    config: &CliConfig,
    format: &OutputFormat,
) -> Result<()> {
    match action {
        ChannelCommands::Open { peer, amount, key } => {
            let micro = parse_usdc_amount(&amount)?;
            let signer = resolve_signing_key(wallet, &key, config)?;
            let from = Address::from_pubkey(&signer.verifying_key());
            let peer_addr = Address::from_str(&peer)
                .map_err(|e| anyhow::anyhow!("invalid peer address: {e}"))?;

            // Use the contract-call mechanism to open a channel.
            let args = serde_json::to_vec(&serde_json::json!({
                "peer": peer_addr.to_string(),
                "deposit": micro,
            }))
            .context("failed to serialize channel open args")?;

            let mut tx = dina_core::Transaction::CallContract {
                from,
                contract: Address::ZERO, // Channel system contract (address 0).
                method: "open_channel".to_string(),
                args,
                usdc_attached: micro,
                nonce: 0,
                fee: 500,
                signature: Sig64([0u8; 64]),
            };

            let msg = tx.signing_bytes();
            let sig = dina_core::crypto::sign(&signer, &msg);

            if let dina_core::Transaction::CallContract {
                ref mut signature, ..
            } = tx
            {
                *signature = Sig64(sig);
            }

            let tx_bytes =
                serde_json::to_vec(&tx).context("failed to serialize transaction")?;
            let tx_hex = format!("0x{}", hex::encode(&tx_bytes));
            let tx_hash = client.send_transaction(&tx_hex).await?;

            if *format == OutputFormat::Json {
                let val = serde_json::json!({
                    "action": "open_channel",
                    "from": from.to_string(),
                    "peer": peer_addr.to_string(),
                    "deposit_usdc": format_usdc(micro),
                    "tx_hash": tx_hash,
                });
                println!("{}", serde_json::to_string(&val)?);
            } else {
                println!("{}", "Payment channel opened.".green());
                println!("  From:    {}", from.to_string().cyan());
                println!("  Peer:    {}", peer_addr.to_string().cyan());
                println!("  Deposit: {}", format_usdc(micro).yellow());
                println!("  Tx Hash: {}", tx_hash.cyan());
            }
        }

        ChannelCommands::Pay {
            channel_id,
            amount,
            key,
        } => {
            let micro = parse_usdc_amount(&amount)?;
            let signer = resolve_signing_key(wallet, &key, config)?;
            let from = Address::from_pubkey(&signer.verifying_key());

            let args = serde_json::to_vec(&serde_json::json!({
                "channel_id": channel_id,
                "amount": micro,
            }))
            .context("failed to serialize channel pay args")?;

            let mut tx = dina_core::Transaction::CallContract {
                from,
                contract: Address::ZERO,
                method: "channel_pay".to_string(),
                args,
                usdc_attached: 0,
                nonce: 0,
                fee: 100,
                signature: Sig64([0u8; 64]),
            };

            let msg = tx.signing_bytes();
            let sig = dina_core::crypto::sign(&signer, &msg);

            if let dina_core::Transaction::CallContract {
                ref mut signature, ..
            } = tx
            {
                *signature = Sig64(sig);
            }

            let tx_bytes =
                serde_json::to_vec(&tx).context("failed to serialize transaction")?;
            let tx_hex = format!("0x{}", hex::encode(&tx_bytes));
            let tx_hash = client.send_transaction(&tx_hex).await?;

            if *format == OutputFormat::Json {
                let val = serde_json::json!({
                    "action": "channel_pay",
                    "channel_id": channel_id,
                    "amount_usdc": format_usdc(micro),
                    "tx_hash": tx_hash,
                });
                println!("{}", serde_json::to_string(&val)?);
            } else {
                println!("{}", "Channel payment sent.".green());
                println!("  Channel: {}", channel_id.cyan());
                println!("  Amount:  {}", format_usdc(micro).yellow());
                println!("  Tx Hash: {}", tx_hash.cyan());
            }
        }

        ChannelCommands::Close { channel_id, key } => {
            let signer = resolve_signing_key(wallet, &key, config)?;
            let from = Address::from_pubkey(&signer.verifying_key());

            let args = serde_json::to_vec(&serde_json::json!({
                "channel_id": channel_id,
            }))
            .context("failed to serialize channel close args")?;

            let mut tx = dina_core::Transaction::CallContract {
                from,
                contract: Address::ZERO,
                method: "close_channel".to_string(),
                args,
                usdc_attached: 0,
                nonce: 0,
                fee: 200,
                signature: Sig64([0u8; 64]),
            };

            let msg = tx.signing_bytes();
            let sig = dina_core::crypto::sign(&signer, &msg);

            if let dina_core::Transaction::CallContract {
                ref mut signature, ..
            } = tx
            {
                *signature = Sig64(sig);
            }

            let tx_bytes =
                serde_json::to_vec(&tx).context("failed to serialize transaction")?;
            let tx_hex = format!("0x{}", hex::encode(&tx_bytes));
            let tx_hash = client.send_transaction(&tx_hex).await?;

            if *format == OutputFormat::Json {
                let val = serde_json::json!({
                    "action": "close_channel",
                    "channel_id": channel_id,
                    "tx_hash": tx_hash,
                });
                println!("{}", serde_json::to_string(&val)?);
            } else {
                println!("{}", "Channel close submitted.".green());
                println!("  Channel: {}", channel_id.cyan());
                println!("  Tx Hash: {}", tx_hash.cyan());
            }
        }

        ChannelCommands::List => {
            let info = client.list_channels().await?;
            print_value(&info, format);
        }
    }

    Ok(())
}

// ===========================================================================
// Faucet command
// ===========================================================================

async fn cmd_faucet(
    client: &RpcClient,
    address: &str,
    _config: &CliConfig,
    format: &OutputFormat,
) -> Result<()> {
    let addr =
        Address::from_str(address).map_err(|e| anyhow::anyhow!("invalid address: {e}"))?;

    let result = client.request_faucet(&addr.to_string()).await?;

    let amount = result
        .get("amount")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let tx_hash = result
        .get("tx_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    if *format == OutputFormat::Json {
        println!("{}", serde_json::to_string(&result)?);
    } else {
        println!("{}", "Faucet request submitted.".green());
        println!("  Address: {}", addr.to_string().cyan());
        println!("  Amount:  {}", format_usdc(amount).yellow());
        println!("  Tx Hash: {}", tx_hash.cyan());
    }

    Ok(())
}

// ===========================================================================
// Validator commands
// ===========================================================================

async fn cmd_validators(
    action: ValidatorCommands,
    client: &RpcClient,
    format: &OutputFormat,
) -> Result<()> {
    match action {
        ValidatorCommands::List => {
            let info = client.list_validators().await?;

            if *format == OutputFormat::Json {
                println!("{}", serde_json::to_string(&info)?);
                return Ok(());
            }

            // Try to display as a table.
            if let Some(validators) = info.as_array() {
                println!("{}", "Validators".bold());
                println!(
                    "  {:<44}  {:>12}  {:>8}",
                    "Address".bold(),
                    "Stake".bold(),
                    "Status".bold()
                );
                println!("  {}", "-".repeat(68));
                for v in validators {
                    let addr = v
                        .get("address")
                        .and_then(|a| a.as_str())
                        .unwrap_or("?");
                    let stake = v
                        .get("stake")
                        .and_then(|s| s.as_u64())
                        .unwrap_or(0);
                    let status = v
                        .get("status")
                        .and_then(|s| s.as_str())
                        .unwrap_or("unknown");

                    let status_colored = match status {
                        "active" => status.green().to_string(),
                        "jailed" => status.red().to_string(),
                        _ => status.yellow().to_string(),
                    };

                    println!(
                        "  {:<44}  {:>12}  {:>8}",
                        addr,
                        format_usdc(stake),
                        status_colored
                    );
                }
            } else {
                print_value(&info, format);
            }
        }

        ValidatorCommands::Info { addr } => {
            let info = client.get_validator(&addr).await?;
            print_value(&info, format);
        }
    }

    Ok(())
}

// ===========================================================================
// Explorer commands
// ===========================================================================

async fn cmd_explorer(
    action: ExplorerCommands,
    client: &RpcClient,
    format: &OutputFormat,
) -> Result<()> {
    match action {
        ExplorerCommands::Blocks { limit } => {
            let info = client.get_recent_blocks(limit).await?;

            if *format == OutputFormat::Json {
                println!("{}", serde_json::to_string(&info)?);
                return Ok(());
            }

            if let Some(blocks) = info.as_array() {
                println!("{}", "Recent Blocks".bold());
                println!(
                    "  {:>8}  {:<18}  {:>6}  {:<66}",
                    "Height".bold(),
                    "Time".bold(),
                    "Txs".bold(),
                    "Hash".bold()
                );
                println!("  {}", "-".repeat(102));
                for b in blocks {
                    let height = b
                        .get("height")
                        .and_then(|h| h.as_u64())
                        .unwrap_or(0);
                    let time = b
                        .get("timestamp")
                        .and_then(|t| t.as_str())
                        .unwrap_or("?");
                    let txs = b
                        .get("tx_count")
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0);
                    let hash = b
                        .get("hash")
                        .and_then(|h| h.as_str())
                        .unwrap_or("?");

                    println!(
                        "  {:>8}  {:<18}  {:>6}  {}",
                        height.to_string().cyan(),
                        time,
                        txs,
                        hash.dimmed()
                    );
                }
            } else {
                print_value(&info, format);
            }
        }

        ExplorerCommands::Tx { hash } => {
            let info = client.get_transaction(&hash).await?;
            print_value(&info, format);
        }

        ExplorerCommands::Search { query } => {
            let info = client.explorer_search(&query).await?;
            print_value(&info, format);
        }
    }

    Ok(())
}
