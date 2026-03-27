import { TESTNET_CONFIG } from "@/lib/constants";

const WS_URL = `ws://${TESTNET_CONFIG.validators[0].ip}:${TESTNET_CONFIG.validators[0].rpcPort}/ws`;

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

const SUBSCRIPTIONS = [
  {
    id: "newBlocks",
    method: 'dina_subscribe("newBlocks")',
    description:
      "Streams newly finalized blocks in real time. Each message contains the full block object with header and transaction list. Useful for building block explorers, indexers, and real-time dashboards.",
    params: [],
    subscribeRequest: JSON.stringify(
      { jsonrpc: "2.0", method: "dina_subscribe", params: ["newBlocks"], id: 1 },
      null,
      2,
    ),
    subscribeResponse: JSON.stringify(
      { jsonrpc: "2.0", id: 1, result: "sub_abc123" },
      null,
      2,
    ),
    notificationExample: JSON.stringify(
      {
        jsonrpc: "2.0",
        method: "dina_subscription",
        params: {
          subscription: "sub_abc123",
          result: {
            number: 48210554,
            hash: "0xdef456...abc",
            parent_hash: "0xabc123...def",
            timestamp: 1711843200200,
            validator: "dina1val1...xyz",
            tx_count: 5,
            transactions: [
              {
                hash: "0xtx2...",
                from: "dina1qxyz...abc",
                to: "dina1qabc...xyz",
                value: "2000000",
              },
            ],
          },
        },
      },
      null,
      2,
    ),
  },
  {
    id: "pendingTransactions",
    method: 'dina_subscribe("pendingTransactions")',
    description:
      "Streams transactions as they enter the mempool, before they are included in a block. Useful for front-running detection, gas estimation, and mempool monitoring.",
    params: [],
    subscribeRequest: JSON.stringify(
      { jsonrpc: "2.0", method: "dina_subscribe", params: ["pendingTransactions"], id: 2 },
      null,
      2,
    ),
    subscribeResponse: JSON.stringify(
      { jsonrpc: "2.0", id: 2, result: "sub_def456" },
      null,
      2,
    ),
    notificationExample: JSON.stringify(
      {
        jsonrpc: "2.0",
        method: "dina_subscription",
        params: {
          subscription: "sub_def456",
          result: {
            hash: "0xpending_tx...abc",
            from: "dina1qxyz...abc",
            to: "dina1qabc...xyz",
            value: "10000000",
            nonce: 43,
            gas_limit: 21000,
          },
        },
      },
      null,
      2,
    ),
  },
  {
    id: "logs",
    method: 'dina_subscribe("logs", {address, topics})',
    description:
      "Streams contract event logs matching the specified filter. Filter by contract address and/or event topics. Useful for tracking token transfers, contract state changes, and application events.",
    params: [
      { name: "address", type: "string", description: "Contract address to filter events from. Optional." },
      { name: "topics", type: "string[]", description: "Array of event topic strings to match. Optional." },
    ],
    subscribeRequest: JSON.stringify(
      {
        jsonrpc: "2.0",
        method: "dina_subscribe",
        params: ["logs", { address: "dina1contract...xyz", topics: ["Transfer"] }],
        id: 3,
      },
      null,
      2,
    ),
    subscribeResponse: JSON.stringify(
      { jsonrpc: "2.0", id: 3, result: "sub_ghi789" },
      null,
      2,
    ),
    notificationExample: JSON.stringify(
      {
        jsonrpc: "2.0",
        method: "dina_subscription",
        params: {
          subscription: "sub_ghi789",
          result: {
            address: "dina1contract...xyz",
            topics: ["Transfer"],
            data: {
              from: "dina1qxyz...abc",
              to: "dina1qabc...xyz",
              amount: "5000000",
            },
            block_number: 48210554,
            transaction_hash: "0xtx2...",
            log_index: 0,
          },
        },
      },
      null,
      2,
    ),
  },
];

