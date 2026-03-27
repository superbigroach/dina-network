import Link from "next/link";

export const metadata = {
  title: "Parallel Wallets (DRC-63) — Dina Network Developer Portal",
  description:
    "Per-user transaction parallelism with Parallel Wallets. The only blockchain primitive that gives users 100x throughput, not just validators.",
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

const MODE_ROWS = [
  {
    mode: "Single",
    wallets: "1",
    speed: "1 tx/block",
    cost: "$0.10 (100 batches x $0.001)",
    bestFor: "Simple transfers",
  },
  {
    mode: "Batch",
    wallets: "1",
    speed: "100/tx",
    cost: "$0.10 (100 batches)",
    bestFor: "Payroll, airdrops",
  },
  {
    mode: "Parallel",
    wallets: "100",
    speed: "100 tx/block",
    cost: "$0.20 (100 txs + distribute/consolidate)",
    bestFor: "Speed-critical",
  },
  {
    mode: "Auto",
    wallets: "Dynamic",
    speed: "Optimized",
    cost: "$0.10-0.20",
    bestFor: "Most users (recommended)",
  },
];

const CONTRACT_FUNCTIONS = [
  {
    name: "createParallelWallet",
    sig: "(authority, walletCount, initialBalance) -> walletId",
    desc: "Deploy a new parallel wallet with N sub-wallets, funding each from the authority.",
  },
  {
    name: "addWallets",
    sig: "(walletId, count, perWalletBalance)",
    desc: "Add more sub-wallets to an existing parallel wallet. Authority only.",
  },
  {
    name: "removeWallets",
    sig: "(walletId, count)",
    desc: "Remove sub-wallets, returning their funds to the authority. Authority only.",
  },
  {
    name: "distribute",
    sig: "(walletId, amounts[])",
    desc: "Distribute funds from the authority to sub-wallets in a single transaction.",
  },
  {
    name: "consolidate",
    sig: "(walletId)",
    desc: "Sweep all sub-wallet balances back to the authority in a single transaction.",
  },
  {
    name: "rebalance",
    sig: "(walletId, targetBalance)",
    desc: "Reset every sub-wallet to a target balance, moving excess back to authority.",
  },
  {
    name: "dissolve",
    sig: "(walletId)",
    desc: "Permanently destroy all sub-wallets and return all funds to the authority.",
  },
  {
    name: "getWalletInfo",
    sig: "(walletId) -> WalletInfo",
    desc: "Returns authority, sub-wallet count, total balance, and per-wallet balances.",
  },
  {
    name: "getSubWallets",
    sig: "(walletId) -> address[]",
    desc: "Returns the list of sub-wallet addresses for a parallel wallet.",
  },
  {
    name: "setWalletCap",
    sig: "(walletId, maxWallets)",
    desc: "Set the maximum number of sub-wallets allowed. Default: 10,000.",
  },
];

const FAQ_ITEMS = [
  {
    q: "Is it more expensive?",
    a: "No. Each transaction costs the same $0.001 fee regardless of whether it comes from a regular wallet or a sub-wallet. The only overhead is the initial distribute (~$0.001) and final consolidate (~$0.001) transactions. For 10,000 payments via 100 parallel wallets, total overhead is about $0.10.",
  },
  {
    q: "How many wallets can I have?",
    a: "Up to 10,000 (safety cap, configurable by the authority). The practical sweet spot is 100 sub-wallets -- enough for 10,000 payments per block when combined with batch transfers. Beyond 100, the marginal benefit decreases because you hit block gas limits.",
  },
  {
    q: "Do wallets cost anything when idle?",
    a: "No. Sub-wallets are free to hold. There is no rent, no minimum balance requirement, and no recurring fee. You only pay transaction fees when you actually send transactions.",
  },
  {
    q: "Can someone else see my sub-wallets are connected?",
    a: "Yes. The authority contract links them on-chain, so the relationship between authority and sub-wallets is publicly visible. This is by design -- it enables auditing and transparency. If you need unlinkable wallets, use separate standard wallets instead.",
  },
  {
    q: "What happens if a sub-wallet key is compromised?",
    a: "The attacker can only spend that sub-wallet's allocated balance. The authority can instantly dissolve the parallel wallet, recovering funds from all other sub-wallets. Sub-wallet keys are derived deterministically from the authority key, so the authority can always regenerate them.",
  },
  {
    q: "Can I use Parallel Wallets with Agent Wallets (DRC-101)?",
    a: "Yes. You can set an Agent Wallet as the authority of a Parallel Wallet, giving an autonomous agent access to N parallel sub-wallets with spending limits enforced at the agent level.",
  },
];

export default function ParallelWalletPage() {
  return (
    <div className="mx-auto max-w-4xl px-6 py-16">
      {/* Header */}
      <p className="text-sm font-medium uppercase tracking-wider text-purple-400 mb-3">
        Wallets / DRC-63
      </p>
      <h1 className="text-4xl font-bold tracking-tight mb-2">
        Parallel Wallets
      </h1>
      <p className="text-xl font-medium text-purple-400 mb-4">
        Per-User Transaction Parallelism
      </p>
      <p className="text-lg text-slate-400 max-w-3xl leading-relaxed mb-4">
        The only blockchain primitive that gives <strong className="text-white">users</strong> parallel
        throughput, not just validators. One authority controls N sub-wallets
        that execute N transactions simultaneously in a single block.
      </p>
      <div className="flex items-center gap-2 mb-12">
        <span className="rounded-full bg-purple-500/20 px-3 py-1 text-xs font-medium text-purple-400">
          DRC-63
        </span>
        <span className="text-xs text-slate-500">
          Dina Request for Comments #63 -- Parallel Wallet Standard
        </span>
      </div>

      {/* Nobody Else Has This */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Nobody Else Has This
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-6">
          Every other chain that claims &ldquo;parallel execution&rdquo; means the{" "}
          <strong className="text-slate-200">validator</strong> parallelizes work internally. The user
          still sends one transaction at a time from one wallet, waiting for each
          nonce to increment. Dina is the first chain where the{" "}
          <strong className="text-slate-200">user</strong> can send 100 transactions in one block from
          one logical identity.
        </p>
        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
          <pre className="text-sm text-slate-300 leading-loose overflow-x-auto font-mono">
            {`Sui:    Validator parallelizes objects   -> user still sends 1 tx at a time
Aptos:  Validator parallelizes execution -> user still sends 1 tx at a time
Solana: Validator parallelizes accounts  -> user still sends 1 tx at a time
Dina:   USER creates 100 wallets -> sends 100 txs in 1 block -> 100x throughput`}
          </pre>
        </div>
        <div className="mt-4 rounded-xl border border-purple-500/30 bg-purple-500/5 p-5">
          <p className="text-sm text-slate-300 leading-relaxed">
            <strong className="text-purple-400">The difference is fundamental.</strong>{" "}
            Validator-level parallelism speeds up the chain for everyone equally.
            User-level parallelism lets a single entity multiply their own throughput
            by 100x without any protocol upgrade, governance vote, or special permission.
            It is a wallet-layer primitive, not a consensus change.
          </p>
        </div>
      </div>

      {/* How It Works */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          How It Works
        </h2>
        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
          <pre className="text-sm text-slate-300 leading-relaxed overflow-x-auto font-mono">
            {`                    +---------------------------+
                    |    Authority Wallet       |
                    |    (your main wallet)     |
                    +-----------+---------------+
                                |
              createParallelWallet(count: N)
                                |
         +------+------+------+------+------+
         |      |      |      |      |      |
         v      v      v      v      v      v
      +-----+-----+-----+-----+-----+-----+
      | W-0 | W-1 | W-2 | W-3 | ... | W-N |    N Sub-Wallets
      +--+--+--+--+--+--+--+--+-----+--+--+    (each has own nonce)
         |     |     |     |           |
         v     v     v     v           v         All execute IN PARALLEL
      +-----+-----+-----+-----+-----+-----+     in the SAME BLOCK
      |tx-0 |tx-1 |tx-2 |tx-3 | ... |tx-N |
      +-----+-----+-----+-----+-----+-----+

  Regular wallet:        1 tx  per block  (sequential nonce)
  Parallel (N=100):    100 tx  per block  (100 independent nonces)
  Parallel + batch:  10,000 payments per block  (100 wallets x 100/tx)`}
          </pre>
        </div>
      </div>

      {/* 4 Modes */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          4 Modes
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-6">
          Parallel Wallets support four operating modes. The SDK picks the best one
          automatically in Auto mode, or you can select explicitly.
        </p>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">Mode</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Wallets</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Speed</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Cost / 10K Payments</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Best For</th>
              </tr>
            </thead>
            <tbody>
              {MODE_ROWS.map((row, i) => (
                <tr
                  key={row.mode}
                  className={i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"}
                >
                  <td className="px-4 py-3 font-medium text-slate-300">{row.mode}</td>
                  <td className="px-4 py-3 text-slate-400">{row.wallets}</td>
                  <td className="px-4 py-3 text-slate-400">{row.speed}</td>
                  <td className="px-4 py-3 text-slate-400">{row.cost}</td>
                  <td className="px-4 py-3 text-slate-400">{row.bestFor}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Cost Breakdown */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Cost Breakdown: 10,000 Payments
        </h2>
        <div className="grid gap-6 md:grid-cols-2 mb-6">
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <h3 className="text-lg font-semibold text-blue-400 mb-3">
              Batch Sequential (1 wallet)
            </h3>
            <div className="space-y-2 text-sm text-slate-300 font-mono">
              <p>10,000 payments / 100 per batch = 100 batches</p>
              <p>100 batches x $0.001 per tx = <strong className="text-white">$0.10</strong></p>
              <p>100 batches x 100ms per block = <strong className="text-white">10 seconds</strong></p>
            </div>
          </div>
          <div className="rounded-xl border border-purple-500/30 bg-purple-500/5 p-6">
            <h3 className="text-lg font-semibold text-purple-400 mb-3">
              Parallel Batch (100 wallets)
            </h3>
            <div className="space-y-2 text-sm text-slate-300 font-mono">
              <p>100 wallets x 100 per batch = 10,000 in 1 block</p>
              <p>100 txs x $0.001 = $0.10 + distribute/consolidate = <strong className="text-white">$0.20</strong></p>
              <p>1 block x 100ms = <strong className="text-white">100 milliseconds</strong></p>
            </div>
          </div>
        </div>
        <div className="rounded-xl border border-green-500/30 bg-green-500/5 p-5 text-center">
          <p className="text-lg font-semibold text-green-400">
            10 cents more, 100x faster.
          </p>
          <p className="text-sm text-slate-400 mt-1">
            $0.20 instead of $0.10. 100ms instead of 10 seconds.
          </p>
        </div>
      </div>

      {/* SDK Code Examples */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          SDK Usage
        </h2>
        <div className="space-y-4">
          <CodeBlock language="typescript" title="Auto mode (recommended) -- SDK picks best strategy">
            {`import { DinaClient, DinaWallet, ParallelWallet, parseUSDC } from "@dina-network/sdk";

const client = new DinaClient({ network: "mainnet" });
const wallet = DinaWallet.fromPrivateKey(process.env.PRIVATE_KEY!);

// Create a parallel wallet -- SDK manages sub-wallets automatically
const pw = new ParallelWallet(wallet, client);

// Send 10,000 payments. SDK decides single/batch/parallel based on count.
const payments = recipients.map((addr) => ({ to: addr, amount: parseUSDC("1") }));
await pw.batchTransfer(payments);  // just works`}
          </CodeBlock>

          <CodeBlock language="typescript" title="Speed mode -- everything in 100ms">
            {`// Force parallel execution for minimum latency
await pw.batchTransfer(payments, { priority: "speed" });

// All 10,000 payments land in a single block (100ms)
// Cost: ~$0.20`}
          </CodeBlock>

          <CodeBlock language="typescript" title="Cost mode -- minimize fees">
            {`// Force sequential batching for minimum cost
await pw.batchTransfer(payments, { priority: "cost" });

// All 10,000 payments complete in ~10 seconds
// Cost: ~$0.10`}
          </CodeBlock>

          <CodeBlock language="typescript" title="Budget mode -- set max fee">
            {`// SDK optimizes parallelism to stay under your fee budget
await pw.batchTransfer(payments, { maxFee: parseUSDC("0.05") });

// SDK will use as many sub-wallets as possible within $0.05`}
          </CodeBlock>

          <CodeBlock language="typescript" title="Preset tiers">
            {`// Starter: up to 10 sub-wallets (good for small apps)
const starter = ParallelWallet.starter(wallet, client);

// Pro: up to 100 sub-wallets (good for most production use)
const pro = ParallelWallet.pro(wallet, client);

// Enterprise: up to 10,000 sub-wallets (exchange-grade)
const enterprise = ParallelWallet.enterprise(wallet, client);`}
          </CodeBlock>

          <CodeBlock language="typescript" title="Manual control">
            {`import { DinaClient, DinaWallet, parseUSDC } from "@dina-network/sdk";

const client = new DinaClient({ network: "mainnet" });
const wallet = DinaWallet.fromPrivateKey(process.env.PRIVATE_KEY!);

// Create a parallel wallet with explicit sub-wallet count
const pw = await client.createParallelWallet({
  authority: wallet.address,
  walletCount: 100,
  perWalletBalance: parseUSDC("10"),  // Fund each sub-wallet with 10 USDC
});

console.log("Parallel Wallet ID:", pw.id);
console.log("Sub-wallet count:  ", pw.subWallets.length);
console.log("Total funded:      ", "1,000 USDC");

// Execute 100 parallel transfers -- one per sub-wallet
const results = await Promise.all(
  pw.subWallets.map((sub, i) =>
    sub.transfer(recipients[i], parseUSDC("1"))
  )
);

// All 100 transactions land in the SAME block
console.log(\`Completed \${results.length} transfers in 1 block\`);

// Rebalance sub-wallets
await pw.rebalance({ authority: wallet, perWalletBalance: parseUSDC("10") });

// Consolidate all funds back to authority
await pw.consolidate({ authority: wallet });

// Dissolve when done
await pw.dissolve({ authority: wallet });`}
          </CodeBlock>
        </div>
      </div>

      {/* On-Chain Contract API */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          On-Chain Contract API (DRC-63)
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-6">
          These are the on-chain functions defined by the DRC-63 standard. The SDK
          wraps all of them, but you can call them directly if building custom tooling.
        </p>
        <div className="space-y-3">
          {CONTRACT_FUNCTIONS.map((fn) => (
            <div
              key={fn.name}
              className="rounded-xl border border-slate-800 bg-slate-900/50 p-4"
            >
              <code className="text-sm font-mono text-purple-400">
                {fn.name}
              </code>
              <code className="text-xs font-mono text-slate-500 ml-1">
                {fn.sig}
              </code>
              <p className="text-sm text-slate-400 mt-1.5">{fn.desc}</p>
            </div>
          ))}
        </div>
      </div>

      {/* Use Cases */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Use Cases
        </h2>
        <div className="space-y-4">
          {[
            {
              title: "Payment Processor",
              desc: "A payment gateway settling merchant payouts. 50 sub-wallets batch-transferring to 100 merchants each = 5,000 settlements per block. End-of-day settlement in a single 100ms block instead of minutes of sequential processing.",
              color: "text-green-400",
            },
            {
              title: "IoT Fleet",
              desc: "10,000 connected devices each mapped to a sub-wallet. Devices report sensor data and trigger micropayments autonomously. Each device transacts in parallel -- no nonce collisions, no queue, no coordinator bottleneck.",
              color: "text-blue-400",
            },
            {
              title: "AI Agent Swarm",
              desc: "An orchestrator AI spawns 100 sub-agents, each with its own sub-wallet. Agents bid on tasks, pay for API calls, and settle with each other -- all in parallel within a single block. The orchestrator consolidates profits after each round.",
              color: "text-amber-400",
            },
            {
              title: "Market Maker",
              desc: "A trading bot uses 100 sub-wallets to place 100 orders simultaneously across a DEX orderbook. Fill rates improve because orders arrive atomically in the same block instead of trickling in across 100 blocks.",
              color: "text-purple-400",
            },
          ].map((uc) => (
            <div
              key={uc.title}
              className="rounded-xl border border-slate-800 bg-slate-900/50 p-5"
            >
              <h3 className={`text-sm font-semibold ${uc.color} mb-2`}>
                {uc.title}
              </h3>
              <p className="text-sm text-slate-400 leading-relaxed">
                {uc.desc}
              </p>
            </div>
          ))}
        </div>
      </div>

      {/* Comparison: Regular vs Parallel */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Regular Wallet vs Parallel Wallet
        </h2>
        <div className="grid gap-6 md:grid-cols-2">
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <h3 className="text-lg font-semibold text-red-400 mb-4">
              Regular Wallet
            </h3>
            <pre className="text-xs text-slate-400 leading-relaxed font-mono overflow-x-auto">
              {`Block 1:  tx-0  (nonce 0)
Block 2:  tx-1  (nonce 1)
Block 3:  tx-2  (nonce 2)
Block 4:  tx-3  (nonce 3)
Block 5:  tx-4  (nonce 4)
  ...
Block 100: tx-99 (nonce 99)

Time for 100 tx: 100 blocks = 10 seconds`}
            </pre>
          </div>
          <div className="rounded-xl border border-purple-500/30 bg-purple-500/5 p-6">
            <h3 className="text-lg font-semibold text-purple-400 mb-4">
              Parallel Wallet (100 sub-wallets)
            </h3>
            <pre className="text-xs text-slate-400 leading-relaxed font-mono overflow-x-auto">
              {`Block 1:  W-0  tx (nonce 0)
          W-1  tx (nonce 0)
          W-2  tx (nonce 0)
          ...
          W-99 tx (nonce 0)

Time for 100 tx: 1 block = 100ms`}
            </pre>
          </div>
        </div>
      </div>

      {/* Security */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Security Model
        </h2>
        <div className="space-y-3">
          {[
            "The authority wallet is the single point of control. Protect its private key with the same care as any high-value wallet.",
            "Sub-wallets can only spend their allocated balance. Even if a sub-wallet key is compromised, the attacker can only access that sub-wallet's funds.",
            "The authority can dissolve the parallel wallet instantly, recovering all funds across all sub-wallets in a single transaction.",
            "Sub-wallet keys are derived deterministically from the authority key and a wallet-specific salt. The authority can always regenerate sub-wallet keys.",
            "On-chain metadata tracks which sub-wallets belong to which authority, preventing unauthorized sub-wallets from joining a parallel wallet.",
            "Rate limits can be applied per-sub-wallet or globally across the parallel wallet via DRC-101 agent constraints.",
          ].map((note, i) => (
            <div
              key={i}
              className="flex items-start gap-3 text-sm text-slate-300"
            >
              <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-purple-500" />
              <span>{note}</span>
            </div>
          ))}
        </div>
      </div>

      {/* FAQ */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          FAQ
        </h2>
        <div className="space-y-4">
          {FAQ_ITEMS.map((item) => (
            <div
              key={item.q}
              className="rounded-xl border border-slate-800 bg-slate-900/50 p-5"
            >
              <h3 className="text-sm font-semibold text-slate-200 mb-2">
                {item.q}
              </h3>
              <p className="text-sm text-slate-400 leading-relaxed">
                {item.a}
              </p>
            </div>
          ))}
        </div>
      </div>

      {/* Navigation */}
      <div className="flex items-center justify-between pt-8 border-t border-slate-800">
        <Link
          href="/docs/wallets/agent"
          className="text-sm text-slate-400 hover:text-blue-400 transition-colors"
        >
          &larr; Agent Wallets
        </Link>
        <Link
          href="/docs/wallets/keys"
          className="text-sm text-slate-400 hover:text-blue-400 transition-colors"
        >
          Key Management &rarr;
        </Link>
      </div>
    </div>
  );
}
