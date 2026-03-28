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

export default function VaultsPage() {
  return (
    <div>
      <h1 className="text-4xl font-bold tracking-tight text-white">
        Yield Vaults
      </h1>
      <p className="mt-4 text-lg text-slate-300">
        Dina yield vaults are ERC-4626-equivalent contracts that let users deposit
        USDC and earn yield. Depositors receive vault shares representing their
        proportional claim on the vault&apos;s total assets. As yield accrues,
        total assets increase while shares stay constant, making each share worth
        more over time.
      </p>

      {/* How Vaults Work */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">How Vaults Work</h2>
        <p className="mt-2 text-slate-300">
          The core mechanism is simple: deposit USDC, receive vault shares, and
          watch your share value grow as yield flows into the vault.
        </p>
        <CodeBlock title="Vault Share Mechanics">
          {`    Vault Lifecycle
    ═══════════════

    1. CREATE          Owner deploys vault with strategy + deposit limit
    2. DEPOSIT         Users deposit USDC, receive shares at current ratio
    3. YIELD           Strategy generates yield (lending, fees, manual)
    4. SHARE PRICE     total_assets grows, total_shares unchanged => price up
    5. WITHDRAW        Users burn shares, receive proportional USDC


    Example Timeline
    ────────────────

    Time    Action              total_assets   total_shares   share_price
    ────    ──────              ────────────   ────────────   ───────────
    T=0     Alice deposits 100     100            100           1.00
    T=1     Bob deposits 100       200            200           1.00
    T=2     Yield +20 accrues      220            200           1.10
    T=3     Alice withdraws 100    110            100           1.10
            Alice receives 110 USDC (earned 10 USDC yield)

    Share Conversion Formulas
    ─────────────────────────
    shares = deposit_amount * total_shares / total_assets
    assets = share_amount  * total_assets / total_shares

    First deposit: shares = amount (1:1 ratio)`}
        </CodeBlock>
      </div>

      {/* Vault Strategies */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Vault Strategies</h2>
        <p className="mt-2 text-slate-300">
          Each vault is configured with a yield strategy that determines where
          returns come from:
        </p>
        <div className="mt-4 overflow-hidden rounded-lg border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900">
                <th className="px-4 py-3 text-left font-medium text-slate-300">Strategy</th>
                <th className="px-4 py-3 text-left font-medium text-slate-300">Yield Source</th>
                <th className="px-4 py-3 text-left font-medium text-slate-300">Use Case</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-green-400">LendingPool</td>
                <td className="px-4 py-3">Interest from{" "}
                  <Link href="/docs/defi/lending" className="text-blue-400 underline decoration-blue-400/30 hover:decoration-blue-400">
                    lending pool
                  </Link>
                </td>
                <td className="px-4 py-3">Production vaults backed by real lending demand</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-green-400">ValidatorRewards</td>
                <td className="px-4 py-3">Transaction fees from validators</td>
                <td className="px-4 py-3">Staking-like yield from network activity</td>
              </tr>
              <tr>
                <td className="px-4 py-3 font-mono text-green-400">Manual</td>
                <td className="px-4 py-3">Owner adds yield directly</td>
                <td className="px-4 py-3">Testnet, treasury-backed vaults, fixed-rate products</td>
              </tr>
            </tbody>
          </table>
        </div>
        <InfoBox title="Testnet: Manual Yield Mode">
          <p>
            On testnet, use the <code className="text-blue-300">Manual</code> strategy
            to simulate real-world returns. The vault owner calls{" "}
            <code className="text-blue-300">add_yield(amount)</code> to increase
            total_assets, which raises the share price for all depositors. This lets
            you test the full deposit/yield/withdraw flow without connecting to a live
            lending pool or validator.
          </p>
        </InfoBox>
      </div>

      {/* Creating a Vault */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Creating a Vault</h2>
        <p className="mt-2 text-slate-300">
          Deploy a new vault by specifying a name, symbol, strategy, and deposit
          limit.
        </p>
        <CodeBlock title="Rust (contract call)">
          {`use defi_vault::{VaultState, VaultStrategy};

// Create a manual-yield vault with 1M USDC limit
let mut vault = VaultState::new(
    "USDC Yield Vault".to_string(),
    "yvUSDC".to_string(),
    owner_address,
    VaultStrategy::Manual { apy_bps: 500 }, // 5% target APY
    1_000_000_000_000, // 1,000,000 USDC (6 decimals)
);`}
        </CodeBlock>
        <CodeBlock title="JavaScript (via SDK)">
          {`import { DinaClient } from "@dina-network/sdk";

const client = new DinaClient({ rpcUrl: "https://rpc.dina.network" });
const wallet = client.wallet("your-private-key");

const tx = await wallet.callContract("defi-vault", "create_vault", {
  name: "USDC Yield Vault",
  symbol: "yvUSDC",
  strategy: { Manual: { apy_bps: 500 } },
  deposit_limit: 1_000_000_000_000,
});`}
        </CodeBlock>
      </div>

      {/* Depositing */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Depositing USDC</h2>
        <p className="mt-2 text-slate-300">
          Deposit USDC to receive vault shares. The number of shares you receive
          depends on the current share price.
        </p>
        <CodeBlock title="Rust">
          {`// Deposit 100 USDC
let shares = vault.deposit(depositor_address, 100_000_000);
// First deposit: shares = 100_000_000 (1:1)

// Check how many shares you'd get before depositing
let preview = vault.preview_deposit(50_000_000);
// preview = 50_000_000 (if no yield has accrued yet)`}
        </CodeBlock>
        <CodeBlock title="JavaScript">
          {`// Preview before depositing
const shares = await client.callView("defi-vault", "preview_deposit", {
  amount: 100_000_000, // 100 USDC
});
console.log("Expected shares:", shares);

// Execute deposit
const tx = await wallet.callContract("defi-vault", "deposit", {
  amount: 100_000_000,
});
console.log("Shares minted:", tx.result);`}
        </CodeBlock>
      </div>

      {/* Withdrawing */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Withdrawing</h2>
        <p className="mt-2 text-slate-300">
          Burn your vault shares to receive the proportional amount of USDC,
          including any yield that has accrued.
        </p>
        <CodeBlock title="Rust">
          {`// Check current value of your shares
let value = vault.get_share_value(100_000_000);
// If yield has accrued: value = 110_000_000 (110 USDC for 100M shares)

// Preview withdrawal
let preview = vault.preview_withdraw(100_000_000);
// preview = 110_000_000

// Withdraw all shares
let usdc_received = vault.withdraw(depositor_address, 100_000_000);
// usdc_received = 110_000_000 (100 USDC + 10 USDC yield)`}
        </CodeBlock>
        <CodeBlock title="JavaScript">
          {`// Check your share balance and value
const shares = await client.callView("defi-vault", "get_share_balance", {
  user: myAddress,
});
const value = await client.callView("defi-vault", "get_share_value", {
  shares,
});
console.log(\`\${shares} shares worth \${value / 1e6} USDC\`);

// Withdraw
const tx = await wallet.callContract("defi-vault", "withdraw", {
  shares,
});
console.log("USDC received:", tx.result / 1e6);`}
        </CodeBlock>
      </div>

      {/* Manual Yield (Testnet) */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Adding Yield (Manual Strategy)</h2>
        <p className="mt-2 text-slate-300">
          For Manual strategy vaults, the owner can add yield at any time. This
          increases <code className="text-blue-300">total_assets</code> without
          minting new shares, which raises the share price for all depositors.
        </p>
        <CodeBlock title="Rust">
          {`// Owner adds 10 USDC yield to the vault
vault.add_yield(owner_address, 10_000_000);

// Effect: if vault had 100 USDC and 100 shares,
// now has 110 USDC and 100 shares => share price = 1.10

// For non-Manual strategies, use harvest() instead
vault.harvest(owner_address, yield_amount, current_timestamp);`}
        </CodeBlock>
        <CodeBlock title="JavaScript">
          {`// Add yield (owner only, Manual strategy only)
await wallet.callContract("defi-vault", "add_yield", {
  amount: 10_000_000, // 10 USDC
});

// For production vaults with LendingPool strategy
await wallet.callContract("defi-vault", "harvest", {
  yield_amount: 10_000_000,
  timestamp: Math.floor(Date.now() / 1000),
});`}
        </CodeBlock>
      </div>

      {/* ERC-4626 Comparison */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">
          Dina Vault vs ERC-4626
        </h2>
        <p className="mt-2 text-slate-300">
          Dina yield vaults implement the same tokenized vault pattern as
          Ethereum&apos;s ERC-4626, adapted for Dina Network&apos;s WASM runtime
          and USDC-native architecture:
        </p>
        <div className="mt-4 overflow-hidden rounded-lg border border-slate-800">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-900">
                <th className="px-4 py-3 text-left font-medium text-slate-300">Feature</th>
                <th className="px-4 py-3 text-left font-medium text-slate-300">ERC-4626 (Ethereum)</th>
                <th className="px-4 py-3 text-left font-medium text-slate-300">Dina Vault</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Underlying Asset</td>
                <td className="px-4 py-3">Any ERC-20</td>
                <td className="px-4 py-3 text-green-400">USDC (native)</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Share Token</td>
                <td className="px-4 py-3">ERC-20 vault token</td>
                <td className="px-4 py-3 text-green-400">Internal share ledger</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Deposit / Withdraw</td>
                <td className="px-4 py-3">deposit(), withdraw(), redeem()</td>
                <td className="px-4 py-3">deposit(), withdraw()</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Preview Functions</td>
                <td className="px-4 py-3">previewDeposit(), previewRedeem()</td>
                <td className="px-4 py-3">preview_deposit(), preview_withdraw()</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Yield Mechanism</td>
                <td className="px-4 py-3">External DeFi protocols</td>
                <td className="px-4 py-3 text-green-400">Lending pool, validators, or manual</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3">Transaction Speed</td>
                <td className="px-4 py-3">~12 seconds</td>
                <td className="px-4 py-3 text-green-400">~100ms</td>
              </tr>
              <tr>
                <td className="px-4 py-3">Gas Cost</td>
                <td className="px-4 py-3">~$5-20 per operation</td>
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
          The vault contract is deployed at{" "}
          <code className="text-blue-300">contracts/defi-vault</code> in the Dina
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
                <td className="px-4 py-3 font-mono text-blue-300">create_vault</td>
                <td className="px-4 py-3">Deploy a new yield vault</td>
                <td className="px-4 py-3">Anyone (becomes owner)</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">deposit</td>
                <td className="px-4 py-3">Deposit USDC, receive shares</td>
                <td className="px-4 py-3">Anyone</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">withdraw</td>
                <td className="px-4 py-3">Burn shares, receive USDC</td>
                <td className="px-4 py-3">Share holder</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">harvest</td>
                <td className="px-4 py-3">Pull yield from strategy into vault</td>
                <td className="px-4 py-3">Owner</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">add_yield</td>
                <td className="px-4 py-3">Manually add yield (Manual strategy only)</td>
                <td className="px-4 py-3">Owner</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">preview_deposit</td>
                <td className="px-4 py-3">Preview shares for a deposit amount</td>
                <td className="px-4 py-3">View</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">preview_withdraw</td>
                <td className="px-4 py-3">Preview USDC for a share amount</td>
                <td className="px-4 py-3">View</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">get_vault</td>
                <td className="px-4 py-3">Vault info (assets, shares, strategy, limits)</td>
                <td className="px-4 py-3">View</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">get_share_balance</td>
                <td className="px-4 py-3">User&apos;s share balance</td>
                <td className="px-4 py-3">View</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">get_share_value</td>
                <td className="px-4 py-3">Current USDC value of shares</td>
                <td className="px-4 py-3">View</td>
              </tr>
              <tr className="border-b border-slate-800/50">
                <td className="px-4 py-3 font-mono text-blue-300">set_deposit_limit</td>
                <td className="px-4 py-3">Update maximum total deposits</td>
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
    </div>
  );
}