const JS_CONNECTION_EXAMPLE = `const ws = new WebSocket("${WS_URL}");

ws.onopen = () => {
  console.log("Connected to Dina Network");

  // Subscribe to new blocks
  ws.send(JSON.stringify({
    jsonrpc: "2.0",
    method: "dina_subscribe",
    params: ["newBlocks"],
    id: 1,
  }));
};

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);

  if (msg.id) {
    // Subscription confirmation
    console.log("Subscribed:", msg.result);
  } else if (msg.method === "dina_subscription") {
    // Incoming notification
    const block = msg.params.result;
    console.log(\`Block #\${block.number} — \${block.tx_count} txs\`);
  }
};

ws.onerror = (err) => {
  console.error("WebSocket error:", err);
};

ws.onclose = () => {
  console.log("Disconnected");
};`;

const RECONNECT_EXAMPLE = `class DinaWebSocket {
  constructor(url, subscriptions = []) {
    this.url = url;
    this.subscriptions = subscriptions;
    this.reconnectDelay = 1000;
    this.maxReconnectDelay = 30000;
    this.connect();
  }

  connect() {
    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      console.log("Connected");
      this.reconnectDelay = 1000; // Reset on success

      // Re-subscribe after reconnection
      this.subscriptions.forEach((sub, i) => {
        this.ws.send(JSON.stringify({
          jsonrpc: "2.0",
          method: "dina_subscribe",
          params: sub,
          id: i + 1,
        }));
      });
    };

    this.ws.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      if (msg.method === "dina_subscription") {
        this.onNotification(msg.params);
      }
    };

    this.ws.onclose = () => {
      console.log(\`Reconnecting in \${this.reconnectDelay}ms...\`);
      setTimeout(() => this.connect(), this.reconnectDelay);
      this.reconnectDelay = Math.min(
        this.reconnectDelay * 2,
        this.maxReconnectDelay,
      );
    };

    this.ws.onerror = () => {
      this.ws.close();
    };
  }

  onNotification(params) {
    // Override this method to handle notifications
    console.log("Notification:", params);
  }

  unsubscribe(subscriptionId) {
    this.ws.send(JSON.stringify({
      jsonrpc: "2.0",
      method: "dina_unsubscribe",
      params: [subscriptionId],
      id: 99,
    }));
  }
}

// Usage
const client = new DinaWebSocket("${WS_URL}", [
  ["newBlocks"],
  ["pendingTransactions"],
  ["logs", { address: "dina1contract...xyz", topics: ["Transfer"] }],
]);

client.onNotification = (params) => {
  console.log("Event:", params.subscription, params.result);
};`;

const UNSUBSCRIBE_EXAMPLE = JSON.stringify(
  { jsonrpc: "2.0", method: "dina_unsubscribe", params: ["sub_abc123"], id: 99 },
  null,
  2,
);

const UNSUBSCRIBE_RESPONSE = JSON.stringify(
  { jsonrpc: "2.0", id: 99, result: true },
  null,
  2,
);

