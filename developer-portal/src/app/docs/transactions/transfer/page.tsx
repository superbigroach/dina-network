import Link from "next/link";

export const metadata = {
  title: "Send USDC — Dina Network Developer Portal",
  description:
    "How to send USDC transfers on the Dina Network using JavaScript, Python, and CLI.",
};

const TX_FIELDS = [
  { field: "from", type: "string", description: "Sender address (0x + 64 hex chars)" },
  { field: "to", type: "string", description: "Recipient address" },
  { field: "amount", type: "u64", description: "USDC amount in micro-units (1 USDC = 1,000,000)" },
  { field: "fee", type: "u64", description: "Gas fee in micro-USDC (minimum 1,000 = 0.001 USDC)" },
  { field: "nonce", type: "u64", description: "Sequential counter for sender's account, prevents replay" },
  { field: "signature", type: "[u8; 64]", description: "Ed25519 signature over the serialized transaction" },
];

export default function TransferPage() {
  return (
    <div>
      {/* Header */}
      <p className="text-sm font-medium uppercase tracking-wider text-blue-400 mb-3">
        Transactions
      </p>
      <h1 className="text-4xl font-bold tracking-tight text-white mb-4">
        Send USDC
      </h1>
      <p className="text-lg text-slate-400 max-w-3xl leading-relaxed mb-10">
        The most fundamental operation on Dina Network: transferring USDC from
        one address to another. Transactions settle in a single block (100ms)
        with deterministic finality. No gas token needed -- fees are paid in
        USDC.
      </p>

      {/* Transaction Structure */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Transaction Structure
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-6">
          Every transfer on Dina Network consists of six fields. The
          transaction is serialized as JSON, signed with Ed25519, and
          submitted to any validator node.
        </p>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">Field</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Type</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Description</th>
              </tr>
            </thead>
            <tbody>
              {TX_FIELDS.map((row, i) => (
                <tr
                  key={row.field}
                  className={i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"}
                >
                  <td className="px-4 py-3 font-mono text-blue-300">{row.field}</td>
                  <td className="px-4 py-3 font-mono text-slate-400">{row.type}</td>
                  <td className="px-4 py-3 text-slate-400">{row.description}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {/* Signing Process */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Signing Process
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-6">
          Dina uses Ed25519 signatures -- the same algorithm used by Solana,
          Cosmos, and SSH. The signing flow is straightforward:
        </p>
        <div className="space-y-4 mb-6">
          {[
            {
              step: "1",
              title: "Construct the transaction object",
              desc: "Set from, to, amount, fee, and nonce. The nonce must be the next unused nonce for your account (query via the API).",
            },
            {
              step: "2",
              title: "Serialize to canonical JSON",
              desc: "Convert the transaction (without the signature field) to a deterministic JSON byte string. Fields must be alphabetically ordered.",
            },
            {
              step: "3",
              title: "Sign with Ed25519",
              desc: "Sign the serialized bytes with your private key to produce a 64-byte signature.",
            },
            {
              step: "4",
              title: "Submit the signed transaction",
              desc: "Send the complete transaction (including signature) to any validator's JSON-RPC or REST endpoint.",
            },
          ].map((s) => (
            <div key={s.step} className="flex gap-4 items-start">
              <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-blue-600/20 text-sm font-bold text-blue-400">
                {s.step}
              </div>
              <div>
                <h3 className="text-sm font-semibold text-white">{s.title}</h3>
                <p className="text-sm text-slate-400 mt-0.5">{s.desc}</p>
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* JavaScript Example */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          JavaScript / TypeScript
        </h2>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`import { DinaClient, DinaWallet, parseUSDC } from '@dina-network/sdk';

// Connect to testnet
const client = new DinaClient('https://rpc-testnet.dina.network');

// Load wallet from mnemonic or key file
const wallet = DinaWallet.fromMnemonic(
  'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about'
);

// Send 25 USDC
const tx = await client.transfer({
  from: wallet.address,
  to: '0x7a8b9c...recipient',
  amount: parseUSDC('25'),  // 25_000_000
});

// Sign and submit
const receipt = await wallet.signAndSend(tx);

console.log('Transaction hash:', receipt.hash);
console.log('Block number:', receipt.blockNumber);
console.log('Status:', receipt.status); // "confirmed"
console.log('Finality:', '100ms - 1 block');`}</code>
        </pre>
      </section>

      {/* Python Example */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">Python</h2>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`from dina import DinaClient, DinaWallet, parse_usdc

# Connect to testnet
client = DinaClient("https://rpc-testnet.dina.network")

# Load wallet
wallet = DinaWallet.from_key_file("./my-wallet.json")

# Send 25 USDC
receipt = client.transfer(
    wallet=wallet,
    to="0x7a8b9c...recipient",
    amount=parse_usdc("25"),  # 25_000_000
)

print(f"Hash: {receipt.hash}")
print(f"Block: {receipt.block_number}")
print(f"Status: {receipt.status}")  # "confirmed"
print(f"Fee paid: {receipt.fee_paid / 1_000_000} USDC")`}</code>
        </pre>
      </section>

      {/* CLI Example */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">CLI</h2>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`# Send 25 USDC to a recipient
dina transfer \\
  --to 0x7a8b9c...recipient \\
  --amount 25.0 \\
  --network testnet

# Output:
# Transaction hash: 0xabc123...
# Block: 148302
# Status: confirmed
# Fee: 0.001000 USDC
# Finality: 1 block (100ms)

# Check transaction status
dina tx status 0xabc123... --network testnet

# Query account nonce (useful for manual tx construction)
dina account nonce 0x7a8b9c...myaddr --network testnet`}</code>
        </pre>
      </section>

      {/* Raw Transaction Example */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Raw Transaction (Advanced)
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          If you need to construct transactions manually without the SDK,
          here is the raw JSON structure and signing process:
        </p>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`// 1. Build the unsigned transaction
const unsignedTx = {
  amount: 25000000,        // 25 USDC
  fee: 1000,               // 0.001 USDC
  from: "0x1a2b3c...sender",
  nonce: 42,
  to: "0x7a8b9c...recipient",
};
// Note: fields MUST be alphabetically ordered for canonical serialization

// 2. Serialize to bytes
const txBytes = new TextEncoder().encode(JSON.stringify(unsignedTx));

// 3. Sign with Ed25519
import { sign } from '@noble/ed25519';
const signature = await sign(txBytes, privateKeyBytes);

// 4. Submit via JSON-RPC
const response = await fetch('https://rpc-testnet.dina.network', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    jsonrpc: '2.0',
    method: 'dina_sendTransaction',
    params: [{
      ...unsignedTx,
      signature: Buffer.from(signature).toString('hex'),
    }],
    id: 1,
  }),
});

const result = await response.json();
console.log('TX Hash:', result.result.hash);`}</code>
        </pre>
      </section>

      {/* Confirmation */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Confirmation and Finality
        </h2>
        <div className="rounded-xl border border-blue-500/30 bg-blue-500/5 p-6">
          <h3 className="text-sm font-semibold text-blue-400 mb-2">
            100ms Finality
          </h3>
          <p className="text-sm text-slate-300 leading-relaxed mb-4">
            Dina Network uses TurboBFT consensus with 100ms block times. Once
            a transaction is included in a block, it is{" "}
            <strong>final and irreversible</strong>. There are no
            reorganizations, no probabilistic confirmations, and no need to
            wait for additional blocks.
          </p>
          <div className="grid gap-4 sm:grid-cols-3">
            <div className="rounded-lg bg-slate-900/60 p-4">
              <p className="text-2xl font-bold text-blue-400">1 block</p>
              <p className="text-xs text-slate-400 mt-1">Confirmations needed</p>
            </div>
            <div className="rounded-lg bg-slate-900/60 p-4">
              <p className="text-2xl font-bold text-blue-400">100ms</p>
              <p className="text-xs text-slate-400 mt-1">Time to finality</p>
            </div>
            <div className="rounded-lg bg-slate-900/60 p-4">
              <p className="text-2xl font-bold text-blue-400">0.001 USDC</p>
              <p className="text-xs text-slate-400 mt-1">Typical transfer fee</p>
            </div>
          </div>
        </div>
      </section>

      {/* Error Handling */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Common Errors
        </h2>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">Error Code</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Meaning</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Fix</th>
              </tr>
            </thead>
            <tbody>
              {[
                { code: "INSUFFICIENT_BALANCE", meaning: "Sender balance < amount + fee", fix: "Check balance with dina_getBalance before sending" },
                { code: "INVALID_NONCE", meaning: "Nonce does not match expected value", fix: "Query current nonce with dina_getAccount" },
                { code: "INVALID_SIGNATURE", meaning: "Signature verification failed", fix: "Ensure canonical JSON serialization (alphabetical field order)" },
                { code: "FEE_TOO_LOW", meaning: "Fee below minimum (1,000 micro-USDC)", fix: "Set fee >= 1000 (0.001 USDC)" },
                { code: "INVALID_RECIPIENT", meaning: "Recipient address is malformed", fix: "Addresses must be 0x + 64 hex characters" },
              ].map((err, i) => (
                <tr
                  key={err.code}
                  className={i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"}
                >
                  <td className="px-4 py-3 font-mono text-red-400">{err.code}</td>
                  <td className="px-4 py-3 text-slate-400">{err.meaning}</td>
                  <td className="px-4 py-3 text-slate-400">{err.fix}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {/* Next Steps */}
      <div className="mt-10 flex flex-wrap gap-4">
        <Link
          href="/docs/transactions/batch"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          Batch Transfers (DRC-19) &rarr;
        </Link>
        <Link
          href="/docs/transactions/fees"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          Gas & Fees &rarr;
        </Link>
        <Link
          href="/docs/transactions/lifecycle"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          Transaction Lifecycle &rarr;
        </Link>
      </div>
    </div>
  );
}
