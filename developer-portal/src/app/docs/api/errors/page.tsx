interface ErrorCodeDoc {
  code: number;
  message: string;
  description: string;
  resolution: string;
  example?: Record<string, unknown>;
}

const STANDARD_ERRORS: ErrorCodeDoc[] = [
  {
    code: -32700,
    message: "Parse error",
    description:
      "The server received invalid JSON. The request body could not be parsed as valid JSON-RPC.",
    resolution:
      "Verify that your request body is valid JSON. Check for missing commas, unmatched brackets, or encoding issues. Use a JSON validator before sending.",
    example: {
      jsonrpc: "2.0",
      error: { code: -32700, message: "Parse error: unexpected token at position 42" },
      id: null,
    },
  },
  {
    code: -32600,
    message: "Invalid request",
    description:
      'The JSON is valid but does not conform to the JSON-RPC 2.0 specification. Missing required fields like "jsonrpc", "method", or "id".',
    resolution:
      'Ensure your request includes "jsonrpc": "2.0", a "method" string, a "params" array or object, and a numeric or string "id".',
    example: {
      jsonrpc: "2.0",
      error: { code: -32600, message: 'Invalid request: missing "method" field' },
      id: null,
    },
  },
  {
    code: -32601,
    message: "Method not found",
    description:
      "The requested RPC method does not exist or is not available on this node.",
    resolution:
      "Check the method name for typos. Refer to the JSON-RPC API reference for the full list of supported methods. Method names are case-sensitive.",
    example: {
      jsonrpc: "2.0",
      error: { code: -32601, message: 'Method not found: "dina_getBlocks"' },
      id: 1,
    },
  },
  {
    code: -32602,
    message: "Invalid params",
    description:
      "The method parameters are invalid -- wrong type, missing required parameters, or extra unexpected parameters.",
    resolution:
      "Check the parameter types and order against the API reference. Ensure addresses are valid strings and numbers are not passed as strings where integers are expected.",
    example: {
      jsonrpc: "2.0",
      error: { code: -32602, message: "Invalid params: expected string for 'address', got number" },
      id: 1,
    },
  },
  {
    code: -32603,
    message: "Internal error",
    description:
      "An unexpected error occurred on the server. This is a catch-all for errors that do not fit other categories.",
    resolution:
      "Retry the request after a brief delay. If the error persists, check the network status page or contact support. Include the request ID when reporting.",
    example: {
      jsonrpc: "2.0",
      error: { code: -32603, message: "Internal error: database connection timeout" },
      id: 1,
    },
  },
];

