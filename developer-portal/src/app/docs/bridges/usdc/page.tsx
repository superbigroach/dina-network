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

export default function BridgedUSDCPage() {
  return (
    <div>
      <h1 className="text-4xl font-bold tracking-tight text-white">
        Bridged USDC Standard
      </h1>
      <p className="mt-4 text-lg text-slate-300">
        Bridged USDC is Circle&apos;s official standard for launching USDC on
        new chains. On Dina Network, all USDC -- regardless of which bridge it
        came through -- is represented as a single, unified Bridged USDC token
        that is 1:1 backed by native USDC on the source chain.
      </p>

      {/* What is Bridged USDC */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">What is Bridged USDC?</h2>
        <p className="mt-3 text-sm text-slate-300">
          When a new blockchain launches, Circle does not immediately deploy
          native USDC. Instead, the chain deploys a Bridged USDC contract
          following Circle&apos;s specification. This contract:
        </p>
        <ul className="mt-4 list-inside list-disc space-y-2 text-sm text-slate-300">
          <li>
            Uses Circle&apos;s standard{" "}
            <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">FiatTokenV2_2</code>{" "}
            implementation (same as native USDC on Ethereum)
          </li>
          <li>Has a designated owner who can transfer ownership to Circle later</li>
          <li>Supports permit-based approvals (EIP-2612) for gasless transfers</li>
          <li>Is fully ERC-20 compatible with 6 decimal places</li>
          <li>
            Maintains the same contract address when upgraded to native USDC
          </li>
        </ul>

        <CodeBlock title="Token Architecture">
          {`  Bridged USDC on Dina Network
  ============================

  Contract: dina1bridged_usdc_v1...
  Standard: Circle FiatTokenV2_2
  Decimals: 6
  Symbol:   USDC
  Name:     Bridged USDC (Dina)

  +--------------------------------------------------+
  |                                                  |
  |  Minting Authority                               |
  |  +-----------+  +-----------+  +------------+   |
  |  | CCTP      |  | Wormhole  |  | LayerZero  |   |
  |  | Minter    |  | Minter    |  | Minter     |   |
  |  +-----------+  +-----------+  +------------+   |
  |  +-----------+  +-----------+                    |
  |  | Axelar    |  | Base      |                    |
  |  | Minter    |  | Bridge    |                    |
  |  +-----------+  | Minter    |                    |
  |                 +-----------+                    |
  |                                                  |
  |  Each bridge has a minting role that can         |
  |  only mint up to the amount locked/burned        |
  |  on the source chain.                            |
  |                                                  |
  +--------------------------------------------------+

  All minters produce the SAME Bridged USDC token.
  Users hold one token, not 5 different wrapped versions.`}
        </CodeBlock>
      </div>

      {/* How It Works on Dina */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">How It Works on Dina</h2>

        <div className="mt-6 space-y-4">
          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">1. Unified Token</h3>
            <p className="mt-2 text-sm text-slate-300">
              Whether you bridge via CCTP, Wormhole, LayerZero, Axelar, or the
              Base Bridge, you always receive the same Bridged USDC token at the
              same contract address. There is no fragmentation.
            </p>
          </div>

          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">2. Bridge Minters</h3>
            <p className="mt-2 text-sm text-slate-300">
              Each bridge has a designated minter role on the Bridged USDC
              contract. The minter can only mint tokens when backed by a
              verifiable lock or burn on the source chain. Minting limits are
              enforced on-chain.
            </p>
          </div>

          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">3. Redemption</h3>
            <p className="mt-2 text-sm text-slate-300">
              Users can bridge back via any supported bridge. Bridged USDC is
              burned on Dina and native USDC is released on the destination
              chain. The bridge you use to exit does not have to match the bridge
              you used to enter.
            </p>
          </div>
        </div>
      </div>

      {/* Token Contract */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Token Contract</h2>

        <div className="mt-4 overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-slate-700">
                <th className="px-4 py-3 font-semibold text-white">Property</th>
                <th className="px-4 py-3 font-semibold text-white">Value</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Contract Address</td>
                <td className="px-4 py-3">
                  <code className="text-blue-400">dina1bridged_usdc_v1...</code>
                  <span className="ml-2 text-xs text-slate-500">(placeholder -- will be published at mainnet launch)</span>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Name</td>
                <td className="px-4 py-3">Bridged USDC (Dina)</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Symbol</td>
                <td className="px-4 py-3">USDC</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Decimals</td>
                <td className="px-4 py-3">6</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Standard</td>
                <td className="px-4 py-3">Circle FiatTokenV2_2</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Permit Support</td>
                <td className="px-4 py-3">Yes (EIP-2612)</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Blocklist</td>
                <td className="px-4 py-3">Yes (Circle standard compliance)</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Circle Upgrade Path */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">
          How Circle Upgrades to Native USDC
        </h2>
        <p className="mt-3 text-sm text-slate-300">
          Circle has a well-defined process for upgrading Bridged USDC to native
          USDC on a new chain:
        </p>

        <CodeBlock title="Upgrade Timeline">
          {`  Phase 1: Bridged USDC (current)
  ================================
  - Dina deploys the Bridged USDC contract
  - Bridges mint Bridged USDC backed by locked native USDC
  - Users interact with Bridged USDC normally
  - Timeline: Day 1

  Phase 2: Circle Evaluation
  ==========================
  - Circle evaluates Dina for:
    * Network security and uptime
    * Transaction volume and TVL
    * Ecosystem maturity
    * Regulatory compliance
  - Timeline: 3-6 months typically

  Phase 3: Native USDC Deployment
  ===============================
  - Circle and Dina coordinate the upgrade:
    1. Bridged USDC contract owner transfers ownership to Circle
    2. Circle assumes minting authority
    3. All Bridged USDC becomes native USDC (same contract address)
    4. Circle manages minting/burning directly
  - Timeline: Coordinated event

  Result:
  +-------------------+     +-------------------+
  | Bridged USDC      | --> | Native USDC       |
  | (Dina mints)      |     | (Circle mints)    |
  | Same address:     |     | Same address:     |
  | dina1bridged_usdc |     | dina1bridged_usdc |
  +-------------------+     +-------------------+

  User impact: ZERO. Same token, same address, same balance.`}
        </CodeBlock>
      </div>

      {/* Bridged vs Native Comparison */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">
          Bridged USDC vs Native USDC
        </h2>

        <div className="mt-4 overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-slate-700">
                <th className="px-4 py-3 font-semibold text-white">Feature</th>
                <th className="px-4 py-3 font-semibold text-white">Bridged USDC</th>
                <th className="px-4 py-3 font-semibold text-white">Native USDC</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Issuer</td>
                <td className="px-4 py-3">Dina Foundation (via bridge minters)</td>
                <td className="px-4 py-3">Circle</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Backing</td>
                <td className="px-4 py-3">1:1 locked USDC on source chains</td>
                <td className="px-4 py-3">1:1 Circle reserves (cash + T-bills)</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Contract Standard</td>
                <td className="px-4 py-3">FiatTokenV2_2 (same as native)</td>
                <td className="px-4 py-3">FiatTokenV2_2</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Contract Address</td>
                <td className="px-4 py-3">dina1bridged_usdc...</td>
                <td className="px-4 py-3">Same address (upgraded in place)</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Minting Authority</td>
                <td className="px-4 py-3">Bridge contracts (CCTP, Wormhole, etc.)</td>
                <td className="px-4 py-3">Circle</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Redeemable for USD</td>
                <td className="px-4 py-3">Via bridge back to native USDC, then Circle</td>
                <td className="px-4 py-3">Directly via Circle</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">User Action to Upgrade</td>
                <td className="px-4 py-3">None required</td>
                <td className="px-4 py-3">N/A</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Code Examples */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Interacting with Bridged USDC</h2>

        <h3 className="mt-6 text-lg font-semibold text-white">Check Balance</h3>
        <CodeBlock title="check-balance.ts">
          {`import { DinaClient, DinaWallet } from 'dina-js';

const client = new DinaClient("https://testnet.dina.network");
const wallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

// Option 1: Use the built-in balance query (USDC is the native currency)
const balance = await client.getBalance(wallet.address);
console.log("Balance:", balance.formatted); // "100.000000 USDC"
console.log("Raw:", balance.raw);           // 100000000 (6 decimals)

// Option 2: Query the Bridged USDC contract directly
const tokenBalance = await client.queryContract({
  contract: "dina1bridged_usdc_v1...",
  method: "balance_of",
  args: { account: wallet.address },
});
console.log("Token balance:", tokenBalance); // 100000000`}
        </CodeBlock>

        <h3 className="mt-8 text-lg font-semibold text-white">Transfer USDC</h3>
        <CodeBlock title="transfer-usdc.ts">
          {`import { DinaClient, DinaWallet } from 'dina-js';

const client = new DinaClient("https://testnet.dina.network");
const wallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

// Simple transfer (recommended -- uses Dina's native USDC transfer)
const tx = await client.transfer({
  from: wallet.address,
  to:   "dina1recipient...",
  amount: 50_000000, // 50 USDC
});

const signed = wallet.sign(tx);
const receipt = await client.send(signed);
console.log("Transfer hash:", receipt.hash);`}
        </CodeBlock>

        <h3 className="mt-8 text-lg font-semibold text-white">Approve and TransferFrom</h3>
        <CodeBlock title="approve-transfer.ts">
          {`import { DinaClient, DinaWallet } from 'dina-js';

const client = new DinaClient("https://testnet.dina.network");
const wallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

// Approve a contract to spend your USDC
const approveTx = await client.callContract(wallet, {
  contract: "dina1bridged_usdc_v1...",
  method: "approve",
  args: {
    spender: "dina1some_defi_contract...",
    amount:  1000_000000, // 1000 USDC
  },
});
console.log("Approval tx:", approveTx.hash);

// Check allowance
const allowance = await client.queryContract({
  contract: "dina1bridged_usdc_v1...",
  method: "allowance",
  args: {
    owner:   wallet.address,
    spender: "dina1some_defi_contract...",
  },
});
console.log("Allowance:", allowance); // 1000000000`}
        </CodeBlock>

        <h3 className="mt-8 text-lg font-semibold text-white">Permit (Gasless Approval)</h3>
        <CodeBlock title="permit.ts">
          {`import { DinaClient, DinaWallet } from 'dina-js';

const client = new DinaClient("https://testnet.dina.network");
const wallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

// EIP-2612 permit: approve without a separate transaction
// The user signs a message off-chain, and the spender submits it
const deadline = Math.floor(Date.now() / 1000) + 3600; // 1 hour
const nonce = await client.queryContract({
  contract: "dina1bridged_usdc_v1...",
  method: "nonces",
  args: { owner: wallet.address },
});

// Sign the permit message
const permitSignature = wallet.signTypedData({
  types: {
    Permit: [
      { name: "owner",    type: "address" },
      { name: "spender",  type: "address" },
      { name: "value",    type: "uint256" },
      { name: "nonce",    type: "uint256" },
      { name: "deadline", type: "uint256" },
    ],
  },
  primaryType: "Permit",
  domain: {
    name:              "Bridged USDC (Dina)",
    version:           "2",
    chainId:           99,
    verifyingContract: "dina1bridged_usdc_v1...",
  },
  message: {
    owner:    wallet.address,
    spender:  "dina1some_contract...",
    value:    100_000000,  // 100 USDC
    nonce:    nonce,
    deadline: deadline,
  },
});

console.log("Permit signature:", permitSignature);
// Send this signature to the spender contract
// The spender calls permit() + transferFrom() in one tx`}
        </CodeBlock>

        <h3 className="mt-8 text-lg font-semibold text-white">Query Token Metadata</h3>
        <CodeBlock title="token-metadata.ts">
          {`import { DinaClient } from 'dina-js';

const client = new DinaClient("https://testnet.dina.network");
const contract = "dina1bridged_usdc_v1...";

// Query token information
const [name, symbol, decimals, totalSupply] = await Promise.all([
  client.queryContract({ contract, method: "name",         args: {} }),
  client.queryContract({ contract, method: "symbol",       args: {} }),
  client.queryContract({ contract, method: "decimals",     args: {} }),
  client.queryContract({ contract, method: "total_supply", args: {} }),
]);

console.log("Name:", name);              // "Bridged USDC (Dina)"
console.log("Symbol:", symbol);          // "USDC"
console.log("Decimals:", decimals);      // 6
console.log("Total Supply:", totalSupply); // Total bridged USDC on Dina`}
        </CodeBlock>
      </div>

      {/* Minter Roles */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Bridge Minter Roles</h2>
        <p className="mt-3 text-sm text-slate-300">
          Each bridge has a designated minter address with a configurable minting
          allowance. This ensures that no single bridge can mint more USDC than
          it has locked on source chains.
        </p>

        <CodeBlock title="query-minters.ts">
          {`import { DinaClient } from 'dina-js';

const client = new DinaClient("https://testnet.dina.network");
const contract = "dina1bridged_usdc_v1...";

// Check if an address is a minter
const isMinter = await client.queryContract({
  contract,
  method: "is_minter",
  args: { account: "dina1cctp_minter..." },
});
console.log("Is CCTP a minter:", isMinter); // true

// Check minting allowance
const allowance = await client.queryContract({
  contract,
  method: "minter_allowance",
  args: { minter: "dina1cctp_minter..." },
});
console.log("CCTP minting allowance:", allowance);
// The minter can mint up to this amount before needing a top-up`}
        </CodeBlock>

        <CodeBlock title="Minter Architecture">
          {`  Bridged USDC Minter Roles
  =========================

  +------------------+------------------+------------------+
  | Minter           | Address          | Allowance        |
  +------------------+------------------+------------------+
  | CCTP Bridge      | dina1cctp_...    | Dynamic (per tx) |
  | Wormhole Bridge  | dina1worm_...    | 10,000,000 USDC  |
  | LayerZero Bridge | dina1lz_...      | 10,000,000 USDC  |
  | Axelar Bridge    | dina1axl_...     | 10,000,000 USDC  |
  | Base Bridge      | dina1base_...    |  5,000,000 USDC  |
  +------------------+------------------+------------------+

  Rules:
  - Only the contract owner can add/remove minters
  - Each mint must be backed by a verifiable lock/burn
  - Allowances are replenished by the contract owner
  - After native USDC upgrade, Circle becomes sole minter`}
        </CodeBlock>
      </div>

      {/* Next steps */}
      <div className="mt-12 rounded-xl border border-slate-800 bg-slate-900/40 p-6">
        <h3 className="text-base font-semibold text-white">Next steps</h3>
        <ul className="mt-3 space-y-2 text-sm text-slate-300">
          <li>
            <Link href="/docs/bridges/cctp" className="text-blue-400 hover:underline">
              Circle CCTP
            </Link>{" "}
            -- the recommended bridge for getting USDC onto Dina.
          </li>
          <li>
            <Link href="/docs/transactions/transfer" className="text-blue-400 hover:underline">
              Send USDC
            </Link>{" "}
            -- transfer Bridged USDC between wallets on Dina.
          </li>
          <li>
            <Link href="/docs/transactions/batch" className="text-blue-400 hover:underline">
              Batch Transfers (DRC-19)
            </Link>{" "}
            -- send USDC to multiple recipients in a single transaction.
          </li>
          <li>
            <Link href="/docs/bridges" className="text-blue-400 hover:underline">
              Bridge Overview
            </Link>{" "}
            -- compare all bridges side by side.
          </li>
        </ul>
      </div>
    </div>
  );
}
