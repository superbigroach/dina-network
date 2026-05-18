# Wallet Guide

## Overview

Dina supports two distinct wallet types designed for different trust models and runtime environments. **PasskeyWallet** is for human users authenticating with biometrics inside mobile or web apps. **DinaWallet** is for server-side processes — backends, treasury operations, and stablecoin mint/burn — where a hardware security module or secrets manager holds the private key.

Choosing the wrong type for your context is a security mistake. A server wallet key must never appear in client-side code; a passkey credential cannot be used in a headless environment because it requires a human interaction prompt.

---

## 1. Two Wallet Types in Dina

### PasskeyWallet — for human users

PasskeyWallet delegates all private key material to the device's Secure Enclave (Apple) or Trusted Execution Environment (Android / TPM). The application never sees or stores a private key.

| Property | Detail |
|----------|--------|
| Cryptographic backing | WebAuthn (FIDO2) — device Secure Enclave or platform authenticator |
| Authentication factor | Face ID, Touch ID, or device PIN |
| Signing | Done inside the Secure Enclave; private key never leaves hardware |
| Recovery | iCloud Keychain (iOS) or Google Password Manager (Android); on-chain guardian recovery via DRC-111 |
| On-chain contract | DRC-111 Smart Wallet — holds assets and enforces per-session spending limits |
| Use in client apps | Yes |
| Use in server/backend | No — requires a human interaction prompt; will hang or fail |

PasskeyWallet is the correct choice for any flow where a real user is present: retail payments, remittance confirmations, DeFi transactions.

### DinaWallet — for server processes

DinaWallet is an Ed25519 keypair. The private key is loaded at runtime from a secrets manager and is never embedded in source code or environment variables checked into version control.

| Property | Detail |
|----------|--------|
| Cryptographic backing | Ed25519 keypair |
| Key storage | GCP Secret Manager, AWS Secrets Manager, or HashiCorp Vault |
| Authentication factor | IAM role / service account (machine identity) |
| Recovery | Key rotation — old key revoked, new key provisioned through the same secrets manager |
| On-chain contract | Standard EOA; no account-abstraction contract required |
| Use in client apps | Never — exposing a server key client-side is a critical vulnerability |
| Use in server/backend | Yes — designed for headless, automated use |

DinaWallet is the correct choice for: El Tesoro HNLc mint/burn, payroll batch disbursements, liquidity provisioning, and any automated treasury operation.

---

## 2. Creating a User Wallet (PasskeyWallet)

Wallet creation is a three-step process: register a passkey credential, deploy a DRC-111 Smart Wallet contract on-chain, then persist the `credentialId` in your application database.

```typescript
import { PasskeyWallet, DinaClient } from 'dina-js';

// Step 1: Register a passkey credential (called once during user onboarding)
// This triggers the OS biometric prompt and creates a credential in the device's
// Secure Enclave. The private key never leaves the device.
const { credentialId, publicKey } = await PasskeyWallet.register(
  'yourapp.com',                              // rpId — your domain (must match origin)
  'user@example.com',                         // userName — shown in the OS prompt
  crypto.getRandomValues(new Uint8Array(32))  // userId — your internal user identifier
);

// Step 2: Deploy a DRC-111 Smart Wallet on-chain
// The smart wallet contract is bound to the passkey's public key.
// All transactions must carry a valid WebAuthn signature to be accepted.
const client = new DinaClient('https://dina-proxy-ca-jy6qm6s57a-nn.a.run.app');

const walletAddress = await client.deploySmartWallet({
  ownerPublicKey: publicKey,   // Ed25519 public key extracted from the WebAuthn credential
  standard: 'DRC-111',
});

// Step 3: Store credentialId in your application database
// credentialId identifies which passkey to use when signing — it is NOT the private key.
// Storing credentialId is safe. Never attempt to extract or store the private key;
// the Secure Enclave does not permit it.
await db.users.update({ userId }, { credentialId, walletAddress });
```

**Why three steps?** The passkey credential and the on-chain smart wallet are separate concerns. The credential lives on the device; the smart wallet lives on-chain. You link them by embedding the credential's public key in the smart wallet constructor. After deployment, every transaction must carry a WebAuthn assertion that proves the user holds the matching private key.

---

## 3. Signing a Transaction with PasskeyWallet

Every on-chain action requires the user to authenticate with their device. The `PasskeyWallet.sign` call triggers Face ID or the platform equivalent.

