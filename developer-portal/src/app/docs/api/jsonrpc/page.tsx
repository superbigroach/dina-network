import { API_ENDPOINTS, TESTNET_CONFIG } from "@/lib/constants";

const RPC_URL = TESTNET_CONFIG.rpcUrl;

interface RpcMethodDoc {
  id: string;
  method: string;
  description: string;
  params: { name: string; type: string; description: string }[];
  returns: { type: string; description: string; fields?: { name: string; type: string; description: string }[] };
  exampleParams: string[];
  exampleResponse: unknown;
}

const METHODS: RpcMethodDoc[] = [
  {
    id: "dina_blockNumber",
    method: "dina_blockNumber",
    description: "Returns the current block height of the chain.",
    params: [],
    returns: {
      type: "number",
      description: "The latest block number.",
    },
    exampleParams: [],
    exampleResponse: { jsonrpc: "2.0", id: 1, result: 48210553 },
  },
  {
    id: "dina_getBalance",
    method: "dina_getBalance",
    description: "Returns the USDC balance for a given address in micro-USDC (1 USDC = 1,000,000 micro-USDC).",
    params: [
      { name: "address", type: "string", description: "The account address to query." },
    ],
    returns: {
      type: "string",
      description: "Balance in micro-USDC as a decimal string.",
    },
    exampleParams: ['"dina1qxyz...abc"'],
    exampleResponse: { jsonrpc: "2.0", id: 1, result: "150000000" },
  },
  {
    id: "dina_getAccount",
    method: "dina_getAccount",
    description: "Returns the full account details including balance, nonce, and code hash.",
    params: [
      { name: "address", type: "string", description: "The account address to query." },
    ],
    returns: {
      type: "object",
      description: "Account details object.",
      fields: [
        { name: "address", type: "string", description: "The account address." },
        { name: "balance", type: "string", description: "Balance in micro-USDC." },
        { name: "nonce", type: "number", description: "Current transaction nonce." },
        { name: "code_hash", type: "string | null", description: "SHA-256 hash of deployed contract code, or null for EOAs." },
      ],
    },
    exampleParams: ['"dina1qxyz...abc"'],
    exampleResponse: {
      jsonrpc: "2.0",
      id: 1,
      result: {
        address: "dina1qxyz...abc",
        balance: "150000000",
        nonce: 42,
        code_hash: null,
      },
    },
  },
  {
    id: "dina_getBlock",
    method: "dina_getBlock",
    description: "Returns a block by its number, including the header and all transactions.",
    params: [
      { name: "blockNumber", type: "number", description: "The block number to retrieve." },
    ],
    returns: {
      type: "object",
      description: "Block object with header and transactions.",
      fields: [
        { name: "number", type: "number", description: "Block height." },
        { name: "hash", type: "string", description: "Block hash (SHA-256)." },
        { name: "parent_hash", type: "string", description: "Hash of the parent block." },
        { name: "timestamp", type: "number", description: "Unix timestamp in milliseconds." },
        { name: "validator", type: "string", description: "Address of the block proposer." },
        { name: "tx_count", type: "number", description: "Number of transactions in the block." },
        { name: "transactions", type: "Transaction[]", description: "Array of transaction objects." },
      ],
    },
    exampleParams: ["48210553"],
    exampleResponse: {
      jsonrpc: "2.0",
      id: 1,
      result: {
        number: 48210553,
        hash: "0xabc123...def",
        parent_hash: "0x987654...fed",
        timestamp: 1711843200100,
        validator: "dina1val0...xyz",
        tx_count: 2,
        transactions: [
          {
            hash: "0xtx1...",
            from: "dina1qxyz...abc",
            to: "dina1qabc...xyz",
            value: "1000000",
            nonce: 42,
            gas_used: 21000,
          },
        ],
      },
    },
  },
  {
    id: "dina_getTransaction",
    method: "dina_getTransaction",
    description: "Returns the details of a transaction by its hash.",
    params: [
      { name: "txHash", type: "string", description: "The transaction hash." },
    ],
    returns: {
      type: "object",
      description: "Transaction details.",
      fields: [
        { name: "hash", type: "string", description: "Transaction hash." },
        { name: "from", type: "string", description: "Sender address." },
        { name: "to", type: "string", description: "Recipient address." },
        { name: "value", type: "string", description: "Transfer amount in micro-USDC." },
        { name: "nonce", type: "number", description: "Sender nonce at time of tx." },
        { name: "gas_used", type: "number", description: "Gas consumed." },
        { name: "gas_price", type: "string", description: "Gas price in micro-USDC." },
        { name: "block_number", type: "number", description: "Block the tx was included in." },
        { name: "status", type: "string", description: '"success" or "failed".' },
      ],
    },
    exampleParams: ['"0xtx1...abc"'],
    exampleResponse: {
      jsonrpc: "2.0",
      id: 1,
      result: {
        hash: "0xtx1...abc",
        from: "dina1qxyz...abc",
        to: "dina1qabc...xyz",
        value: "5000000",
        nonce: 42,
        gas_used: 21000,
        gas_price: "1",
        block_number: 48210553,
        status: "success",
      },
    },
  },
  {
    id: "dina_sendTransaction",
    method: "dina_sendTransaction",
    description: "Broadcasts a signed transaction to the network and returns the transaction hash.",
    params: [
      { name: "signedTxHex", type: "string", description: "The signed transaction encoded as a hex string." },
    ],
    returns: {
      type: "string",
      description: "The transaction hash of the submitted transaction.",
    },
    exampleParams: ['"0xf86c0a8502540be400..."'],
    exampleResponse: { jsonrpc: "2.0", id: 1, result: "0xtx_new_hash...abc" },
  },
  {
    id: "dina_estimateGas",
    method: "dina_estimateGas",
    description: "Estimates the gas required to execute a transaction without broadcasting it.",
    params: [
      {
        name: "txParams",
        type: "object",
        description: 'Transaction parameters: { from, to, value, data? }.',
      },
    ],
    returns: {
      type: "number",
      description: "Estimated gas units.",
    },
    exampleParams: ['{"from":"dina1qxyz...abc","to":"dina1qabc...xyz","value":"1000000"}'],
    exampleResponse: { jsonrpc: "2.0", id: 1, result: 21000 },
  },
  {
    id: "dina_getTransactionReceipt",
    method: "dina_getTransactionReceipt",
    description: "Returns the receipt for a mined transaction, including emitted events.",
    params: [
      { name: "txHash", type: "string", description: "The transaction hash." },
    ],
    returns: {
      type: "object",
      description: "Transaction receipt.",
      fields: [
        { name: "transaction_hash", type: "string", description: "Transaction hash." },
        { name: "block_number", type: "number", description: "Block the tx was included in." },
        { name: "status", type: "string", description: '"success" or "failed".' },
        { name: "gas_used", type: "number", description: "Actual gas consumed." },
        { name: "events", type: "Event[]", description: "Array of emitted events." },
      ],
    },
    exampleParams: ['"0xtx1...abc"'],
    exampleResponse: {
      jsonrpc: "2.0",
      id: 1,
      result: {
        transaction_hash: "0xtx1...abc",
        block_number: 48210553,
        status: "success",
        gas_used: 21000,
        events: [
          {
            address: "dina1qabc...xyz",
            topics: ["Transfer"],
            data: { from: "dina1qxyz...abc", to: "dina1qabc...xyz", amount: "5000000" },
          },
        ],
      },
    },
  },
  {
    id: "dina_chainId",
    method: "dina_chainId",
    description: 'Returns the chain identifier string for the connected network.',
    params: [],
    returns: {
      type: "string",
      description: 'The chain ID (e.g. "dina-testnet-1").',
    },
    exampleParams: [],
    exampleResponse: { jsonrpc: "2.0", id: 1, result: "dina-testnet-1" },
  },
  {
    id: "dina_networkInfo",
    method: "dina_networkInfo",
    description: "Returns network configuration and runtime statistics, including validator count and block time.",
    params: [],
    returns: {
      type: "object",
      description: "Network information.",
      fields: [
        { name: "chain_id", type: "string", description: "Chain identifier." },
        { name: "block_time_ms", type: "number", description: "Target block time in milliseconds." },
        { name: "max_txs_per_block", type: "number", description: "Maximum transactions per block." },
        { name: "validator_count", type: "number", description: "Active validator count." },
        { name: "current_block", type: "number", description: "Latest block number." },
        { name: "pending_txs", type: "number", description: "Transactions in mempool." },
      ],
    },
    exampleParams: [],
    exampleResponse: {
      jsonrpc: "2.0",
      id: 1,
      result: {
        chain_id: "dina-testnet-1",
        block_time_ms: 100,
        max_txs_per_block: 10000,
        validator_count: 3,
        current_block: 48210553,
        pending_txs: 12,
      },
    },
  },
  {
    id: "dina_peerCount",
    method: "dina_peerCount",
    description: "Returns the number of peers currently connected to the node.",
    params: [],
    returns: {
      type: "number",
      description: "Connected peer count.",
    },
    exampleParams: [],
    exampleResponse: { jsonrpc: "2.0", id: 1, result: 8 },
  },
  {
    id: "dina_syncing",
    method: "dina_syncing",
    description: "Returns the sync status of the node. Returns false if the node is fully synced, or an object with sync progress details.",
    params: [],
    returns: {
      type: "boolean | object",
      description: "false if synced, otherwise an object with current_block, highest_block, and starting_block.",
    },
    exampleParams: [],
    exampleResponse: {
      jsonrpc: "2.0",
      id: 1,
      result: {
        syncing: true,
        current_block: 48210000,
        highest_block: 48210553,
        starting_block: 0,
      },
    },
  },
];

