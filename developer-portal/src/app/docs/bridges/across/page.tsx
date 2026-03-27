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

export default function AcrossPage() {
  return (
    <div>
      <h1 className="text-4xl font-bold tracking-tight text-white">
        Across Protocol Integration
      </h1>
      <p className="mt-4 text-lg text-slate-300">
        Across is the fastest bridge for moving USDC to and from Dina Network.
        It uses a relayer-based spoke pool model where professional relayers fill
        deposits instantly from their own capital, then get repaid later via
        optimistic verification on the HubPool.
      </p>

      {/* How Across Works */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">How Across Works</h2>
        <p className="mt-3 text-sm text-slate-300">
          Unlike burn-and-mint bridges that wait for source chain finality, Across
          relayers pre-fund transfers on the destination chain. This means users
          receive tokens in 1-3 minutes regardless of source chain confirmation
          times.
        </p>

        <CodeBlock title="Across Flow Diagram">
          {`  Source Chain (Dina)               Across Relayers           Destination (e.g. Base)
  ====================            ================           =======================

  1. User calls deposit()
     on SpokePool
     - locks input tokens
     - emits DepositEvent
           |
           v
  2. Deposit recorded          3. Relayer observes
     with nonce, fee,             the deposit event
     fill deadline                     |
                                       v
                                4. Relayer fills on
                                   destination chain  -----> 5. Recipient receives
                                   from own capital          output tokens instantly
                                       |
                                       v
                                6. Relayer submits
                                   fill proof to
                                   HubPool (Ethereum)
                                       |
                                       v
                                7. After optimistic
                                   verification,
                                   relayer is repaid
                                   from locked funds`}
        </CodeBlock>

        <div className="mt-6 grid gap-4 sm:grid-cols-3">
          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-4">
            <h3 className="text-sm font-semibold text-white">Speed</h3>
            <p className="mt-1 text-2xl font-bold text-blue-400">1-3 min</p>
            <p className="mt-1 text-xs text-slate-400">
              Fastest bridge option -- relayers pre-fund transfers
            </p>
          </div>
          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-4">
            <h3 className="text-sm font-semibold text-white">Fee</h3>
            <p className="mt-1 text-2xl font-bold text-green-400">0.1-0.5%</p>
            <p className="mt-1 text-xs text-slate-400">
              Competitive relayer fees based on market conditions
            </p>
          </div>
          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-4">
            <h3 className="text-sm font-semibold text-white">Fallback</h3>
            <p className="mt-1 text-2xl font-bold text-amber-400">Slow Relay</p>
            <p className="mt-1 text-xs text-slate-400">
              Merkle proof path guarantees eventual settlement
            </p>
          </div>
        </div>
      </div>

      {/* Chain IDs */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Across Chain IDs</h2>
        <p className="mt-3 text-sm text-slate-300">
          Across uses standard EVM chain IDs. Use chain ID{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">99999</code>{" "}
          for Dina Network.
        </p>

        <div className="mt-6 overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-slate-700">
                <th className="px-4 py-3 font-semibold text-white">Chain</th>
                <th className="px-4 py-3 font-semibold text-white">Chain ID</th>
                <th className="px-4 py-3 font-semibold text-white">SpokePool Contract</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Dina Network</td>
                <td className="px-4 py-3">99999</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-blue-400">dina1across_spoke_pool...</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Ethereum</td>
                <td className="px-4 py-3">1</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Base</td>
                <td className="px-4 py-3">8453</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">0x09aea4b2242abC8bb4BB78D537A67a245A7bEC64</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Arbitrum</td>
                <td className="px-4 py-3">42161</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">0xe35e9842fceaCA96570B734083f4a58e8F7C5f2A</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Optimism</td>
                <td className="px-4 py-3">10</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">0x6f26Bf09B1C792e3228e5467807a900A503c0281</code>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Bridge USDC from Dina to Base */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Bridge USDC from Dina to Base</h2>
        <p className="mt-3 text-sm text-slate-300">
          Create a deposit on Dina&apos;s SpokePool. A relayer will observe the
          deposit and fill it on Base within 1-3 minutes.
        </p>

        <CodeBlock title="bridge-across.ts">
          {`import { DinaClient, DinaWallet } from 'dina-js';

const client = new DinaClient("https://testnet.dina.network");
const wallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

const ACROSS_SPOKE_POOL = "dina1across_spoke_pool...";
const BRIDGED_USDC = "dina1bridged_usdc...";
const BASE_USDC = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";

function parseUSDC(amount: string): bigint {
  return BigInt(Math.round(parseFloat(amount) * 1_000_000));
}

// Create a deposit for relayers to fill on Base
const tx = await client.callContract(wallet, {
  contract: ACROSS_SPOKE_POOL,
  method: 'deposit',
  args: {
    destinationChainId: 8453,              // Base
    recipient: baseAddress,
    inputToken: BRIDGED_USDC,
    outputToken: BASE_USDC,
    inputAmount: parseUSDC("100"),
    outputAmount: parseUSDC("99.70"),       // after ~0.30% relayer fee
    relayerFeePct: 30,                      // 0.30% in basis points
    quoteTimestamp: Math.floor(Date.now() / 1000),
    fillDeadline: Math.floor(Date.now() / 1000) + 3600,
    exclusivityDeadline: 0,                 // no exclusive relayer
    message: "0x",                          // no message
  },
});

console.log("Deposit tx:", tx.hash);
console.log("Deposit nonce:", tx.result);
// Relayer will fill on Base within 1-3 minutes`}
        </CodeBlock>
      </div>

      {/* Receive on Dina (relayer fill) */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Receive Fills on Dina</h2>
        <p className="mt-3 text-sm text-slate-300">
          When someone bridges tokens to Dina, a relayer calls{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">fill_relay</code>{" "}
          on Dina&apos;s SpokePool to deliver tokens to the recipient. This is
          typically handled by automated relayer bots.
        </p>

        <CodeBlock title="fill-relay.ts (relayer side)">
          {`// Relayers monitor deposits across all chains and fill them
// This is typically automated by relayer bots

const fillTx = await client.callContract(relayerWallet, {
  contract: ACROSS_SPOKE_POOL,
  method: 'fill_relay',
  args: {
    depositor: originDepositor,
    recipient: dinaRecipient,
    inputToken: originToken,
    outputToken: BRIDGED_USDC,
    inputAmount: depositInputAmount,
    outputAmount: depositOutputAmount,
    repaymentChainId: 1,           // relayer wants repayment on Ethereum
    originChainId: 8453,           // deposit originated on Base
    depositNonce: 42,
    fillDeadline: depositFillDeadline,
    message: "0x",
  },
});

console.log("Fill tx:", fillTx.hash);`}
        </CodeBlock>
      </div>

      {/* Slow Relay Fallback */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Slow Relay Fallback</h2>
        <p className="mt-3 text-sm text-slate-300">
          If no relayer fills a deposit before the deadline, the deposit can
          be settled via a Merkle proof published by the HubPool on Ethereum.
          This slow path takes longer but guarantees eventual settlement.
        </p>

        <CodeBlock title="slow-relay.ts">
          {`// Execute a slow relay using a Merkle proof from the HubPool
const tx = await client.callContract(wallet, {
  contract: ACROSS_SPOKE_POOL,
  method: 'execute_slow_relay_leaf',
  args: {
    relayData: {
      depositor: depositorAddress,
      recipient: recipientAddress,
      inputToken: originInputToken,
      outputToken: BRIDGED_USDC,
      inputAmount: parseUSDC("100"),
      outputAmount: parseUSDC("99.70"),
      originChainId: 8453,
      depositNonce: 42,
      fillDeadline: originalDeadline,
      message: "0x",
    },
    proof: {
      proof: merkleProofHashes,     // array of 32-byte hashes
      leafIndex: 7,
      root: publishedMerkleRoot,
    },
  },
});`}
        </CodeBlock>
      </div>

      {/* Fee Structure */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Fee Structure</h2>
        <p className="mt-3 text-sm text-slate-300">
          Across fees are set by the relayer market. When creating a deposit,
          you specify the <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">relayerFeePct</code> and{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">outputAmount</code>.
          Relayers choose which deposits to fill based on profitability.
        </p>

        <div className="mt-6 overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-slate-700">
                <th className="px-4 py-3 font-semibold text-white">Transfer Size</th>
                <th className="px-4 py-3 font-semibold text-white">Typical Fee</th>
                <th className="px-4 py-3 font-semibold text-white">Speed</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">&lt; $1,000</td>
                <td className="px-4 py-3">~0.3-0.5%</td>
                <td className="px-4 py-3">1-2 min</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">$1,000 - $100,000</td>
                <td className="px-4 py-3">~0.1-0.3%</td>
                <td className="px-4 py-3">1-3 min</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">&gt; $100,000</td>
                <td className="px-4 py-3">~0.05-0.1%</td>
                <td className="px-4 py-3">2-5 min</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Supported Chains */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Supported Chains</h2>
        <p className="mt-3 text-sm text-slate-300">
          Across supports bridging between Dina and these chains:
        </p>
        <div className="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4">
          {[
            "Ethereum", "Base", "Arbitrum", "Optimism", "Polygon",
            "ZKsync", "Linea", "Mode", "Blast", "Scroll",
          ].map((chain) => (
            <div
              key={chain}
              className="rounded-lg border border-slate-800 bg-slate-900/40 px-3 py-2 text-center text-sm text-slate-300"
            >
              {chain}
            </div>
          ))}
        </div>
      </div>

      {/* Why Across is Fast */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">
          Why Across is the Fastest Bridge
        </h2>
        <p className="mt-3 text-sm text-slate-300">
          Traditional bridges wait for source chain finality before releasing
          tokens. This can take 15-30 minutes on Ethereum or 7 days on optimistic
          rollups. Across eliminates this wait entirely:
        </p>
        <ol className="mt-4 list-inside list-decimal space-y-3 text-sm text-slate-300">
          <li>
            <span className="font-medium text-white">Relayer pre-funding:</span>{" "}
            Relayers send their own capital to the recipient immediately, without
            waiting for the source chain deposit to be confirmed.
          </li>
          <li>
            <span className="font-medium text-white">Optimistic verification:</span>{" "}
            Relayers are repaid via the HubPool after an optimistic challenge
            period. If a fill was fraudulent, it can be challenged and the
            relayer is slashed.
          </li>
          <li>
            <span className="font-medium text-white">Competitive relayer market:</span>{" "}
            Multiple relayers compete to fill deposits, driving down fees and
            improving speed.
          </li>
        </ol>
      </div>

      {/* Next steps */}
      <div className="mt-12 rounded-xl border border-slate-800 bg-slate-900/40 p-6">
        <h3 className="text-base font-semibold text-white">Next steps</h3>
        <ul className="mt-3 space-y-2 text-sm text-slate-300">
          <li>
            <Link href="/docs/bridges/stargate" className="text-blue-400 hover:underline">
              Stargate Bridge
            </Link>{" "}
            -- liquidity pool-based bridging via LayerZero.
          </li>
          <li>
            <Link href="/docs/bridges/cctp" className="text-blue-400 hover:underline">
              Circle CCTP
            </Link>{" "}
            -- native burn-and-mint USDC bridging.
          </li>
          <li>
            <Link href="/docs/bridges" className="text-blue-400 hover:underline">
              Bridge Overview
            </Link>{" "}
            -- compare all bridge options.
          </li>
          <li>
            <Link href="/docs/bridges/usdc" className="text-blue-400 hover:underline">
              Bridged USDC Standard
            </Link>{" "}
            -- understand the USDC.e token on Dina.
          </li>
        </ul>
      </div>
    </div>
  );
}
