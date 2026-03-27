export default function ComparePage() {
  return (
    <div className="max-w-none">
      {/* ------------------------------------------------------------------ */}
      {/* Hero Section */}
      {/* ------------------------------------------------------------------ */}
      <div className="text-center">
        <h1 className="text-5xl font-bold tracking-tight text-white">
          How Dina Compares
        </h1>
        <p className="mx-auto mt-4 max-w-2xl text-lg text-slate-300">
          An honest comparison with every major blockchain. No cherry-picked
          benchmarks &mdash; just real-world numbers and architectural
          differences.
        </p>
      </div>

      {/* ------------------------------------------------------------------ */}
      {/* Main Comparison Table */}
      {/* ------------------------------------------------------------------ */}
      <h2 className="mt-16 text-3xl font-semibold text-white">
        Full comparison table
      </h2>
      <p className="mt-2 text-sm text-slate-400">
        Scroll horizontally to see all columns. TPS figures are real-world
        sustained throughput, not theoretical maximums.
      </p>

      <div className="mt-6 overflow-x-auto rounded-xl border border-slate-800">
        <table className="w-full min-w-[1200px] text-left text-sm">
          <thead className="sticky top-0 z-10 border-b border-slate-700 bg-slate-900">
            <tr>
              {[
                "Chain",
                "Language",
                "VM",
                "Consensus",
                "TPS (real)",
                "Finality",
                "Gas Token",
                "Standards",
                "Validators",
                "Per-User Parallel Txs",
                "Launched",
              ].map((h) => (
                <th
                  key={h}
                  className="whitespace-nowrap px-4 py-3 text-xs font-semibold uppercase tracking-wider text-slate-400"
                >
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800/60">
            {CHAINS.map((c) => (
              <tr
                key={c.name}
                className={
                  c.name === "Dina"
                    ? "bg-blue-950/30"
                    : "bg-slate-950/40 hover:bg-slate-900/50"
                }
              >
                <td className="whitespace-nowrap px-4 py-3">
                  <span
                    className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold ${c.badge}`}
                  >
                    {c.name}
                  </span>
                </td>
                <td className="px-4 py-3 text-slate-300">{c.language}</td>
                <td className="px-4 py-3 text-slate-300">{c.vm}</td>
                <td className="px-4 py-3 text-slate-300">{c.consensus}</td>
                <td className="px-4 py-3 font-mono text-slate-200">{c.tps}</td>
                <td className="px-4 py-3 text-slate-300">{c.finality}</td>
                <td className="px-4 py-3 text-slate-300">{c.gasToken}</td>
                <td className="px-4 py-3 text-center text-slate-300">
                  {c.standards}
                </td>
                <td className="px-4 py-3 text-center text-slate-300">
                  {c.validators}
                </td>
                <td className="px-4 py-3 text-center">
                  {c.parallelTxs ? (
                    <span className="rounded-full bg-green-900/50 px-2 py-0.5 text-xs font-semibold text-green-400">
                      {c.parallelTxs}
                    </span>
                  ) : (
                    <span className="text-slate-600">&mdash;</span>
                  )}
                </td>
                <td className="px-4 py-3 text-slate-400">{c.launched}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* ------------------------------------------------------------------ */}
      {/* Why Dina Wins — vs EVM Chains */}
      {/* ------------------------------------------------------------------ */}
      <h2 className="mt-20 text-3xl font-semibold text-white">
        Why Dina wins
      </h2>

      <div className="mt-8 rounded-xl border border-slate-800 bg-slate-900/40 p-6">
        <h3 className="flex items-center gap-3 text-xl font-semibold text-white">
          <span className="rounded-full bg-indigo-900/60 px-3 py-1 text-sm text-indigo-300">
            vs EVM Chains
          </span>
          Ethereum, Base, Optimism, Arbitrum, Polygon
        </h3>

        <div className="mt-4 space-y-3 text-sm leading-relaxed text-slate-300">
          <p>
            <strong className="text-slate-100">EVM limitations:</strong>{" "}
            The Ethereum Virtual Machine processes transactions sequentially.
            Ethereum L1 sustains roughly 15&ndash;30 TPS. Even the fastest
            EVM L2s (Base, Arbitrum, Optimism) max out around 100&ndash;200
            TPS in practice because they inherit the EVM&rsquo;s sequential
            execution model.
          </p>
          <p>
            <strong className="text-slate-100">L2 trade-offs:</strong>{" "}
            L2 rollups like Base and Optimism rely on a single sequencer,
            creating a centralization bottleneck. Optimistic rollups impose
            a 7-day challenge window for withdrawals back to L1. Users still
            need ETH for L1 gas when bridging, and each L2 requires a
            separate bridge &mdash; fragmenting liquidity across chains.
          </p>
          <p>
            <strong className="text-slate-100">
              Dina&rsquo;s advantages:
            </strong>{" "}
            WASM is faster than the EVM and compiles from any language (Rust,
            C, Go, AssemblyScript). USDC is the native gas token &mdash; no
            volatile gas token to manage. 100ms BFT finality means
            transactions are irreversible immediately, not after a 7-day
            optimistic window. Parallel execution lanes deliver 10,000+ TPS
            on a single chain with no sequencer bottleneck.
          </p>
        </div>
      </div>

      {/* vs Rust/Move Chains */}
      <div className="mt-6 rounded-xl border border-slate-800 bg-slate-900/40 p-6">
        <h3 className="flex items-center gap-3 text-xl font-semibold text-white">
          <span className="rounded-full bg-orange-900/60 px-3 py-1 text-sm text-orange-300">
            vs Rust / Move Chains
          </span>
          Solana, Sui, Aptos
        </h3>

        <div className="mt-4 space-y-3 text-sm leading-relaxed text-slate-300">
          <p>
            <strong className="text-slate-100">Solana:</strong>{" "}
            Advertised at 65,000 TPS, but real-world throughput is closer to
            4,000 TPS &mdash; and roughly 80% of those are validator vote
            transactions, not user activity. Solana&rsquo;s ~1,900 validators
            slow consensus rounds and have contributed to multiple network
            outages. Its account model requires developers to manually
            declare read/write locks, adding significant complexity.
          </p>
          <p>
            <strong className="text-slate-100">Sui:</strong>{" "}
            Object-based parallelism is a smart design choice, but the Move
            language has a small ecosystem (~10 standards). Sui achieves
            ~10,000 TPS in benchmarks but requires developers to reason
            about object ownership for every transaction.
          </p>
          <p>
            <strong className="text-slate-100">Aptos:</strong>{" "}
            Uses Block-STM for speculative parallel execution &mdash; Dina
            uses this same technique via its lane-based executor. Aptos
            reaches ~10,000 TPS but uses the Move language with a similarly
            small standard set. Neither Sui nor Aptos has a stablecoin
            as native gas.
          </p>
          <p>
            <strong className="text-slate-100">
              Dina&rsquo;s advantages:
            </strong>{" "}
            Swarm Wallets (DRC-63) enable per-user parallelism &mdash;
            something no other chain offers. Fewer validators (3&ndash;7)
            means faster BFT rounds without sacrificing security for the
            target use case. USDC-native gas eliminates token volatility.
            82 DRC standards vs their ~10&ndash;20.
          </p>
        </div>
      </div>

      {/* vs NEAR */}
      <div className="mt-6 rounded-xl border border-slate-800 bg-slate-900/40 p-6">
        <h3 className="flex items-center gap-3 text-xl font-semibold text-white">
          <span className="rounded-full bg-teal-900/60 px-3 py-1 text-sm text-teal-300">
            vs NEAR
          </span>
          NEAR Protocol
        </h3>

        <div className="mt-4 space-y-3 text-sm leading-relaxed text-slate-300">
          <p>
            NEAR uses dynamic sharding (Nightshade) with Rust-based smart
            contracts compiled to WASM &mdash; architecturally similar to
            Dina in several ways. NEAR achieves ~1,000 TPS per shard and
            has strong developer experience with named accounts and
            human-readable addresses.
          </p>
          <p>
            <strong className="text-slate-100">
              Dina&rsquo;s advantages:
            </strong>{" "}
            Single-chain simplicity avoids the cross-shard communication
            overhead that adds latency on NEAR. Dina achieves higher
            per-chain throughput through Block-STM lanes. Swarm Wallets
            allow a single user to run thousands of parallel transactions
            without account contention. NEAR uses NEAR token for gas; Dina
            uses USDC natively.
          </p>
        </div>
      </div>

      {/* ------------------------------------------------------------------ */}
      {/* Parallel Execution Comparison */}
      {/* ------------------------------------------------------------------ */}
      <h2 className="mt-16 text-2xl font-semibold text-white">
        Parallel execution comparison
      </h2>
      <p className="mt-2 text-sm text-slate-400">
        How each chain handles concurrent transaction processing.
      </p>

      <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {PARALLEL_MODELS.map((m) => (
          <div
            key={m.chain}
            className={`rounded-xl border p-5 ${
              m.chain === "Dina"
                ? "border-blue-700/60 bg-blue-950/30"
                : "border-slate-800 bg-slate-900/40"
            }`}
          >
            <div className="flex items-center justify-between">
              <span
                className={`rounded-full px-2.5 py-0.5 text-xs font-semibold ${m.badge}`}
              >
                {m.chain}
              </span>
              <span className="text-xs text-slate-500">{m.model}</span>
            </div>
            <p className="mt-3 text-sm leading-relaxed text-slate-300">
              {m.description}
            </p>
          </div>
        ))}
      </div>

      {/* ------------------------------------------------------------------ */}
      {/* Smart Contract Standards */}
      {/* ------------------------------------------------------------------ */}
      <h2 className="mt-16 text-2xl font-semibold text-white">
        Smart contract standards
      </h2>

      <div className="mt-6 overflow-x-auto rounded-xl border border-slate-800">
        <table className="w-full text-left text-sm">
          <thead className="border-b border-slate-700 bg-slate-900">
            <tr>
              <th className="px-4 py-3 text-xs font-semibold uppercase tracking-wider text-slate-400">
                Chain
              </th>
              <th className="px-4 py-3 text-xs font-semibold uppercase tracking-wider text-slate-400">
                Standards Count
              </th>
              <th className="px-4 py-3 text-xs font-semibold uppercase tracking-wider text-slate-400">
                Notable Standards
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800/60">
            <tr className="bg-slate-950/40">
              <td className="px-4 py-3">
                <span className="rounded-full bg-indigo-900/50 px-2.5 py-0.5 text-xs font-semibold text-indigo-300">
                  Ethereum
                </span>
              </td>
              <td className="px-4 py-3 text-slate-300">~30 widely used</td>
              <td className="px-4 py-3 text-slate-400">
                ERC-20, ERC-721, ERC-1155, ERC-4626, ERC-2981
              </td>
            </tr>
            <tr className="bg-slate-950/40">
              <td className="px-4 py-3">
                <span className="rounded-full bg-purple-900/50 px-2.5 py-0.5 text-xs font-semibold text-purple-300">
                  Solana
                </span>
              </td>
              <td className="px-4 py-3 text-slate-300">~5 SPL standards</td>
              <td className="px-4 py-3 text-slate-400">
                SPL Token, Token-2022, Associated Token Account
              </td>
            </tr>
            <tr className="bg-slate-950/40">
              <td className="px-4 py-3">
                <span className="rounded-full bg-cyan-900/50 px-2.5 py-0.5 text-xs font-semibold text-cyan-300">
                  Sui / Aptos
                </span>
              </td>
              <td className="px-4 py-3 text-slate-300">~10 Move standards</td>
              <td className="px-4 py-3 text-slate-400">
                Coin, NFT, Object ownership, Kiosk
              </td>
            </tr>
            <tr className="bg-blue-950/30">
              <td className="px-4 py-3">
                <span className="rounded-full bg-blue-900/50 px-2.5 py-0.5 text-xs font-semibold text-blue-300">
                  Dina
                </span>
              </td>
              <td className="px-4 py-3 font-semibold text-white">
                82 DRC standards
              </td>
              <td className="px-4 py-3 text-slate-300">
                30 ported from ERC + 52 novel (see below)
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <h3 className="mt-8 text-lg font-semibold text-white">
        Dina&rsquo;s 52 novel DRC categories
      </h3>
      <div className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {NOVEL_CATEGORIES.map((cat) => (
          <div
            key={cat.name}
            className="rounded-lg border border-slate-800 bg-slate-900/40 p-4"
          >
            <div className="text-sm font-semibold text-blue-400">
              {cat.name}
            </div>
            <p className="mt-1 text-xs text-slate-400">{cat.description}</p>
          </div>
        ))}
      </div>

      {/* ------------------------------------------------------------------ */}
      {/* Unique to Dina */}
      {/* ------------------------------------------------------------------ */}
      <h2 className="mt-16 text-2xl font-semibold text-white">
        Things only Dina has
      </h2>
      <p className="mt-2 text-sm text-slate-400">
        Features that exist on no other blockchain today.
      </p>

      <div className="mt-6 space-y-4">
        {UNIQUE_FEATURES.map((f, i) => (
          <div
            key={f.title}
            className="flex gap-4 rounded-xl border border-blue-800/40 bg-blue-950/20 p-5"
          >
            <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-blue-900/60 text-sm font-bold text-blue-300">
              {i + 1}
            </div>
            <div>
              <div className="font-semibold text-white">{f.title}</div>
              <p className="mt-1 text-sm text-slate-300">{f.description}</p>
            </div>
          </div>
        ))}
      </div>

      {/* ------------------------------------------------------------------ */}
      {/* When NOT to Use Dina */}
      {/* ------------------------------------------------------------------ */}
      <h2 className="mt-16 text-2xl font-semibold text-white">
        When not to use Dina
      </h2>
      <p className="mt-2 text-sm text-slate-400">
        Every chain has trade-offs. Here is where Dina is not the right fit.
      </p>

      <div className="mt-6 space-y-4">
        {TRADEOFFS.map((t) => (
          <div
            key={t.scenario}
            className="rounded-xl border border-amber-900/40 bg-amber-950/10 p-5"
          >
            <div className="text-sm font-semibold text-amber-300">
              {t.scenario}
            </div>
            <p className="mt-1 text-sm text-slate-300">{t.recommendation}</p>
          </div>
        ))}
      </div>

      <div className="mt-6 rounded-xl border border-slate-800 bg-slate-900/40 p-5">
        <div className="text-sm font-semibold text-green-400">
          Dina is purpose-built for:
        </div>
        <ul className="mt-2 space-y-1 text-sm text-slate-300">
          <li className="flex items-start gap-2">
            <span className="mt-1 block h-1.5 w-1.5 shrink-0 rounded-full bg-green-500" />
            IoT device networks with hardware-attested transactions
          </li>
          <li className="flex items-start gap-2">
            <span className="mt-1 block h-1.5 w-1.5 shrink-0 rounded-full bg-green-500" />
            Autonomous AI agents with spending limits and accountability
          </li>
          <li className="flex items-start gap-2">
            <span className="mt-1 block h-1.5 w-1.5 shrink-0 rounded-full bg-green-500" />
            Health and wellness apps with real-time micro-payments
          </li>
          <li className="flex items-start gap-2">
            <span className="mt-1 block h-1.5 w-1.5 shrink-0 rounded-full bg-green-500" />
            High-throughput payment systems settling in USDC
          </li>
        </ul>
      </div>

      {/* ------------------------------------------------------------------ */}
      {/* Migration Guide */}
      {/* ------------------------------------------------------------------ */}
      <h2 className="mt-16 text-2xl font-semibold text-white">
        Migration guide
      </h2>
      <p className="mt-2 text-sm text-slate-400">
        Migrating from an existing chain? Here is the high-level path.
      </p>

      <div className="mt-6 space-y-6">
        {/* From EVM */}
        <div className="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
          <h3 className="text-base font-semibold text-indigo-400">
            From EVM chains (Ethereum, Base, Polygon, Arbitrum, Optimism)
          </h3>
          <div className="mt-3 space-y-3 text-sm text-slate-300">
            <div>
              <span className="font-semibold text-slate-100">
                Contracts:
              </span>{" "}
              Rewrite Solidity contracts in Rust targeting WASM. Dina&rsquo;s
              DRC standards mirror many ERCs (DRC-20 = ERC-20, DRC-721 =
              ERC-721, etc.), so the data model maps closely.
            </div>
            <div>
              <span className="font-semibold text-slate-100">SDK:</span>{" "}
              Replace{" "}
              <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">
                ethers.js
              </code>{" "}
              with{" "}
              <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">
                dina-js
              </code>
              . Key mapping:
            </div>
            <div className="overflow-x-auto rounded-lg border border-slate-800 bg-slate-800 p-4">
              <pre className="!m-0 !border-0 !bg-transparent !p-0">
                <code className="text-xs leading-relaxed text-slate-200">
{`// ethers.js                     // dina-js
const provider = new            const client = new
  ethers.JsonRpcProvider(url)      DinaClient(url)

const wallet = new              const wallet =
  ethers.Wallet(pk, provider)      Wallet.fromPrivateKey(pk)

await wallet.sendTransaction    await client.transfer(
  ({ to, value })                  wallet, to, amount)

contract.balanceOf(addr)        client.getBalance(addr)`}
                </code>
              </pre>
            </div>
          </div>
        </div>

        {/* From Solana */}
        <div className="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
          <h3 className="text-base font-semibold text-purple-400">
            From Solana
          </h3>
          <div className="mt-3 space-y-3 text-sm text-slate-300">
            <div>
              <span className="font-semibold text-slate-100">
                Contracts:
              </span>{" "}
              Solana programs are already Rust &mdash; the code ports more
              directly. Replace Anchor&rsquo;s account model with
              Dina&rsquo;s contract storage API. Remove manual account locking;
              Dina&rsquo;s parallel executor handles contention automatically.
            </div>
            <div>
              <span className="font-semibold text-slate-100">SDK:</span>{" "}
              Replace{" "}
              <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">
                @solana/web3.js
              </code>{" "}
              with{" "}
              <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">
                dina-js
              </code>
              . Ed25519 key format is the same. No need to manage associated
              token accounts &mdash; Dina accounts hold USDC natively.
            </div>
          </div>
        </div>

        {/* From Move chains */}
        <div className="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
          <h3 className="text-base font-semibold text-cyan-400">
            From Move chains (Sui, Aptos)
          </h3>
          <div className="mt-3 space-y-3 text-sm text-slate-300">
            <div>
              <span className="font-semibold text-slate-100">
                Contracts:
              </span>{" "}
              Move&rsquo;s resource model maps conceptually to
              Dina&rsquo;s typed contract storage. Rewrite Move modules in
              Rust targeting WASM. Dina handles object ownership at the
              wallet level (Swarm Wallets) rather than the language level.
            </div>
            <div>
              <span className="font-semibold text-slate-100">SDK:</span>{" "}
              Replace Sui/Aptos TypeScript SDK with{" "}
              <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">
                dina-js
              </code>
              . Transaction construction is simpler &mdash; no PTBs
              (Programmable Transaction Blocks) required.
            </div>
          </div>
        </div>
      </div>

      <div className="mt-16 border-t border-slate-800 pt-8 text-center text-sm text-slate-500">
        Data last updated March 2026. TPS figures are based on publicly
        available mainnet data and independent benchmarks.
      </div>
    </div>
  );
}

/* ====================================================================== */
/* Static Data                                                            */
/* ====================================================================== */

interface ChainData {
  name: string;
  badge: string;
  language: string;
  vm: string;
  consensus: string;
  tps: string;
  finality: string;
  gasToken: string;
  standards: string;
  validators: string;
  parallelTxs: string | null;
  launched: string;
}

const CHAINS: ChainData[] = [
  {
    name: "Dina",
    badge: "bg-blue-900/50 text-blue-300 ring-1 ring-blue-700/50",
    language: "Rust / any (WASM)",
    vm: "WASM",
    consensus: "TurboBFT",
    tps: "10,000+",
    finality: "100ms",
    gasToken: "USDC",
    standards: "82 DRC",
    validators: "3-7",
    parallelTxs: "Swarm Wallets",
    launched: "2025",
  },
  {
    name: "Ethereum",
    badge: "bg-indigo-900/50 text-indigo-300",
    language: "Solidity / Vyper",
    vm: "EVM",
    consensus: "PoS (Gasper)",
    tps: "15-30",
    finality: "~13 min",
    gasToken: "ETH",
    standards: "~30 ERCs",
    validators: "~900K",
    parallelTxs: null,
    launched: "2015",
  },
  {
    name: "Base",
    badge: "bg-blue-800/50 text-blue-200",
    language: "Solidity / Vyper",
    vm: "EVM (OP Stack)",
    consensus: "Single sequencer",
    tps: "~100",
    finality: "2s (soft), 7d (L1)",
    gasToken: "ETH",
    standards: "~30 ERCs",
    validators: "1 (sequencer)",
    parallelTxs: null,
    launched: "2023",
  },
  {
    name: "Optimism",
    badge: "bg-red-900/50 text-red-300",
    language: "Solidity / Vyper",
    vm: "EVM (OP Stack)",
    consensus: "Single sequencer",
    tps: "~100",
    finality: "2s (soft), 7d (L1)",
    gasToken: "ETH",
    standards: "~30 ERCs",
    validators: "1 (sequencer)",
    parallelTxs: null,
    launched: "2021",
  },
  {
    name: "Arbitrum",
    badge: "bg-sky-900/50 text-sky-300",
    language: "Solidity / Vyper",
    vm: "EVM (Nitro)",
    consensus: "Single sequencer",
    tps: "~100",
    finality: "~1s (soft), 7d (L1)",
    gasToken: "ETH",
    standards: "~30 ERCs",
    validators: "1 (sequencer)",
    parallelTxs: null,
    launched: "2021",
  },
  {
    name: "Polygon",
    badge: "bg-violet-900/50 text-violet-300",
    language: "Solidity / Vyper",
    vm: "EVM",
    consensus: "PoS + Heimdall",
    tps: "~65",
    finality: "~2 min",
    gasToken: "POL",
    standards: "~30 ERCs",
    validators: "~100",
    parallelTxs: null,
    launched: "2020",
  },
  {
    name: "Avalanche",
    badge: "bg-red-800/50 text-red-200",
    language: "Solidity / Vyper",
    vm: "EVM (C-Chain)",
    consensus: "Snowball",
    tps: "~50",
    finality: "~2s",
    gasToken: "AVAX",
    standards: "~30 ERCs",
    validators: "~1,700",
    parallelTxs: null,
    launched: "2020",
  },
  {
    name: "BNB Chain",
    badge: "bg-yellow-900/50 text-yellow-300",
    language: "Solidity / Vyper",
    vm: "EVM",
    consensus: "PoSA",
    tps: "~150",
    finality: "~3s",
    gasToken: "BNB",
    standards: "~30 ERCs",
    validators: "21",
    parallelTxs: null,
    launched: "2020",
  },
  {
    name: "Solana",
    badge: "bg-purple-900/50 text-purple-300",
    language: "Rust / C",
    vm: "SVM (Sealevel)",
    consensus: "PoS + PoH",
    tps: "~4,000",
    finality: "~400ms",
    gasToken: "SOL",
    standards: "~5 SPL",
    validators: "~1,900",
    parallelTxs: null,
    launched: "2020",
  },
  {
    name: "Sui",
    badge: "bg-cyan-900/50 text-cyan-300",
    language: "Move",
    vm: "MoveVM",
    consensus: "Mysticeti BFT",
    tps: "~10,000",
    finality: "~400ms",
    gasToken: "SUI",
    standards: "~10",
    validators: "~100",
    parallelTxs: null,
    launched: "2023",
  },
  {
    name: "Aptos",
    badge: "bg-emerald-900/50 text-emerald-300",
    language: "Move",
    vm: "MoveVM",
    consensus: "AptosBFT",
    tps: "~10,000",
    finality: "~900ms",
    gasToken: "APT",
    standards: "~10",
    validators: "~120",
    parallelTxs: null,
    launched: "2022",
  },
  {
    name: "NEAR",
    badge: "bg-teal-900/50 text-teal-300",
    language: "Rust / JS",
    vm: "WASM",
    consensus: "Nightshade PoS",
    tps: "~1,000/shard",
    finality: "~2s",
    gasToken: "NEAR",
    standards: "~10 NEPs",
    validators: "~250",
    parallelTxs: null,
    launched: "2020",
  },
  {
    name: "Sei",
    badge: "bg-rose-900/50 text-rose-300",
    language: "Rust / Solidity",
    vm: "EVM + WASM",
    consensus: "Twin-Turbo",
    tps: "~5,000",
    finality: "~400ms",
    gasToken: "SEI",
    standards: "~15",
    validators: "~40",
    parallelTxs: null,
    launched: "2023",
  },
  {
    name: "Bitcoin",
    badge: "bg-amber-900/50 text-amber-300",
    language: "Script",
    vm: "Bitcoin Script",
    consensus: "PoW (Nakamoto)",
    tps: "~7",
    finality: "~60 min",
    gasToken: "BTC",
    standards: "~3",
    validators: "~15K nodes",
    parallelTxs: null,
    launched: "2009",
  },
  {
    name: "Cardano",
    badge: "bg-blue-900/40 text-blue-200",
    language: "Plutus (Haskell)",
    vm: "Plutus VM",
    consensus: "Ouroboros PoS",
    tps: "~250",
    finality: "~5 min",
    gasToken: "ADA",
    standards: "~8 CIPs",
    validators: "~3,200",
    parallelTxs: null,
    launched: "2017",
  },
  {
    name: "Polkadot",
    badge: "bg-pink-900/50 text-pink-300",
    language: "Rust (Substrate)",
    vm: "WASM",
    consensus: "GRANDPA/BABE",
    tps: "~1,000/para",
    finality: "~60s",
    gasToken: "DOT",
    standards: "~10 PSPs",
    validators: "~300",
    parallelTxs: null,
    launched: "2020",
  },
];

interface ParallelModel {
  chain: string;
  badge: string;
  model: string;
  description: string;
}

const PARALLEL_MODELS: ParallelModel[] = [
  {
    chain: "Ethereum",
    badge: "bg-indigo-900/50 text-indigo-300",
    model: "Sequential",
    description:
      "Processes one transaction at a time. Every transaction executes against the global state in order, making throughput fundamentally bottlenecked.",
  },
  {
    chain: "Solana",
    badge: "bg-purple-900/50 text-purple-300",
    model: "Account locking",
    description:
      "Transactions declare which accounts they read/write. Sealevel schedules non-conflicting transactions in parallel. Hot accounts (popular DEXs, oracles) create contention.",
  },
  {
    chain: "Sui",
    badge: "bg-cyan-900/50 text-cyan-300",
    model: "Object ownership",
    description:
      "Owned objects can be processed in parallel without consensus. Shared objects still require ordering. Developers must reason about object ownership for every operation.",
  },
  {
    chain: "Aptos",
    badge: "bg-emerald-900/50 text-emerald-300",
    model: "Block-STM",
    description:
      "Speculatively executes all transactions in parallel, then detects conflicts and re-executes. No upfront declaration needed. Very efficient when conflict rate is low.",
  },
  {
    chain: "Dina",
    badge: "bg-blue-900/50 text-blue-300 ring-1 ring-blue-700/50",
    model: "Block-STM lanes + Swarm",
    description:
      "Block-STM speculative execution across dedicated lanes, combined with Swarm Wallets (DRC-63) that give each user a pool of addresses. This eliminates hot-account contention entirely, achieving true per-user parallelism.",
  },
];

interface NovelCategory {
  name: string;
  description: string;
}

const NOVEL_CATEGORIES: NovelCategory[] = [
  {
    name: "Agent Wallets",
    description:
      "DRC-101 through DRC-106. Autonomous AI agents with spending limits, delegation, and audit trails.",
  },
  {
    name: "Swarm Coordination",
    description:
      "DRC-63, DRC-64, DRC-65. Per-user wallet pools, swarm state sync, and parallel transaction routing.",
  },
  {
    name: "IoT & Device",
    description:
      "DRC-70 through DRC-79. Device attestation, secure hardware integration, sensor data anchoring.",
  },
  {
    name: "Machine Economy",
    description:
      "DRC-80 through DRC-89. Machine-to-machine payments, resource metering, compute markets.",
  },
  {
    name: "AI Inference",
    description:
      "DRC-110 through DRC-119. On-chain inference billing, model registries, verifiable compute proofs.",
  },
  {
    name: "Payment Channels",
    description:
      "DRC-40 through DRC-49. Offline-capable channels, multi-hop routing, instant settlement.",
  },
];

interface UniqueFeature {
  title: string;
  description: string;
}

const UNIQUE_FEATURES: UniqueFeature[] = [
  {
    title: "Swarm Wallets (DRC-63)",
    description:
      "Each user gets a pool of linked addresses. Transactions from different Swarm Wallets never conflict, enabling true per-user parallelism. No other chain has this.",
  },
  {
    title: "USDC as native gas",
    description:
      "Gas is paid in USDC, not a volatile native token. Users and businesses never need to hold a separate token to transact. Fees are predictable and denominated in dollars.",
  },
  {
    title: "Agent Wallets (DRC-101)",
    description:
      "Autonomous AI agents can hold funds with configurable spending limits, time-bound delegations, and full audit trails. Purpose-built for the agent economy.",
  },
  {
    title: "Device Attestation",
    description:
      "Hardware-verified transactions via secure elements. Physical IoT devices cryptographically prove they initiated a transaction, preventing spoofing.",
  },
  {
    title: "52 novel smart contract standards",
    description:
      "Purpose-built standards for agents, robots, IoT devices, machine payments, and AI inference. These categories do not exist on any other chain.",
  },
  {
    title: "Payment channels with offline settlement",
    description:
      "Bi-directional payment channels that work offline and settle on-chain when connectivity returns. Designed for IoT devices in low-connectivity environments.",
  },
  {
    title: "100ms BFT finality",
    description:
      "Not soft confirmation, not optimistic. Full Byzantine Fault Tolerant finality in 100ms. Transactions are mathematically irreversible after a single block.",
  },
];

interface Tradeoff {
  scenario: string;
  recommendation: string;
}

const TRADEOFFS: Tradeoff[] = [
  {
    scenario: "You need maximum decentralization",
    recommendation:
      "Use Ethereum. With ~900K validators and 10 years of battle-tested security, Ethereum is the gold standard for decentralization. Dina intentionally uses 3-7 validators to optimize for speed.",
  },
  {
    scenario: "You need a large existing DeFi ecosystem",
    recommendation:
      "Use Ethereum or Solana. Uniswap, Aave, Maker, and hundreds of DeFi protocols live on EVM chains. Solana has Raydium, Marinade, and Jupiter. Dina does not have a DeFi ecosystem yet.",
  },
  {
    scenario: "You need EVM compatibility for existing Solidity contracts",
    recommendation:
      "Use Base or Arbitrum. If you have a large Solidity codebase and cannot rewrite it, an EVM L2 is the fastest path to production. Dina requires rewriting contracts in Rust/WASM.",
  },
  {
    scenario: "You need censorship resistance as a core requirement",
    recommendation:
      "Use Bitcoin or Ethereum. Their large validator sets and geographic distribution make censorship extremely difficult. Dina's small validator set is a trade-off for throughput.",
  },
];
