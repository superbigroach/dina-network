import Link from "next/link";

export const metadata = {
  title: "Swarm Wallets (DRC-63) — Dina Network Developer Portal",
  description:
    "Parallel transaction execution with Swarm Wallets. 1 authority, N agent wallets, N transactions per block.",
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

export default function SwarmWalletPage() {
  return (
    <div className="mx-auto max-w-4xl px-6 py-16">
      {/* Header */}
      <p className="text-sm font-medium uppercase tracking-wider text-purple-400 mb-3">
        Wallets / DRC-63
      </p>
      <h1 className="text-4xl font-bold tracking-tight mb-4">
        Swarm Wallets
      </h1>
      <p className="text-lg text-slate-400 max-w-3xl leading-relaxed mb-4">
        The killer feature. Swarm wallets break through the fundamental
        throughput bottleneck that limits every blockchain: sequential
        per-account transactions. One authority controls N agent wallets
        that execute N transactions in parallel, in a single block.
      </p>
      <div className="flex items-center gap-2 mb-12">
        <span className="rounded-full bg-purple-500/20 px-3 py-1 text-xs font-medium text-purple-400">
          DRC-63
        </span>
        <span className="text-xs text-slate-500">
          Dina Request for Comments #63 — Swarm Wallet Standard
        </span>
      </div>

      {/* The Problem */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">The Problem</h2>
        <div className="rounded-xl border border-red-500/30 bg-red-500/5 p-6">
          <p className="text-sm text-slate-300 leading-relaxed mb-4">
            Every blockchain — Ethereum, Solana, Bitcoin — requires each account to
            submit transactions <strong>sequentially</strong> using an incrementing
            nonce. This means a single wallet can only process{" "}
            <strong>one transaction per block</strong>, regardless of how fast the
            chain is.
          </p>
          <p className="text-sm text-slate-300 leading-relaxed">
            If you need to pay 10,000 people, you must submit 10,000 sequential
            transactions from one wallet. Even on a fast chain with 100ms blocks,
            that takes <strong>~17 minutes</strong>. On Ethereum (12s blocks),
            it takes <strong>~33 hours</strong>.
          </p>
        </div>
      </div>

      {/* The Solution */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">The Solution</h2>
        <div className="rounded-xl border border-purple-500/30 bg-purple-500/5 p-6">
          <p className="text-sm text-slate-300 leading-relaxed mb-4">
            Swarm wallets give you <strong>N independent agent wallets</strong>,
            each with its own nonce, all controlled by a single authority. Each
            agent submits transactions independently, in parallel. The blockchain
            processes all of them in the same block.
          </p>
          <p className="text-sm text-slate-300 leading-relaxed">
            <strong>100 agent wallets = 100 transactions per block.</strong>{" "}
            Combined with Dina&apos;s 100ms block time and batch transactions
            (100 transfers per tx), a single swarm can process{" "}
            <strong>10,000 payments per block</strong>.
          </p>
        </div>
      </div>

      {/* ASCII diagram */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Parallel Execution Architecture
        </h2>
        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
          <pre className="text-sm text-slate-300 leading-relaxed overflow-x-auto font-mono">
            {`                     +----------------------+
                     |   Authority Wallet   |
                     |   (Master Control)   |
                     +----------+-----------+
                                |
               createSwarmWallet(agentCount: 5)
                                |
          +-----+-----+-----+-----+-----+
          |     |     |     |     |     |
          v     v     v     v     v     v
       +-----+-----+-----+-----+-----+
       | A-0 | A-1 | A-2 | A-3 | A-4 |    5 Agent Wallets
       +--+--+--+--+--+--+--+--+--+--+    (each has own nonce)
          |     |     |     |     |
          v     v     v     v     v        All execute IN PARALLEL
       +-----+-----+-----+-----+-----+    in the SAME BLOCK
       |tx-0 |tx-1 |tx-2 |tx-3 |tx-4 |
       +-----+-----+-----+-----+-----+

  Regular wallet:  1 tx  per block  (sequential nonce)
  Swarm (5 agents): 5 tx per block  (parallel nonces)
  Swarm (100 agents): 100 tx per block
  Swarm (100 agents) + batch(100): 10,000 payments per block`}
          </pre>
        </div>
      </div>

      {/* Throughput math */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Throughput Math
        </h2>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">
                  Configuration
                </th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">
                  Tx / Block
                </th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">
                  Payments / Block
                </th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">
                  Time for 10K payments
                </th>
              </tr>
            </thead>
            <tbody>
              {[
                {
                  config: "Regular wallet",
                  txBlock: "1",
                  payBlock: "1",
                  time: "~17 minutes",
                },
                {
                  config: "Regular + batch(100)",
                  txBlock: "1",
                  payBlock: "100",
                  time: "~10 seconds",
                },
                {
                  config: "Swarm (10 agents)",
                  txBlock: "10",
                  payBlock: "10",
                  time: "~100 seconds",
                },
                {
                  config: "Swarm (100 agents)",
                  txBlock: "100",
                  payBlock: "100",
                  time: "~10 seconds",
                },
                {
                  config: "Swarm (100) + batch(100)",
                  txBlock: "100",
                  payBlock: "10,000",
                  time: "~100ms (1 block)",
                },
              ].map((row, i) => (
                <tr
                  key={row.config}
                  className={
                    i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"
                  }
                >
                  <td className="px-4 py-3 font-medium text-slate-300">
                    {row.config}
                  </td>
                  <td className="px-4 py-3 text-slate-400">{row.txBlock}</td>
                  <td className="px-4 py-3 text-slate-400">{row.payBlock}</td>
                  <td className="px-4 py-3 text-slate-400">{row.time}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p className="text-xs text-slate-500 mt-3">
          Assumes Dina Network 100ms block time. Batch(100) means 100 transfers
          bundled into a single transaction.
        </p>
      </div>

      {/* Code: create a swarm */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Create a Swarm
        </h2>
        <div className="space-y-4">
          <CodeBlock language="typescript" title="Create a 100-agent swarm">
            {`import { DinaClient, DinaWallet, parseUSDC } from "@dina-network/sdk";

const client = new DinaClient({ network: "mainnet" });
const masterWallet = DinaWallet.fromPrivateKey(process.env.MASTER_KEY!);

// Create a swarm with 100 agent wallets
const swarm = await client.createSwarmWallet({
  authority: masterWallet.address,
  agentCount: 100,
  perAgentBalance: parseUSDC("10"),  // Fund each agent with 10 USDC
});

console.log("Swarm ID:    ", swarm.id);
console.log("Agent count: ", swarm.agents.length);
console.log("Total funded:", "1,000 USDC");

// Each agent is an independent wallet with its own key and nonce
swarm.agents.forEach((agent, i) => {
  console.log(\`  Agent \${i}: \${agent.address}\`);
});`}
          </CodeBlock>

          <CodeBlock language="typescript" title="Execute 100 parallel transfers">
            {`// Pay 100 different recipients in a single block
const recipients = await getPayrollRecipients(); // array of 100 addresses

const results = await Promise.all(
  swarm.agents.map((agent, i) =>
    agent.transfer(recipients[i], parseUSDC("1"))
  )
);

// All 100 transactions land in the SAME block
console.log(\`Completed \${results.length} transfers in 1 block\`);
console.log("Block:", results[0].blockNumber); // All same block number`}
          </CodeBlock>

          <CodeBlock language="typescript" title="Batch transfers for maximum throughput">
            {`// Each agent sends a batch of 100 transfers = 10,000 total payments
const allRecipients = await getAirdropRecipients(); // 10,000 addresses

const batches = chunkArray(allRecipients, 100); // Split into 100 batches of 100

const results = await Promise.all(
  swarm.agents.map((agent, i) =>
    agent.batchTransfer(
      batches[i].map((addr) => ({
        to: addr,
        amount: parseUSDC("0.10"),
      }))
    )
  )
);

// 10,000 payments complete in ~100ms (1 block)
console.log("Total payments:", results.reduce((sum, r) => sum + r.count, 0));`}
          </CodeBlock>
        </div>
      </div>

      {/* Comparison */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Regular Wallet vs Swarm Wallet
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
              Swarm Wallet (100 agents)
            </h3>
            <pre className="text-xs text-slate-400 leading-relaxed font-mono overflow-x-auto">
              {`Block 1:  Agent-0  tx (nonce 0)
          Agent-1  tx (nonce 0)
          Agent-2  tx (nonce 0)
          ...
          Agent-99 tx (nonce 0)

Time for 100 tx: 1 block = 100ms`}
            </pre>
          </div>
        </div>
      </div>

      {/* Managing the swarm */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Managing the Swarm
        </h2>
        <div className="space-y-4">
          <CodeBlock language="typescript" title="Rebalance agent funds">
            {`// Move funds between agents or top up from authority
await swarm.rebalance({
  authority: masterWallet,
  perAgentBalance: parseUSDC("10"), // Reset each agent to 10 USDC
});`}
          </CodeBlock>

          <CodeBlock language="typescript" title="Add more agents">
            {`// Scale up the swarm
await swarm.addAgents({
  authority: masterWallet,
  count: 50,                        // Add 50 more agents (total: 150)
  perAgentBalance: parseUSDC("10"),
});`}
          </CodeBlock>

          <CodeBlock language="typescript" title="Dissolve the swarm">
            {`// Return all funds to the authority and destroy agent wallets
await swarm.dissolve({
  authority: masterWallet,
});
// All remaining USDC across all agents is returned to the authority wallet.`}
          </CodeBlock>
        </div>
      </div>

      {/* Real-world use cases */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Real-World Use Cases
        </h2>
        <div className="space-y-4">
          {[
            {
              title: "Payroll Processing",
              desc: "A company with 5,000 employees can pay everyone in a single block. Create a swarm with 50 agents, each batch-transferring to 100 employees. Total time: 100ms.",
              color: "text-green-400",
            },
            {
              title: "Token Airdrops",
              desc: "Distribute tokens to 100,000 holders without spending hours submitting sequential transactions. A 100-agent swarm with batch(100) processes 10,000 per block — done in 10 blocks (1 second).",
              color: "text-blue-400",
            },
            {
              title: "Exchange Settlement",
              desc: "A DEX settles all matched orders at end-of-block using a swarm. Hundreds of trades settle simultaneously instead of creating a backlog.",
              color: "text-amber-400",
            },
            {
              title: "IoT Payment Networks",
              desc: "Thousands of IoT devices making micropayments — each device maps to an agent in a swarm, processing payments in parallel without nonce conflicts.",
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

      {/* Security considerations */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Security Considerations
        </h2>
        <div className="space-y-3">
          {[
            "The authority wallet is the single point of control. Protect its private key with the same care as any high-value wallet.",
            "Agent wallets can only spend their allocated balance. Even if an agent key is compromised, the attacker can only access that agent's funds.",
            "The authority can dissolve the swarm instantly, recovering all funds across all agents in a single transaction.",
            "Agent keys are derived deterministically from the authority key and a swarm-specific salt. The authority can always regenerate agent keys.",
            "On-chain swarm metadata tracks which agents belong to which authority, preventing unauthorized agents from joining a swarm.",
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
