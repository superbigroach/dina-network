mod rpc_client;

use std::str::FromStr;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

use dina_core::transaction::Sig64;
use dina_core::types::Address;

use crate::rpc_client::RpcClient;

/// Dina CLI -- interact with the Dina Network.
#[derive(Parser, Debug)]
#[command(name = "dina", version, about = "Dina Network CLI tool")]
struct Cli {
    /// JSON-RPC endpoint URL.
    #[arg(long, default_value = "http://localhost:8545", global = true)]
    rpc_url: String,

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
        /// Amount in micro-USDC (6 decimals).
        amount: u64,
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
}

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

/// Load a signing key from a file (raw 32-byte Ed25519 secret).
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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = RpcClient::new(&cli.rpc_url);

    match cli.command {
        Commands::Keygen { output } => cmd_keygen(&output)?,
        Commands::Balance { address } => cmd_balance(&client, &address).await?,
        Commands::Transfer {
            to,
            amount,
            key,
            memo,
        } => cmd_transfer(&client, &to, amount, &key, memo).await?,
        Commands::Deploy {
            wasm_file,
            key,
            init_args,
        } => cmd_deploy(&client, &wasm_file, &key, &init_args).await?,
        Commands::Call {
            contract,
            method,
            args,
            value,
            key,
        } => cmd_call(&client, &contract, &method, &args, value, &key).await?,
        Commands::Device { action } => match action {
            DeviceCommands::Register { key } => cmd_device_register(&client, &key).await?,
            DeviceCommands::Info { pubkey } => cmd_device_info(&client, &pubkey).await?,
        },
        Commands::Block { height } => cmd_block(&client, &height).await?,
        Commands::Tx { hash } => cmd_tx(&client, &hash).await?,
        Commands::Status => cmd_status(&client).await?,
    }

    Ok(())
}

/// Generate a new Ed25519 keypair, save the private key, and print the address.
fn cmd_keygen(output: &str) -> Result<()> {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    let address = Address::from_pubkey(&verifying_key);

    // Save the 32-byte private key
    std::fs::write(output, signing_key.as_bytes())
        .with_context(|| format!("failed to write key to '{output}'"))?;

    // Also save the public key alongside
    let pubkey_path = format!("{output}.pub");
    std::fs::write(&pubkey_path, verifying_key.as_bytes())
        .with_context(|| format!("failed to write public key to '{pubkey_path}'"))?;

    println!("Key pair generated successfully.");
    println!("  Private key: {output}");
    println!("  Public key:  {pubkey_path}");
    println!("  Address:     {address}");
    println!(
        "  Public key:  0x{}",
        hex::encode(verifying_key.as_bytes())
    );

    Ok(())
}

/// Query the balance of an address via JSON-RPC.
async fn cmd_balance(client: &RpcClient, address: &str) -> Result<()> {
    let addr =
        Address::from_str(address).map_err(|e| anyhow::anyhow!("invalid address: {e}"))?;

    let balance = client.get_balance(&addr.to_string()).await?;

    let usdc_whole = balance / 1_000_000;
    let usdc_frac = balance % 1_000_000;

    println!("Address: {addr}");
    println!("Balance: {usdc_whole}.{usdc_frac:06} USDC ({balance} micro-USDC)");

    Ok(())
}

/// Send a USDC transfer transaction.
async fn cmd_transfer(
    client: &RpcClient,
    to: &str,
    amount: u64,
    key_path: &str,
    memo: Option<String>,
) -> Result<()> {
    let signing_key = load_signing_key(key_path)?;
    let from = Address::from_pubkey(&signing_key.verifying_key());
    let to_addr =
        Address::from_str(to).map_err(|e| anyhow::anyhow!("invalid recipient address: {e}"))?;

    let memo_bytes = memo.map(|m| m.into_bytes());

    // Build the transaction
    let mut tx = dina_core::Transaction::Transfer {
        from,
        to: to_addr,
        amount,
        memo: memo_bytes,
        device_witness: None,
        nonce: 0, // In production: query nonce from chain via dina_getAccount
        fee: 100, // Default fee: 100 micro-USDC
        signature: Sig64([0u8; 64]),
    };

    // Sign it
    let msg = tx.signing_bytes();
    let sig = dina_core::crypto::sign(&signing_key, &msg);

    if let dina_core::Transaction::Transfer {
        ref mut signature, ..
    } = tx
    {
        *signature = Sig64(sig);
    }

    // Serialize and submit
    let tx_bytes = serde_json::to_vec(&tx).context("failed to serialize transaction")?;
    let tx_hex = format!("0x{}", hex::encode(&tx_bytes));

    let tx_hash = client.send_transaction(&tx_hex).await?;

    let usdc_whole = amount / 1_000_000;
    let usdc_frac = amount % 1_000_000;

    println!("Transfer submitted successfully.");
    println!("  From:    {from}");
    println!("  To:      {to_addr}");
    println!("  Amount:  {usdc_whole}.{usdc_frac:06} USDC");
    println!("  Tx Hash: {tx_hash}");

    Ok(())
}