```typescript
import { PasskeyWallet, DinaClient } from 'dina-js';

const client = new DinaClient('https://dina-proxy-ca-jy6qm6s57a-nn.a.run.app');

// Build the transaction (does not require user interaction)
const tx = client.buildTransfer({
  from: userWalletAddress,
  to: recipientAddress,
  amount: 500_000n, // 500 HNLc — the token has 6 decimal places
});

// Sign with passkey — this triggers Face ID or the OS biometric prompt.
// The OS passes the transaction hash to the Secure Enclave, which signs it
// and returns an authenticator assertion. The private key never leaves hardware.
const { authenticatorData, clientDataJSON, signature, counter } =
  await PasskeyWallet.sign(credentialId, tx.hash());

// Submit the signed transaction to the network.
// The DRC-111 contract on-chain verifies the WebAuthn assertion and executes
// the transfer only if the signature is valid and the counter is strictly increasing.
await client.submitWithPasskey(tx, {
  authenticatorData,
  clientDataJSON,
  signature,
  counter,
});
```

**Counter enforcement:** The `counter` value increments inside the Secure Enclave on every signing operation and is embedded in `authenticatorData`. The DRC-111 contract stores the last seen counter and rejects any assertion whose counter is not strictly greater than the stored value. This prevents signature replay attacks even if an attacker captures a valid `(authenticatorData, signature)` pair.

---

## 4. Setting Up Guardian Recovery (DRC-111)

A passkey credential is tied to a physical device. If the user loses their device before iCloud or Google Password Manager syncs the credential, they need an alternative recovery path. DRC-111 provides **guardian recovery**: a set of trusted addresses that can collectively authorize a wallet ownership transfer after a mandatory cooldown.

### 4.1 Add a Guardian

Guardians are added by the current wallet owner. A typical setup assigns one guardian address to a family member's wallet, one to a business partner, or one to a custodial service that the user trusts.

```typescript
import { PasskeyWallet, DinaClient } from 'dina-js';

const client = new DinaClient('https://dina-proxy-ca-jy6qm6s57a-nn.a.run.app');

// Build the addGuardian call — this modifies the DRC-111 contract state.
const tx = client.buildContractCall({
  contract: userWalletAddress,
  method: 'add_guardian',
  args: { guardian: guardianAddress },
});

// User must approve this with their passkey.
const { authenticatorData, clientDataJSON, signature, counter } =
  await PasskeyWallet.sign(credentialId, tx.hash());

await client.submitWithPasskey(tx, { authenticatorData, clientDataJSON, signature, counter });
```

Repeat for each guardian. The DRC-111 contract enforces a minimum guardian threshold (default: majority of registered guardians must sign to initiate recovery).

### 4.2 Initiate Recovery

When the user has lost their device, a guardian initiates the recovery process by calling `recover` with the proposed new owner address and their signature:

```typescript
// Called by a guardian on behalf of the locked-out user.
// guardianWallet here is a DinaWallet or another PasskeyWallet belonging to the guardian.
const recoveryTx = client.buildContractCall({
  contract: lostUserWalletAddress,
  method: 'recover',
  args: {
    new_owner: newOwnerPublicKey,       // The user's new passkey public key
    guardian_signatures: [guardianSig], // Each guardian must sign the same new_owner value
  },
});

await client.submit(recoveryTx, guardianWallet);
```

### 4.3 The 24-Hour Cooldown

After the required number of guardian signatures has been collected, the contract enters a **24-hour timelock** before ownership transfers. The cooldown exists as an anti-theft delay: if an attacker compromises guardian keys and attempts an unauthorized recovery, the legitimate owner sees the pending recovery event on-chain and has 24 hours to cancel it by signing a `cancel_recovery` transaction with their still-valid passkey.

```typescript
// If the recovery was unauthorized, the real owner can cancel within the cooldown window.
const cancelTx = client.buildContractCall({
  contract: userWalletAddress,
  method: 'cancel_recovery',
  args: {},
});

const { authenticatorData, clientDataJSON, signature, counter } =
  await PasskeyWallet.sign(credentialId, cancelTx.hash());

await client.submitWithPasskey(cancelTx, { authenticatorData, clientDataJSON, signature, counter });
```

After 24 hours with no cancellation, any account can call `finalize_recovery` to complete the transfer. The new passkey is now the wallet owner; the guardian set remains unchanged.

---

## 5. Server Wallet Setup (DinaWallet)

Server wallets run in Node.js on Cloud Run, Lambda, or any headless environment. The key is loaded from a secrets manager at startup or per-request — never from a file on disk or an environment variable in a Dockerfile.

