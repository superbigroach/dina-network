import Link from "next/link";

export const metadata = {
  title: "DRC Standards — Dina Network Developer Portal",
  description:
    "Complete reference of all 82 DRC smart contract standards for tokens, DeFi, agents, IoT, and the machine economy.",
};

interface DrcStandard {
  drc: string;
  name: string;
  description: string;
  erc?: string;
  dinaOnly?: boolean;
}

const TOKEN_STANDARDS: DrcStandard[] = [
  { drc: "DRC-1", name: "Fungible Token", description: "Standard fungible token interface with transfer, approve, and allowance", erc: "ERC-20" },
  { drc: "DRC-2", name: "Device Identity", description: "On-chain registry for hardware device Ed25519 keys and attestation", dinaOnly: true },
  { drc: "DRC-4", name: "Permit (Gasless Approval)", description: "Off-chain signed approvals using Ed25519 signatures", erc: "ERC-2612" },
  { drc: "DRC-5", name: "Soulbound Token", description: "Non-transferable tokens bound to a single address for credentials and badges", erc: "ERC-5192" },
  { drc: "DRC-6", name: "NFT", description: "Non-fungible token with ownership tracking and safe transfers", erc: "ERC-721" },
  { drc: "DRC-7", name: "Multi-Token", description: "Both fungible and non-fungible tokens in a single contract", erc: "ERC-1155" },
  { drc: "DRC-8", name: "Token-Bound Account", description: "Smart accounts owned by NFTs, allowing tokens to own assets", erc: "ERC-6551" },
  { drc: "DRC-9", name: "Rental / Lending", description: "Time-limited usage rights for NFTs without transferring ownership", erc: "ERC-4907" },
  { drc: "DRC-10", name: "Royalties", description: "On-chain royalty payment info for secondary sales", erc: "ERC-2981" },
  { drc: "DRC-11", name: "Semi-Fungible Token", description: "Tokens with both value and slot attributes for structured finance", erc: "ERC-3525" },
  { drc: "DRC-12", name: "Vault (Yield)", description: "Tokenized yield-bearing vault for deposits, lending pools, and staking", erc: "ERC-4626" },
  { drc: "DRC-13", name: "Compliant Token", description: "Transfer restrictions, KYC agents, freeze/unfreeze for regulated securities", erc: "ERC-3643" },
  { drc: "DRC-14", name: "Contract Signature", description: "Standard for smart contracts to validate signatures on their behalf", erc: "ERC-1271" },
  { drc: "DRC-15", name: "Meta-Transactions", description: "Gasless transactions via trusted forwarders for sponsored UX", erc: "ERC-2771" },
  { drc: "DRC-16", name: "Proxy (Upgradeable)", description: "Upgradeable proxy pattern with admin-controlled implementation swaps", erc: "ERC-1967" },
  { drc: "DRC-17", name: "Hooks", description: "Pre/post transfer hooks for custom logic on token send/receive", erc: "ERC-777 (hooks)" },
  { drc: "DRC-18", name: "Scriptable", description: "Attach off-chain script URIs to tokens for programmable behavior", erc: "ERC-5169" },
];

const FINANCIAL_STANDARDS: DrcStandard[] = [
  { drc: "DRC-19", name: "Batch Transfer", description: "Send USDC to up to 100 recipients in one atomic transaction", dinaOnly: true },
  { drc: "DRC-20", name: "Timelock", description: "Time-delayed execution for governance proposals and admin actions", erc: "OpenZeppelin Timelock" },
  { drc: "DRC-21", name: "Multisig Wallet", description: "M-of-N multi-signature wallet requiring threshold approval", erc: "Gnosis Safe" },
  { drc: "DRC-22", name: "Vesting Schedule", description: "Linear and cliff-based token vesting for team and investor allocations", dinaOnly: true },
  { drc: "DRC-23", name: "Staking Pool", description: "Delegated staking with reward distribution and unbonding periods", dinaOnly: true },
  { drc: "DRC-24", name: "Insurance Pool", description: "Pooled insurance fund with claims, premiums, and payout governance", dinaOnly: true },
  { drc: "DRC-25", name: "Escrow", description: "Two-party escrow with arbiter dispute resolution for commerce", dinaOnly: true },
  { drc: "DRC-26", name: "Subscription", description: "Recurring payment authorization with cancel and renewal cycles", dinaOnly: true },
  { drc: "DRC-27", name: "Revenue Split", description: "Automatic revenue distribution across multiple recipients by percentage", dinaOnly: true },
  { drc: "DRC-28", name: "Auction", description: "English and Dutch auction mechanisms for NFTs and token sales", dinaOnly: true },
  { drc: "DRC-29", name: "Atomic Swap", description: "Trustless cross-asset swaps using hash-time-locked contracts", erc: "HTLC" },
  { drc: "DRC-30", name: "Streaming Payment", description: "Continuous per-second payment streams for salaries and subscriptions", erc: "Sablier-like" },
];

