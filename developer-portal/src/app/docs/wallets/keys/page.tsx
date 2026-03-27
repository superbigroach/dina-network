import Link from "next/link";

export const metadata = {
  title: "Key Management — Dina Network Developer Portal",
  description:
    "Ed25519 key pairs, encryption at rest, key rotation, and future MPC key splitting on Dina Network.",
};

function CodeBlock({
  language,
  title,
  children,
}: {
  language: string;
  title?: string;
  children: string;
}) {
  return (
    <div className="rounded-xl border border-slate-800 overflow-hidden">
      {title && (
        <div className="flex items-center justify-between border-b border-slate-800 bg-slate-900/80 px-4 py-2">
          <span className="text-xs font-medium text-slate-400">{title}</span>
          <span className="rounded bg-slate-800 px-2 py-0.5 text-[10px] font-mono text-slate-500 uppercase">
            {language}
          </span>
        </div>
      )}
      <pre className="bg-slate-900/50 p-4 overflow-x-auto text-sm leading-relaxed">
        <code className="text-slate-300">{children}</code>
      </pre>
    </div>
  );
}

export default function KeyManagementPage() {
  return (
    <div className="mx-auto max-w-4xl px-6 py-16">
      {/* Header */}
      <p className="text-sm font-medium uppercase tracking-wider text-blue-400 mb-3">
        Wallets
      </p>
      <h1 className="text-4xl font-bold tracking-tight mb-4">
        Key Management
      </h1>
      <p className="text-lg text-slate-400 max-w-3xl leading-relaxed mb-12">
        Everything you need to know about Ed25519 key pairs, encrypting keys at
        rest, rotating keys, and the upcoming MPC key splitting feature for
        mainnet.
      </p>

      {/* Ed25519 Key Pairs */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Ed25519 Key Pairs
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Dina Network uses{" "}
          <strong className="text-slate-200">Ed25519</strong> (Edwards-curve
          Digital Signature Algorithm) for all wallet cryptography. Ed25519 is
          the same signing algorithm used by Solana, Cosmos, SSH, and
          Signal. It provides:
        </p>
        <div className="grid gap-4 md:grid-cols-2 mb-6">
          {[
            {
              title: "Fast signing and verification",
              desc: "~76,000 signatures per second on a single core. Verification is even faster.",
            },
            {
              title: "Small key and signature sizes",
              desc: "32-byte private keys, 32-byte public keys, 64-byte signatures. Minimizes on-chain storage costs.",
            },
            {
              title: "Deterministic signatures",
              desc: "No random nonce generation during signing, eliminating an entire class of RNG-related vulnerabilities.",
            },
            {
              title: "Strong security guarantees",
              desc: "128-bit security level. Resistant to timing attacks by design. No known practical attacks.",
            },
          ].map((item) => (
            <div
              key={item.title}
              className="rounded-xl border border-slate-800 bg-slate-900/50 p-5"
            >
              <h3 className="text-sm font-semibold text-slate-200 mb-1">
                {item.title}
              </h3>
              <p className="text-sm text-slate-400 leading-relaxed">
                {item.desc}
              </p>
            </div>
          ))}
        </div>

        <CodeBlock language="typescript" title="Key pair anatomy">
          {`import { DinaWallet } from "@dina-network/sdk";

const wallet = DinaWallet.generate();

// Private key: 32 bytes (64 hex chars)
// This is the secret — never share it.
console.log("Private key:", wallet.privateKey);
// "3a5b9c7d1e2f4a6b8c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b"

// Public key: 32 bytes (64 hex chars)
// This is safe to share. Used to verify signatures.
console.log("Public key:", wallet.publicKey);
// "ed25519:1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b"

// Address: 0x + SHA-256(publicKey) = 66 chars
// Derived deterministically from the public key.
console.log("Address:", wallet.address);
// "0x7a3b1f4e8c9d2a5b6f0e1d3c4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b"`}
        </CodeBlock>
      </div>

      {/* Key Storage Best Practices */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Key Storage Best Practices
        </h2>
        <div className="space-y-4">
          {[
            {
              title: "Never store plain-text private keys on disk",
              desc: "Always encrypt private keys before writing them to any storage medium. Even in development, practice encryption to avoid accidentally shipping insecure code.",
              severity: "critical",
            },
            {
              title: "Use environment variables for ephemeral access",
              desc: "For server-side applications, load private keys from environment variables or a secrets manager. Never hardcode keys in source code.",
              severity: "critical",
            },
            {
              title: "Separate hot and cold wallets",
              desc: "Keep the majority of funds in a cold wallet (offline, air-gapped). Use hot wallets only for active operations with limited balances.",
              severity: "recommended",
            },
            {
              title: "Use hardware security modules (HSMs) for high-value wallets",
              desc: "For production systems holding significant value, use AWS CloudHSM, Google Cloud HSM, or a YubiKey to store the signing key in tamper-resistant hardware.",
              severity: "recommended",
            },
            {
              title: "Audit key access logs",
              desc: "Track every time a private key is decrypted or used to sign a transaction. Anomalous access patterns may indicate compromise.",
              severity: "recommended",
            },
          ].map((item) => (
            <div
              key={item.title}
              className={`rounded-xl border p-5 ${
                item.severity === "critical"
                  ? "border-red-500/30 bg-red-500/5"
                  : "border-slate-800 bg-slate-900/50"
              }`}
            >
              <div className="flex items-center gap-2 mb-1">
                <h3 className="text-sm font-semibold text-slate-200">
                  {item.title}
                </h3>
                {item.severity === "critical" && (
                  <span className="rounded-full bg-red-500/20 px-2 py-0.5 text-[10px] font-medium text-red-400 uppercase">
                    Critical
                  </span>
                )}
              </div>
              <p className="text-sm text-slate-400 leading-relaxed">
                {item.desc}
              </p>
            </div>
          ))}
        </div>
      </div>

      {/* Encryption at Rest */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Encryption at Rest
        </h2>

        {/* Dev / simple */}
        <div className="mb-8">
          <h3 className="text-lg font-semibold text-amber-400 mb-3">
            Development: XOR Cipher
          </h3>
          <p className="text-sm text-slate-400 leading-relaxed mb-4">
            For local development and testing, the Dina CLI uses a simple XOR
            cipher with a user-provided password. This is{" "}
            <strong className="text-amber-400">not suitable for production</strong>{" "}
            but is fast and easy to implement.
          </p>
          <CodeBlock language="typescript" title="XOR encryption (development only)">
            {`import { xorEncrypt, xorDecrypt } from "@dina-network/sdk/crypto";

const privateKey = wallet.privateKey;
const password = "my-dev-password";

// Encrypt
const encrypted = xorEncrypt(privateKey, password);
// Store 'encrypted' to disk — it is safe to commit to a dev config.

// Decrypt
const decrypted = xorDecrypt(encrypted, password);
console.log(decrypted === privateKey); // true`}
          </CodeBlock>
          <div className="mt-3 rounded-lg border border-amber-500/30 bg-amber-500/5 px-4 py-3">
            <p className="text-xs text-amber-300">
              XOR cipher is trivially breakable with known-plaintext attacks. Use
              it only for local development where convenience outweighs security.
            </p>
          </div>
        </div>

        {/* Production */}
        <div>
          <h3 className="text-lg font-semibold text-green-400 mb-3">
            Production: AES-256-GCM
          </h3>
          <p className="text-sm text-slate-400 leading-relaxed mb-4">
            For production deployments, encrypt private keys with{" "}
            <strong className="text-slate-200">AES-256-GCM</strong>. This
            provides authenticated encryption: both confidentiality and integrity
            verification.
          </p>
          <CodeBlock language="typescript" title="AES-256-GCM encryption (production)">
            {`import {
  encryptPrivateKey,
  decryptPrivateKey,
} from "@dina-network/sdk/crypto";

const privateKey = wallet.privateKey;

// Derive a 256-bit key from a password using Argon2id
// (or use a raw 32-byte key from a secrets manager)
const password = process.env.WALLET_ENCRYPTION_PASSWORD!;

// Encrypt — returns { ciphertext, iv, salt, tag }
const encrypted = await encryptPrivateKey(privateKey, password);

// Store the encrypted object to disk or database
await fs.writeFile("wallet.enc.json", JSON.stringify(encrypted));

// Decrypt
const raw = await fs.readFile("wallet.enc.json", "utf-8");
const decrypted = await decryptPrivateKey(JSON.parse(raw), password);

console.log(decrypted === privateKey); // true`}
          </CodeBlock>
          <CodeBlock language="json" title="Encrypted key file format">
            {`{
  "version": 1,
  "algorithm": "aes-256-gcm",
  "kdf": "argon2id",
  "kdfParams": {
    "salt": "base64...",
    "iterations": 3,
    "memory": 65536,
    "parallelism": 4
  },
  "iv": "base64...",
  "tag": "base64...",
  "ciphertext": "base64..."
}`}
          </CodeBlock>
        </div>
      </div>

      {/* Key Rotation */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Key Rotation
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Dina wallets are free to create. Rotating a key means creating a new
          wallet, transferring all assets from the old wallet to the new one,
          and decommissioning the old key. This is a best practice for
          long-running production systems.
        </p>
        <CodeBlock language="typescript" title="Key rotation workflow">
          {`import { DinaClient, DinaWallet } from "@dina-network/sdk";

const client = new DinaClient({ network: "mainnet" });

// Step 1: Generate a new wallet
const newWallet = DinaWallet.generate();

// Step 2: Transfer all assets from old wallet to new wallet
const oldWallet = DinaWallet.fromPrivateKey(process.env.OLD_KEY!);

const balances = await client.getBalances(oldWallet.address);
for (const token of balances) {
  await client.transfer({
    from: oldWallet,
    to: newWallet.address,
    amount: token.balance,
    token: token.symbol,
  });
}

// Step 3: Update your application config with the new key
console.log("New address:", newWallet.address);
console.log("New key:    ", newWallet.privateKey);
// Update environment variables, secrets manager, etc.

// Step 4: Securely destroy the old private key
// Overwrite the old key in memory and delete from storage.`}
        </CodeBlock>

        <div className="mt-4 space-y-2 text-sm text-slate-400">
          <p>
            <strong className="text-slate-200">How often to rotate:</strong>{" "}
            Rotate keys at least quarterly for production wallets, or
            immediately if you suspect any compromise.
          </p>
          <p>
            <strong className="text-slate-200">Agent and swarm wallets:</strong>{" "}
            Agent wallets do not need manual rotation. The owner can revoke and
            recreate them. Swarm wallets can be dissolved and recreated with new
            agent keys.
          </p>
        </div>
      </div>

      {/* MPC Key Splitting (Future) */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Future: MPC Key Splitting
        </h2>
        <div className="rounded-xl border border-blue-500/30 bg-blue-500/5 p-6 mb-6">
          <div className="flex items-center gap-2 mb-2">
            <span className="rounded-full bg-blue-500/20 px-2.5 py-0.5 text-xs font-medium text-blue-400">
              Planned for Mainnet
            </span>
          </div>
          <p className="text-sm text-slate-300 leading-relaxed">
            Multi-Party Computation (MPC) key splitting will allow a private key
            to be split into N shares, where any M-of-N shares can reconstruct
            the key for signing. No single party ever holds the complete key.
          </p>
        </div>

        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6 mb-6">
          <pre className="text-sm text-slate-300 leading-relaxed overflow-x-auto font-mono">
            {`  Traditional Key:
    Private Key = [████████████████████████████████]
    Single point of failure — if leaked, everything is lost.

  MPC Key Splitting (2-of-3):
    Share A = [████████░░░░░░░░░░░░░░░░░░░░░░░░]  (User device)
    Share B = [░░░░░░░░████████░░░░░░░░░░░░░░░░]  (Dina Network)
    Share C = [░░░░░░░░░░░░░░░░████████░░░░░░░░]  (Recovery service)

    Any 2 shares can sign a transaction.
    No single share reveals the private key.
    Lose one share? Use the other two to recover.`}
          </pre>
        </div>

        <h3 className="text-lg font-semibold mb-3">
          How This Compares to Circle
        </h3>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Circle&apos;s Programmable Wallets use MPC to split ECDSA (secp256k1) keys
          into three shares distributed across Circle&apos;s infrastructure, the
          user&apos;s device, and a recovery service. The user never holds the
          full key, and Circle never holds enough shares to sign unilaterally.
        </p>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Dina Network will implement a similar model but for{" "}
          <strong className="text-slate-200">Ed25519 keys</strong> using
          Schnorr-based threshold signatures. The key differences:
        </p>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">
                  Feature
                </th>
                <th className="px-4 py-3 text-left font-medium text-blue-400">
                  Circle (Current)
                </th>
                <th className="px-4 py-3 text-left font-medium text-purple-400">
                  Dina MPC (Planned)
                </th>
              </tr>
            </thead>
            <tbody>
              {[
                {
                  feature: "Curve",
                  circle: "secp256k1 (ECDSA)",
                  dina: "Ed25519 (Schnorr)",
                },
                {
                  feature: "Threshold",
                  circle: "2-of-3",
                  dina: "Configurable M-of-N",
                },
                {
                  feature: "Share holders",
                  circle: "Circle, device, backup",
                  dina: "User-defined parties",
                },
                {
                  feature: "Custodial risk",
                  circle: "Semi-custodial (Circle holds 1 share)",
                  dina: "Non-custodial (user chooses all parties)",
                },
                {
                  feature: "Recovery",
                  circle: "PIN + security questions",
                  dina: "Social recovery + backup share",
                },
              ].map((row, i) => (
                <tr
                  key={row.feature}
                  className={
                    i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"
                  }
                >
                  <td className="px-4 py-3 font-medium text-slate-300">
                    {row.feature}
                  </td>
                  <td className="px-4 py-3 text-slate-400">{row.circle}</td>
                  <td className="px-4 py-3 text-slate-400">{row.dina}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <CodeBlock language="typescript" title="MPC key splitting (preview API, subject to change)">
          {`import { DinaWallet, MPCKeyManager } from "@dina-network/sdk";

// Split an existing key into 3 shares (2-of-3 threshold)
const wallet = DinaWallet.generate();
const mpc = new MPCKeyManager();

const shares = await mpc.splitKey(wallet.privateKey, {
  threshold: 2,   // Any 2 shares can sign
  totalShares: 3, // 3 shares total
});

// Distribute shares to different parties
// shares[0] → User's device (encrypted local storage)
// shares[1] → Dina Network (encrypted, non-custodial escrow)
// shares[2] → User's backup (paper, hardware token, trusted contact)

// Sign a transaction with any 2 shares
const signature = await mpc.thresholdSign(
  transactionBytes,
  [shares[0], shares[1]], // Any 2 of 3
);

// The signature is indistinguishable from a regular Ed25519 signature.
// The blockchain does not know MPC was used.`}
        </CodeBlock>
      </div>

      {/* Navigation */}
      <div className="flex items-center justify-between pt-8 border-t border-slate-800">
        <Link
          href="/docs/wallets/swarm"
          className="text-sm text-slate-400 hover:text-blue-400 transition-colors"
        >
          &larr; Swarm Wallets
        </Link>
        <Link
          href="/docs/wallets/hd"
          className="text-sm text-slate-400 hover:text-blue-400 transition-colors"
        >
          HD Wallets &rarr;
        </Link>
      </div>
    </div>
  );
}
