import Link from "next/link";

function CodeBlock({ children, title }: { children: string; title?: string }) {
  return (
    <div className="mt-4 overflow-hidden rounded-lg border border-slate-800">
      {title && (
        <div className="border-b border-slate-800 bg-slate-900 px-4 py-2 text-xs font-medium text-slate-400">
          {title}
        </div>
      )}
      <pre className="!m-0 !rounded-none !border-0 bg-slate-800 p-4">
        <code className="text-sm leading-relaxed text-slate-200">{children}</code>
      </pre>
    </div>
  );
}

export default function AuthenticationPage() {
  return (
    <div>
      <h1 className="text-4xl font-bold tracking-tight text-white">
        Authentication
      </h1>
      <p className="mt-4 text-lg text-slate-300">
        Dina Network uses Ed25519 key pairs for all authentication and
        transaction signing. For server-side integrations, you can generate
        API keys that are tied to your wallet identity.
      </p>

      {/* Ed25519 Key Pairs */}
      <h2 className="mt-12 text-2xl font-semibold text-white">
        Ed25519 key pairs
      </h2>
      <p className="mt-3 text-sm leading-relaxed text-slate-300">
        Every identity on Dina is an Ed25519 key pair. The 32-byte public key
        is hashed with BLAKE2b to produce a{" "}
        <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">
          dina1...
        </code>{" "}
        address. The 64-byte secret key (seed + public key) is used to sign
        transactions and API requests.
      </p>

      <CodeBlock title="generate-keypair.ts">
        {`import { DinaWallet } from "dina-js";

// Generate a new keypair
const wallet = DinaWallet.generate();

console.log("Address:    ", wallet.address);
console.log("Public key: ", wallet.publicKeyHex);  // 32 bytes hex
console.log("Secret key: ", wallet.secretKeyHex);  // 64 bytes hex

// Restore from existing secret key
const restored = DinaWallet.fromSecretKey(
  "a1b2c3d4e5f6...your_secret_key_hex"
);`}
      </CodeBlock>

      <div className="mt-6 rounded-xl border border-yellow-600/30 bg-yellow-600/5 p-5">
        <h3 className="text-sm font-semibold text-yellow-400">
          Security notice
        </h3>
        <p className="mt-1.5 text-sm text-slate-300">
          Never expose your secret key in client-side code, version control,
          or logs. Use environment variables or a secret manager for
          production deployments. Consider HD wallets (
          <Link
            href="/docs/wallets/hd"
            className="text-blue-400 hover:underline"
          >
            DRC-44
          </Link>
          ) for key derivation from a single mnemonic.
        </p>
      </div>

      {/* API Keys */}
      <h2 className="mt-14 text-2xl font-semibold text-white">
        API keys for server-side access
      </h2>
      <p className="mt-3 text-sm leading-relaxed text-slate-300">
        For backend services that need to query the chain without signing
        transactions, you can generate API keys. An API key is a bearer token
        tied to your wallet address. It grants read access to all public
        endpoints and write access only to submit pre-signed transactions.
      </p>

      <CodeBlock title="generate-api-key.ts">
        {`import { DinaClient, DinaWallet } from "dina-js";

const client = new DinaClient("https://testnet.dina.network");
const wallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

// Generate an API key signed by your wallet
const apiKey = await client.createApiKey({
  wallet: wallet,
  label: "production-backend",
  permissions: ["read", "submit_tx"],
  expiresIn: "90d", // optional, defaults to no expiry
});

console.log("API Key:", apiKey.token);
console.log("Key ID: ", apiKey.id);`}
      </CodeBlock>

      <p className="mt-4 text-sm text-slate-300">
        Use the API key as a Bearer token in the{" "}
        <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">
          Authorization
        </code>{" "}
        header:
      </p>

      <CodeBlock title="curl">
        {`curl https://testnet.dina.network/accounts/dina1qxy2kgdygj... \\
  -H "Authorization: Bearer dina_key_a1b2c3d4e5f6..."`}
      </CodeBlock>

      {/* Rate Limits */}
      <h2 className="mt-14 text-2xl font-semibold text-white">Rate limits</h2>
      <p className="mt-3 text-sm leading-relaxed text-slate-300">
        Rate limits are applied per API key (or per IP for unauthenticated
        requests). Exceeding the limit returns HTTP 429 with a{" "}
        <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">
          Retry-After
        </code>{" "}
        header.
      </p>

      <div className="mt-6 overflow-x-auto rounded-xl border border-slate-800">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-slate-800 bg-slate-900/60">
              <th className="px-4 py-3 text-left font-semibold text-slate-300">
                Tier
              </th>
              <th className="px-4 py-3 text-left font-semibold text-slate-300">
                Read requests
              </th>
              <th className="px-4 py-3 text-left font-semibold text-slate-300">
                Write requests
              </th>
              <th className="px-4 py-3 text-left font-semibold text-slate-300">
                WebSocket subscriptions
              </th>
            </tr>
          </thead>
          <tbody className="text-slate-300">
            <tr className="border-b border-slate-800/50">
              <td className="px-4 py-3 font-medium text-slate-200">
                Unauthenticated
              </td>
              <td className="px-4 py-3">100 / min</td>
              <td className="px-4 py-3">10 / min</td>
              <td className="px-4 py-3">2</td>
            </tr>
            <tr className="border-b border-slate-800/50">
              <td className="px-4 py-3 font-medium text-slate-200">
                API key (free)
              </td>
              <td className="px-4 py-3">1,000 / min</td>
              <td className="px-4 py-3">100 / min</td>
              <td className="px-4 py-3">10</td>
            </tr>
            <tr>
              <td className="px-4 py-3 font-medium text-slate-200">
                API key (pro)
              </td>
              <td className="px-4 py-3">10,000 / min</td>
              <td className="px-4 py-3">1,000 / min</td>
              <td className="px-4 py-3">100</td>
            </tr>
          </tbody>
        </table>
      </div>

      {/* Signing Transactions */}
      <h2 className="mt-14 text-2xl font-semibold text-white">
        Signing transactions
      </h2>
      <p className="mt-3 text-sm leading-relaxed text-slate-300">
        Every state-changing operation on Dina must be signed by the sender's
        Ed25519 secret key. The signature covers the canonical encoding of the
        transaction body, preventing tampering and replay attacks.
      </p>

      <CodeBlock title="sign-transaction.ts">
        {`import { DinaClient, DinaWallet, DinaTx } from "dina-js";

const client = new DinaClient("https://testnet.dina.network");
const wallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

// Build the transaction
const tx = DinaTx.transfer({
  from: wallet.address,
  to: "dina1recipient...",
  amount: 5_000000, // 5 USDC
  nonce: await client.getNonce(wallet.address),
  gasLimit: 21000,
  gasPrice: 100, // 0.000100 USDC
});

// Sign: produces a 64-byte Ed25519 signature
const signature = wallet.sign(tx.encode());
console.log("Signature:", signature.hex);

// Attach signature and broadcast
const signedTx = tx.withSignature(signature);
const receipt = await client.send(signedTx);
console.log("Confirmed:", receipt.hash);`}
      </CodeBlock>

      <h3 className="mt-8 text-lg font-semibold text-white">
        Signature verification
      </h3>
      <p className="mt-2 text-sm leading-relaxed text-slate-300">
        You can verify any transaction signature locally without contacting
        the network. This is useful for validating webhooks and off-chain
        messages.
      </p>

      <CodeBlock title="verify-signature.ts">
        {`import { DinaWallet, DinaTx } from "dina-js";

// Recreate the transaction from raw data
const tx = DinaTx.fromHex(rawTxHex);

// Verify the signature against the sender's public key
const isValid = DinaWallet.verify(
  tx.encode(),          // the signed payload
  tx.signature,         // 64-byte Ed25519 signature
  tx.senderPublicKey    // 32-byte public key
);

console.log("Valid signature:", isValid); // true or false`}
      </CodeBlock>

      {/* Replay protection */}
      <h2 className="mt-14 text-2xl font-semibold text-white">
        Replay protection
      </h2>
      <p className="mt-3 text-sm leading-relaxed text-slate-300">
        Each account has a monotonically increasing nonce. A transaction is
        only valid if its nonce matches the account's current nonce. After
        execution, the nonce increments by one. This prevents an attacker
        from re-submitting a previously signed transaction.
      </p>
      <p className="mt-3 text-sm leading-relaxed text-slate-300">
        Additionally, transactions include the{" "}
        <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">
          chainId
        </code>{" "}
        in the signed payload, so a transaction signed for testnet cannot be
        replayed on mainnet.
      </p>

      <CodeBlock title="check-nonce.ts">
        {`const account = await client.getAccount(wallet.address);
console.log("Current nonce:", account.nonce);

// Always fetch the latest nonce before signing
const tx = DinaTx.transfer({
  from: wallet.address,
  to: "dina1recipient...",
  amount: 1_000000,
  nonce: account.nonce, // must match exactly
  chainId: "dina-testnet-1",
});`}
      </CodeBlock>

      {/* Next steps */}
      <div className="mt-12 rounded-xl border border-slate-800 bg-slate-900/40 p-6">
        <h3 className="text-base font-semibold text-white">Next steps</h3>
        <ul className="mt-3 space-y-2 text-sm text-slate-300">
          <li>
            <Link
              href="/docs/wallets/keys"
              className="text-blue-400 hover:underline"
            >
              Key management
            </Link>{" "}
            -- best practices for storing and rotating keys in production.
          </li>
          <li>
            <Link
              href="/docs/api/jsonrpc"
              className="text-blue-400 hover:underline"
            >
              JSON-RPC API
            </Link>{" "}
            -- full reference for all RPC methods including authentication
            headers.
          </li>
          <li>
            <Link
              href="/docs/wallets/agent"
              className="text-blue-400 hover:underline"
            >
              Agent wallets (DRC-101)
            </Link>{" "}
            -- delegate signing to AI agents with scoped permissions.
          </li>
        </ul>
      </div>
    </div>
  );
}