const AGENT_STANDARDS: DrcStandard[] = [
  { drc: "DRC-101", name: "Agent Wallet", description: "AI agent-owned wallet with spending limits, daily caps, and allowlists", dinaOnly: true },
  { drc: "DRC-102", name: "Capability Token", description: "Delegated permission tokens with expiry for agent access control", dinaOnly: true },
  { drc: "DRC-103", name: "Service Agreement", description: "Machine-to-machine SLA contracts with delivery proofs and penalties", dinaOnly: true },
  { drc: "DRC-104", name: "Swarm Coordination", description: "Multi-agent task assignment, completion tracking, and reward distribution", dinaOnly: true },
  { drc: "DRC-105", name: "Sensor Attestation", description: "IoT sensor data authenticity proofs with Ed25519 signed readings", dinaOnly: true },
  { drc: "DRC-106", name: "Data Market", description: "Marketplace for buying and selling sensor data and AI training datasets", dinaOnly: true },
  { drc: "DRC-107", name: "Reputation", description: "On-chain reputation scoring for agents and devices with decay", dinaOnly: true },
  { drc: "DRC-108", name: "Resource Token", description: "Tokenized compute, bandwidth, and storage resource allocation", dinaOnly: true },
  { drc: "DRC-109", name: "Emergency Stop", description: "Circuit breaker for autonomous systems with instant halt capability", dinaOnly: true },
  { drc: "DRC-110", name: "Firmware Registry", description: "On-chain firmware hashes with version tracking and revocation", dinaOnly: true },
  { drc: "DRC-111", name: "Smart Wallet", description: "Programmable wallet with session keys, social recovery, and batched calls", dinaOnly: true },
  { drc: "DRC-112", name: "View Keys", description: "Selective disclosure for privacy compliance -- grant read-only access to auditors", dinaOnly: true },
  { drc: "DRC-113", name: "Relay Protocol", description: "BLE mesh relay incentive and routing for offline settlement propagation", dinaOnly: true },
];

const MACHINE_ECONOMY_STANDARDS: DrcStandard[] = [
  { drc: "DRC-31", name: "Agent Registry", description: "Global registry of AI agents with capabilities, status, and versioning", dinaOnly: true },
  { drc: "DRC-32", name: "Task Queue", description: "On-chain task queue with priority ordering and agent assignment", dinaOnly: true },
  { drc: "DRC-33", name: "ML Model Registry", description: "Registry for machine learning model hashes, versions, and performance metrics", dinaOnly: true },
  { drc: "DRC-34", name: "Energy Market", description: "Peer-to-peer energy trading between IoT devices and microgrids", dinaOnly: true },
  { drc: "DRC-35", name: "Fleet Management", description: "Coordinated management of robot/vehicle fleets with task routing", dinaOnly: true },
  { drc: "DRC-36", name: "Supply Chain Track", description: "Product provenance tracking from manufacturing to delivery", dinaOnly: true },
  { drc: "DRC-37", name: "Oracle Feed", description: "Decentralized data oracle with multi-source aggregation and staking", dinaOnly: true },
  { drc: "DRC-38", name: "Compute Market", description: "Marketplace for GPU/CPU compute time with proof-of-execution", dinaOnly: true },
  { drc: "DRC-39", name: "Bandwidth Market", description: "Tokenized network bandwidth allocation and trading", dinaOnly: true },
  { drc: "DRC-40", name: "Storage Market", description: "Decentralized storage allocation with proof-of-storage verification", dinaOnly: true },
  { drc: "DRC-41", name: "Inference Market", description: "Marketplace for AI inference requests with quality-of-service guarantees", dinaOnly: true },
  { drc: "DRC-42", name: "Robotics Task", description: "Structured physical task definitions for robotic agents with completion proofs", dinaOnly: true },
];

