import Link from "next/link";
import {
  Zap,
  Shield,
  Wallet,
  ArrowRight,
  Code,
  Layers,
  Clock,
  DollarSign,
  Cpu,
  Bot,
  FileCode2,
  Radio,
} from "lucide-react";

const STATS = [
  { value: "100,000+", label: "TPS", icon: Zap },
  { value: "100ms", label: "Finality", icon: Clock },
  { value: "82", label: "DRC Standards", icon: FileCode2 },
  { value: "$0.001", label: "Fees", icon: DollarSign },
];

const FEATURES = [
  {
    icon: Cpu,
    title: "Parallel Execution",
    description:
      "Block-STM lane-based processing scales with CPU cores",
    gradient: "from-blue-500 to-cyan-400",
  },
  {
    icon: Layers,
    title: "Swarm Wallets",
    description:
      "100 parallel wallets per user. No other chain has this.",
    gradient: "from-purple-500 to-pink-400",
  },
  {
    icon: DollarSign,
    title: "USDC Native",
    description:
      "No gas token. Pay fees in USDC directly.",
    gradient: "from-green-500 to-emerald-400",
  },
  {
    icon: Radio,
    title: "Payment Channels",
    description:
      "5ms offline transactions, settle on-chain in batches",
    gradient: "from-orange-500 to-amber-400",
  },
  {
    icon: Bot,
    title: "Agent Wallets",
    description:
      "DRC-101 autonomous AI agent wallets with spending limits",
    gradient: "from-violet-500 to-fuchsia-400",
  },
  {
    icon: FileCode2,
    title: "82 DRC Standards",
    description:
      "More built-in contracts than any blockchain",
    gradient: "from-rose-500 to-red-400",
  },
];

const CODE_EXAMPLE = `import { DinaWallet, DinaClient } from 'dina-js';

const wallet = DinaWallet.generate();
const client = new DinaClient('https://rpc.dina.network');
const balance = await client.getBalance(wallet.address);`;

const TRUSTED_BY = ["Cognitum Seed devices", "Lucilla Health App"];

const FOOTER_LINKS = [
  {
    heading: "Developers",
    links: [
      { label: "Documentation", href: "/docs" },
      { label: "API Reference", href: "/docs/api/jsonrpc" },
      { label: "SDKs", href: "/docs/sdk/javascript" },
      { label: "CLI", href: "/docs/sdk/cli" },
    ],
  },
  {
    heading: "Network",
    links: [
      { label: "Explorer", href: "/explorer" },
      { label: "Faucet", href: "/faucet" },
      { label: "Validators", href: "/docs/validators" },
      { label: "Status", href: "/docs/network" },
    ],
  },
  {
    heading: "Resources",
    links: [
      { label: "GitHub", href: "https://github.com/superbigroach/dina-network" },
      { label: "Architecture", href: "/docs/architecture" },
      { label: "DRC Standards", href: "/docs/contracts/standards" },
      { label: "Benchmarks", href: "/docs/parallel/benchmarks" },
    ],
  },
];

