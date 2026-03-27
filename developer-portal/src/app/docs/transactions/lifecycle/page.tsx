import Link from "next/link";

export const metadata = {
  title: "Transaction Lifecycle — Dina Network Developer Portal",
  description:
    "Step-by-step walkthrough of a Dina Network transaction from creation to finality.",
};

const LIFECYCLE_STEPS = [
  {
    step: "1",
    title: "Create",
    desc: "The sender constructs a transaction object with from, to, amount, fee, and nonce fields. The nonce is the next sequential counter for the sender's account.",
    detail: "Query the current nonce via dina_getAccount. If the sender has sent 41 transactions, the next nonce is 42.",
    color: "text-blue-400",
    bgColor: "bg-blue-600/20",
  },
  {
    step: "2",
    title: "Sign",
    desc: "The transaction is serialized to canonical JSON (alphabetical field order) and signed with the sender's Ed25519 private key, producing a 64-byte signature.",
    detail: "The signature covers all fields except the signature itself. Ed25519 signatures are deterministic -- signing the same bytes with the same key always produces the same signature.",
    color: "text-purple-400",
    bgColor: "bg-purple-600/20",
  },
  {
    step: "3",
    title: "Submit",
    desc: "The signed transaction is submitted to any validator node via JSON-RPC (dina_sendTransaction) or REST API (POST /transactions).",
    detail: "The receiving node validates the signature, checks the nonce, verifies sufficient balance, and rejects immediately if any check fails. If valid, the node returns a transaction hash.",
    color: "text-cyan-400",
    bgColor: "bg-cyan-600/20",
  },
  {
    step: "4",
    title: "Mempool",
    desc: "The transaction enters the mempool -- a staging area for pending transactions. The validator broadcasts it to other validators via the gossip protocol.",
    detail: "Transactions in the mempool are ordered by fee (highest first). The block-producing validator selects transactions from its mempool to fill the next block, up to 10,000 transactions per block.",
    color: "text-green-400",
    bgColor: "bg-green-600/20",
  },
  {
    step: "5",
    title: "Block Inclusion",
    desc: "The round leader constructs a block containing the transaction and proposes it to the validator set via TurboBFT consensus. After prevote and precommit phases, the block is committed.",
    detail: "The block is committed when 2/3+ validators sign precommit messages for it. This takes approximately 100ms. Independent transactions are assigned to different execution lanes for parallel processing.",
    color: "text-amber-400",
    bgColor: "bg-amber-600/20",
  },
  {
    step: "6",
    title: "Execute",
    desc: "The transaction is executed: the sender's balance is debited, the recipient's balance is credited, the fee is collected, and the sender's nonce is incremented.",
    detail: "For contract calls, the WASM runtime loads the contract code, deserializes the state, calls the dispatch function, and persists the updated state. Gas is metered during execution.",
    color: "text-orange-400",
    bgColor: "bg-orange-600/20",
  },
  {
    step: "7",
    title: "Receipt",
    desc: "A transaction receipt is generated containing the hash, block number, gas used, status (success/failure), and any events emitted by contract execution.",
    detail: "The receipt is available immediately via dina_getTransactionReceipt. There is no need to poll or wait -- 1 block = final.",
    color: "text-rose-400",
    bgColor: "bg-rose-600/20",
  },
];

