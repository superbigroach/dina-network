import Link from "next/link";

export const metadata = {
  title: "Gas & Fees — Dina Network Developer Portal",
  description:
    "Fee structure, gas estimation, and fee distribution on the Dina Network.",
};

const FEE_TABLE = [
  { operation: "USDC Transfer", gas: "21,000", usdcCost: "0.001000", notes: "Standard address-to-address transfer" },
  { operation: "Batch Transfer (per recipient)", gas: "21,000 + 3,000/recipient", usdcCost: "~0.001143 for 10", notes: "DRC-19 batch, amortized base cost" },
  { operation: "Contract Deploy", gas: "21,000 + 200/byte", usdcCost: "Varies by WASM size", notes: "Base + per-byte storage cost" },
  { operation: "Contract Call (mutation)", gas: "30,000", usdcCost: "0.001429", notes: "State-changing contract interaction" },
  { operation: "Contract Call (view)", gas: "0", usdcCost: "Free", notes: "Read-only calls cost nothing" },
  { operation: "Device Registration", gas: "25,000", usdcCost: "0.001190", notes: "DRC-2 device identity registration" },
  { operation: "Agent Wallet Creation", gas: "30,000", usdcCost: "0.001429", notes: "DRC-101 agent wallet setup" },
  { operation: "Swarm Wallet Creation", gas: "30,000 + 5,000/agent", usdcCost: "Varies by agent count", notes: "DRC-63 swarm wallet with N agents" },
  { operation: "Payment Channel Open", gas: "30,000", usdcCost: "0.001429", notes: "Lock funds in bilateral channel" },
  { operation: "Payment Channel Close", gas: "30,000", usdcCost: "0.001429", notes: "Settle and release funds on-chain" },
];

