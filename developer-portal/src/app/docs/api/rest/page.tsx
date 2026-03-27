import { API_ENDPOINTS, TESTNET_CONFIG } from "@/lib/constants";

const REST_URL = TESTNET_CONFIG.restUrl;

interface RestEndpointDoc {
  id: string;
  method: string;
  path: string;
  description: string;
  pathParams?: { name: string; type: string; description: string }[];
  queryParams?: { name: string; type: string; required: boolean; description: string }[];
  requestBody?: { type: string; description: string; fields: { name: string; type: string; description: string }[] };
  responseFields: { name: string; type: string; description: string }[];
  exampleCurl: string;
  exampleResponse: unknown;
}

const ENDPOINTS: RestEndpointDoc[] = [
  {
    id: "get-health",
    method: "GET",
    path: "/health",
    description: "Returns the health status of the node along with the current block height. Use this for monitoring and readiness checks.",
    responseFields: [
      { name: "status", type: "string", description: '"healthy" or "degraded".' },
      { name: "block_height", type: "number", description: "Current block number." },
      { name: "uptime_seconds", type: "number", description: "Node uptime in seconds." },
      { name: "peer_count", type: "number", description: "Connected peers." },
      { name: "version", type: "string", description: "Node software version." },
    ],
    exampleCurl: `curl ${REST_URL}/health`,
    exampleResponse: {
      status: "healthy",
      block_height: 48210553,
      uptime_seconds: 864000,
      peer_count: 8,
      version: "0.1.0",
    },
  },
  {
    id: "get-account",
    method: "GET",
    path: "/accounts/:address",
    description: "Returns detailed account information including balance, nonce, and contract code hash.",
    pathParams: [
      { name: "address", type: "string", description: "The Dina account address." },
    ],
    responseFields: [
      { name: "address", type: "string", description: "The account address." },
      { name: "balance", type: "string", description: "Balance in micro-USDC." },
      { name: "nonce", type: "number", description: "Current transaction nonce." },
      { name: "code_hash", type: "string | null", description: "SHA-256 hash of contract code, or null." },
      { name: "created_at", type: "string", description: "ISO 8601 timestamp of account creation." },
    ],
    exampleCurl: `curl ${REST_URL}/accounts/dina1qxyz...abc`,
    exampleResponse: {
      address: "dina1qxyz...abc",
      balance: "150000000",
      nonce: 42,
      code_hash: null,
      created_at: "2025-10-01T12:00:00Z",
    },
  },
  {
    id: "post-transactions",
    method: "POST",
    path: "/transactions",
    description: "Submits a signed transaction to the network. The transaction is broadcast to validators and included in the next available block.",
    requestBody: {
      type: "application/json",
      description: "Signed transaction payload.",
      fields: [
        { name: "tx_hex", type: "string", description: "The signed transaction encoded as a hex string." },
      ],
    },
    responseFields: [
      { name: "hash", type: "string", description: "The transaction hash." },
      { name: "status", type: "string", description: '"pending" -- tx is in the mempool.' },
    ],
    exampleCurl: `curl -X POST ${REST_URL}/transactions \\
  -H "Content-Type: application/json" \\
  -d '{"tx_hex":"0xf86c0a8502540be400..."}'`,
    exampleResponse: {
      hash: "0xtx_new_hash...abc",
      status: "pending",
    },
  },
  {
    id: "get-block",
    method: "GET",
    path: "/blocks/:number",
    description: "Returns a block by its number, including the header and all included transactions.",
    pathParams: [
      { name: "number", type: "number", description: "The block number to retrieve." },
    ],
    responseFields: [
      { name: "number", type: "number", description: "Block height." },
      { name: "hash", type: "string", description: "Block hash." },
      { name: "parent_hash", type: "string", description: "Parent block hash." },
      { name: "timestamp", type: "number", description: "Unix timestamp in milliseconds." },
      { name: "validator", type: "string", description: "Block proposer address." },
      { name: "tx_count", type: "number", description: "Transaction count." },
      { name: "transactions", type: "Transaction[]", description: "Array of transaction objects." },
    ],
    exampleCurl: `curl ${REST_URL}/blocks/48210553`,
    exampleResponse: {
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
        },
      ],
    },
  },
  {
    id: "get-latest-block",
    method: "GET",
    path: "/blocks/latest",
    description: "Returns the most recently finalized block. This is a convenience endpoint equivalent to fetching the block at the current height.",
    responseFields: [
      { name: "number", type: "number", description: "Block height." },
      { name: "hash", type: "string", description: "Block hash." },
      { name: "parent_hash", type: "string", description: "Parent block hash." },
      { name: "timestamp", type: "number", description: "Unix timestamp in milliseconds." },
      { name: "validator", type: "string", description: "Block proposer address." },
      { name: "tx_count", type: "number", description: "Transaction count." },
      { name: "transactions", type: "Transaction[]", description: "Array of transaction objects." },
    ],
    exampleCurl: `curl ${REST_URL}/blocks/latest`,
    exampleResponse: {
      number: 48210553,
      hash: "0xabc123...def",
      parent_hash: "0x987654...fed",
      timestamp: 1711843200100,
      validator: "dina1val0...xyz",
      tx_count: 0,
      transactions: [],
    },
  },
  {
    id: "get-transaction",
    method: "GET",
    path: "/transactions/:hash",
    description: "Returns the details of a transaction by its hash, including status and block inclusion information.",
    pathParams: [
      { name: "hash", type: "string", description: "The transaction hash." },
    ],
    responseFields: [
      { name: "hash", type: "string", description: "Transaction hash." },
      { name: "from", type: "string", description: "Sender address." },
      { name: "to", type: "string", description: "Recipient address." },
      { name: "value", type: "string", description: "Amount in micro-USDC." },
      { name: "nonce", type: "number", description: "Sender nonce." },
      { name: "gas_used", type: "number", description: "Gas consumed." },
      { name: "gas_price", type: "string", description: "Gas price in micro-USDC." },
      { name: "block_number", type: "number", description: "Block the tx was included in." },
      { name: "status", type: "string", description: '"success" or "failed".' },
    ],
    exampleCurl: `curl ${REST_URL}/transactions/0xtx1...abc`,
    exampleResponse: {
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
  {
    id: "get-peers",
    method: "GET",
    path: "/peers",
    description: "Returns the list of peers currently connected to this node.",
    responseFields: [
      { name: "count", type: "number", description: "Total connected peers." },
      { name: "peers", type: "Peer[]", description: "Array of peer objects." },
      { name: "peers[].id", type: "string", description: "Peer node ID." },
      { name: "peers[].address", type: "string", description: "Peer IP address and port." },
      { name: "peers[].latency_ms", type: "number", description: "Latency to peer in milliseconds." },
    ],
    exampleCurl: `curl ${REST_URL}/peers`,
    exampleResponse: {
      count: 3,
      peers: [
        { id: "node-1", address: "35.193.28.189:26656", latency_ms: 12 },
        { id: "node-2", address: "136.115.115.11:26656", latency_ms: 8 },
      ],
    },
  },
  {
    id: "get-devices",
    method: "GET",
    path: "/devices",
    description: "Returns the list of registered IoT devices that participate in data attestation on the network.",
    responseFields: [
      { name: "count", type: "number", description: "Total registered devices." },
      { name: "devices", type: "Device[]", description: "Array of device objects." },
      { name: "devices[].device_id", type: "string", description: "Unique device identifier." },
      { name: "devices[].owner", type: "string", description: "Owner wallet address." },
      { name: "devices[].registered_at", type: "string", description: "ISO 8601 registration timestamp." },
      { name: "devices[].last_attestation", type: "string", description: "ISO 8601 timestamp of last attestation." },
    ],
    exampleCurl: `curl ${REST_URL}/devices`,
    exampleResponse: {
      count: 1,
      devices: [
        {
          device_id: "cog-seed-001",
          owner: "dina1qxyz...abc",
          registered_at: "2025-10-01T12:00:00Z",
          last_attestation: "2025-12-15T08:30:00Z",
        },
      ],
    },
  },
  {
    id: "post-faucet",
    method: "POST",
    path: "/faucet/:address",
    description: "Requests testnet USDC from the faucet. Each address can request up to 100 USDC per day on testnet. Not available on mainnet.",
    pathParams: [
      { name: "address", type: "string", description: "The account address to fund." },
    ],
    responseFields: [
      { name: "tx_hash", type: "string", description: "Transaction hash of the faucet transfer." },
      { name: "amount", type: "string", description: "Amount sent in micro-USDC." },
      { name: "remaining_daily", type: "string", description: "Remaining daily allowance in micro-USDC." },
    ],
    exampleCurl: `curl -X POST ${REST_URL}/faucet/dina1qxyz...abc`,
    exampleResponse: {
      tx_hash: "0xfaucet_tx...abc",
      amount: "100000000",
      remaining_daily: "0",
    },
  },
];

function MethodBadge({ method }: { method: string }) {
  const colors: Record<string, string> = {
    GET: "bg-green-500/10 text-green-400 ring-green-500/20",
    POST: "bg-blue-500/10 text-blue-400 ring-blue-500/20",
    PUT: "bg-amber-500/10 text-amber-400 ring-amber-500/20",
    DELETE: "bg-red-500/10 text-red-400 ring-red-500/20",
  };

  return (
    <span className={`rounded-full px-2.5 py-0.5 text-xs font-bold ring-1 ring-inset ${colors[method] || colors.GET}`}>
      {method}
    </span>
  );
}

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

function FieldTable({ fields, title }: { fields: { name: string; type: string; description: string }[]; title: string }) {
  return (
    <div>
      <h4 className="mb-3 text-sm font-semibold uppercase tracking-wider text-slate-500">{title}</h4>
      <div className="overflow-x-auto rounded-lg border border-slate-700/60">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-slate-700/60 bg-slate-800/40">
              <th className="px-4 py-2.5 text-left font-semibold text-slate-300">Field</th>
              <th className="px-4 py-2.5 text-left font-semibold text-slate-300">Type</th>
              <th className="px-4 py-2.5 text-left font-semibold text-slate-300">Description</th>
            </tr>
          </thead>
          <tbody>
            {fields.map((f) => (
              <tr key={f.name} className="border-b border-slate-800/40">
                <td className="px-4 py-2.5">
                  <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs font-mono text-green-400">{f.name}</code>
                </td>
                <td className="px-4 py-2.5">
                  <code className="text-xs font-mono text-purple-400">{f.type}</code>
                </td>
                <td className="px-4 py-2.5 text-slate-400">{f.description}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function EndpointSection({ doc }: { doc: RestEndpointDoc }) {
  return (
    <section id={doc.id} className="scroll-mt-24">
      <div className="rounded-xl border border-slate-800/60 bg-slate-900/30 p-6 backdrop-blur-sm">
        {/* Header */}
        <div className="mb-4 flex items-center gap-3">
          <MethodBadge method={doc.method} />
          <h3 className="text-lg font-bold font-mono text-white">{doc.path}</h3>
        </div>

        <p className="mb-6 text-sm leading-relaxed text-slate-400">{doc.description}</p>

        {/* Path Parameters */}
        {doc.pathParams && doc.pathParams.length > 0 && (
          <div className="mb-6">
            <FieldTable fields={doc.pathParams} title="Path Parameters" />
          </div>
        )}

        {/* Query Parameters */}
        {doc.queryParams && doc.queryParams.length > 0 && (
          <div className="mb-6">
            <FieldTable
              fields={doc.queryParams.map((q) => ({
                name: q.name,
                type: q.type,
                description: `${q.required ? "(required) " : "(optional) "}${q.description}`,
              }))}
              title="Query Parameters"
            />
          </div>
        )}

        {/* Request Body */}
        {doc.requestBody && (
          <div className="mb-6">
            <FieldTable fields={doc.requestBody.fields} title="Request Body" />
          </div>
        )}

        {/* Response Fields */}
        <div className="mb-6">
          <FieldTable fields={doc.responseFields} title="Response Body" />
        </div>

        {/* Example Request */}
        <div className="mb-4">
          <h4 className="mb-3 text-sm font-semibold uppercase tracking-wider text-slate-500">Example Request</h4>
          <CodeBlock title="curl">{doc.exampleCurl}</CodeBlock>
        </div>

        {/* Example Response */}
        <div>
          <h4 className="mb-3 text-sm font-semibold uppercase tracking-wider text-slate-500">Example Response</h4>
          <CodeBlock title="200 OK">{JSON.stringify(doc.exampleResponse, null, 2)}</CodeBlock>
        </div>
      </div>
    </section>
  );
}

export default function RestApiPage() {
  // Cross-reference the registered REST endpoints
  const registeredEndpoints = Object.values(API_ENDPOINTS.rest);

  return (
    <div>
      {/* Header */}
      <div className="mb-10">
        <div className="mb-3 flex items-center gap-2">
          <span className="rounded-full bg-green-500/10 px-2.5 py-0.5 text-xs font-medium text-green-400 ring-1 ring-inset ring-green-500/20">
            API Reference
          </span>
        </div>
        <h1 className="text-4xl font-extrabold tracking-tight">REST API</h1>
        <p className="mt-3 text-lg leading-relaxed text-slate-400">
          The Dina Network REST API provides a conventional HTTP interface for querying chain state and submitting
          transactions. All responses are JSON.
        </p>
      </div>

      {/* Base URL */}
      <div className="mb-10 rounded-xl border border-slate-800/60 bg-slate-900/40 p-5">
        <h2 className="mb-2 text-sm font-semibold uppercase tracking-wider text-slate-500">Base URL</h2>
        <code className="text-sm font-mono text-green-400">{REST_URL}</code>
        <p className="mt-2 text-sm text-slate-500">
          All endpoints return JSON with{" "}
          <code className="rounded bg-slate-800 px-1 py-0.5 text-xs text-slate-300">Content-Type: application/json</code>.
          Error responses follow the standard{" "}
          <a href="/docs/api/errors" className="text-blue-400 underline decoration-blue-400/30 hover:decoration-blue-400">
            error format
          </a>
          .
        </p>
      </div>

      {/* Quick Nav */}
      <div className="mb-10 rounded-xl border border-slate-800/60 bg-slate-900/40 p-5">
        <h2 className="mb-4 text-sm font-semibold uppercase tracking-wider text-slate-500">Endpoints</h2>
        <div className="space-y-1.5">
          {ENDPOINTS.map((doc) => (
            <a
              key={doc.id}
              href={`#${doc.id}`}
              className="group flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors hover:bg-slate-800/60"
            >
              <MethodBadge method={doc.method} />
              <span className="font-mono text-slate-300 group-hover:text-white">{doc.path}</span>
              <span className="hidden text-slate-600 sm:inline">--</span>
              <span className="hidden text-slate-500 sm:inline">{doc.description.slice(0, 50)}...</span>
            </a>
          ))}
        </div>
      </div>

      {/* Registered Endpoints from Constants */}
      <div className="mb-10 rounded-xl border border-slate-800/60 bg-slate-900/40 p-5">
        <h2 className="mb-4 text-sm font-semibold uppercase tracking-wider text-slate-500">Registered Endpoints</h2>
        <p className="mb-3 text-sm text-slate-500">
          These {registeredEndpoints.length} endpoints are registered in the Dina Network REST interface:
        </p>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-700/60">
                <th className="px-4 py-2 text-left font-semibold text-slate-300">Method</th>
                <th className="px-4 py-2 text-left font-semibold text-slate-300">Path</th>
                <th className="px-4 py-2 text-left font-semibold text-slate-300">Description</th>
              </tr>
            </thead>
            <tbody>
              {registeredEndpoints.map((ep) => (
                <tr key={ep.path} className="border-b border-slate-800/40">
                  <td className="px-4 py-2">
                    <MethodBadge method={ep.method} />
                  </td>
                  <td className="px-4 py-2">
                    <code className="text-xs font-mono text-slate-300">{ep.path}</code>
                  </td>
                  <td className="px-4 py-2 text-slate-400">{ep.description}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Endpoint Sections */}
      <div className="space-y-8">
        {ENDPOINTS.map((doc) => (
          <EndpointSection key={doc.id} doc={doc} />
        ))}
      </div>
    </div>
  );
}