const ADVANCED_STANDARDS: DrcStandard[] = [
  { drc: "DRC-43", name: "Payable Token", description: "Token with built-in payment streaming and automatic USDC conversion", dinaOnly: true },
  { drc: "DRC-44", name: "Flash Loan", description: "Uncollateralized single-transaction loans for arbitrage and liquidation", erc: "Aave Flash Loan" },
  { drc: "DRC-45", name: "Liquidity Pool", description: "Constant-product AMM for token pair swaps with fee accrual", erc: "Uniswap V2" },
  { drc: "DRC-46", name: "Concentrated Liquidity", description: "Range-bound liquidity provision for capital-efficient market making", erc: "Uniswap V3" },
  { drc: "DRC-47", name: "Lending Protocol", description: "Collateralized lending with variable interest rates and liquidation", erc: "Aave/Compound" },
  { drc: "DRC-48", name: "Governance", description: "Token-weighted voting with proposal lifecycle and execution", erc: "Governor Bravo" },
  { drc: "DRC-49", name: "Delegation", description: "Vote delegation with split delegation and expiring delegations", erc: "ERC-5805" },
  { drc: "DRC-50", name: "DAO Treasury", description: "DAO-controlled treasury with proposal-based spending and budgets", dinaOnly: true },
  { drc: "DRC-51", name: "Bonding Curve", description: "Algorithmic pricing curve for token issuance and buy/sell mechanics", dinaOnly: true },
  { drc: "DRC-52", name: "Options Contract", description: "On-chain call/put options with settlement and exercise logic", dinaOnly: true },
  { drc: "DRC-53", name: "Perpetual Futures", description: "Perpetual futures contracts with funding rate mechanism", dinaOnly: true },
  { drc: "DRC-54", name: "Prediction Market", description: "Binary and scalar prediction markets with oracle resolution", dinaOnly: true },
  { drc: "DRC-55", name: "Credit Score", description: "On-chain credit scoring based on repayment history and collateral ratio", dinaOnly: true },
  { drc: "DRC-56", name: "Invoice Factoring", description: "Tokenized invoices with discount purchase and maturity settlement", dinaOnly: true },
  { drc: "DRC-57", name: "Cross-Chain Bridge", description: "Lock-mint bridge protocol for USDC transfers to/from other chains", dinaOnly: true },
  { drc: "DRC-58", name: "Wrapped Asset", description: "Wrapped representations of external chain assets on Dina", erc: "WETH-like" },
  { drc: "DRC-59", name: "Name Service", description: "Human-readable name resolution for addresses (dina.name)", erc: "ENS" },
  { drc: "DRC-60", name: "Access Control", description: "Role-based access control with hierarchical role management", erc: "OpenZeppelin AccessControl" },
  { drc: "DRC-61", name: "Timelock Controller", description: "Timelocked admin operations with proposer/executor/canceller roles", erc: "OZ TimelockController" },
  { drc: "DRC-62", name: "Payment Splitter", description: "Immutable payment splitting to multiple payees by shares", erc: "OZ PaymentSplitter" },
  { drc: "DRC-63", name: "Swarm Wallet", description: "Single authority controlling N agent wallets for parallel transaction execution", dinaOnly: true },
  { drc: "DRC-64", name: "Vector Index", description: "On-chain vector embedding storage for AI semantic search", dinaOnly: true },
  { drc: "DRC-65", name: "Knowledge Graph", description: "On-chain knowledge graph with entities, relations, and traversal queries", dinaOnly: true },
  { drc: "DRC-66", name: "Agent Marketplace", description: "Marketplace for discovering, hiring, and reviewing AI agent services", dinaOnly: true },
  { drc: "DRC-67", name: "Data DAO", description: "Collectively-owned data pools with contribution tracking and revenue sharing", dinaOnly: true },
  { drc: "DRC-68", name: "Proof of Work Done", description: "Verifiable proof that a computational task was performed correctly", dinaOnly: true },
  { drc: "DRC-69", name: "Device Mesh", description: "Peer discovery and topology management for device mesh networks", dinaOnly: true },
  { drc: "DRC-70", name: "Geo-Fenced Token", description: "Tokens with geographic usage restrictions enforced via GPS attestation", dinaOnly: true },
  { drc: "DRC-71", name: "Time-Weighted Voting", description: "Voting power weighted by token holding duration", dinaOnly: true },
  { drc: "DRC-72", name: "Quadratic Funding", description: "Matching pool distribution using quadratic funding formula", dinaOnly: true },
  { drc: "DRC-73", name: "Conditional Transfer", description: "USDC transfers that execute only when oracle conditions are met", dinaOnly: true },
  { drc: "DRC-74", name: "Privacy Pool", description: "Shielded transaction pool with compliance-friendly selective disclosure", dinaOnly: true },
  { drc: "DRC-75", name: "ZK Attestation", description: "Zero-knowledge proof verification for private credential assertions", dinaOnly: true },
  { drc: "DRC-76", name: "Composable NFT", description: "NFTs that can contain and equip other NFTs in a parent-child hierarchy", dinaOnly: true },
  { drc: "DRC-77", name: "Dynamic NFT", description: "NFTs with mutable metadata that updates based on on-chain events", dinaOnly: true },
  { drc: "DRC-78", name: "Loyalty Points", description: "Non-transferable loyalty program with earn/redeem mechanics and tiers", dinaOnly: true },
  { drc: "DRC-79", name: "Coupon / Voucher", description: "Single-use or multi-use discount vouchers with expiry and conditions", dinaOnly: true },
  { drc: "DRC-80", name: "Identity Aggregator", description: "Aggregated identity from multiple attestation sources with trust scoring", dinaOnly: true },
  { drc: "DRC-81", name: "Rate Limiter", description: "On-chain rate limiting for contract calls with per-address quotas", dinaOnly: true },
  { drc: "DRC-82", name: "Circuit Breaker", description: "Automatic contract pause when anomalous activity exceeds thresholds", dinaOnly: true },
];

