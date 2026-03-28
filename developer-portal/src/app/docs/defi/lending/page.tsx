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

export default function LendingPage() {
  return (
    <div>
      <h1 className="text-4xl font-bold tracking-tight text-white">
        Lending Pool
      </h1>
      <p className="mt-4 text-lg text-slate-300">
        The Dina lending pool is an Aave-style money market where suppliers earn
        interest by providing USDC liquidity, and borrowers pay interest to access
        that liquidity. Interest rates adjust automatically based on pool
        utilization using a piecewise-linear rate model.
      </p>

      {/* How Lending Works */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">How Lending Works</h2>
        <p className="mt-2 text-slate-300">
          Suppliers deposit USDC into the pool and receive supply shares that
          represent their proportional claim. Borrowers take USDC from the pool
          and accrue interest over time. The interest paid by borrowers flows to
          suppliers (minus a protocol reserve cut).
        </p>
        <CodeBlock title="Lending Pool Flow">
          {`    Suppliers                    Lending Pool                  Borrowers
    ─────────                    ────────────                  ─────────

    Alice ──── 100 USDC ────>   ┌──────────────┐
                                │              │
    Bob   ──── 50 USDC  ────>  │  Pool: 150   │ ────> Charlie borrows 80
                                │  Borrowed: 80│
                                │  Available:70│ <──── Charlie repays 80+interest
                                │              │
    Alice <─── 105 USDC ────── │  Interest     │
    Bob   <─── 52.5 USDC ───── │  distributed  │
                                └──────────────┘
                                     │
                                     └── Protocol reserves (10% of interest)`}
        </CodeBlock>
      </div>

      {/* Interest Rate Model */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Interest Rate Model</h2>
        <p className="mt-2 text-slate-300">
          The lending pool uses the same piecewise-linear interest rate model as
          Aave. Rates stay low when utilization is below the optimal target, then
          spike sharply above it to incentivize repayment and new supply.
        </p>
        <CodeBlock title="Interest Rate Curve">
          {`    Borrow Rate (APY)
        |
   112% |                                                    *
        |                                                  /
   100% |                                                /
        |                                              /
    80% |                                           /
        |                                         /
    60% |                                      /
        |                                    /
    40% |                                 /
        |                               /
    20% |                            /
        |                         /
    12% |  * ─ ─ ─ ─ ─ ─ ─ ─ ─ *     <── slope1 (gradual: base to base+slope1)
        | /                    |
     2% *                      |          <── base_rate
        |                      |
        +──────────────────────+──────────────────────────────
        0%                    80%                            100%
                      optimal_utilization              Utilization

    Default Parameters:
    ───────────────────
    base_rate    =   200 bps  (2%)
    slope1       =  1000 bps  (10%)  -- rate increase below optimal
    slope2       = 10000 bps  (100%) -- rate increase above optimal (steep!)
    optimal_util =  8000 bps  (80%)

    Formulas:
    ─────────
    utilization = total_borrowed / total_supplied

    if utilization <= optimal:
      borrow_rate = base_rate + (utilization / optimal) * slope1

    if utilization > optimal:
      borrow_rate = base_rate + slope1
                    + ((utilization - optimal) / (1 - optimal)) * slope2

    supply_rate = borrow_rate * utilization * (1 - reserve_factor)`}
        </CodeBlock>
        <InfoBox title="Why Supply APY is Always Less Than Borrow APY">
          <p>
            Supply APY = Borrow APY * Utilization * (1 - Reserve Factor). Since
            utilization is always &lt;= 100% and the protocol takes a reserve cut,
            suppliers always earn less than borrowers pay. The spread funds the
            protocol treasury and ensures solvency.
          </p>
        </InfoBox>
      </div>

      {/* Creating a Pool */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Creating a Pool</h2>
        <p className="mt-2 text-slate-300">
          Deploy a new lending pool with custom interest rate parameters.
        </p>
        <CodeBlock title="Rust (contract call)">
          {`use defi_lending::LendingPoolState;

let mut pool = LendingPoolState::new(
    owner_address,
    200,    // base_rate_bps: 2%
    1000,   // slope1_bps: 10%
    10000,  // slope2_bps: 100%
    8000,   // optimal_utilization_bps: 80%
    1000,   // reserve_factor_bps: 10%
);`}
        </CodeBlock>
        <CodeBlock title="JavaScript (via SDK)">
          {`import { DinaClient } from "@dina-network/sdk";

const client = new DinaClient({ rpcUrl: "https://rpc.dina.network" });
const wallet = client.wallet("your-private-key");

const tx = await wallet.callContract("defi-lending", "create_pool", {
  base_rate_bps: 200,
  slope1_bps: 1000,
  slope2_bps: 10000,
  optimal_utilization_bps: 8000,
  reserve_factor_bps: 1000,
});`}
        </CodeBlock>
      </div>

      {/* Supplying */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Supplying USDC</h2>
        <p className="mt-2 text-slate-300">
          Supply USDC to the pool to start earning interest. You receive supply
          shares that grow in value as borrowers pay interest.
        </p>
        <CodeBlock title="Rust">
          {`// Supply 100 USDC
let shares = pool.supply(supplier_address, 100_000_000, current_timestamp);
// First supply: shares = 100_000_000 (1:1)

// Check your balance (includes accrued interest)
let balance = pool.get_supply_balance(&supplier_address);
// balance > 100_000_000 after interest accrues

// Withdraw supply + earned interest
let amount = pool.withdraw_supply(supplier_address, shares, current_timestamp);`}
        </CodeBlock>
        <CodeBlock title="JavaScript">
          {`// Supply 100 USDC
const tx = await wallet.callContract("defi-lending", "supply", {
  amount: 100_000_000,
  timestamp: Math.floor(Date.now() / 1000),
});
console.log("Supply shares:", tx.result);

// Check balance with accrued interest
const balance = await client.callView("defi-lending", "get_supply_balance", {
  user: myAddress,
});
console.log("Current balance:", balance / 1e6, "USDC");

// Withdraw
const withdrawTx = await wallet.callContract("defi-lending", "withdraw_supply", {
  shares: myShares,
  timestamp: Math.floor(Date.now() / 1000),
});`}
        </CodeBlock>
      </div>

      {/* Borrowing */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Borrowing USDC</h2>
        <p className="mt-2 text-slate-300">
          Borrow USDC from the pool. Interest accrues over time based on the
          current borrow rate. Repay at any time to clear your debt.
        </p>
        <CodeBlock title="Rust">
          {`// Borrow 50 USDC
pool.borrow(borrower_address, 50_000_000, current_timestamp);

// Check current debt (principal + accrued interest)
let debt = pool.get_borrow_balance_for(&borrower_address);
// debt > 50_000_000 after time passes

// Repay full debt
let excess = pool.repay(borrower_address, debt, current_timestamp);
// excess = 0 (exact repayment)

// Overpay: excess is returned
let excess = pool.repay(borrower_address, 999_000_000, current_timestamp);
// excess = leftover amount`}
        </CodeBlock>
        <CodeBlock title="JavaScript">
          {`// Borrow 50 USDC
await wallet.callContract("defi-lending", "borrow", {
  amount: 50_000_000,
  timestamp: Math.floor(Date.now() / 1000),
});

// Check current debt
const debt = await client.callView("defi-lending", "get_borrow_balance", {
  user: myAddress,
});
console.log("Current debt:", debt / 1e6, "USDC");

// Repay
const tx = await wallet.callContract("defi-lending", "repay", {
  amount: debt, // repay full amount
  timestamp: Math.floor(Date.now() / 1000),
});
console.log("Excess returned:", tx.result / 1e6, "USDC");`}
        </CodeBlock>
        <InfoBox title="Testnet: No Collateral Required">
          <p>
            On testnet, borrowing does not require collateral. This simplifies
            testing the interest rate mechanics. In production, borrowers would
            need to post collateral (via a separate collateral manager contract)
            and would be subject to liquidation if their position becomes
            undercollateralized.
          </p>
        </InfoBox>
      </div>

      {/* Current Rates */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Checking Current Rates</h2>
        <p className="mt-2 text-slate-300">
          Query the pool&apos;s current utilization and interest rates at any time.
        </p>
        <CodeBlock title="Rust">
          {`// Current utilization (basis points, 5000 = 50%)
let util = pool.get_utilization_bps();

// Current APYs (basis points)
let borrow_apy = pool.get_borrow_apy_bps(); // e.g. 820 = 8.20%
let supply_apy = pool.get_supply_apy_bps(); // e.g. 369 = 3.69%

println!("Utilization: {:.2}%", util as f64 / 100.0);
println!("Borrow APY:  {:.2}%", borrow_apy as f64 / 100.0);
println!("Supply APY:  {:.2}%", supply_apy as f64 / 100.0);`}
        </CodeBlock>
        <CodeBlock title="JavaScript">
          {`const util = await client.callView("defi-lending", "get_utilization");
const borrowApy = await client.callView("defi-lending", "get_borrow_apy");
const supplyApy = await client.callView("defi-lending", "get_supply_apy");

console.log(\`Utilization: \${(util / 100).toFixed(2)}%\`);
console.log(\`Borrow APY:  \${(borrowApy / 100).toFixed(2)}%\`);
console.log(\`Supply APY:  \${(supplyApy / 100).toFixed(2)}%\`);

// Example output at 50% utilization:
// Utilization: 50.00%
// Borrow APY:  8.25%
// Supply APY:  3.71%`}
        </CodeBlock>
      </div>

      {/* Aave Comparison */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">
          Dina Lending vs Aave
        </h2>
        <p className="mt-2 text-slate-300">
          The Dina lending pool implements the same interest rate model and
          share-based accounting as Aave V3, adapted for Dina Network:
        </p>
        <div className="mt-4 overflow-hidden rounded-lg border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900">
                <th className="px-4 py-3 text-left font-medium text-slate-300">Feature</th>
                <th className="px-4 py-3 text-left font-medium text-slate-300">Aave V3 (Ethereum)</th>
                <th className="px-4 py-3 text-left font-medium text-slate-300">Dina Lending</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Rate Model</td>
                <td className="px-4 py-3">Piecewise linear with kink</td>
                <td className="px-4 py-3 text-green-400">Piecewise linear with kink (identical)</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Supply Accounting</td>
                <td className="px-4 py-3">aTokens (rebasing)</td>
                <td className="px-4 py-3 text-green-400">Supply shares (non-rebasing, simpler)</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Borrow Tracking</td>
                <td className="px-4 py-3">Variable debt tokens + index</td>
                <td className="px-4 py-3">Borrow positions + index</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Reserve Factor</td>
                <td className="px-4 py-3">Per-asset (5-20%)</td>
                <td className="px-4 py-3">Configurable (default 10%)</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Collateral</td>
                <td className="px-4 py-3">Multi-asset, health factor</td>
                <td className="px-4 py-3">Testnet: none required</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Interest Accrual</td>
                <td className="px-4 py-3">Per-block</td>
                <td className="px-4 py-3 text-green-400">Per-second (more precise)</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Transaction Speed</td>
                <td className="px-4 py-3">~12 seconds</td>
                <td className="px-4 py-3 text-green-400">~100ms</td>
              </tr>
              <tr>
                <td className="px-4 py-3">Gas Cost</td>
                <td className="px-4 py-3">~$10-50 per operation</td>
                <td className="px-4 py-3 text-green-400">&lt; $0.001</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Contract Reference */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Contract Reference</h2>
        <p className="mt-2 text-slate-300">
          The lending pool contract is deployed at{" "}
          <code className="text-blue-300">contracts/defi-lending</code> in the Dina
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
                <td className="px-4 py-3">Deploy a new lending pool</td>
                <td className="px-4 py-3">Anyone (becomes owner)</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">supply</td>
                <td className="px-4 py-3">Deposit USDC, start earning interest</td>
                <td className="px-4 py-3">Anyone</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">withdraw_supply</td>
                <td className="px-4 py-3">Withdraw supplied USDC + earned interest</td>
                <td className="px-4 py-3">Supplier</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">borrow</td>
                <td className="px-4 py-3">Borrow USDC from the pool</td>
                <td className="px-4 py-3">Anyone</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">repay</td>
                <td className="px-4 py-3">Repay borrowed USDC + interest</td>
                <td className="px-4 py-3">Borrower</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">get_supply_balance</td>
                <td className="px-4 py-3">Current balance with accrued interest</td>
                <td className="px-4 py-3">View</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">get_borrow_balance</td>
                <td className="px-4 py-3">Current debt with accrued interest</td>
                <td className="px-4 py-3">View</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">get_utilization</td>
                <td className="px-4 py-3">Current utilization rate (bps)</td>
                <td className="px-4 py-3">View</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">get_supply_apy</td>
                <td className="px-4 py-3">Current supply APY (bps)</td>
                <td className="px-4 py-3">View</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">get_borrow_apy</td>
                <td className="px-4 py-3">Current borrow APY (bps)</td>
                <td className="px-4 py-3">View</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">accrue_interest</td>
                <td className="px-4 py-3">Update interest indices (called automatically)</td>
                <td className="px-4 py-3">Anyone</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">collect_reserves</td>
                <td className="px-4 py-3">Withdraw accumulated protocol revenue</td>
                <td className="px-4 py-3">Owner</td>
              </tr>
              <tr>
                <td className="px-4 py-3 font-mono text-blue-300">pause / unpause</td>
                <td className="px-4 py-3">Emergency pause/resume operations</td>
                <td className="px-4 py-3">Owner</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Integration with Vaults */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Integration with Yield Vaults</h2>
        <p className="mt-2 text-slate-300">
          The lending pool serves as a yield source for{" "}
          <Link
            href="/docs/defi/vaults"
            className="text-blue-400 underline decoration-blue-400/30 hover:decoration-blue-400"
          >
            yield vaults
          </Link>
          . When a vault uses the <code className="text-blue-300">LendingPool</code> strategy,
          it deposits users&apos; USDC into the lending pool and periodically harvests
          earned interest back into the vault, increasing the share price for all
          vault depositors.
        </p>
        <CodeBlock title="Vault + Lending Pool Architecture">
          {`    Users                   Yield Vault              Lending Pool
    ─────                   ───────────              ────────────

    Deposit USDC ────>  Vault receives USDC  ────>  Supply to pool
                        Mint shares to user          Earn interest

    Harvest     ────>   Pull interest from   <────  Interest accrued
                        pool into vault              from borrowers

    Withdraw    ────>   Burn shares           ────>  Withdraw from pool
                        Return USDC to user          Return principal + yield`}
        </CodeBlock>
      </div>
    </div>
  );
}
