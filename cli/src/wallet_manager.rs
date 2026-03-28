use std::path::PathBuf;

use anyhow::{Context, Result};
use argon2::Argon2;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use dina_core::types::Address;

/// On-disk representation of an encrypted wallet.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalletFile {
    pub name: String,
    pub address: String,
    pub pubkey_hex: String,
    /// Hex-encoded Argon2id salt (16 bytes).
    pub salt_hex: String,
    /// Hex-encoded XChaCha20-Poly1305 nonce (24 bytes).
    pub nonce_hex: String,
    /// Hex-encoded ciphertext (private key encrypted with AEAD).
    pub encrypted_key_hex: String,
    pub created_at: String,
}

/// Manages local wallets stored under `~/.dina/wallets/`.
pub struct WalletManager {
    wallet_dir: PathBuf,
}

impl WalletManager {
    #[allow(dead_code)]
    pub fn new(wallet_dir: PathBuf) -> Self {
        Self { wallet_dir }
    }

    /// Create from the default `~/.dina/wallets/` path.
    pub fn default_path() -> Result<Self> {
        let home = dirs::home_dir().context("cannot determine home directory")?;
        let wallet_dir = home.join(".dina").join("wallets");
        std::fs::create_dir_all(&wallet_dir)
            .with_context(|| format!("failed to create wallet dir {:?}", wallet_dir))?;
        Ok(Self { wallet_dir })
    }

    /// Return the path to the wallet directory.
    pub fn wallet_dir(&self) -> &PathBuf {
        &self.wallet_dir
    }