/// Deploy a smart contract.
async fn cmd_deploy(
    client: &RpcClient,
    wasm_file: &str,
    key_path: &str,
    init_args_hex: &str,
) -> Result<()> {
    let signing_key = load_signing_key(key_path)?;
    let from = Address::from_pubkey(&signing_key.verifying_key());

    let wasm_bytes = std::fs::read(wasm_file)
        .with_context(|| format!("failed to read WASM file '{wasm_file}'"))?;

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
        fee: 1000,
        signature: Sig64([0u8; 64]),
    };

    let msg = tx.signing_bytes();
    let sig = dina_core::crypto::sign(&signing_key, &msg);

    if let dina_core::Transaction::DeployContract {
        ref mut signature, ..
    } = tx
    {
        *signature = Sig64(sig);
    }

    let tx_bytes = serde_json::to_vec(&tx).context("failed to serialize transaction")?;
    let tx_hex = format!("0x{}", hex::encode(&tx_bytes));

    let tx_hash = client.send_transaction(&tx_hex).await?;

    println!("Contract deployment submitted.");
    println!("  Deployer: {from}");
    println!("  WASM:     {wasm_file}");
    println!("  Tx Hash:  {tx_hash}");

    Ok(())
}

/// Call a method on a deployed smart contract.
async fn cmd_call(
    client: &RpcClient,
    contract: &str,
    method: &str,
    args_hex: &str,
    value: u64,
    key_path: &str,
) -> Result<()> {
    let signing_key = load_signing_key(key_path)?;
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
    let sig = dina_core::crypto::sign(&signing_key, &msg);

    if let dina_core::Transaction::CallContract {
        ref mut signature, ..
    } = tx
    {
        *signature = Sig64(sig);
    }

    let tx_bytes = serde_json::to_vec(&tx).context("failed to serialize transaction")?;
    let tx_hex = format!("0x{}", hex::encode(&tx_bytes));

    let tx_hash = client.send_transaction(&tx_hex).await?;

    println!("Contract call submitted.");
    println!("  From:     {from}");
    println!("  Contract: {contract_addr}");
    println!("  Method:   {method}");
    println!("  Value:    {value} micro-USDC");
    println!("  Tx Hash:  {tx_hash}");

    Ok(())
}

/// Register a device on-chain.
async fn cmd_device_register(client: &RpcClient, key_path: &str) -> Result<()> {
    let signing_key = load_signing_key(key_path)?;
    let owner = Address::from_pubkey(&signing_key.verifying_key());
    let device_pubkey = signing_key.verifying_key().to_bytes();

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Build a minimal attestation
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
    let sig = dina_core::crypto::sign(&signing_key, &msg);

    if let dina_core::Transaction::RegisterDevice {
        ref mut signature, ..
    } = tx
    {
        *signature = Sig64(sig);
    }

    let tx_bytes = serde_json::to_vec(&tx).context("failed to serialize transaction")?;
    let tx_hex = format!("0x{}", hex::encode(&tx_bytes));

    let tx_hash = client.send_transaction(&tx_hex).await?;

    println!("Device registration submitted.");
    println!("  Owner:      {owner}");
    println!("  Device key: 0x{}", hex::encode(device_pubkey));
    println!("  Tx Hash:    {tx_hash}");

    Ok(())
}

/// Get device info by public key.
async fn cmd_device_info(client: &RpcClient, pubkey: &str) -> Result<()> {
    let info = client.get_device(pubkey).await?;
    println!("{}", serde_json::to_string_pretty(&info)?);
    Ok(())
}

/// Get block information by height or "latest".
async fn cmd_block(client: &RpcClient, height: &str) -> Result<()> {
    let info = if height == "latest" {
        client.get_latest_block().await?
    } else {
        let h: u64 = height
            .parse()
            .with_context(|| format!("invalid block height: '{height}'"))?;
        client.get_block(h).await?
    };

    println!("{}", serde_json::to_string_pretty(&info)?);
    Ok(())
}

/// Get transaction information by hash.
async fn cmd_tx(client: &RpcClient, hash: &str) -> Result<()> {
    let info = client.get_transaction(hash).await?;
    println!("{}", serde_json::to_string_pretty(&info)?);
    Ok(())
}

/// Show node status and network information.
async fn cmd_status(client: &RpcClient) -> Result<()> {
    let info = client.network_info().await?;

    println!("Dina Network Node Status");
    println!("========================");
    println!(
        "  Chain ID:         {}",
        info.get("chain_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
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