const DINA_ERRORS: ErrorCodeDoc[] = [
  {
    code: 1001,
    message: "Account not found",
    description:
      "The specified account address does not exist on the chain. The account has never received a transaction or been created.",
    resolution:
      "Verify the address is correct and on the right network (testnet vs mainnet). Fund the account via the faucet on testnet, or send USDC from an existing account.",
    example: {
      jsonrpc: "2.0",
      error: { code: 1001, message: "Account not found: dina1qxyz...abc" },
      id: 1,
    },
  },
  {
    code: 1002,
    message: "Insufficient balance",
    description:
      "The sender account does not have enough USDC to cover the transfer amount plus gas fees.",
    resolution:
      "Check the account balance with dina_getBalance. Ensure the total of the transfer value plus estimated gas does not exceed the available balance. On testnet, use the faucet to top up.",
    example: {
      jsonrpc: "2.0",
      error: {
        code: 1002,
        message: "Insufficient balance: required 10000000, available 5000000",
      },
      id: 1,
    },
  },
  {
    code: 1003,
    message: "Invalid nonce",
    description:
      "The transaction nonce does not match the expected next nonce for the account. This usually means a transaction was already submitted with this nonce, or the nonce is too far ahead.",
    resolution:
      "Query the current account nonce with dina_getAccount and use the returned nonce value for your next transaction. If you are sending multiple transactions in parallel, increment the nonce manually for each one.",
    example: {
      jsonrpc: "2.0",
      error: { code: 1003, message: "Invalid nonce: expected 43, got 42" },
      id: 1,
    },
  },
  {
    code: 1004,
    message: "Invalid signature",
    description:
      "The transaction signature is invalid. The signature does not match the sender address, or the signed data is malformed.",
    resolution:
      "Verify that you are signing the correct transaction payload with the correct private key. Ensure the chain ID in the transaction matches the network you are connected to. Use the SDK signing utilities to avoid encoding issues.",
    example: {
      jsonrpc: "2.0",
      error: { code: 1004, message: "Invalid signature: verification failed for sender dina1qxyz...abc" },
      id: 1,
    },
  },
  {
    code: 1005,
    message: "Transaction too large",
    description:
      "The raw transaction size exceeds the maximum allowed size. The limit is 256 KB per transaction.",
    resolution:
      "Reduce the transaction data payload. If deploying a contract, consider splitting it into smaller modules. For batch transfers, reduce the number of recipients per batch.",
    example: {
      jsonrpc: "2.0",
      error: { code: 1005, message: "Transaction too large: 312000 bytes exceeds 256000 byte limit" },
      id: 1,
    },
  },
  {
    code: 1006,
    message: "Gas limit exceeded",
    description:
      "The transaction requires more gas than the specified gas limit, or exceeds the per-block gas cap.",
    resolution:
      "Increase the gas limit in your transaction parameters. Use dina_estimateGas to get an accurate estimate before submitting. For complex contract calls, the estimate includes a safety margin.",
    example: {
      jsonrpc: "2.0",
      error: { code: 1006, message: "Gas limit exceeded: required 150000, limit 21000" },
      id: 1,
    },
  },
  {
    code: 1007,
    message: "Contract execution failed",
    description:
      "The smart contract execution reverted or panicked. The transaction was included in a block but the state changes were rolled back.",
    resolution:
      "Check the contract code for revert conditions. The error message may include a revert reason string from the contract. Test your contract call with dina_estimateGas first to catch failures before submitting.",
    example: {
      jsonrpc: "2.0",
      error: {
        code: 1007,
        message: 'Contract execution failed: revert "Insufficient allowance"',
      },
      id: 1,
    },
  },
  {
    code: 1008,
    message: "Rate limit exceeded",
    description:
      "The client has sent too many requests in a short period. The node enforces rate limits to maintain service quality.",
    resolution:
      "Implement exponential backoff in your client. Wait for the duration specified in the Retry-After header before sending new requests. For high-throughput applications, run your own validator node.",
    example: {
      jsonrpc: "2.0",
      error: { code: 1008, message: "Rate limit exceeded: 100 requests per second" },
      id: 1,
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

function ErrorSection({ error }: { error: ErrorCodeDoc }) {
  return (
    <section id={`error-${error.code}`} className="scroll-mt-24">
      <div className="rounded-xl border border-slate-800/60 bg-slate-900/30 p-6 backdrop-blur-sm">
        <div className="mb-4 flex items-center gap-3">
          <span className="rounded-lg bg-red-500/10 px-2.5 py-1 text-sm font-bold font-mono text-red-400 ring-1 ring-inset ring-red-500/20">
            {error.code}
          </span>
          <h3 className="text-lg font-bold text-white">{error.message}</h3>
        </div>

        <p className="mb-4 text-sm leading-relaxed text-slate-400">{error.description}</p>

        <div className="mb-4 rounded-lg border border-slate-700/60 bg-slate-800/30 p-4">
          <h4 className="mb-2 text-xs font-semibold uppercase tracking-wider text-slate-500">How to resolve</h4>
          <p className="text-sm text-slate-300">{error.resolution}</p>
        </div>

        {error.example && (
          <div>
            <h4 className="mb-2 text-xs font-semibold uppercase tracking-wider text-slate-500">Example Response</h4>
            <CodeBlock title="JSON">{JSON.stringify(error.example, null, 2)}</CodeBlock>
          </div>
        )}
      </div>
    </section>
  );
}

export default function ErrorCodesPage() {
  const allErrors = [...STANDARD_ERRORS, ...DINA_ERRORS];

  return (
    <div>
      {/* Header */}
      <div className="mb-10">
        <div className="mb-3 flex items-center gap-2">
          <span className="rounded-full bg-red-500/10 px-2.5 py-0.5 text-xs font-medium text-red-400 ring-1 ring-inset ring-red-500/20">
            API Reference
          </span>
        </div>
        <h1 className="text-4xl font-extrabold tracking-tight">Error Codes</h1>
        <p className="mt-3 text-lg leading-relaxed text-slate-400">
          Complete reference for all error codes returned by the Dina Network JSON-RPC and REST APIs.
          Error responses follow the JSON-RPC 2.0 error object format.
        </p>
      </div>

      {/* Error Format */}
      <div className="mb-10 rounded-xl border border-slate-800/60 bg-slate-900/40 p-5">
        <h2 className="mb-3 text-sm font-semibold uppercase tracking-wider text-slate-500">Error Response Format</h2>
        <p className="mb-4 text-sm text-slate-400">
          All error responses include a standard error object with a numeric code and human-readable message.
          REST API errors use the same code and message format in the response body with the appropriate HTTP status code.
        </p>
        <CodeBlock title="JSON">
          {JSON.stringify(
            {
              jsonrpc: "2.0",
              error: {
                code: "<number>",
                message: "<string>",
              },
              id: 1,
            },
            null,
            2,
          ).replace(/"/g, '"')}
        </CodeBlock>
      </div>

      {/* Summary Table */}
      <div className="mb-10 rounded-xl border border-slate-800/60 bg-slate-900/40 p-5">
        <h2 className="mb-4 text-sm font-semibold uppercase tracking-wider text-slate-500">All Error Codes</h2>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-700/60">
                <th className="px-4 py-2.5 text-left font-semibold text-slate-300">Code</th>
                <th className="px-4 py-2.5 text-left font-semibold text-slate-300">Message</th>
                <th className="px-4 py-2.5 text-left font-semibold text-slate-300">Category</th>
              </tr>
            </thead>
            <tbody>
              {STANDARD_ERRORS.map((e) => (
                <tr key={e.code} className="border-b border-slate-800/40">
                  <td className="px-4 py-2.5">
                    <a href={`#error-${e.code}`} className="font-mono text-red-400 hover:text-red-300">
                      {e.code}
                    </a>
                  </td>
                  <td className="px-4 py-2.5 text-slate-300">{e.message}</td>
                  <td className="px-4 py-2.5">
                    <span className="rounded-full bg-slate-700/40 px-2 py-0.5 text-xs text-slate-400">JSON-RPC Standard</span>
                  </td>
                </tr>
              ))}
              {DINA_ERRORS.map((e) => (
                <tr key={e.code} className="border-b border-slate-800/40">
                  <td className="px-4 py-2.5">
                    <a href={`#error-${e.code}`} className="font-mono text-red-400 hover:text-red-300">
                      {e.code}
                    </a>
                  </td>
                  <td className="px-4 py-2.5 text-slate-300">{e.message}</td>
                  <td className="px-4 py-2.5">
                    <span className="rounded-full bg-blue-500/10 px-2 py-0.5 text-xs text-blue-400">Dina Network</span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Standard JSON-RPC Errors */}
      <div className="mb-10">
        <h2 className="mb-6 text-2xl font-bold">Standard JSON-RPC Errors</h2>
        <p className="mb-6 text-sm text-slate-400">
          These error codes are defined by the{" "}
          <a
            href="https://www.jsonrpc.org/specification#error_object"
            target="_blank"
            rel="noopener noreferrer"
            className="text-blue-400 underline decoration-blue-400/30 hover:decoration-blue-400"
          >
            JSON-RPC 2.0 specification
          </a>
          . They indicate protocol-level issues with the request itself.
        </p>

        <div className="space-y-6">
          {STANDARD_ERRORS.map((e) => (
            <ErrorSection key={e.code} error={e} />
          ))}
        </div>
      </div>

      {/* Dina Network Errors */}
      <div className="mb-10">
        <h2 className="mb-6 text-2xl font-bold">Dina Network Errors</h2>
        <p className="mb-6 text-sm text-slate-400">
          These error codes are specific to the Dina Network and indicate application-level issues with transactions,
          accounts, or contracts.
        </p>

        <div className="space-y-6">
          {DINA_ERRORS.map((e) => (
            <ErrorSection key={e.code} error={e} />
          ))}
        </div>
      </div>

      {/* HTTP Status Codes */}
      <div className="rounded-xl border border-slate-800/60 bg-slate-900/40 p-5">
        <h2 className="mb-4 text-sm font-semibold uppercase tracking-wider text-slate-500">REST API HTTP Status Codes</h2>
        <p className="mb-4 text-sm text-slate-400">
          The REST API maps error codes to standard HTTP status codes:
        </p>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-700/60">
                <th className="px-4 py-2 text-left font-semibold text-slate-300">HTTP Status</th>
                <th className="px-4 py-2 text-left font-semibold text-slate-300">Error Codes</th>
                <th className="px-4 py-2 text-left font-semibold text-slate-300">Description</th>
              </tr>
            </thead>
            <tbody>
              <tr className="border-b border-slate-800/40">
                <td className="px-4 py-2 font-mono text-green-400">200</td>
                <td className="px-4 py-2 text-slate-400">--</td>
                <td className="px-4 py-2 text-slate-400">Successful request.</td>
              </tr>
              <tr className="border-b border-slate-800/40">
                <td className="px-4 py-2 font-mono text-amber-400">400</td>
                <td className="px-4 py-2 text-slate-400">-32700, -32600, -32602, 1003, 1004, 1005</td>
                <td className="px-4 py-2 text-slate-400">Bad request -- malformed or invalid parameters.</td>
              </tr>
              <tr className="border-b border-slate-800/40">
                <td className="px-4 py-2 font-mono text-amber-400">404</td>
                <td className="px-4 py-2 text-slate-400">-32601, 1001</td>
                <td className="px-4 py-2 text-slate-400">Resource or method not found.</td>
              </tr>
              <tr className="border-b border-slate-800/40">
                <td className="px-4 py-2 font-mono text-amber-400">409</td>
                <td className="px-4 py-2 text-slate-400">1002, 1006</td>
                <td className="px-4 py-2 text-slate-400">Conflict -- insufficient balance or gas.</td>
              </tr>
              <tr className="border-b border-slate-800/40">
                <td className="px-4 py-2 font-mono text-amber-400">422</td>
                <td className="px-4 py-2 text-slate-400">1007</td>
                <td className="px-4 py-2 text-slate-400">Unprocessable entity -- contract execution failed.</td>
              </tr>
              <tr className="border-b border-slate-800/40">
                <td className="px-4 py-2 font-mono text-amber-400">429</td>
                <td className="px-4 py-2 text-slate-400">1008</td>
                <td className="px-4 py-2 text-slate-400">Too many requests -- rate limit exceeded.</td>
              </tr>
              <tr className="border-b border-slate-800/40">
                <td className="px-4 py-2 font-mono text-red-400">500</td>
                <td className="px-4 py-2 text-slate-400">-32603</td>
                <td className="px-4 py-2 text-slate-400">Internal server error.</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