function CodeBlock({ children, title }: { children: string; title?: string }) {
  return (
    <div className="overflow-hidden rounded-lg border border-slate-700/60">
      {title && (
        <div className="border-b border-slate-700/60 bg-slate-800/80 px-4 py-2 text-xs font-medium text-slate-400">
          {title}
        </div>
      )}
      <pre className="overflow-x-auto bg-slate-800/50 px-4 py-3 text-sm leading-relaxed">
        <code className="font-mono text-slate-300">{children}</code>
      </pre>
    </div>
  );
}

function ParamTable({ params }: { params: { name: string; type: string; description: string }[] }) {
  if (params.length === 0) {
    return <p className="text-sm text-slate-500 italic">This method takes no parameters.</p>;
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-slate-700/60">
            <th className="px-4 py-2.5 text-left font-semibold text-slate-300">Parameter</th>
            <th className="px-4 py-2.5 text-left font-semibold text-slate-300">Type</th>
            <th className="px-4 py-2.5 text-left font-semibold text-slate-300">Description</th>
          </tr>
        </thead>
        <tbody>
          {params.map((p) => (
            <tr key={p.name} className="border-b border-slate-800/40">
              <td className="px-4 py-2.5">
                <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs font-mono text-blue-400">{p.name}</code>
              </td>
              <td className="px-4 py-2.5">
                <code className="text-xs font-mono text-purple-400">{p.type}</code>
              </td>
              <td className="px-4 py-2.5 text-slate-400">{p.description}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function ReturnBlock({ returns }: { returns: RpcMethodDoc["returns"] }) {
  return (
    <div>
      <div className="mb-2 flex items-center gap-2">
        <span className="text-sm font-semibold text-slate-300">Returns:</span>
        <code className="text-xs font-mono text-purple-400">{returns.type}</code>
      </div>
      <p className="mb-3 text-sm text-slate-400">{returns.description}</p>
      {returns.fields && returns.fields.length > 0 && (
        <div className="overflow-x-auto rounded-lg border border-slate-700/60">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-700/60 bg-slate-800/40">
                <th className="px-4 py-2 text-left font-semibold text-slate-300">Field</th>
                <th className="px-4 py-2 text-left font-semibold text-slate-300">Type</th>
                <th className="px-4 py-2 text-left font-semibold text-slate-300">Description</th>
              </tr>
            </thead>
            <tbody>
              {returns.fields.map((f) => (
                <tr key={f.name} className="border-b border-slate-800/40">
                  <td className="px-4 py-2">
                    <code className="text-xs font-mono text-green-400">{f.name}</code>
                  </td>
                  <td className="px-4 py-2">
                    <code className="text-xs font-mono text-purple-400">{f.type}</code>
                  </td>
                  <td className="px-4 py-2 text-slate-400">{f.description}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function MethodSection({ doc }: { doc: RpcMethodDoc }) {
  const requestBody = JSON.stringify(
    {
      jsonrpc: "2.0",
      method: doc.method,
      params: doc.exampleParams.length > 0 ? doc.exampleParams : [],
      id: 1,
    },
    null,
    2,
  )
    // Clean up the stringified params to look like real values
    .replace(/"(\d+)"/g, "$1")
    .replace(/"\{/g, "{")
    .replace(/\}"/g, "}")
    .replace(/\\"/g, '"');

  const curlExample = `curl -X POST ${RPC_URL} \\
  -H "Content-Type: application/json" \\
  -d '${JSON.stringify({
    jsonrpc: "2.0",
    method: doc.method,
    params: doc.exampleParams.length > 0 ? doc.exampleParams : [],
    id: 1,
  }).replace(/"/g, '\\"').replace(/\\\\"/g, '"')}'`;

  const curlSimple = `curl -X POST ${RPC_URL} \\
  -H "Content-Type: application/json" \\
  -d '${JSON.stringify({ jsonrpc: "2.0", method: doc.method, params: doc.exampleParams, id: 1 })}'`;

  return (
    <section id={doc.id} className="scroll-mt-24">
      <div className="rounded-xl border border-slate-800/60 bg-slate-900/30 p-6 backdrop-blur-sm">
        <div className="mb-4 flex items-center gap-3">
          <h3 className="text-xl font-bold text-white">{doc.method}</h3>
          <span className="rounded-full bg-blue-500/10 px-2.5 py-0.5 text-xs font-medium text-blue-400 ring-1 ring-inset ring-blue-500/20">
            JSON-RPC
          </span>
        </div>

        <p className="mb-6 text-sm leading-relaxed text-slate-400">{doc.description}</p>

        {/* Parameters */}
        <div className="mb-6">
          <h4 className="mb-3 text-sm font-semibold uppercase tracking-wider text-slate-500">Parameters</h4>
          <ParamTable params={doc.params} />
        </div>

        {/* Returns */}
        <div className="mb-6">
          <h4 className="mb-3 text-sm font-semibold uppercase tracking-wider text-slate-500">Response</h4>
          <ReturnBlock returns={doc.returns} />
        </div>

        {/* Example Request */}
        <div className="mb-4">
          <h4 className="mb-3 text-sm font-semibold uppercase tracking-wider text-slate-500">Example Request</h4>
          <CodeBlock title="curl">{curlSimple}</CodeBlock>
        </div>

        {/* Example Request Body */}
        <div className="mb-4">
          <h4 className="mb-3 text-sm font-semibold uppercase tracking-wider text-slate-500">Request Body</h4>
          <CodeBlock title="JSON">{requestBody}</CodeBlock>
        </div>

        {/* Example Response */}
        <div>
          <h4 className="mb-3 text-sm font-semibold uppercase tracking-wider text-slate-500">Example Response</h4>
          <CodeBlock title="JSON">{JSON.stringify(doc.exampleResponse, null, 2)}</CodeBlock>
        </div>
      </div>
    </section>
  );
}

export default function JsonRpcApiPage() {
  // Pull method names from constants for cross-reference
  const registeredMethods = API_ENDPOINTS.jsonrpc.map((m) => m.method);

  return (
    <div>
      {/* Header */}
      <div className="mb-10">
        <div className="mb-3 flex items-center gap-2">
          <span className="rounded-full bg-blue-500/10 px-2.5 py-0.5 text-xs font-medium text-blue-400 ring-1 ring-inset ring-blue-500/20">
            API Reference
          </span>
        </div>
        <h1 className="text-4xl font-extrabold tracking-tight">JSON-RPC API</h1>
        <p className="mt-3 text-lg leading-relaxed text-slate-400">
          The Dina Network JSON-RPC API follows the{" "}
          <a
            href="https://www.jsonrpc.org/specification"
            target="_blank"
            rel="noopener noreferrer"
            className="text-blue-400 underline decoration-blue-400/30 hover:decoration-blue-400"
          >
            JSON-RPC 2.0 specification
          </a>
          . All requests are sent as HTTP POST to the RPC endpoint.
        </p>
      </div>

      {/* Base URL */}
      <div className="mb-10 rounded-xl border border-slate-800/60 bg-slate-900/40 p-5">
        <h2 className="mb-2 text-sm font-semibold uppercase tracking-wider text-slate-500">Base URL</h2>
        <code className="text-sm font-mono text-green-400">{RPC_URL}</code>
        <p className="mt-2 text-sm text-slate-500">
          All JSON-RPC requests use <code className="rounded bg-slate-800 px-1 py-0.5 text-xs text-slate-300">POST</code> with{" "}
          <code className="rounded bg-slate-800 px-1 py-0.5 text-xs text-slate-300">Content-Type: application/json</code>.
        </p>
      </div>

      {/* Quick Nav */}
      <div className="mb-10 rounded-xl border border-slate-800/60 bg-slate-900/40 p-5">
        <h2 className="mb-4 text-sm font-semibold uppercase tracking-wider text-slate-500">Methods</h2>
        <div className="grid grid-cols-1 gap-1.5 sm:grid-cols-2 lg:grid-cols-3">
          {METHODS.map((doc) => (
            <a
              key={doc.id}
              href={`#${doc.id}`}
              className="group flex items-center gap-2 rounded-lg px-3 py-2 text-sm transition-colors hover:bg-slate-800/60"
            >
              <span className="font-mono text-blue-400 group-hover:text-blue-300">{doc.method}</span>
            </a>
          ))}
        </div>
      </div>

      {/* Method Sections */}
      <div className="space-y-8">
        {METHODS.map((doc) => (
          <MethodSection key={doc.id} doc={doc} />
        ))}
      </div>

      {/* Registered Methods Cross-Reference */}
      <div className="mt-12 rounded-xl border border-slate-800/60 bg-slate-900/30 p-6">
        <h2 className="mb-3 text-lg font-bold text-white">Registered Methods</h2>
        <p className="mb-4 text-sm text-slate-400">
          The following {registeredMethods.length} methods are registered in the Dina Network RPC interface:
        </p>
        <div className="flex flex-wrap gap-2">
          {registeredMethods.map((m) => (
            <a
              key={m}
              href={`#${m}`}
              className="rounded-lg bg-slate-800/60 px-3 py-1.5 text-xs font-mono text-slate-300 transition-colors hover:bg-slate-700/60 hover:text-white"
            >
              {m}
            </a>
          ))}
        </div>
      </div>
    </div>
  );
}