function StandardsTable({ standards, title }: { standards: DrcStandard[]; title: string }) {
  return (
    <div className="mb-12">
      <h2 className="text-2xl font-semibold text-white mb-4">{title}</h2>
      <div className="overflow-x-auto rounded-xl border border-slate-800">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-slate-800 bg-slate-900/80">
              <th className="px-4 py-3 text-left font-medium text-slate-400 w-24">DRC</th>
              <th className="px-4 py-3 text-left font-medium text-slate-400 w-48">Name</th>
              <th className="px-4 py-3 text-left font-medium text-slate-400">Description</th>
              <th className="px-4 py-3 text-left font-medium text-slate-400 w-36">ERC Equivalent</th>
            </tr>
          </thead>
          <tbody>
            {standards.map((s, i) => (
              <tr
                key={s.drc}
                className={i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"}
              >
                <td className="px-4 py-3 font-mono text-blue-300 whitespace-nowrap">
                  {s.drc}
                </td>
                <td className="px-4 py-3 font-medium text-slate-200 whitespace-nowrap">
                  {s.name}
                  {s.dinaOnly && (
                    <span className="ml-2 rounded-full bg-purple-500/20 px-2 py-0.5 text-xs font-medium text-purple-400">
                      Dina-only
                    </span>
                  )}
                </td>
                <td className="px-4 py-3 text-slate-400">{s.description}</td>
                <td className="px-4 py-3 text-slate-500 whitespace-nowrap">
                  {s.erc || "--"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

export default function DrcStandardsPage() {
  const totalStandards =
    TOKEN_STANDARDS.length +
    FINANCIAL_STANDARDS.length +
    AGENT_STANDARDS.length +
    MACHINE_ECONOMY_STANDARDS.length +
    ADVANCED_STANDARDS.length;

  const dinaOnlyCount = [
    ...TOKEN_STANDARDS,
    ...FINANCIAL_STANDARDS,
    ...AGENT_STANDARDS,
    ...MACHINE_ECONOMY_STANDARDS,
    ...ADVANCED_STANDARDS,
  ].filter((s) => s.dinaOnly).length;

  return (
    <div>
      {/* Header */}
      <p className="text-sm font-medium uppercase tracking-wider text-blue-400 mb-3">
        Smart Contracts
      </p>
      <h1 className="text-4xl font-bold tracking-tight text-white mb-4">
        DRC Standards
      </h1>
      <p className="text-lg text-slate-400 max-w-3xl leading-relaxed mb-6">
        The complete library of {totalStandards} DRC (Dina Request for Comments)
        standards. These define the interfaces, behaviors, and conventions for
        smart contracts on Dina Network -- covering tokens, DeFi, AI agents,
        IoT, and the machine economy.
      </p>

      {/* Stats */}
      <div className="grid gap-4 sm:grid-cols-4 mb-12">
        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-5">
          <p className="text-3xl font-bold text-blue-400">{totalStandards}</p>
          <p className="text-xs text-slate-400 mt-1">Total standards</p>
        </div>
        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-5">
          <p className="text-3xl font-bold text-purple-400">{dinaOnlyCount}</p>
          <p className="text-xs text-slate-400 mt-1">Dina-only (novel)</p>
        </div>
        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-5">
          <p className="text-3xl font-bold text-green-400">5</p>
          <p className="text-xs text-slate-400 mt-1">Categories</p>
        </div>
        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-5">
          <p className="text-3xl font-bold text-amber-400">WASM</p>
          <p className="text-xs text-slate-400 mt-1">Runtime target</p>
        </div>
      </div>

      {/* Legend */}
      <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-4 mb-8 flex flex-wrap gap-4 items-center">
        <span className="text-sm text-slate-400">Legend:</span>
        <span className="flex items-center gap-2 text-sm">
          <span className="rounded-full bg-purple-500/20 px-2.5 py-0.5 text-xs font-medium text-purple-400">
            Dina-only
          </span>
          <span className="text-slate-500">Novel standards that do not exist on Ethereum or other chains</span>
        </span>
      </div>

      {/* Category Navigation */}
      <div className="flex flex-wrap gap-2 mb-10">
        {[
          { label: `Token Standards (${TOKEN_STANDARDS.length})`, id: "token" },
          { label: `Financial (${FINANCIAL_STANDARDS.length})`, id: "financial" },
          { label: `Agent Standards (${AGENT_STANDARDS.length})`, id: "agent" },
          { label: `Machine Economy (${MACHINE_ECONOMY_STANDARDS.length})`, id: "machine" },
          { label: `Advanced (${ADVANCED_STANDARDS.length})`, id: "advanced" },
        ].map((cat) => (
          <a
            key={cat.id}
            href={`#${cat.id}`}
            className="rounded-lg border border-slate-800 bg-slate-900/30 px-4 py-2 text-sm text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
          >
            {cat.label}
          </a>
        ))}
      </div>

      {/* Tables */}
      <div id="token">
        <StandardsTable
          standards={TOKEN_STANDARDS}
          title="Token Standards (DRC-1 through DRC-18)"
        />
      </div>

      <div id="financial">
        <StandardsTable
          standards={FINANCIAL_STANDARDS}
          title="Financial Standards (DRC-19 through DRC-30)"
        />
      </div>

      <div id="agent">
        <StandardsTable
          standards={AGENT_STANDARDS}
          title="Agent Standards (DRC-101 through DRC-113)"
        />
      </div>

      <div id="machine">
        <StandardsTable
          standards={MACHINE_ECONOMY_STANDARDS}
          title="Machine Economy (DRC-31 through DRC-42)"
        />
      </div>

      <div id="advanced">
        <StandardsTable
          standards={ADVANCED_STANDARDS}
          title="Advanced Standards (DRC-43 through DRC-82)"
        />
      </div>

      {/* Numbering Scheme */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Numbering Scheme
        </h2>
        <div className="overflow-x-auto rounded-xl border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900/80">
                <th className="px-4 py-3 text-left font-medium text-slate-400">Range</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Category</th>
                <th className="px-4 py-3 text-left font-medium text-slate-400">Description</th>
              </tr>
            </thead>
            <tbody>
              {[
                { range: "DRC 1-18", cat: "Token Standards", desc: "Ports of established ERC standards adapted for WASM/USDC" },
                { range: "DRC 19-30", cat: "Financial", desc: "DeFi primitives: batch transfers, multisig, vesting, escrow, streaming" },
                { range: "DRC 31-42", cat: "Machine Economy", desc: "Agent registry, task queues, compute/energy/storage markets" },
                { range: "DRC 43-82", cat: "Advanced", desc: "Flash loans, AMMs, governance, privacy, AI, dynamic NFTs, identity" },
                { range: "DRC 101-113", cat: "Agent Standards", desc: "Novel standards for AI agents, IoT, privacy, and mesh networking" },
                { range: "DRC 200+", cat: "Community", desc: "Community-proposed standards (open for submission)" },
              ].map((row, i) => (
                <tr
                  key={row.range}
                  className={i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"}
                >
                  <td className="px-4 py-3 font-mono text-blue-300">{row.range}</td>
                  <td className="px-4 py-3 font-medium text-slate-300">{row.cat}</td>
                  <td className="px-4 py-3 text-slate-400">{row.desc}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {/* Composability */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Composability
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          DRC standards are designed to be composed. A single contract can
          implement multiple standards. Common composition patterns:
        </p>
        <div className="space-y-3">
          {[
            { combo: "DRC-1 + DRC-4 + DRC-17", desc: "Fungible token with gasless approvals and lifecycle hooks" },
            { combo: "DRC-6 + DRC-9 + DRC-10", desc: "Rentable NFT with creator royalties" },
            { combo: "DRC-101 + DRC-102 + DRC-107", desc: "Agent wallet with delegated capabilities and reputation" },
            { combo: "DRC-1 + DRC-13 + DRC-112", desc: "Compliant token with auditor view keys" },
            { combo: "DRC-63 + DRC-19", desc: "Swarm wallet with batch transfers (10,000 payments/block)" },
            { combo: "DRC-6 + DRC-76 + DRC-77", desc: "Composable dynamic NFT that evolves based on events" },
            { combo: "DRC-101 + DRC-109 + DRC-31", desc: "Registered agent with emergency stop capability" },
          ].map((item) => (
            <div
              key={item.combo}
              className="rounded-lg border border-slate-800 bg-slate-900/30 p-4"
            >
              <code className="text-sm font-mono text-blue-300">{item.combo}</code>
              <p className="text-sm text-slate-400 mt-1">{item.desc}</p>
            </div>
          ))}
        </div>
      </section>

      {/* Proposing a New DRC */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Propose a New DRC
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Community members can propose new DRC standards starting at DRC-200.
          The process involves:
        </p>
        <div className="space-y-4">
          {[
            { step: "1", title: "Open a GitHub issue", desc: "Title: DRC-XXX: [Standard Name]. Include use case, motivation, and rough interface." },
            { step: "2", title: "Write the specification", desc: "Abstract, motivation, method signatures with types, events, security considerations." },
            { step: "3", title: "Reference implementation", desc: "Create a contract crate at contracts/drcN-name/ with src/lib.rs and tests." },
            { step: "4", title: "Submit pull request", desc: "DRC committee reviews for interface consistency, no overlap, test coverage, and security." },
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

      {/* Next Steps */}
      <div className="mt-10 flex flex-wrap gap-4">
        <Link
          href="/docs/contracts/deploy"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          &larr; Deploy Contract
        </Link>
        <Link
          href="/docs/contracts/wasm"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          WASM Runtime &rarr;
        </Link>
        <Link
          href="/docs/contracts/call"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          Call Contract &rarr;
        </Link>
      </div>
    </div>
  );
}
