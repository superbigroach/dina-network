import Link from "next/link";

export const metadata = {
  title: "Agent Wallets (DRC-101) — Dina Network Developer Portal",
  description:
    "Create autonomous wallets for AI agents with configurable spending limits on Dina Network.",
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

const USE_CASES = [
  {
    title: "AI Trading Bots",
    desc: "Deploy a GPT-powered trading agent with a $500/day budget. The agent makes trades autonomously. If it malfunctions, the daily limit caps losses.",
  },
  {
    title: "IoT Micropayments",
    desc: "A sensor network pays for data relay in real-time. Each device gets an agent wallet with a $0.10/tx limit and $5/day cap.",
  },
  {
    title: "Automated Subscription Services",
    desc: "A SaaS backend uses an agent wallet to collect recurring USDC payments. The owner sets allowed contracts to only the subscription smart contract.",
  },
  {
    title: "Game NPCs",
    desc: "In-game characters with their own wallets that can buy and sell items on a marketplace, within developer-defined spending bounds.",
  },
];

export default function AgentWalletPage() {
  return (
    <div className="mx-auto max-w-4xl px-6 py-16">
      {/* Header */}
      <p className="text-sm font-medium uppercase tracking-wider text-amber-400 mb-3">
        Wallets / DRC-101
      </p>
      <h1 className="text-4xl font-bold tracking-tight mb-4">
        Agent Wallets
      </h1>
      <p className="text-lg text-slate-400 max-w-3xl leading-relaxed mb-4">
        Autonomous wallets for AI agents, bots, and automated services.
        The owner creates the wallet, sets spending guardrails, and the agent
        operates freely within those constraints — no human approval required
        for each transaction.
      </p>
      <div className="flex items-center gap-2 mb-12">
        <span className="rounded-full bg-amber-500/20 px-3 py-1 text-xs font-medium text-amber-400">
          DRC-101
        </span>
        <span className="text-xs text-slate-500">
          Dina Request for Comments #101 — Agent Wallet Standard
        </span>
      </div>

      {/* How it works */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          How Agent Wallets Work
        </h2>
        <div className="space-y-6">
          <div className="grid gap-4 md:grid-cols-3">
            {[
              {
                step: "1",
                title: "Owner creates agent wallet",
                desc: "The owner (a standard wallet) creates an agent wallet and funds it. The owner defines daily limits, per-transaction limits, and optionally an allowlist of contracts the agent can interact with.",
              },
              {
                step: "2",
                title: "Agent operates autonomously",
                desc: "The agent uses its own Ed25519 key pair to sign transactions. As long as each transaction is within the configured limits, it executes immediately on-chain with no owner approval.",
              },
              {
                step: "3",
                title: "Owner monitors and controls",
                desc: "The owner can view all agent transactions, adjust limits, top up the balance, or revoke access instantly. Revocation freezes the agent wallet in the same block.",
              },
            ].map((item) => (
              <div
                key={item.step}
                className="rounded-xl border border-slate-800 bg-slate-900/50 p-5"
              >
                <div className="flex items-center gap-3 mb-3">
                  <div className="flex h-7 w-7 items-center justify-center rounded-full bg-amber-500/20 text-sm font-bold text-amber-400">
                    {item.step}
                  </div>
                  <h3 className="text-sm font-semibold">{item.title}</h3>
                </div>
                <p className="text-sm text-slate-400 leading-relaxed">
                  {item.desc}
                </p>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Architecture diagram */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Architecture
        </h2>
        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
          <pre className="text-sm text-slate-300 leading-relaxed overflow-x-auto font-mono">
            {`  +-------------------+
  |   Owner Wallet    |    Standard wallet (human-controlled)
  |   (Ed25519 key)   |
  +--------+----------+
           |
           |  createAgentWallet()
           |  - dailyLimit: 100 USDC
           |  - perTxLimit: 10 USDC
           |  - allowedContracts: [0x...]
           v
  +-------------------+
  |   Agent Wallet    |    Autonomous wallet (bot-controlled)
  |   (Ed25519 key)   |
  |                   |
  |  Can spend up to  |
  |  10 USDC per tx   |
  |  100 USDC per day |
  +--------+----------+
           |
           |  agent.transfer(to, amount)
           |  agent.callContract(addr, data)
           v
  +-------------------+
  |  Dina Blockchain  |    Validates limits on-chain
  +-------------------+`}
          </pre>
        </div>
      </div>

      {/* Code examples */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Create an Agent Wallet
        </h2>
        <div className="space-y-4">
          <CodeBlock language="typescript" title="Create agent wallet (JavaScript)">
            {`import { DinaClient, DinaWallet, parseUSDC } from "@dina-network/sdk";

const client = new DinaClient({ network: "mainnet" });
const masterWallet = DinaWallet.fromPrivateKey(process.env.OWNER_KEY!);

// Create an agent wallet with spending limits
const agentWallet = await client.createAgentWallet({
  owner: masterWallet.address,
  dailyLimit: parseUSDC("100"),     // Max 100 USDC per day
  perTxLimit: parseUSDC("10"),      // Max 10 USDC per transaction
  allowedContracts: [               // Optional: restrict to specific contracts
    "0xabc123...def456",            // e.g., a DEX contract
    "0x789012...345678",            // e.g., a staking contract
  ],
});

console.log("Agent address:", agentWallet.address);
console.log("Agent key:    ", agentWallet.privateKey);
// Store the agent's private key in your bot's secure config.`}
          </CodeBlock>

          <CodeBlock language="typescript" title="Agent sends a transaction">
            {`// In your bot / agent process:
const agent = DinaWallet.fromPrivateKey(process.env.AGENT_KEY!);

// Transfer USDC (must be within limits)
const tx = await client.transfer({
  from: agent,
  to: "0xrecipient...",
  amount: parseUSDC("5"),
  token: "USDC",
});

console.log("Transaction hash:", tx.hash);
// If this would exceed the daily or per-tx limit,
// the transaction is rejected on-chain.`}
          </CodeBlock>

          <CodeBlock language="python" title="Create agent wallet (Python)">
            {`from dina_network import DinaClient, DinaWallet, parse_usdc

client = DinaClient(network="mainnet")
master = DinaWallet.from_private_key(os.environ["OWNER_KEY"])

agent_wallet = client.create_agent_wallet(
    owner=master.address,
    daily_limit=parse_usdc("100"),
    per_tx_limit=parse_usdc("10"),
    allowed_contracts=["0xabc123...def456"],
)

print(f"Agent address: {agent_wallet.address}")`}
          </CodeBlock>
        </div>
      </div>

      {/* Adjust limits */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Adjust Limits
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          The owner can update spending limits at any time. Changes take effect
          in the next block.
        </p>
        <CodeBlock language="typescript" title="Update agent limits">
          {`// Only the owner can call this
await client.updateAgentWallet({
  agent: agentWallet.address,
  owner: masterWallet,
  dailyLimit: parseUSDC("200"),     // Increase daily limit
  perTxLimit: parseUSDC("25"),      // Increase per-tx limit
  allowedContracts: [               // Update allowed contracts
    "0xabc123...def456",
    "0xnewcontract...789",
  ],
});`}
        </CodeBlock>
      </div>

      {/* Revoke access */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Revoke Agent Access
        </h2>
        <div className="rounded-xl border border-red-500/30 bg-red-500/5 p-6 mb-4">
          <h3 className="text-sm font-semibold text-red-400 mb-2">
            Instant Revocation
          </h3>
          <p className="text-sm text-slate-300 leading-relaxed">
            Revoking an agent wallet freezes it immediately. Any pending
            transactions from the agent are rejected. Remaining funds are
            returned to the owner wallet. This is irreversible — you must
            create a new agent wallet to restore access.
          </p>
        </div>
        <CodeBlock language="typescript" title="Revoke agent access">
          {`// Emergency: immediately freeze the agent wallet
await client.revokeAgentWallet({
  agent: agentWallet.address,
  owner: masterWallet,
});

// All remaining funds are returned to the owner.
// The agent's key pair is now useless.`}
        </CodeBlock>
      </div>

      {/* Use cases */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">Use Cases</h2>
        <div className="grid gap-4 md:grid-cols-2">
          {USE_CASES.map((uc) => (
            <div
              key={uc.title}
              className="rounded-xl border border-slate-800 bg-slate-900/50 p-5"
            >
              <h3 className="text-sm font-semibold text-amber-400 mb-2">
                {uc.title}
              </h3>
              <p className="text-sm text-slate-400 leading-relaxed">
                {uc.desc}
              </p>
            </div>
          ))}
        </div>
      </div>

      {/* On-chain details */}
      <div className="mb-12">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          On-Chain Enforcement
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Agent wallet limits are enforced at the protocol level, not in
          application code. When the Dina validator processes a transaction
          from an agent wallet, it checks:
        </p>
        <ol className="list-decimal list-inside space-y-2 text-sm text-slate-300 ml-2">
          <li>
            <strong>Per-transaction limit:</strong> Is{" "}
            <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
              tx.value &lt;= perTxLimit
            </code>
            ?
          </li>
          <li>
            <strong>Daily limit:</strong> Is{" "}
            <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
              dailySpent + tx.value &lt;= dailyLimit
            </code>
            ?
          </li>
          <li>
            <strong>Contract allowlist:</strong> If set, is the target contract
            in the allowed list?
          </li>
          <li>
            <strong>Revocation check:</strong> Has the owner revoked this agent?
          </li>
        </ol>
        <p className="text-sm text-slate-400 leading-relaxed mt-4">
          If any check fails, the transaction is rejected and the agent&apos;s nonce
          is not incremented. This means a malicious or buggy agent cannot drain
          the wallet beyond its configured limits, even if it has the private key.
        </p>
      </div>

      {/* Navigation */}
      <div className="flex items-center justify-between pt-8 border-t border-slate-800">
        <Link
          href="/docs/wallets/create"
          className="text-sm text-slate-400 hover:text-blue-400 transition-colors"
        >
          &larr; Create a Wallet
        </Link>
        <Link
          href="/docs/wallets/swarm"
          className="text-sm text-slate-400 hover:text-blue-400 transition-colors"
        >
          Swarm Wallets &rarr;
        </Link>
      </div>
    </div>
  );
}