export default function WebSocketApiPage() {
  return (
    <div>
      {/* Header */}
      <div className="mb-10">
        <div className="mb-3 flex items-center gap-2">
          <span className="rounded-full bg-purple-500/10 px-2.5 py-0.5 text-xs font-medium text-purple-400 ring-1 ring-inset ring-purple-500/20">
            API Reference
          </span>
        </div>
        <h1 className="text-4xl font-extrabold tracking-tight">WebSocket API</h1>
        <p className="mt-3 text-lg leading-relaxed text-slate-400">
          Subscribe to real-time events from the Dina Network. The WebSocket API pushes new blocks, pending transactions,
          and contract events to your client as they happen.
        </p>
      </div>

      {/* Connection URL */}
      <div className="mb-10 rounded-xl border border-slate-800/60 bg-slate-900/40 p-5">
        <h2 className="mb-2 text-sm font-semibold uppercase tracking-wider text-slate-500">Connection URL</h2>
        <code className="text-sm font-mono text-green-400">{WS_URL}</code>
        <p className="mt-2 text-sm text-slate-500">
          Connect using any WebSocket client. The server speaks JSON-RPC 2.0 over the WebSocket transport.
          Each validator node exposes a WebSocket endpoint on port {TESTNET_CONFIG.validators[0].rpcPort}.
        </p>
      </div>

      {/* Quick Start */}
      <div className="mb-10">
        <h2 className="mb-4 text-2xl font-bold">Quick Start</h2>
        <p className="mb-4 text-sm text-slate-400">
          Connect to the WebSocket endpoint and subscribe to events using standard JSON-RPC calls.
        </p>
        <CodeBlock title="JavaScript">{JS_CONNECTION_EXAMPLE}</CodeBlock>
      </div>

      {/* Subscription Methods */}
      <div className="mb-10">
        <h2 className="mb-6 text-2xl font-bold">Subscription Methods</h2>

        <div className="space-y-8">
          {SUBSCRIPTIONS.map((sub) => (
            <section key={sub.id} id={sub.id} className="scroll-mt-24">
              <div className="rounded-xl border border-slate-800/60 bg-slate-900/30 p-6 backdrop-blur-sm">
                <div className="mb-4 flex items-center gap-3">
                  <h3 className="text-xl font-bold text-white font-mono">{sub.method}</h3>
                  <span className="rounded-full bg-purple-500/10 px-2.5 py-0.5 text-xs font-medium text-purple-400 ring-1 ring-inset ring-purple-500/20">
                    Subscription
                  </span>
                </div>

                <p className="mb-6 text-sm leading-relaxed text-slate-400">{sub.description}</p>

                {/* Filter Parameters */}
                {sub.params.length > 0 && (
                  <div className="mb-6">
                    <h4 className="mb-3 text-sm font-semibold uppercase tracking-wider text-slate-500">Filter Parameters</h4>
                    <div className="overflow-x-auto rounded-lg border border-slate-700/60">
                      <table className="w-full text-sm">
                        <thead>
                          <tr className="border-b border-slate-700/60 bg-slate-800/40">
                            <th className="px-4 py-2 text-left font-semibold text-slate-300">Parameter</th>
                            <th className="px-4 py-2 text-left font-semibold text-slate-300">Type</th>
                            <th className="px-4 py-2 text-left font-semibold text-slate-300">Description</th>
                          </tr>
                        </thead>
                        <tbody>
                          {sub.params.map((p) => (
                            <tr key={p.name} className="border-b border-slate-800/40">
                              <td className="px-4 py-2">
                                <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs font-mono text-blue-400">{p.name}</code>
                              </td>
                              <td className="px-4 py-2">
                                <code className="text-xs font-mono text-purple-400">{p.type}</code>
                              </td>
                              <td className="px-4 py-2 text-slate-400">{p.description}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  </div>
                )}

                {/* Subscribe Request */}
                <div className="mb-4">
                  <h4 className="mb-3 text-sm font-semibold uppercase tracking-wider text-slate-500">Subscribe Request</h4>
                  <CodeBlock title="JSON">{sub.subscribeRequest}</CodeBlock>
                </div>

                {/* Subscribe Response */}
                <div className="mb-4">
                  <h4 className="mb-3 text-sm font-semibold uppercase tracking-wider text-slate-500">Confirmation Response</h4>
                  <CodeBlock title="JSON">{sub.subscribeResponse}</CodeBlock>
                </div>

                {/* Notification Example */}
                <div>
                  <h4 className="mb-3 text-sm font-semibold uppercase tracking-wider text-slate-500">Notification Message</h4>
                  <CodeBlock title="JSON">{sub.notificationExample}</CodeBlock>
                </div>
              </div>
            </section>
          ))}
        </div>
      </div>

      {/* Unsubscribe */}
      <div className="mb-10" id="unsubscribe">
        <h2 className="mb-4 text-2xl font-bold">Unsubscribing</h2>
        <p className="mb-4 text-sm text-slate-400">
          To stop receiving notifications for a subscription, send a{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-xs font-mono text-blue-400">dina_unsubscribe</code>{" "}
          request with the subscription ID.
        </p>

        <div className="space-y-4">
          <CodeBlock title="Request">{UNSUBSCRIBE_EXAMPLE}</CodeBlock>
          <CodeBlock title="Response">{UNSUBSCRIBE_RESPONSE}</CodeBlock>
        </div>
      </div>

      {/* Reconnection Best Practices */}
      <div className="mb-10" id="reconnection">
        <h2 className="mb-4 text-2xl font-bold">Reconnection Best Practices</h2>

        <div className="mb-6 rounded-xl border border-slate-800/60 bg-slate-900/40 p-5">
          <ul className="space-y-3 text-sm text-slate-400">
            <li className="flex gap-3">
              <span className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-blue-500/10 text-xs font-bold text-blue-400">1</span>
              <span>
                <strong className="text-slate-200">Use exponential backoff.</strong> Start with a 1-second delay and double
                it on each failure, up to a maximum of 30 seconds. Reset the delay on successful connection.
              </span>
            </li>
            <li className="flex gap-3">
              <span className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-blue-500/10 text-xs font-bold text-blue-400">2</span>
              <span>
                <strong className="text-slate-200">Re-subscribe after reconnection.</strong> WebSocket subscriptions are not
                persisted on the server. After reconnecting, re-send all <code className="rounded bg-slate-800 px-1 py-0.5 text-xs font-mono text-slate-300">dina_subscribe</code> requests.
              </span>
            </li>
            <li className="flex gap-3">
              <span className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-blue-500/10 text-xs font-bold text-blue-400">3</span>
              <span>
                <strong className="text-slate-200">Handle gaps.</strong> After reconnecting, query missed blocks via the
                REST or JSON-RPC API. Compare the last block you received with the current block height to identify any gaps.
              </span>
            </li>
            <li className="flex gap-3">
              <span className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-blue-500/10 text-xs font-bold text-blue-400">4</span>
              <span>
                <strong className="text-slate-200">Use heartbeats.</strong> Send a <code className="rounded bg-slate-800 px-1 py-0.5 text-xs font-mono text-slate-300">dina_blockNumber</code>{" "}
                request every 30 seconds as a ping. If no response arrives within 5 seconds, close and reconnect.
              </span>
            </li>
            <li className="flex gap-3">
              <span className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-blue-500/10 text-xs font-bold text-blue-400">5</span>
              <span>
                <strong className="text-slate-200">Connect to multiple validators.</strong> For high availability, maintain
                connections to at least two validator nodes and deduplicate events client-side using block hashes or transaction hashes.
              </span>
            </li>
          </ul>
        </div>

        <h3 className="mb-4 text-lg font-semibold">Reference Implementation</h3>
        <p className="mb-4 text-sm text-slate-400">
          A production-ready WebSocket client with automatic reconnection and re-subscription:
        </p>
        <CodeBlock title="JavaScript — DinaWebSocket class">{RECONNECT_EXAMPLE}</CodeBlock>
      </div>

      {/* Rate Limits */}
      <div className="rounded-xl border border-amber-500/20 bg-amber-500/5 p-5">
        <h3 className="mb-2 text-sm font-semibold text-amber-400">Rate Limits</h3>
        <p className="text-sm text-slate-400">
          Each WebSocket connection supports up to <strong className="text-slate-200">10 active subscriptions</strong>.
          The server will send a maximum of <strong className="text-slate-200">1,000 messages per second</strong> per connection.
          If the client falls behind, the server will buffer up to 10,000 messages before closing the connection.
          For high-throughput use cases, use multiple connections across different validator nodes.
        </p>
      </div>
    </div>
  );
}