```typescript
// In your backend service (Node.js on Cloud Run, Lambda, etc.)
import { DinaWallet, DinaClient } from 'dina-js';
import { SecretManagerServiceClient } from '@google-cloud/secret-manager';

const secretClient = new SecretManagerServiceClient();

async function getTreasuryWallet(): Promise<DinaWallet> {
  // Load private key from GCP Secret Manager.
  // The Cloud Run service account must have roles/secretmanager.secretAccessor on this secret.
  // Never hardcode the key string; never log it; never pass it through an environment variable
  // that is visible in Cloud Run console or CI/CD logs.
  const [version] = await secretClient.accessSecretVersion({
    name: 'projects/your-project/secrets/dina-treasury-key/versions/latest',
  });

  const privateKeyHex = version.payload!.data!.toString();
  return DinaWallet.fromPrivateKey(privateKeyHex);
}

// Use the treasury wallet to mint HNLc for a verified user deposit
async function mintHNLc(userAddress: string, amountMicro: bigint): Promise<string> {
  const client = new DinaClient(process.env.DINA_RPC_URL!);
  const treasury = await getTreasuryWallet();

  // The treasury wallet must be the designated minter on the HNLc DRC-1 contract.
  const txHash = await client.transfer(treasury, {
    to: userAddress,
    amount: amountMicro,
  });

  return txHash;
}

// Process a payroll batch — multiple transfers in a single call
async function processPayroll(
  payments: Array<{ address: string; amountMicro: bigint }>
): Promise<string> {
  const client = new DinaClient(process.env.DINA_RPC_URL!);
  const treasury = await getTreasuryWallet();

  const txHash = await client.batchTransfer(treasury, payments);
  return txHash;
}
```

**AWS Secrets Manager alternative:**

```typescript
import { SecretsManagerClient, GetSecretValueCommand } from '@aws-sdk/client-secrets-manager';

const awsClient = new SecretsManagerClient({ region: 'us-east-1' });

async function getTreasuryWallet(): Promise<DinaWallet> {
  const response = await awsClient.send(
    new GetSecretValueCommand({ SecretId: 'dina/treasury/private-key' })
  );
  return DinaWallet.fromPrivateKey(response.SecretString!);
}
```

**HashiCorp Vault alternative:**

```typescript
import Vault from 'node-vault';

const vault = Vault({ endpoint: process.env.VAULT_ADDR, token: process.env.VAULT_TOKEN });

async function getTreasuryWallet(): Promise<DinaWallet> {
  const { data } = await vault.read('secret/dina/treasury');
  return DinaWallet.fromPrivateKey(data.private_key);
}
```

---

## 6. Security Checklist

### PasskeyWallet (user wallets)

- **Never log credentialId and signature together.** The `credentialId` identifies which passkey to use; the `signature` proves possession at a specific counter value. Logging both together creates a partial replay record — even though the counter prevents a direct replay, it reveals usage patterns and aids targeted phishing attacks.
- **Verify counter strictly increases on your server before forwarding to the chain.** The DRC-111 contract enforces this on-chain, but validating on your backend too catches replay attempts before they consume network resources.
- **Store credentialId in your database; never store or transmit the private key.** You do not have the private key — the Secure Enclave holds it. If a step in your code is trying to "extract" a private key from a WebAuthn credential, something has gone wrong.
- **Validate the rpId on the server.** The WebAuthn assertion includes the origin; confirm it matches your production domain before submitting to chain. This prevents phishing sites from relaying your users' assertions.
- **Never prompt for passkey signing in the background or without a visible user action.** OS platforms block silent signing calls. More importantly, users must understand what they are authorizing — a silent prompt on a payment is a consent failure.

### DinaWallet (server wallets)

- **Rotate keys quarterly.** Schedule a rotation: provision new key in the secrets manager, update the authorized signer on any smart contracts that reference it, revoke the old key. Never reuse keys beyond their rotation period.
- **Use IAM-restricted secret access.** The Cloud Run service account (or Lambda execution role) should have `roles/secretmanager.secretAccessor` (GCP) or a narrow resource-based policy (AWS) that grants access only to the specific secret for the treasury key. No wildcard access.
- **Never put a server wallet private key in `.env` files checked into version control.** Even with `.gitignore`, secrets in env files leak through CI/CD logs, Docker build artifacts, and developer laptop backups.
- **Log wallet addresses, not private keys.** Audit logs should record `from: 0xABC...` (the on-chain address) and `txHash: 0xDEF...`. Nothing more.
- **Use separate keys for separate roles.** The mint key, the fee-payer key, and the treasury sweep key should be three different secrets with three different IAM policies. Compromising one should not compromise all operations.
- **Prefer short-lived tokens for Vault access.** If using HashiCorp Vault, authenticate with a machine identity that issues a short-lived token (TTL <= 1 hour) rather than a long-lived root token.
