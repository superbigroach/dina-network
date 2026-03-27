import Link from "next/link";

export const metadata = {
  title: "Wallets — Dina Network Developer Portal",
  description:
    "Overview of Standard, Agent (DRC-101), and Swarm (DRC-63) wallet types on the Dina Network.",
};

const WALLET_TYPES = [
  {
    name: "Standard Wallet",
    href: "/docs/wallets/create",
    badge: "Core",
    badgeColor: "bg-blue-500/20 text-blue-400",
    description:
      "A regular non-custodial wallet for sending, receiving, and holding DINA and USDC. Secured by a single Ed25519 key pair. Ideal for end users, developers, and basic dApp interactions.",
    features: [
      "Single Ed25519 key pair",
      "BIP-39 mnemonic backup",
      "HD derivation support",
      "Sign transactions directly",
      "Non-custodial — you own the keys",
    ],
  },
  {
    name: "Agent Wallet",
    href: "/docs/wallets/agent",
    badge: "DRC-101",
    badgeColor: "bg-amber-500/20 text-amber-400",
    description:
      "An autonomous wallet designed for AI agents and automated services. The owner sets spending limits per transaction and per day, and the agent operates freely within those constraints — no human approval needed for each action.",
    features: [
      "Owner-defined spending limits",
      "Daily and per-transaction caps",
      "Allowlisted contract interactions",
      "Instant revocation by owner",
      "Perfect for AI agents and IoT",
    ],
  },
  {
    name: "Swarm Wallet",
    href: "/docs/wallets/swarm",
    badge: "DRC-63",
    badgeColor: "bg-purple-500/20 text-purple-400",
    description:
      "The killer feature. A single authority controls N agent wallets that execute transactions in parallel, bypassing the sequential nonce bottleneck that limits every other blockchain. 100 agents means 100x throughput.",
    features: [
      "1 authority controls N agents",
      "Parallel transaction execution",
      "100x throughput multiplier",
      "Batch payment processing",
      "On-chain payroll and airdrops",
    ],
  },
];

const COMPARISON_ROWS = [
  { label: "Signing", standard: "Single key", agent: "Delegated key", swarm: "N delegated keys" },
  { label: "Parallelism", standard: "1 tx / block", agent: "1 tx / block", swarm: "N tx / block" },
  { label: "Spending limits", standard: "None (full access)", agent: "Daily + per-tx caps", swarm: "Per-agent caps" },
  { label: "Owner control", standard: "Self", agent: "External owner", swarm: "Authority wallet" },
  { label: "Revocation", standard: "N/A", agent: "Instant by owner", swarm: "Instant by authority" },
  { label: "Use case", standard: "End users, devs", agent: "AI bots, IoT", swarm: "Mass payments, airdrops" },
  { label: "DRC standard", standard: "--", agent: "DRC-101", swarm: "DRC-63" },
  { label: "Mnemonic backup", standard: "Yes", agent: "No (derived)", swarm: "No (derived)" },
  { label: "Key type", standard: "Ed25519", agent: "Ed25519", swarm: "Ed25519" },
];

