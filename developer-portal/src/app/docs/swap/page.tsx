import Link from "next/link";

function CodeBlock({ children, title }: { children: string; title?: string }) {
  return (
    <div className="mt-4 overflow-hidden rounded-lg border border-slate-800">
      {title && (
        <div className="border-b border-slate-800 bg-slate-900 px-4 py-2 text-xs font-medium text-slate-400">
          {title}
        </div>
      )}
      <pre className="!m-0 !rounded-none !border-0 bg-slate-800 p-4">
        <code className="text-sm leading-relaxed text-slate-200">{children}</code>
      </pre>
    </div>
  );
}

function InfoBox({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="mt-4 rounded-lg border border-blue-800/50 bg-blue-950/30 p-4">
      <p className="text-sm font-semibold text-blue-300">{title}</p>
      <div className="mt-1 text-sm text-slate-300">{children}</div>
    </div>
  );
}

export default function SwapPage() {
  return (
    <div>
      <h1 className="text-4xl font-bold tracking-tight text-white">
        DinaDEX — Decentralized Exchange
      </h1>
      <p className="mt-4 text-lg text-slate-300">
        DinaDEX is an on-chain automated market maker (AMM) built on Dina Network.
        It uses the constant product model (x * y = k) pioneered by Uniswap V2,
        but executes swaps in ~100ms instead of 12 seconds on Ethereum. Any{" "}
        <Link
          href="/docs/contracts/standards"
          className="text-blue-400 underline decoration-blue-400/30 hover:decoration-blue-400"
        >
          DRC-1 token
        </Link>{" "}
        can be traded, including bridged USDC, bridged ETH, and native ecosystem tokens.
      </p>

      {/* AMM Diagram */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">How the AMM Works</h2>
        <p className="mt-2 text-slate-300">
          Every trading pair has a liquidity pool holding reserves of both tokens.
          The product of the two reserves (x * y = k) stays constant through every
          swap, which automatically determines the exchange rate.
        </p>
        <CodeBlock title="Constant Product AMM">
          {`    Price Curve (x * y = k)

    Token B
    Reserve
      |
  500 |*
      | \\
  400 |  *
      |   \\
  300 |    *
      |      *
  200 |        *
      |           *
  100 |               *                    *
      |                       *                          *
      +----+----+----+----+----+----+----+----+----+----+---
      0   100  200  300  400  500  600  700  800  900  1000
                        Token A Reserve

  When a trader swaps Token A for Token B:
  1. Token A is added to the pool  (reserve_a increases)
  2. Token B is removed from pool  (reserve_b decreases)
  3. The new reserves still satisfy x * y = k
  4. Larger swaps move the price more (higher slippage)

  Formula (swap exact input):
    input_with_fee = input_amount * (10000 - fee_bps)
    output = (reserve_out * input_with_fee) / (reserve_in * 10000 + input_with_fee)`}
        </CodeBlock>
      </div>

      {/* Fee Structure */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Fee Structure</h2>
        <p className="mt-2 text-slate-300">
          Each swap charges a 0.30% fee, split between liquidity providers and the
          protocol:
        </p>
        <div className="mt-4 overflow-hidden rounded-lg border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900">
                <th className="px-4 py-3 text-left font-medium text-slate-300">Recipient</th>
                <th className="px-4 py-3 text-left font-medium text-slate-300">Fee</th>
                <th className="px-4 py-3 text-left font-medium text-slate-300">Description</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Liquidity Providers</td>
                <td className="px-4 py-3 font-mono text-green-400">0.25%</td>
                <td className="px-4 py-3">Earned proportionally to LP share; auto-compounded in the pool</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Protocol Treasury</td>
                <td className="px-4 py-3 font-mono text-blue-400">0.05%</td>
                <td className="px-4 py-3">Accumulated on-chain; withdrawn by the contract owner</td>
              </tr>
              <tr>
                <td className="px-4 py-3 font-semibold text-white">Total</td>
                <td className="px-4 py-3 font-mono font-semibold text-white">0.30%</td>
                <td className="px-4 py-3">30 basis points per swap</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Creating a Pool */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Creating a Pool</h2>
        <p className="mt-2 text-slate-300">
          Anyone can create a trading pair between two DRC-1 tokens. Pools are
          identified by a unique ID and indexed by the canonical token pair.
        </p>
        <CodeBlock title="Rust (contract call)">
          {`use dex_swap::DexState;

// Initialise the DEX (done once by deployer)
let mut dex = DexState::new("owner_address".to_string());

// Create a USDC / ETH pool
let event = dex.create_pool("USDC", "ETH");
// => PoolCreated { pool_id: 1, token_a: "ETH", token_b: "USDC" }

// Pool is created with 0.3% fee, empty reserves`}
        </CodeBlock>
        <CodeBlock title="JavaScript (via SDK)">
          {`import { DinaClient } from "@dina-network/sdk";

const client = new DinaClient({ rpcUrl: "http://35.184.213.248:8545" });
const wallet = client.wallet("your-private-key");

// Create a new trading pair
const tx = await wallet.callContract("dex-swap", "create_pool", {
  token_a: "USDC",
  token_b: "ETH",
});
console.log("Pool created:", tx.events[0]); // PoolCreated event`}
        </CodeBlock>
      </div>

      {/* Adding Liquidity */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Adding Liquidity</h2>
        <p className="mt-2 text-slate-300">
          Liquidity providers deposit both tokens into a pool and receive LP tokens
          representing their share. The first deposit sets the initial price ratio;
          subsequent deposits must match the current ratio.
        </p>
        <CodeBlock title="Rust">
          {`// First liquidity deposit — sets the price at 1 ETH = 2 USDC
let event = dex.add_liquidity(
    1,           // pool_id
    "alice",     // provider address
    200_000,     // 200,000 USDC (6 decimals)
    100_000,     // 100,000 ETH-equivalent
    0,           // min_lp_tokens (set > 0 for slippage protection)
);
// LP tokens minted = sqrt(200_000 * 100_000) = 141_421

// Subsequent deposit — must match 2:1 ratio
let event = dex.add_liquidity(1, "bob", 20_000, 10_000, 0);`}
        </CodeBlock>
        <CodeBlock title="JavaScript">
          {`// Add liquidity through the SDK
const tx = await wallet.callContract("dex-swap", "add_liquidity", {
  pool_id: 1,
  amount_a: 200_000,
  amount_b: 100_000,
  min_lp_tokens: 140_000, // slippage protection
});`}
        </CodeBlock>
        <InfoBox title="LP Token Formula">
          <p>
            First deposit: <code className="text-blue-300">lp_tokens = sqrt(amount_a * amount_b)</code>
          </p>
          <p className="mt-1">
            Subsequent: <code className="text-blue-300">lp_tokens = min(amount_a * total_lp / reserve_a, amount_b * total_lp / reserve_b)</code>
          </p>
        </InfoBox>
      </div>

      {/* Swapping Tokens */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Swapping Tokens</h2>
        <p className="mt-2 text-slate-300">
          Two swap modes are available: <strong>exact input</strong> (specify how much you
          send) and <strong>exact output</strong> (specify how much you want to receive).
          Both include slippage protection parameters.
        </p>
        <CodeBlock title="Swap Exact Input (Rust)">
          {`// Swap 1,000 USDC for ETH, expecting at least 490 ETH
let event = dex.swap_exact_in(
    1,          // pool_id
    "bob",      // trader
    "USDC",     // input token
    1_000,      // input amount
    490,        // min_output (slippage protection!)
);
// => Swap { output_amount: 494, ... }

// If output < 490, the transaction reverts with "slippage exceeded"`}
        </CodeBlock>
        <CodeBlock title="Swap Exact Output (Rust)">
          {`// Get exactly 500 ETH, willing to pay up to 1,100 USDC
let event = dex.swap_exact_out(
    1,          // pool_id
    "bob",      // trader
    "ETH",      // output token (what you want)
    500,        // exact output amount
    1_100,      // max_input (slippage protection!)
);
// => Swap { input_amount: 1_020, output_amount: 500, ... }`}
        </CodeBlock>
        <CodeBlock title="JavaScript">
          {`// Swap with slippage protection
const quote = await client.callView("dex-swap", "get_quote", {
  pool_id: 1,
  input_token: "USDC",
  input_amount: 1_000,
});
console.log("Expected output:", quote.output_amount);
console.log("Price impact:", quote.price_impact_bps, "bps");
console.log("Fee:", quote.fee_amount);

// Execute with 1% slippage tolerance
const minOutput = Math.floor(quote.output_amount * 0.99);
const tx = await wallet.callContract("dex-swap", "swap_exact_in", {
  pool_id: 1,
  input_token: "USDC",
  input_amount: 1_000,
  min_output: minOutput,
});`}
        </CodeBlock>
      </div>

      {/* Getting Quotes */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Getting Quotes</h2>
        <p className="mt-2 text-slate-300">
          Use <code className="text-blue-300">get_quote</code> for read-only price
          estimates before executing a swap. It returns the expected output, fee
          amount, and price impact.
        </p>
        <CodeBlock title="Rust">
          {`let quote = dex.get_quote(1, "USDC", 10_000);
// Quote {
//   input_token: "USDC",
//   input_amount: 10_000,
//   output_token: "ETH",
//   output_amount: 4_935,
//   price_impact_bps: 98,     // 0.98% price impact
//   fee_amount: 30,            // 0.3% of input
// }`}
        </CodeBlock>
        <CodeBlock title="JavaScript">
          {`const quote = await client.callView("dex-swap", "get_quote", {
  pool_id: 1,
  input_token: "USDC",
  input_amount: 10_000,
});

console.log(\`Swap \${quote.input_amount} \${quote.input_token}\`);
console.log(\`  => \${quote.output_amount} \${quote.output_token}\`);
console.log(\`  Fee: \${quote.fee_amount} (0.3%)\`);
console.log(\`  Price impact: \${(quote.price_impact_bps / 100).toFixed(2)}%\`);`}
        </CodeBlock>
      </div>

      {/* Multi-hop Routing */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Multi-hop Routing</h2>
        <p className="mt-2 text-slate-300">
          When no direct pool exists between two tokens, DinaDEX can route through
          intermediate pools. The <code className="text-blue-300">swap_route</code> function
          chains multiple swaps, and <code className="text-blue-300">get_best_route</code> finds
          the optimal path automatically.
        </p>
        <CodeBlock title="Multi-hop Swap Architecture">
          {`    USDC ──────> ETH ──────> SOL
     |            |            |
   Pool 1      Pool 2      Result
   (USDC/ETH)  (ETH/SOL)

   1. 1,000 USDC swapped for ~494 ETH in Pool 1
   2. 494 ETH swapped for ~1,950 SOL in Pool 2
   3. Final output: 1,950 SOL (with min_output check)`}
        </CodeBlock>
        <CodeBlock title="Rust">
          {`// Manual multi-hop: USDC → ETH → SOL
let events = dex.swap_route(
    "bob",
    &[1, 2],    // pool_id path: Pool 1 (USDC/ETH), Pool 2 (ETH/SOL)
    "USDC",     // starting token
    1_000,      // input amount
    1_900,      // min final output (slippage on whole route)
);

// Auto-route: find the best path
let (path, expected_output) = dex.get_best_route("USDC", "SOL", 1_000);
// path = [1, 2], expected_output = 1_950`}
        </CodeBlock>
        <CodeBlock title="JavaScript">
          {`// Find best route automatically
const { path, output } = await client.callView("dex-swap", "get_best_route", {
  input_token: "USDC",
  output_token: "SOL",
  amount: 1_000,
});

// Execute the route with slippage protection
const minOutput = Math.floor(output * 0.98); // 2% slippage tolerance
const tx = await wallet.callContract("dex-swap", "swap_route", {
  path,
  input_token: "USDC",
  input_amount: 1_000,
  min_output: minOutput,
});`}
        </CodeBlock>
      </div>

      {/* Comparison */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">
          Dina DEX vs Uniswap V2
        </h2>
        <p className="mt-2 text-slate-300">
          DinaDEX uses the same proven constant product model as Uniswap V2, but
          on Dina Network&apos;s high-performance infrastructure:
        </p>
        <div className="mt-4 overflow-hidden rounded-lg border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900">
                <th className="px-4 py-3 text-left font-medium text-slate-300">Feature</th>
                <th className="px-4 py-3 text-left font-medium text-slate-300">Uniswap V2 (Ethereum)</th>
                <th className="px-4 py-3 text-left font-medium text-slate-300">DinaDEX (Dina Network)</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">AMM Model</td>
                <td className="px-4 py-3">x * y = k</td>
                <td className="px-4 py-3 text-green-400">x * y = k (identical)</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Swap Finality</td>
                <td className="px-4 py-3">~12 seconds</td>
                <td className="px-4 py-3 text-green-400">~100ms (120x faster)</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Gas Cost</td>
                <td className="px-4 py-3">~$5-50 per swap</td>
                <td className="px-4 py-3 text-green-400">&lt; $0.001 per swap</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Swap Fee</td>
                <td className="px-4 py-3">0.30%</td>
                <td className="px-4 py-3">0.30% (configurable per pool)</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">MEV / Front-running</td>
                <td className="px-4 py-3">Significant risk</td>
                <td className="px-4 py-3 text-green-400">Minimal (100ms blocks)</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Runtime</td>
                <td className="px-4 py-3">EVM (Solidity)</td>
                <td className="px-4 py-3">WASM (Rust)</td>
              </tr>
              <tr>
                <td className="px-4 py-3">Token Standard</td>
                <td className="px-4 py-3">ERC-20</td>
                <td className="px-4 py-3">DRC-1 (fungible tokens)</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Supported Tokens */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Supported Tokens</h2>
        <p className="mt-2 text-slate-300">
          DinaDEX supports any DRC-1 compliant token deployed on Dina Network.
          Common tradable assets include:
        </p>
        <ul className="mt-4 space-y-2 text-slate-300">
          <li className="flex items-center gap-2">
            <span className="inline-block h-2 w-2 rounded-full bg-blue-400" />
            <strong>Bridged USDC</strong> — Circle-backed stablecoin via{" "}
            <Link href="/docs/bridges/cctp" className="text-blue-400 underline decoration-blue-400/30 hover:decoration-blue-400">CCTP</Link>
            {" "}or other bridges
          </li>
          <li className="flex items-center gap-2">
            <span className="inline-block h-2 w-2 rounded-full bg-purple-400" />
            <strong>Bridged ETH</strong> — Wrapped Ether bridged from Ethereum / Base
          </li>
          <li className="flex items-center gap-2">
            <span className="inline-block h-2 w-2 rounded-full bg-green-400" />
            <strong>Native ecosystem tokens</strong> — Any DRC-1 token deployed by projects on Dina
          </li>
          <li className="flex items-center gap-2">
            <span className="inline-block h-2 w-2 rounded-full bg-orange-400" />
            <strong>Bridged assets</strong> — SOL, AVAX, and other assets via{" "}
            <Link href="/docs/bridges" className="text-blue-400 underline decoration-blue-400/30 hover:decoration-blue-400">bridge protocols</Link>
          </li>
        </ul>
      </div>

      {/* Contract Reference */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Contract Reference</h2>
        <p className="mt-2 text-slate-300">
          The full DEX contract is deployed at{" "}
          <code className="text-blue-300">contracts/dex-swap</code> in the Dina
          Network repository. Key entry points:
        </p>
        <div className="mt-4 overflow-hidden rounded-lg border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900">
                <th className="px-4 py-3 text-left font-medium text-slate-300">Function</th>
                <th className="px-4 py-3 text-left font-medium text-slate-300">Description</th>
                <th className="px-4 py-3 text-left font-medium text-slate-300">Auth</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">create_pool</td>
                <td className="px-4 py-3">Create a new trading pair</td>
                <td className="px-4 py-3">Anyone</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">add_liquidity</td>
                <td className="px-4 py-3">Deposit tokens, receive LP tokens</td>
                <td className="px-4 py-3">Anyone</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">remove_liquidity</td>
                <td className="px-4 py-3">Burn LP tokens, withdraw both tokens</td>
                <td className="px-4 py-3">LP holder</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">swap_exact_in</td>
                <td className="px-4 py-3">Swap exact input for minimum output</td>
                <td className="px-4 py-3">Anyone</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">swap_exact_out</td>
                <td className="px-4 py-3">Swap maximum input for exact output</td>
                <td className="px-4 py-3">Anyone</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">get_quote</td>
                <td className="px-4 py-3">Read-only price estimate</td>
                <td className="px-4 py-3">View</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">swap_route</td>
                <td className="px-4 py-3">Multi-hop swap through pool path</td>
                <td className="px-4 py-3">Anyone</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">get_best_route</td>
                <td className="px-4 py-3">Find optimal 1-2 hop route</td>
                <td className="px-4 py-3">View</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">set_fee</td>
                <td className="px-4 py-3">Change pool fee (max 10%)</td>
                <td className="px-4 py-3">Owner</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">collect_protocol_fees</td>
                <td className="px-4 py-3">Withdraw accumulated protocol fees</td>
                <td className="px-4 py-3">Owner</td>
              </tr>
              <tr>
                <td className="px-4 py-3 font-mono text-blue-300">pause_pool / unpause_pool</td>
                <td className="px-4 py-3">Emergency pause/resume trading</td>
                <td className="px-4 py-3">Owner</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
