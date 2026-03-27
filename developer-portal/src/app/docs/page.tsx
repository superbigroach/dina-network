import Link from "next/link";

const QUICK_LINKS = [
  {
    title: "Wallets",
    description: "Create Ed25519 wallets, agent wallets (DRC-101), and swarm wallets (DRC-63) for multi-agent coordination.",
    href: "/docs/wallets",
    icon: (
      <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
        <path strokeLinecap="round" strokeLinejoin="round" d="M21 12a2.25 2.25 0 00-2.25-2.25H15a3 3 0 11-6 0H5.25A2.25 2.25 0 003 12m18 0v6a2.25 2.25 0 01-2.25 2.25H5.25A2.25 2.25 0 013 18v-6m18 0V9M3 12V9m18 0a2.25 2.25 0 00-2.25-2.25H5.25A2.25 2.25 0 003 9m18 0V6a2.25 2.25 0 00-2.25-2.25H5.25A2.25 2.25 0 003 6v3" />
      </svg>
    ),
  },
  {
    title: "Transactions",
    description: "Send USDC transfers, batch payments (DRC-19), and manage gas fees on a USDC-native chain.",
    href: "/docs/transactions/transfer",
    icon: (
      <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
        <path strokeLinecap="round" strokeLinejoin="round" d="M7.5 21L3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5" />
      </svg>
    ),
  },
  {
    title: "Smart Contracts",
    description: "Deploy and call WASM smart contracts using the 82 DRC standards for tokens, NFTs, DeFi, and more.",
    href: "/docs/contracts/deploy",
    icon: (
      <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
        <path strokeLinecap="round" strokeLinejoin="round" d="M17.25 6.75L22.5 12l-5.25 5.25m-10.5 0L1.5 12l5.25-5.25m7.5-3l-4.5 16.5" />
      </svg>
    ),
  },
  {
    title: "API Reference",
    description: "JSON-RPC, REST, and WebSocket APIs for querying blocks, submitting transactions, and subscribing to events.",
    href: "/docs/api/jsonrpc",
    icon: (
      <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
        <path strokeLinecap="round" strokeLinejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
      </svg>
    ),
  },
  {
    title: "SDKs",
    description: "Official client libraries for JavaScript/TypeScript, Python, Rust, and a full-featured CLI.",
    href: "/docs/sdk/javascript",
    icon: (
      <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
        <path strokeLinecap="round" strokeLinejoin="round" d="M6.75 7.5l3 2.25-3 2.25m4.5 0h3m-9 8.25h13.5A2.25 2.25 0 0021 18V6a2.25 2.25 0 00-2.25-2.25H5.25A2.25 2.25 0 003 6v12a2.25 2.25 0 002.25 2.25z" />
      </svg>
    ),
  },
];

const DIFFERENTIATORS = [
  {
    label: "Parallel Execution",
    detail: "Transactions are processed across independent lanes simultaneously, enabling 100,000+ TPS without sequential bottlenecks.",
  },
  {
    label: "Swarm Wallets",
    detail: "DRC-63 swarm wallets allow multiple AI agents to coordinate on-chain with shared balances, delegated signing, and consensus thresholds.",
  },
  {
    label: "USDC-Native",
    detail: "No volatile gas token. All fees, balances, and contract interactions are denominated in USDC with 6-decimal precision.",
  },
  {
    label: "82 DRC Standards",
    detail: "A comprehensive library of Dina Request for Comment standards covering tokens, NFTs, DeFi, identity, governance, IoT, and AI agent interactions.",
  },
];

export default function DocsOverviewPage() {
  return (
    <div>
      {/* Heading */}
      <h1 className="text-4xl font-bold tracking-tight text-white">
        Documentation
      </h1>
      <p className="mt-4 text-lg leading-relaxed text-slate-300">
        Welcome to the Dina Network developer documentation. Dina is a
        high-performance Layer 1 blockchain purpose-built for AI agents and
        real-world payments. It combines parallel transaction execution,
        USDC-native economics, and 82 DRC smart contract standards to give
        developers everything they need to build the next generation of
        on-chain applications.
      </p>

      {/* Quick Links Grid */}
      <h2 className="mt-12 text-2xl font-semibold text-white">
        Explore the docs
      </h2>
      <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {QUICK_LINKS.map((link) => (
          <Link
            key={link.href}
            href={link.href}
            className="group rounded-xl border border-slate-800 bg-slate-900/50 p-5 transition-all hover:border-blue-500/40 hover:bg-slate-800/60"
          >
            <div className="mb-3 inline-flex rounded-lg bg-blue-600/10 p-2 text-blue-400 group-hover:bg-blue-600/20">
              {link.icon}
            </div>
            <h3 className="text-base font-semibold text-white">
              {link.title}
            </h3>
            <p className="mt-1.5 text-sm leading-relaxed text-slate-400">
              {link.description}
            </p>
          </Link>
        ))}
      </div>

      {/* What makes Dina different */}
      <h2 className="mt-14 text-2xl font-semibold text-white">
        What makes Dina different
      </h2>
      <div className="mt-6 space-y-5">
        {DIFFERENTIATORS.map((d) => (
          <div
            key={d.label}
            className="rounded-xl border border-slate-800 bg-slate-900/40 p-5"
          >
            <h3 className="text-base font-semibold text-blue-400">
              {d.label}
            </h3>
            <p className="mt-1.5 text-sm leading-relaxed text-slate-300">
              {d.detail}
            </p>
          </div>
        ))}
      </div>

      {/* CTA */}
      <div className="mt-12 rounded-xl border border-slate-800 bg-gradient-to-r from-blue-600/10 to-purple-600/10 p-6">
        <h3 className="text-lg font-semibold text-white">Ready to build?</h3>
        <p className="mt-1 text-sm text-slate-300">
          Follow the quickstart guide to create your first wallet and send a
          transaction in under five minutes.
        </p>
        <Link
          href="/docs/quickstart"
          className="mt-4 inline-flex items-center gap-2 rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-blue-500"
        >
          Get started
          <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M13.5 4.5L21 12m0 0l-7.5 7.5M21 12H3" />
          </svg>
        </Link>
      </div>
    </div>
  );
}
