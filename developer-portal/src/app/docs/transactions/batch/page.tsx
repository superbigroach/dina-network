import Link from "next/link";

export const metadata = {
  title: "Batch Transfers (DRC-19) — Dina Network Developer Portal",
  description:
    "Send USDC to up to 100 recipients in a single transaction using DRC-19 batch transfers.",
};

const SAVINGS = [
  { recipients: 1, individual: "21,000", batch: "21,000", saving: "0%" },
  { recipients: 10, individual: "210,000", batch: "51,000", saving: "76%" },
  { recipients: 50, individual: "1,050,000", batch: "171,000", saving: "84%" },
  { recipients: 100, individual: "2,100,000", batch: "321,000", saving: "85%" },
];

export default function BatchTransferPage() {
  return (
    <div>
      {/* Header */}
      <p className="text-sm font-medium uppercase tracking-wider text-blue-400 mb-3">
        Transactions
      </p>
      <h1 className="text-4xl font-bold tracking-tight text-white mb-4">
        Batch Transfers (DRC-19)
      </h1>
      <p className="text-lg text-slate-400 max-w-3xl leading-relaxed mb-10">
        Send USDC to up to 100 recipients in a single atomic transaction. DRC-19
        batch transfers reduce gas costs by up to 85% compared to individual
        transfers, and when combined with Swarm Wallets (DRC-63), enable 10,000
        payments per block.
      </p>

      {/* How It Works */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          How It Works
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-6">
          A batch transfer packages multiple recipient/amount pairs into a
          single transaction. The validator processes all transfers atomically
          -- either all succeed or all fail. This is ideal for payroll,
          airdrops, reward distribution, and any scenario where you need to
          pay many addresses at once.
        </p>
        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6 mb-6">
          <h3 className="text-sm font-semibold text-blue-400 mb-3">Key Properties</h3>
          <ul className="space-y-2">
            {[
              "Up to 100 recipients per batch transaction",
              "Atomic execution: all-or-nothing, no partial failures",
              "Single signature covers all transfers",
              "Single nonce increment (no nonce management headaches)",
              "85% gas savings compared to 100 individual transfers",
              "Same 100ms finality as regular transfers",
            ].map((item) => (
              <li key={item} className="flex items-start gap-2 text-sm text-slate-300">
                <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-blue-500" />
                {item}
              </li>
            ))}
          </ul>
        </div>
      </section>

      {/* TypeScript Example */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          JavaScript / TypeScript
        </h2>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`import { DinaClient, DinaWallet, parseUSDC } from '@dina-network/sdk';

const client = new DinaClient('https://rpc-testnet.dina.network');
const wallet = DinaWallet.fromKeyFile('./my-wallet.json');

// Batch transfer to multiple recipients
const batch = await client.batchTransfer({
  from: wallet.address,
  recipients: [
    { to: "0xaaa...alice",   amount: parseUSDC("10") },   // 10 USDC
    { to: "0xbbb...bob",     amount: parseUSDC("20") },   // 20 USDC
    { to: "0xccc...carol",   amount: parseUSDC("5.50") }, // 5.50 USDC
    { to: "0xddd...dave",    amount: parseUSDC("100") },  // 100 USDC
    { to: "0xeee...eve",     amount: parseUSDC("0.25") }, // 0.25 USDC
  ],
});

// Sign and submit (one signature, one nonce)
const receipt = await wallet.signAndSend(batch);

console.log('Batch hash:', receipt.hash);
console.log('Recipients paid:', receipt.transferCount); // 5
console.log('Total USDC:', receipt.totalAmount);        // 135,750,000
console.log('Gas used:', receipt.gasUsed);               // ~36,000`}</code>
        </pre>
      </section>

      {/* Python Example */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">Python</h2>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`from dina import DinaClient, DinaWallet, parse_usdc

client = DinaClient("https://rpc-testnet.dina.network")
wallet = DinaWallet.from_key_file("./my-wallet.json")

receipt = client.batch_transfer(
    wallet=wallet,
    recipients=[
        {"to": "0xaaa...alice", "amount": parse_usdc("10")},
        {"to": "0xbbb...bob",   "amount": parse_usdc("20")},
        {"to": "0xccc...carol", "amount": parse_usdc("5.50")},
        {"to": "0xddd...dave",  "amount": parse_usdc("100")},
        {"to": "0xeee...eve",   "amount": parse_usdc("0.25")},
    ],
)

print(f"Batch hash: {receipt.hash}")
print(f"Recipients: {receipt.transfer_count}")
print(f"Total USDC: {receipt.total_amount / 1_000_000}")`}</code>
        </pre>
      </section>

      {/* CLI Example */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">CLI</h2>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`# Batch transfer from a JSON file
cat recipients.json
# [
#   { "to": "0xaaa...alice", "amount": 10000000 },
#   { "to": "0xbbb...bob",   "amount": 20000000 },
#   { "to": "0xccc...carol", "amount": 5500000 }
# ]

dina transfer batch \\
  --file recipients.json \\
  --network testnet

# Inline batch transfer
dina transfer batch \\
  --to 0xaaa...alice=10.0 \\
  --to 0xbbb...bob=20.0 \\
  --to 0xccc...carol=5.50 \\
  --network testnet`}</code>
        </pre>
      </section>

      {/* Gas Savings Table */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Gas Savings vs Individual Transfers
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-6">
          Batch transfers amortize the base transaction cost (21,000 gas) across
          all recipients, adding only 3,000 gas per additional recipient.
        </p>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">Recipients</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Individual Gas</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Batch Gas</th>
                <th className="px-4 py-3 text-left font-medium text-green-400">Saving</th>
              </tr>
            </thead>
            <tbody>
              {SAVINGS.map((row, i) => (
                <tr
                  key={row.recipients}
                  className={i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"}
                >
                  <td className="px-4 py-3 text-slate-300">{row.recipients}</td>
                  <td className="px-4 py-3 font-mono text-slate-400">{row.individual}</td>
                  <td className="px-4 py-3 font-mono text-blue-300">{row.batch}</td>
                  <td className="px-4 py-3 font-semibold text-green-400">{row.saving}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {/* Swarm + Batch */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Swarm Wallets + Batch Transfers
        </h2>
        <div className="rounded-xl border border-purple-500/30 bg-purple-500/5 p-6">
          <h3 className="text-sm font-semibold text-purple-400 mb-2">
            10,000 Payments Per Block
          </h3>
          <p className="text-sm text-slate-300 leading-relaxed mb-4">
            The real power of DRC-19 emerges when combined with{" "}
            <Link href="/docs/wallets/swarm" className="text-purple-400 hover:text-purple-300 underline">
              Swarm Wallets (DRC-63)
            </Link>
            . A single Swarm Wallet with 100 agent wallets can each submit a
            batch of 100 recipients in parallel. That is{" "}
            <strong className="text-white">100 agents x 100 recipients = 10,000 payments</strong>{" "}
            in a single 100ms block.
          </p>
          <pre className="overflow-x-auto rounded-lg bg-slate-800 p-4 text-sm leading-relaxed">
            <code className="text-slate-200">{`import { DinaClient, SwarmWallet, parseUSDC } from '@dina-network/sdk';

const client = new DinaClient('https://rpc-testnet.dina.network');
const swarm = await SwarmWallet.load('./swarm-authority.json', client);

// Each of the 100 agents sends a batch of 100 recipients
const promises = swarm.agents.map((agent, agentIdx) => {
  const recipients = payrollSlice(agentIdx, 100); // get this agent's slice
  return agent.batchTransfer({
    from: agent.address,
    recipients: recipients.map(r => ({
      to: r.address,
      amount: parseUSDC(r.salary),
    })),
  });
});

// All 100 batches execute in PARALLEL within the same block
const receipts = await Promise.all(promises);
console.log(\`Paid \${receipts.length * 100} employees in one block\`);
// => "Paid 10000 employees in one block"`}</code>
          </pre>
        </div>
      </section>

      {/* Limits and Constraints */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Limits and Constraints
        </h2>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">Constraint</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Value</th>
              </tr>
            </thead>
            <tbody>
              {[
                { label: "Max recipients per batch", value: "100" },
                { label: "Min recipients per batch", value: "2 (use regular transfer for 1)" },
                { label: "Max total USDC per batch", value: "Sender's balance minus fee" },
                { label: "Fee formula", value: "21,000 + (3,000 x recipients) gas" },
                { label: "Duplicate recipients", value: "Allowed (amounts are additive)" },
                { label: "Zero-amount recipients", value: "Not allowed (will reject)" },
              ].map((row, i) => (
                <tr
                  key={row.label}
                  className={i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"}
                >
                  <td className="px-4 py-3 font-medium text-slate-300">{row.label}</td>
                  <td className="px-4 py-3 text-slate-400">{row.value}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {/* Next Steps */}
      <div className="mt-10 flex flex-wrap gap-4">
        <Link
          href="/docs/transactions/transfer"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          &larr; Single Transfers
        </Link>
        <Link
          href="/docs/transactions/fees"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          Gas & Fees &rarr;
        </Link>
        <Link
          href="/docs/wallets/swarm"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          Swarm Wallets &rarr;
        </Link>
      </div>
    </div>
  );
}