export default function HomePage() {
  return (
    <div className="relative overflow-hidden">
      {/* Background gradient orbs */}
      <div className="pointer-events-none absolute inset-0">
        <div className="absolute -top-40 left-1/4 h-[500px] w-[500px] rounded-full bg-blue-600/10 blur-[120px]" />
        <div className="absolute -top-20 right-1/4 h-[400px] w-[400px] rounded-full bg-purple-600/10 blur-[120px]" />
      </div>

      {/* ===== Hero Section ===== */}
      <section className="relative mx-auto max-w-7xl px-6 pt-24 pb-20 text-center lg:pt-36 lg:pb-32">
        <div className="mb-6 inline-flex items-center gap-2 rounded-full border border-slate-700/60 bg-slate-900/60 px-4 py-1.5 text-sm text-slate-300 backdrop-blur-sm">
          <Zap className="h-3.5 w-3.5 text-blue-400" />
          Built in Rust. Proven on mainnet.
        </div>

        <h1 className="mx-auto max-w-4xl text-5xl font-extrabold leading-[1.1] tracking-tight sm:text-6xl lg:text-7xl">
          <span className="bg-gradient-to-r from-white via-blue-100 to-white bg-clip-text text-transparent">
            The Fastest Blockchain
          </span>
          <br />
          <span className="bg-gradient-to-r from-blue-400 to-purple-500 bg-clip-text text-transparent">
            Ever Built
          </span>
        </h1>

        <p className="mx-auto mt-6 max-w-2xl text-lg leading-relaxed text-slate-400 sm:text-xl">
          100,000+ TPS. 100ms finality. USDC-native. 82 smart contract
          standards.
        </p>

        {/* CTA Buttons */}
        <div className="mt-10 flex items-center justify-center gap-4">
          <Link
            href="/docs"
            className="group inline-flex items-center gap-2 rounded-xl bg-gradient-to-r from-blue-600 to-purple-600 px-6 py-3.5 text-sm font-semibold text-white shadow-lg shadow-blue-600/25 transition-all hover:shadow-blue-600/40 hover:brightness-110"
          >
            Get Started
            <ArrowRight className="h-4 w-4 transition-transform group-hover:translate-x-0.5" />
          </Link>
          <Link
            href="/wallets"
            className="inline-flex items-center gap-2 rounded-xl border border-slate-700 bg-slate-900/60 px-6 py-3.5 text-sm font-semibold text-slate-200 backdrop-blur-sm transition-all hover:border-slate-600 hover:bg-slate-800/60"
          >
            <Wallet className="h-4 w-4" />
            Create Wallet
          </Link>
        </div>
      </section>

      {/* ===== Stats Grid ===== */}
      <section className="relative mx-auto max-w-5xl px-6 pb-24">
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
          {STATS.map((stat) => {
            const Icon = stat.icon;
            return (
              <div
                key={stat.label}
                className="group rounded-2xl border border-slate-800/60 bg-slate-900/40 p-6 text-center backdrop-blur-sm transition-colors hover:border-slate-700/60 hover:bg-slate-800/40"
              >
                <Icon className="mx-auto mb-3 h-5 w-5 text-blue-400 transition-colors group-hover:text-blue-300" />
                <div className="text-2xl font-bold tracking-tight sm:text-3xl">
                  {stat.value}
                </div>
                <div className="mt-1 text-sm text-slate-400">{stat.label}</div>
              </div>
            );
          })}
        </div>
      </section>

      {/* ===== Feature Cards ===== */}
      <section className="relative mx-auto max-w-7xl px-6 pb-28">
        <div className="mb-14 text-center">
          <h2 className="text-3xl font-bold tracking-tight sm:text-4xl">
            Built for the next era of apps
          </h2>
          <p className="mt-3 text-slate-400">
            Everything developers need, baked into the protocol.
          </p>
        </div>

        <div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
          {FEATURES.map((f) => {
            const Icon = f.icon;
            return (
              <div
                key={f.title}
                className="group relative rounded-2xl border border-slate-800/60 bg-slate-900/30 p-7 backdrop-blur-sm transition-all hover:border-slate-700/50 hover:bg-slate-800/30"
              >
                {/* Glow on hover */}
                <div
                  className={`absolute -inset-px rounded-2xl bg-gradient-to-br ${f.gradient} opacity-0 blur-xl transition-opacity group-hover:opacity-[0.07]`}
                />
                <div className="relative">
                  <div
                    className={`mb-4 inline-flex rounded-xl bg-gradient-to-br ${f.gradient} p-2.5 shadow-lg`}
                  >
                    <Icon className="h-5 w-5 text-white" />
                  </div>
                  <h3 className="text-lg font-semibold">{f.title}</h3>
                  <p className="mt-2 text-sm leading-relaxed text-slate-400">
                    {f.description}
                  </p>
                </div>
              </div>
            );
          })}
        </div>
      </section>

      {/* ===== Code Preview ===== */}
      <section className="relative mx-auto max-w-4xl px-6 pb-28">
        <div className="mb-10 text-center">
          <h2 className="text-3xl font-bold tracking-tight sm:text-4xl">
            Start building in seconds
          </h2>
          <p className="mt-3 text-slate-400">
            Create a wallet and query the chain in three lines.
          </p>
        </div>

        <div className="overflow-hidden rounded-2xl border border-slate-800/60 bg-slate-900/60 shadow-2xl shadow-blue-900/10">
          {/* Tab bar */}
          <div className="flex items-center gap-2 border-b border-slate-800/60 px-5 py-3">
            <div className="flex gap-1.5">
              <span className="h-3 w-3 rounded-full bg-slate-700" />
              <span className="h-3 w-3 rounded-full bg-slate-700" />
              <span className="h-3 w-3 rounded-full bg-slate-700" />
            </div>
            <div className="ml-3 flex items-center gap-1.5 text-xs text-slate-500">
              <Code className="h-3.5 w-3.5" />
              quickstart.ts
            </div>
          </div>
          <pre className="!m-0 !rounded-none !border-0 !bg-transparent px-6 py-5 text-sm leading-relaxed">
            <code className="text-slate-300">{CODE_EXAMPLE}</code>
          </pre>
        </div>
      </section>

      {/* ===== Trusted By ===== */}
      <section className="relative mx-auto max-w-4xl px-6 pb-28 text-center">
        <p className="mb-6 text-xs font-semibold uppercase tracking-widest text-slate-500">
          Trusted by
        </p>
        <div className="flex flex-wrap items-center justify-center gap-8">
          {TRUSTED_BY.map((name) => (
            <div
              key={name}
              className="flex items-center gap-2.5 rounded-xl border border-slate-800/50 bg-slate-900/40 px-6 py-3 text-sm font-medium text-slate-300"
            >
              <Shield className="h-4 w-4 text-blue-400" />
              {name}
            </div>
          ))}
        </div>
      </section>

      {/* ===== Footer ===== */}
      <footer className="border-t border-slate-800/60 bg-slate-950">
        <div className="mx-auto max-w-7xl px-6 py-16">
          <div className="grid gap-10 sm:grid-cols-2 lg:grid-cols-4">
            {/* Brand column */}
            <div>
              <div className="flex items-center gap-2.5">
                <div className="h-7 w-7 rounded-lg bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-xs font-bold">
                  D
                </div>
                <span className="text-base font-semibold">Dina Network</span>
              </div>
              <p className="mt-3 text-sm leading-relaxed text-slate-500">
                The fastest USDC-native blockchain. Built for real-world
                payments, AI agents, and health data.
              </p>
            </div>

            {/* Link columns */}
            {FOOTER_LINKS.map((col) => (
              <div key={col.heading}>
                <h4 className="text-sm font-semibold text-slate-300">
                  {col.heading}
                </h4>
                <ul className="mt-3 space-y-2">
                  {col.links.map((link) => (
                    <li key={link.label}>
                      <Link
                        href={link.href}
                        className="text-sm text-slate-500 transition-colors hover:text-slate-300"
                      >
                        {link.label}
                      </Link>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>

          <div className="mt-14 border-t border-slate-800/60 pt-6 text-center text-xs text-slate-600">
            &copy; {new Date().getFullYear()} Dina Network. All rights reserved.
          </div>
        </div>
      </footer>
    </div>
  );
}