export default function WalletsPage() {
  return (
    <div className="mx-auto max-w-5xl px-6 py-16">
      {/* Header */}
      <div className="mb-12">
        <p className="text-sm font-medium uppercase tracking-wider text-blue-400 mb-3">
          Documentation
        </p>
        <h1 className="text-4xl font-bold tracking-tight mb-4">Wallets</h1>
        <p className="text-lg text-slate-400 max-w-3xl leading-relaxed">
          Dina Network provides three wallet types — each designed for a different
          level of autonomy and throughput. All wallets use Ed25519 cryptography
          and are fully non-custodial: your keys, your coins.
        </p>
      </div>

      {/* Security callout */}
      <div className="rounded-xl border border-blue-500/30 bg-blue-500/5 p-6 mb-12">
        <h3 className="text-sm font-semibold text-blue-400 mb-2">Security Model</h3>
        <p className="text-slate-300 text-sm leading-relaxed">
          Every wallet on Dina Network uses <strong>Ed25519</strong> key pairs — the same
          algorithm used by Solana, Cosmos, and SSH. Keys are generated client-side
          and never leave your device. Dina Network never has access to your
          private keys. Addresses are derived as{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
            0x + SHA-256(pubkey)
          </code>{" "}
          producing a 66-character hex string.
        </p>
      </div>

      {/* Wallet type cards */}
      <div className="grid gap-6 md:grid-cols-3 mb-16">
        {WALLET_TYPES.map((wallet) => (
          <Link
            key={wallet.name}
            href={wallet.href}
            className="group rounded-xl border border-slate-800 bg-slate-900/50 p-6 transition-all hover:border-slate-700 hover:bg-slate-900"
          >
            <div className="flex items-center gap-3 mb-3">
              <h2 className="text-lg font-semibold group-hover:text-white transition-colors">
                {wallet.name}
              </h2>
              <span
                className={`rounded-full px-2.5 py-0.5 text-xs font-medium ${wallet.badgeColor}`}
              >
                {wallet.badge}
              </span>
            </div>
            <p className="text-sm text-slate-400 leading-relaxed mb-4">
              {wallet.description}
            </p>
            <ul className="space-y-1.5">
              {wallet.features.map((f) => (
                <li
                  key={f}
                  className="flex items-start gap-2 text-sm text-slate-300"
                >
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-blue-500" />
                  {f}
                </li>
              ))}
            </ul>
          </Link>
        ))}
      </div>

      {/* Comparison table */}
      <div className="mb-16">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Comparison Table
        </h2>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">
                  Feature
                </th>
                <th className="px-4 py-3 text-left font-medium text-blue-400">
                  Standard
                </th>
                <th className="px-4 py-3 text-left font-medium text-amber-400">
                  Agent (DRC-101)
                </th>
                <th className="px-4 py-3 text-left font-medium text-purple-400">
                  Swarm (DRC-63)
                </th>
              </tr>
            </thead>
            <tbody>
              {COMPARISON_ROWS.map((row, i) => (
                <tr
                  key={row.label}
                  className={
                    i % 2 === 0
                      ? "bg-slate-950/50"
                      : "bg-slate-900/30"
                  }
                >
                  <td className="px-4 py-3 font-medium text-slate-300">
                    {row.label}
                  </td>
                  <td className="px-4 py-3 text-slate-400">{row.standard}</td>
                  <td className="px-4 py-3 text-slate-400">{row.agent}</td>
                  <td className="px-4 py-3 text-slate-400">{row.swarm}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* When to use each */}
      <div className="mb-16">
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          When to Use Each Wallet Type
        </h2>
        <div className="space-y-6">
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <h3 className="text-lg font-semibold text-blue-400 mb-2">
              Standard Wallet
            </h3>
            <p className="text-sm text-slate-400 leading-relaxed">
              Use a standard wallet when a human is directly signing transactions.
              This is the default for end users interacting with dApps, developers
              testing on devnet, or any scenario where one person controls one
              wallet. Supports BIP-39 mnemonic backup for recovery.
            </p>
          </div>
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <h3 className="text-lg font-semibold text-amber-400 mb-2">
              Agent Wallet (DRC-101)
            </h3>
            <p className="text-sm text-slate-400 leading-relaxed">
              Use an agent wallet when you need an autonomous process to spend
              funds without human approval on every transaction. The owner sets
              guardrails (daily limits, per-tx limits, allowed contracts) and the
              agent operates freely within those bounds. Perfect for AI trading
              bots, IoT devices making micropayments, or automated subscription
              services.
            </p>
          </div>
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
            <h3 className="text-lg font-semibold text-purple-400 mb-2">
              Swarm Wallet (DRC-63)
            </h3>
            <p className="text-sm text-slate-400 leading-relaxed">
              Use a swarm wallet when you need massive parallelism. Every
              blockchain is bottlenecked by sequential nonce-based transactions
              per account. Swarm wallets break through this by giving you N agent
              wallets that each process transactions independently, in parallel.
              100 agents = 100x throughput in a single block. Use this for payroll,
              airdrops, exchange settlement, or any high-volume payment scenario.
            </p>
          </div>
        </div>
      </div>

      {/* Quick links */}
      <div>
        <h2 className="text-2xl font-bold tracking-tight mb-6">
          Wallet Guides
        </h2>
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {[
            { label: "Create a Wallet", href: "/docs/wallets/create", desc: "Generate, import, and manage wallets" },
            { label: "Agent Wallets", href: "/docs/wallets/agent", desc: "Autonomous AI agent wallets (DRC-101)" },
            { label: "Swarm Wallets", href: "/docs/wallets/swarm", desc: "Parallel transaction execution (DRC-63)" },
            { label: "Key Management", href: "/docs/wallets/keys", desc: "Ed25519 keys, encryption, rotation" },
            { label: "HD Wallets", href: "/docs/wallets/hd", desc: "BIP-39 mnemonics and derivation paths" },
          ].map((link) => (
            <Link
              key={link.href}
              href={link.href}
              className="group rounded-lg border border-slate-800 bg-slate-900/30 p-4 transition-all hover:border-blue-500/40 hover:bg-slate-900/60"
            >
              <h3 className="font-semibold text-sm group-hover:text-blue-400 transition-colors mb-1">
                {link.label}
              </h3>
              <p className="text-xs text-slate-500">{link.desc}</p>
            </Link>
          ))}
        </div>
      </div>
    </div>
  );
}