    /// Derive a 32-byte key from password using Argon2id with a random salt.
    fn derive_password_key(password: &str, salt: &[u8; 16]) -> Result<[u8; 32]> {
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(19456, 2, 1, Some(32))
                .map_err(|e| anyhow::anyhow!("failed to create Argon2 params: {}", e))?,
        );
        let mut key = [0u8; 32];
        argon2
            .hash_password_into(password.as_bytes(), salt, &mut key)
            .map_err(|e| anyhow::anyhow!("Argon2 key derivation failed: {}", e))?;
        Ok(key)
    }

    /// Encrypt data using XChaCha20-Poly1305 with AEAD.
    fn encrypt(data: &[u8], key: &[u8; 32]) -> Result<([u8; 24], Vec<u8>)> {
        let cipher = XChaCha20Poly1305::new(key.into());
        let mut nonce_bytes = [0u8; 24];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| anyhow::anyhow!("encryption failed: {}", e))?;
        Ok((nonce_bytes, ciphertext))
    }

    /// Decrypt data using XChaCha20-Poly1305 with AEAD.
    fn decrypt(ciphertext: &[u8], key: &[u8; 32], nonce: &[u8; 24]) -> Result<Vec<u8>> {
        let cipher = XChaCha20Poly1305::new(key.into());
        let nonce = XNonce::from_slice(nonce);
        cipher.decrypt(nonce, ciphertext).map_err(|_| {
            anyhow::anyhow!("decryption failed — incorrect password or corrupted data")
        })
    }

    fn wallet_path(&self, name: &str) -> PathBuf {
        self.wallet_dir.join(format!("{name}.json"))
    }

    fn default_file_path(&self) -> PathBuf {
        self.wallet_dir.join("_default")
    }

    /// Create a new wallet with the given name and password.
    pub fn create_wallet(&self, name: &str, password: &str) -> Result<WalletFile> {
        let path = self.wallet_path(name);
        if path.exists() {
            anyhow::bail!("wallet '{name}' already exists");
        }

        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let address = Address::from_pubkey(&verifying_key);

        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let pw_key = Self::derive_password_key(password, &salt)?;
        let (nonce, ciphertext) = Self::encrypt(signing_key.as_bytes(), &pw_key)?;

        let now = chrono::Utc::now().to_rfc3339();

        let wallet = WalletFile {
            name: name.to_string(),
            address: address.to_string(),
            pubkey_hex: hex::encode(verifying_key.as_bytes()),
            salt_hex: hex::encode(salt),
            nonce_hex: hex::encode(nonce),
            encrypted_key_hex: hex::encode(ciphertext),
            created_at: now,
        };

        let json = serde_json::to_string_pretty(&wallet).context("failed to serialize wallet")?;
        std::fs::write(&path, json)
            .with_context(|| format!("failed to write wallet file {:?}", path))?;

        // If this is the first wallet, set it as default.
        if self.default_wallet()?.is_none() {
            self.set_default(name)?;
        }

        Ok(wallet)
    }

    /// List all wallets in the wallet directory.
    pub fn list_wallets(&self) -> Result<Vec<WalletFile>> {
        let mut wallets = Vec::new();

        if !self.wallet_dir.exists() {
            return Ok(wallets);
        }

        for entry in
            std::fs::read_dir(&self.wallet_dir).context("failed to read wallet directory")?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let data = std::fs::read_to_string(&path)
                    .with_context(|| format!("failed to read {:?}", path))?;
                if let Ok(wallet) = serde_json::from_str::<WalletFile>(&data) {
                    wallets.push(wallet);
                }
            }
        }

        wallets.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(wallets)
    }

    /// Load a wallet's private/public key pair by decrypting with the password.
    /// Returns (private_key_bytes, public_key_bytes).
    pub fn load_wallet(&self, name: &str, password: &str) -> Result<([u8; 32], [u8; 32])> {
        let path = self.wallet_path(name);
        let data =
            std::fs::read_to_string(&path).with_context(|| format!("wallet '{name}' not found"))?;
        let wallet: WalletFile =
            serde_json::from_str(&data).context("failed to parse wallet file")?;

        let salt_bytes = hex::decode(&wallet.salt_hex).context("invalid salt hex")?;
        if salt_bytes.len() != 16 {
            anyhow::bail!("corrupted wallet file: wrong salt length");
        }
        let mut salt = [0u8; 16];
        salt.copy_from_slice(&salt_bytes);

        let nonce_bytes = hex::decode(&wallet.nonce_hex).context("invalid nonce hex")?;
        if nonce_bytes.len() != 24 {
            anyhow::bail!("corrupted wallet file: wrong nonce length");
        }
        let mut nonce = [0u8; 24];
        nonce.copy_from_slice(&nonce_bytes);

        let ciphertext =
            hex::decode(&wallet.encrypted_key_hex).context("invalid encrypted key hex")?;

        let pw_key = Self::derive_password_key(password, &salt)?;
        let decrypted = Self::decrypt(&ciphertext, &pw_key, &nonce)
            .context("incorrect password or corrupted wallet file")?;

        if decrypted.len() != 32 {
            anyhow::bail!("corrupted wallet file: wrong key length");
        }

        let mut privkey = [0u8; 32];
        privkey.copy_from_slice(&decrypted);

        // Verify the password is correct by re-deriving the public key.
        let signing_key = SigningKey::from_bytes(&privkey);
        let verifying_key = signing_key.verifying_key();
        let derived_hex = hex::encode(verifying_key.as_bytes());

        if derived_hex != wallet.pubkey_hex {
            anyhow::bail!("incorrect password for wallet '{name}'");
        }

        let mut pubkey = [0u8; 32];
        pubkey.copy_from_slice(verifying_key.as_bytes());

        Ok((privkey, pubkey))
    }

    /// Delete a wallet by name.
    pub fn delete_wallet(&self, name: &str) -> Result<()> {
        let path = self.wallet_path(name);
        if !path.exists() {
            anyhow::bail!("wallet '{name}' not found");
        }
        std::fs::remove_file(&path)
            .with_context(|| format!("failed to delete wallet {:?}", path))?;

        // If this was the default, remove the default marker.
        if self.default_wallet()?.as_deref() == Some(name) {
            let _ = std::fs::remove_file(self.default_file_path());
        }

        Ok(())
    }

    /// Export the private key as a hex string.
    pub fn export_wallet(&self, name: &str, password: &str) -> Result<String> {
        let (privkey, _) = self.load_wallet(name, password)?;
        Ok(hex::encode(privkey))
    }

    /// Import a wallet from a hex-encoded private key.
    pub fn import_wallet(
        &self,
        name: &str,
        password: &str,
        private_key_hex: &str,
    ) -> Result<WalletFile> {
        let path = self.wallet_path(name);
        if path.exists() {
            anyhow::bail!("wallet '{name}' already exists");
        }

        let raw = private_key_hex
            .strip_prefix("0x")
            .unwrap_or(private_key_hex);
        let key_bytes = hex::decode(raw).context("invalid hex in private key")?;
        if key_bytes.len() != 32 {
            anyhow::bail!("private key must be 32 bytes, got {}", key_bytes.len());
        }

        let mut privkey = [0u8; 32];
        privkey.copy_from_slice(&key_bytes);

        let signing_key = SigningKey::from_bytes(&privkey);
        let verifying_key = signing_key.verifying_key();
        let address = Address::from_pubkey(&verifying_key);

        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let pw_key = Self::derive_password_key(password, &salt)?;
        let (nonce, ciphertext) = Self::encrypt(&privkey, &pw_key)?;

        let now = chrono::Utc::now().to_rfc3339();

        let wallet = WalletFile {
            name: name.to_string(),
            address: address.to_string(),
            pubkey_hex: hex::encode(verifying_key.as_bytes()),
            salt_hex: hex::encode(salt),
            nonce_hex: hex::encode(nonce),
            encrypted_key_hex: hex::encode(ciphertext),
            created_at: now,
        };

        let json = serde_json::to_string_pretty(&wallet).context("failed to serialize wallet")?;
        std::fs::write(&path, json)
            .with_context(|| format!("failed to write wallet file {:?}", path))?;

        if self.default_wallet()?.is_none() {
            self.set_default(name)?;
        }

        Ok(wallet)
    }

    /// Get the name of the default wallet, if one is set.
    pub fn default_wallet(&self) -> Result<Option<String>> {
        let path = self.default_file_path();
        if !path.exists() {
            return Ok(None);
        }
        let name = std::fs::read_to_string(&path)
            .context("failed to read default wallet file")?
            .trim()
            .to_string();

        // Verify the wallet still exists.
        if !self.wallet_path(&name).exists() {
            return Ok(None);
        }
        Ok(Some(name))
    }

    /// Set the default wallet by name.
    pub fn set_default(&self, name: &str) -> Result<()> {
        let path = self.wallet_path(name);
        if !path.exists() {
            anyhow::bail!("wallet '{name}' not found");
        }
        std::fs::write(self.default_file_path(), name)
            .context("failed to write default wallet file")?;
        Ok(())
    }

    /// Load a SigningKey from a named wallet (convenience method).
    pub fn signing_key(&self, name: &str, password: &str) -> Result<SigningKey> {
        let (privkey, _) = self.load_wallet(name, password)?;
        Ok(SigningKey::from_bytes(&privkey))
    }
}