export default function FeesPage() {
  return (
    <div>
      {/* Header */}
      <p className="text-sm font-medium uppercase tracking-wider text-blue-400 mb-3">
        Transactions
      </p>
      <h1 className="text-4xl font-bold tracking-tight text-white mb-4">
        Gas & Fees
      </h1>
      <p className="text-lg text-slate-400 max-w-3xl leading-relaxed mb-10">
        Dina Network fees are denominated entirely in USDC. There is no separate
        gas token. This means transaction costs are stable and predictable --
        critical for machine-to-machine payments where cost volatility is
        unacceptable.
      </p>

      {/* Key Concept */}
      <section className="mb-12">
        <div className="rounded-xl border border-blue-500/30 bg-blue-500/5 p-6">
          <h3 className="text-sm font-semibold text-blue-400 mb-2">
            No Gas Token Needed
          </h3>
          <p className="text-sm text-slate-300 leading-relaxed">
            Unlike Ethereum (where you need ETH for gas) or Solana (where you
            need SOL for rent), Dina uses USDC for everything. If you have USDC,
            you can transact. Period. This eliminates the &ldquo;I have tokens
            but no gas&rdquo; problem that plagues every other chain.
          </p>
        </div>
      </section>

      {/* Fee Structure Table */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Fee Structure
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-6">
          Gas is a unit of computation. The gas price on Dina is fixed at{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
            1 gas = 0.000047619 micro-USDC
          </code>{" "}
          (approximately 21,000 gas = 0.001 USDC). This price is set by
          validator consensus and adjusts quarterly.
        </p>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">Operation</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Gas</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">USDC Cost</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Notes</th>
              </tr>
            </thead>
            <tbody>
              {FEE_TABLE.map((row, i) => (
                <tr
                  key={row.operation}
                  className={i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"}
                >
                  <td className="px-4 py-3 font-medium text-slate-300">{row.operation}</td>
                  <td className="px-4 py-3 font-mono text-blue-300">{row.gas}</td>
                  <td className="px-4 py-3 font-mono text-green-400">{row.usdcCost}</td>
                  <td className="px-4 py-3 text-slate-500">{row.notes}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {/* Fee Distribution */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Fee Distribution
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-6">
          Transaction fees are split between the validator that produced the
          block and the network treasury.
        </p>
        <div className="grid gap-4 sm:grid-cols-2 mb-6">
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <div className="flex items-center gap-3 mb-3">
              <div className="h-4 w-4 rounded-full bg-blue-500" />
              <h3 className="text-lg font-semibold text-white">80% Validator</h3>
            </div>
            <p className="text-sm text-slate-400 leading-relaxed">
              The block-producing validator receives 80% of all fees in the
              block. This incentivizes validators to include transactions
              promptly and maintain high uptime.
            </p>
          </div>
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <div className="flex items-center gap-3 mb-3">
              <div className="h-4 w-4 rounded-full bg-purple-500" />
              <h3 className="text-lg font-semibold text-white">20% Treasury</h3>
            </div>
            <p className="text-sm text-slate-400 leading-relaxed">
              The network treasury receives 20% of all fees. Treasury funds are
              used for protocol development, ecosystem grants, and network
              infrastructure. Treasury spending is governed by validator
              consensus.
            </p>
          </div>
        </div>
      </section>

      {/* Gas Estimation */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Gas Estimation API
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Use the{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
            dina_estimateGas
          </code>{" "}
          RPC method to estimate gas before submitting a transaction. This is
          especially useful for contract calls where gas consumption depends on
          the contract logic.
        </p>

        <h3 className="text-lg font-semibold text-white mb-3 mt-6">JSON-RPC</h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`// Estimate gas for a transfer
const response = await fetch('https://rpc-testnet.dina.network', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    jsonrpc: '2.0',
    method: 'dina_estimateGas',
    params: [{
      from: '0x1a2b3c...sender',
      to: '0x7a8b9c...recipient',
      amount: 25000000,
    }],
    id: 1,
  }),
});

const result = await response.json();
console.log('Estimated gas:', result.result.gas);       // 21000
console.log('Estimated fee:', result.result.fee);       // 1000 (micro-USDC)
console.log('Fee in USDC:', result.result.feeUSDC);     // "0.001000"`}</code>
        </pre>

        <h3 className="text-lg font-semibold text-white mb-3 mt-6">SDK</h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`import { DinaClient, parseUSDC } from '@dina-network/sdk';

const client = new DinaClient('https://rpc-testnet.dina.network');

// Estimate for a simple transfer
const estimate = await client.estimateGas({
  from: '0x1a2b3c...sender',
  to: '0x7a8b9c...recipient',
  amount: parseUSDC('25'),
});

console.log('Gas:', estimate.gas);         // 21000
console.log('Fee (USDC):', estimate.fee);  // 0.001

// Estimate for a contract call
const contractEstimate = await client.estimateGas({
  from: '0x1a2b3c...sender',
  to: '0xcontract...addr',
  method: 'set_greeting',
  args: { new_greeting: 'Hello!' },
});

console.log('Gas:', contractEstimate.gas);   // 30000
console.log('Fee (USDC):', contractEstimate.fee); // 0.001429`}</code>
        </pre>

        <h3 className="text-lg font-semibold text-white mb-3 mt-6">CLI</h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`# Estimate gas for a transfer
dina estimate \\
  --to 0x7a8b9c...recipient \\
  --amount 25.0 \\
  --network testnet

# Output:
# Gas estimate: 21,000
# Fee estimate: 0.001000 USDC

# Estimate gas for a contract call
dina estimate \\
  --to 0xcontract...addr \\
  --method set_greeting \\
  --args '{"new_greeting": "Hello!"}' \\
  --network testnet`}</code>
        </pre>
      </section>

      {/* Contract Deploy Cost */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Contract Deployment Costs
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Contract deployment gas depends on the compiled WASM binary size.
          The formula is:
        </p>
        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6 mb-6">
          <code className="text-lg font-mono text-blue-300">
            gas = 21,000 + (200 x wasm_bytes)
          </code>
        </div>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">Contract Size</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Gas</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">USDC Cost</th>
              </tr>
            </thead>
            <tbody>
              {[
                { size: "10 KB (simple)", gas: "2,021,000", cost: "~0.096 USDC" },
                { size: "50 KB (medium)", gas: "10,021,000", cost: "~0.477 USDC" },
                { size: "100 KB (complex)", gas: "20,021,000", cost: "~0.953 USDC" },
                { size: "500 KB (large)", gas: "100,021,000", cost: "~4.763 USDC" },
              ].map((row, i) => (
                <tr
                  key={row.size}
                  className={i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"}
                >
                  <td className="px-4 py-3 text-slate-300">{row.size}</td>
                  <td className="px-4 py-3 font-mono text-blue-300">{row.gas}</td>
                  <td className="px-4 py-3 font-mono text-green-400">{row.cost}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {/* Tips */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Fee Optimization Tips
        </h2>
        <div className="space-y-4">
          {[
            {
              title: "Use batch transfers for multiple payments",
              desc: "DRC-19 batch transfers save up to 85% gas compared to individual transfers. Always batch when paying more than one recipient.",
            },
            {
              title: "Use view calls for reads",
              desc: "View calls (read-only contract queries) are completely free. Never use a mutation when a view call suffices.",
            },
            {
              title: "Minimize WASM binary size",
              desc: "Strip debug symbols, use wasm-opt, and avoid unnecessary dependencies. Smaller binaries = cheaper deploys.",
            },
            {
              title: "Keep contract state compact",
              desc: "Gas for contract calls is proportional to state serialization size. Use u8 instead of String for enums, and prune stale data.",
            },
          ].map((tip) => (
            <div
              key={tip.title}
              className="rounded-xl border border-slate-800 bg-slate-900/50 p-5"
            >
              <h3 className="text-sm font-semibold text-white mb-1">{tip.title}</h3>
              <p className="text-sm text-slate-400">{tip.desc}</p>
            </div>
          ))}
        </div>
      </section>

      {/* Next Steps */}
      <div className="mt-10 flex flex-wrap gap-4">
        <Link
          href="/docs/transactions/batch"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          &larr; Batch Transfers
        </Link>
        <Link
          href="/docs/transactions/channels"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          Payment Channels &rarr;
        </Link>
      </div>
    </div>
  );
}