export default function LifecyclePage() {
  return (
    <div>
      {/* Header */}
      <p className="text-sm font-medium uppercase tracking-wider text-blue-400 mb-3">
        Transactions
      </p>
      <h1 className="text-4xl font-bold tracking-tight text-white mb-4">
        Transaction Lifecycle
      </h1>
      <p className="text-lg text-slate-400 max-w-3xl leading-relaxed mb-10">
        Every transaction on Dina Network follows a deterministic lifecycle from
        creation to finality. The entire journey takes approximately 100ms -- a
        single block. There are no probabilistic confirmations and no
        reorganizations.
      </p>

      {/* Visual Flow Diagram */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-6">
          Transaction Flow
        </h2>
        <div className="overflow-x-auto rounded-xl border border-slate-800 bg-slate-900/50 p-6">
          <div className="flex items-center gap-2 min-w-[700px]">
            {["Create", "Sign", "Submit", "Mempool", "Block", "Execute", "Receipt"].map(
              (label, i, arr) => (
                <div key={label} className="flex items-center gap-2">
                  <div className="flex flex-col items-center">
                    <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-blue-600/20 text-sm font-bold text-blue-400">
                      {i + 1}
                    </div>
                    <span className="mt-1.5 text-xs font-medium text-slate-400">
                      {label}
                    </span>
                  </div>
                  {i < arr.length - 1 && (
                    <svg
                      className="h-4 w-8 text-slate-600"
                      fill="none"
                      viewBox="0 0 32 16"
                      stroke="currentColor"
                      strokeWidth={2}
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        d="M0 8h24m0 0l-6-6m6 6l-6 6"
                      />
                    </svg>
                  )}
                </div>
              )
            )}
          </div>
          <div className="mt-4 flex items-center gap-2 text-xs text-slate-500">
            <span className="rounded bg-slate-800 px-2 py-1">Client-side</span>
            <span className="text-slate-700">|</span>
            <span className="rounded bg-slate-800 px-2 py-1">Network</span>
            <span className="text-slate-700">|</span>
            <span className="rounded bg-slate-800 px-2 py-1">Validator</span>
          </div>
        </div>
      </section>

      {/* Detailed Steps */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-6">
          Step-by-Step Detail
        </h2>
        <div className="space-y-6">
          {LIFECYCLE_STEPS.map((s) => (
            <div
              key={s.step}
              className="rounded-xl border border-slate-800 bg-slate-900/50 p-6"
            >
              <div className="flex items-start gap-4">
                <div
                  className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-lg ${s.bgColor} text-sm font-bold ${s.color}`}
                >
                  {s.step}
                </div>
                <div>
                  <h3 className={`text-lg font-semibold ${s.color} mb-2`}>
                    {s.title}
                  </h3>
                  <p className="text-sm text-slate-300 leading-relaxed mb-2">
                    {s.desc}
                  </p>
                  <p className="text-sm text-slate-500 leading-relaxed">
                    {s.detail}
                  </p>
                </div>
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* Parallel Execution */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Parallel Execution
        </h2>
        <div className="rounded-xl border border-blue-500/30 bg-blue-500/5 p-6">
          <h3 className="text-sm font-semibold text-blue-400 mb-2">
            Lane-Based Parallelism
          </h3>
          <p className="text-sm text-slate-300 leading-relaxed mb-4">
            Dina Network does not process transactions sequentially. When a
            block is committed, the executor analyzes all transactions and
            assigns them to independent <strong>execution lanes</strong> based
            on their account dependencies:
          </p>
          <ul className="space-y-2 mb-4">
            {[
              "Transactions touching different accounts run in parallel on separate CPU cores",
              "Transactions that share accounts (same sender or same recipient) are serialized within a lane",
              "The lane assignment is deterministic -- all validators compute the same lanes",
              "With 8 CPU cores and independent transactions, throughput increases 8x",
            ].map((item) => (
              <li key={item} className="flex items-start gap-2 text-sm text-slate-300">
                <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-blue-500" />
                {item}
              </li>
            ))}
          </ul>
          <pre className="overflow-x-auto rounded-lg bg-slate-800 p-4 text-sm leading-relaxed">
            <code className="text-slate-200">{`Block #148302 contains 6 transactions:

Lane 1: Alice -> Bob (10 USDC)         \\
Lane 1: Alice -> Carol (5 USDC)         |-- serialized (same sender)
                                         |
Lane 2: Dave -> Eve (20 USDC)           |-- runs in parallel with Lane 1
                                         |
Lane 3: Frank -> Grace (50 USDC)        |-- runs in parallel with Lanes 1 & 2
Lane 3: Grace -> Heidi (30 USDC)        /   serialized within Lane 3

Total execution: max(Lane 1, Lane 2, Lane 3)
Instead of:      Lane 1 + Lane 2 + Lane 3`}</code>
          </pre>
        </div>
      </section>

      {/* Finality */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Finality
        </h2>
        <div className="grid gap-4 sm:grid-cols-3 mb-6">
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-5">
            <p className="text-3xl font-bold text-blue-400">1 block</p>
            <p className="text-sm text-slate-400 mt-2">
              Confirmations needed. Once included in a committed block, the
              transaction is final.
            </p>
          </div>
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-5">
            <p className="text-3xl font-bold text-blue-400">100ms</p>
            <p className="text-sm text-slate-400 mt-2">
              Block time. TurboBFT consensus with 3-7 validators achieves
              sub-200ms finality.
            </p>
          </div>
          <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-5">
            <p className="text-3xl font-bold text-blue-400">0 reorgs</p>
            <p className="text-sm text-slate-400 mt-2">
              BFT consensus means committed blocks are irreversible. No
              chain reorganizations, ever.
            </p>
          </div>
        </div>

        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
          <h3 className="text-sm font-semibold text-white mb-3">
            Comparison with Other Chains
          </h3>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-slate-800">
                  <th className="px-4 py-2 text-left font-medium text-slate-400">Chain</th>
                  <th className="px-4 py-2 text-left font-medium text-slate-400">Block Time</th>
                  <th className="px-4 py-2 text-left font-medium text-slate-400">Finality</th>
                  <th className="px-4 py-2 text-left font-medium text-slate-400">Reorg Risk</th>
                </tr>
              </thead>
              <tbody>
                {[
                  { chain: "Dina Network", block: "100ms", finality: "100ms (1 block)", reorg: "None (BFT)" },
                  { chain: "Ethereum", block: "12s", finality: "~15 min (64 slots)", reorg: "Low (Casper)" },
                  { chain: "Bitcoin", block: "10 min", finality: "~60 min (6 blocks)", reorg: "Possible (PoW)" },
                  { chain: "Solana", block: "400ms", finality: "~13s (32 slots)", reorg: "Possible" },
                  { chain: "Base (L2)", block: "2s", finality: "~15 min (L1 finality)", reorg: "Possible until L1" },
                ].map((row, i) => (
                  <tr
                    key={row.chain}
                    className={i % 2 === 0 ? "bg-slate-950/50" : "bg-slate-900/30"}
                  >
                    <td className="px-4 py-2 font-medium text-slate-300">{row.chain}</td>
                    <td className="px-4 py-2 text-slate-400">{row.block}</td>
                    <td className="px-4 py-2 text-slate-400">{row.finality}</td>
                    <td className="px-4 py-2 text-slate-400">{row.reorg}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </section>

      {/* Receipt Structure */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Transaction Receipt
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          After execution, a receipt is generated and permanently stored on-chain.
          Query it via{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs text-blue-300">
            dina_getTransactionReceipt
          </code>
          :
        </p>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`{
  "hash": "0xabc123...def456",
  "blockNumber": 148302,
  "blockHash": "0x789abc...012def",
  "from": "0x1a2b3c...sender",
  "to": "0x7a8b9c...recipient",
  "amount": 25000000,
  "fee": 1000,
  "gasUsed": 21000,
  "nonce": 42,
  "status": "success",
  "executionLane": 2,
  "events": [],
  "timestamp": 1711500000000
}`}</code>
        </pre>
      </section>

      {/* Querying */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Querying Transaction Status
        </h2>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`import { DinaClient } from '@dina-network/sdk';

const client = new DinaClient('https://rpc-testnet.dina.network');

// Get transaction by hash
const tx = await client.getTransaction('0xabc123...def456');
console.log('Status:', tx.status);           // "success"
console.log('Block:', tx.blockNumber);       // 148302
console.log('Lane:', tx.executionLane);      // 2

// Get receipt with events (useful for contract calls)
const receipt = await client.getTransactionReceipt('0xabc123...def456');
console.log('Events:', receipt.events);      // [ { name: "Transfer", ... } ]
console.log('Gas used:', receipt.gasUsed);   // 21000`}</code>
        </pre>
      </section>

      {/* Next Steps */}
      <div className="mt-10 flex flex-wrap gap-4">
        <Link
          href="/docs/transactions/channels"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          &larr; Payment Channels
        </Link>
        <Link
          href="/docs/contracts/deploy"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          Deploy Smart Contract &rarr;
        </Link>
        <Link
          href="/docs/parallel"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          Parallel Execution Deep Dive &rarr;
        </Link>
      </div>
    </div>
  );
}
