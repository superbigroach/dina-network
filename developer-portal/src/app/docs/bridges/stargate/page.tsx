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

export default function StargatePage() {
  return (
    <div>
      <h1 className="text-4xl font-bold tracking-tight text-white">
        Stargate / LayerZero Integration
      </h1>
      <p className="mt-4 text-lg text-slate-300">
        Stargate provides unified liquidity pool bridging for Dina Network,
        powered by LayerZero&apos;s cross-chain messaging protocol. LP providers
        deposit tokens into pools on each chain, and cross-chain swaps draw from
        these pools for instant settlement.
      </p>

      {/* How Stargate Works */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">How Stargate Works</h2>
        <p className="mt-3 text-sm text-slate-300">
          Stargate maintains unified liquidity pools across all connected chains.
          When a user swaps tokens cross-chain, tokens are removed from the source
          pool and released from the destination pool. LayerZero handles the
          cross-chain messaging to coordinate the swap.
        </p>

        <CodeBlock title="Stargate Flow Diagram">
          {`  Source Chain (Dina)              LayerZero               Destination (e.g. Base)
  ====================          ============            =======================

  1. User calls swap()
     on Stargate Router
     - specifies dst chain,
       pool, amount, recipient
           |
           v
  2. Tokens removed from
     Dina USDC pool
     - fee deducted
     - LP providers earn fees
           |
           v
  3. LayerZero message    -----> 4. LayerZero endpoint
     sent to destination          on Base receives
     chain with swap details      the message
                                       |
                                       v
                                 5. Stargate Router on
                                    Base releases tokens
                                    from Base USDC pool
                                       |
                                       v
                                 6. Recipient receives
                                    USDC on Base`}
        </CodeBlock>

        <div className="mt-6 grid gap-4 sm:grid-cols-3">
          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-4">
            <h3 className="text-sm font-semibold text-white">Speed</h3>
            <p className="mt-1 text-2xl font-bold text-blue-400">1-3 min</p>
            <p className="mt-1 text-xs text-slate-400">
              Liquidity pools on each chain -- no lock/mint delay
            </p>
          </div>
          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-4">
            <h3 className="text-sm font-semibold text-white">Fee</h3>
            <p className="mt-1 text-2xl font-bold text-green-400">~0.06%</p>
            <p className="mt-1 text-xs text-slate-400">
              Low swap fee shared between LPs and protocol
            </p>
          </div>
          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-4">
            <h3 className="text-sm font-semibold text-white">Earn</h3>
            <p className="mt-1 text-2xl font-bold text-purple-400">LP Yield</p>
            <p className="mt-1 text-xs text-slate-400">
              Provide liquidity and earn fees from every swap
            </p>
          </div>
        </div>
      </div>

      {/* LayerZero Chain IDs */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">LayerZero Chain IDs</h2>
        <p className="mt-3 text-sm text-slate-300">
          Stargate uses LayerZero endpoint chain IDs (different from EVM chain IDs).
          Use chain ID{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">299</code>{" "}
          for Dina Network.
        </p>

        <div className="mt-6 overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-slate-700">
                <th className="px-4 py-3 font-semibold text-white">Chain</th>
                <th className="px-4 py-3 font-semibold text-white">LZ Chain ID</th>
                <th className="px-4 py-3 font-semibold text-white">USDC Pool ID</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Dina Network</td>
                <td className="px-4 py-3">299</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-blue-400">1</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Ethereum</td>
                <td className="px-4 py-3">101</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">1</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Base</td>
                <td className="px-4 py-3">184</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">1</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Arbitrum</td>
                <td className="px-4 py-3">110</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">1</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Optimism</td>
                <td className="px-4 py-3">111</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">1</code>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Swap USDC Dina to Base */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Swap USDC from Dina to Base</h2>
        <p className="mt-3 text-sm text-slate-300">
          Use the Stargate Router to swap USDC cross-chain. Tokens are drawn from
          Dina&apos;s USDC pool and released from Base&apos;s pool.
        </p>

        <CodeBlock title="swap-stargate.ts">
          {`import { DinaClient, DinaWallet } from 'dina-js';

const client = new DinaClient("https://testnet.dina.network");
const wallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

const STARGATE_ROUTER = "dina1stargate_router...";

function parseUSDC(amount: string): bigint {
  return BigInt(Math.round(parseFloat(amount) * 1_000_000));
}

// Swap 100 USDC from Dina to Base via Stargate
const tx = await client.callContract(wallet, {
  contract: STARGATE_ROUTER,
  method: 'swap',
  args: {
    dstChainId: 184,                         // Base (LayerZero chain ID)
    srcPoolId: 1,                            // USDC pool on Dina
    dstPoolId: 1,                            // USDC pool on Base
    refundAddress: wallet.address,
    amountLD: parseUSDC("100"),
    minAmountLD: parseUSDC("99.90"),         // 0.1% slippage tolerance
    lzTxParams: {
      dstGasForCall: 200000,
      dstNativeAmount: 0,
      dstNativeAddr: "0x",
    },
    to: baseRecipientAddress,
    payload: "0x",                           // no additional payload
  },
});

console.log("Swap tx:", tx.hash);
console.log("LayerZero nonce:", tx.result);
// Tokens arrive on Base within 1-3 minutes`}
        </CodeBlock>
      </div>

      {/* Provide Liquidity */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Provide Liquidity</h2>
        <p className="mt-3 text-sm text-slate-300">
          LP providers deposit tokens into Stargate pools and earn a share of
          every cross-chain swap fee. LP tokens represent your proportional share
          of the pool.
        </p>

        <CodeBlock title="liquidity.ts">
          {`// Add liquidity to the USDC pool on Dina
const addTx = await client.callContract(wallet, {
  contract: STARGATE_ROUTER,
  method: 'add_liquidity',
  args: {
    poolId: 1,                               // USDC pool
    amount: parseUSDC("1000"),               // deposit 1,000 USDC
  },
});
console.log("LP tokens received:", addTx.result);

// Check your LP balance
const lpBalance = await client.callContract(wallet, {
  contract: STARGATE_ROUTER,
  method: 'lp_balance_of',
  args: {
    poolId: 1,
    provider: wallet.address,
  },
});
console.log("LP balance:", lpBalance.result);

// Remove liquidity (burn LP tokens to get back USDC)
const removeTx = await client.callContract(wallet, {
  contract: STARGATE_ROUTER,
  method: 'remove_liquidity',
  args: {
    poolId: 1,
    lpAmount: lpBalance.result,              // withdraw everything
  },
});
console.log("USDC returned:", removeTx.result);`}
        </CodeBlock>
      </div>

      {/* Pool Architecture */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Pool Architecture</h2>
        <p className="mt-3 text-sm text-slate-300">
          Stargate pools use a &quot;Delta algorithm&quot; to balance liquidity
          across chains. Each pool tracks credits with remote pools to ensure
          the system stays balanced even under heavy one-directional flow.
        </p>

        <div className="mt-6 overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-slate-700">
                <th className="px-4 py-3 font-semibold text-white">Pool ID</th>
                <th className="px-4 py-3 font-semibold text-white">Token</th>
                <th className="px-4 py-3 font-semibold text-white">Shared Decimals</th>
                <th className="px-4 py-3 font-semibold text-white">Description</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">1</td>
                <td className="px-4 py-3">USDC</td>
                <td className="px-4 py-3">6</td>
                <td className="px-4 py-3">Primary stablecoin pool</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">2</td>
                <td className="px-4 py-3">USDT</td>
                <td className="px-4 py-3">6</td>
                <td className="px-4 py-3">Tether stablecoin pool</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">13</td>
                <td className="px-4 py-3">ETH</td>
                <td className="px-4 py-3">18</td>
                <td className="px-4 py-3">Native ETH pool</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Fee Structure */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Fee Structure</h2>
        <p className="mt-3 text-sm text-slate-300">
          Stargate charges a small swap fee that is split between LP providers
          and the protocol treasury.
        </p>

        <div className="mt-6 overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-slate-700">
                <th className="px-4 py-3 font-semibold text-white">Component</th>
                <th className="px-4 py-3 font-semibold text-white">Rate</th>
                <th className="px-4 py-3 font-semibold text-white">Recipient</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Swap Fee</td>
                <td className="px-4 py-3">0.06%</td>
                <td className="px-4 py-3">LP providers + protocol</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">LP Share</td>
                <td className="px-4 py-3">~0.05%</td>
                <td className="px-4 py-3">Distributed to all LPs proportionally</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Protocol Share</td>
                <td className="px-4 py-3">~0.01%</td>
                <td className="px-4 py-3">Protocol treasury</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Supported Chains */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Supported Chains</h2>
        <p className="mt-3 text-sm text-slate-300">
          Stargate supports bridging between Dina and these chains via LayerZero:
        </p>
        <div className="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4">
          {[
            "Ethereum", "Base", "Arbitrum", "Optimism", "Polygon",
            "Avalanche", "BNB Chain", "Fantom", "Metis", "Kava",
            "Linea", "Mantle",
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

      {/* Why Stargate is Fast */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">
          Why Stargate Provides Instant Settlement
        </h2>
        <ol className="mt-4 list-inside list-decimal space-y-3 text-sm text-slate-300">
          <li>
            <span className="font-medium text-white">Pre-deployed liquidity:</span>{" "}
            LP providers have already deposited tokens into pools on every chain.
            There is no need to wait for tokens to be transferred -- they are
            already there.
          </li>
          <li>
            <span className="font-medium text-white">Unified pools:</span>{" "}
            Unlike wrapped token bridges, Stargate pools hold native tokens.
            Users receive the real asset on the destination chain, not a
            synthetic or wrapped version.
          </li>
          <li>
            <span className="font-medium text-white">LayerZero messaging:</span>{" "}
            Cross-chain coordination happens via LayerZero&apos;s ultra-light node
            network, which delivers messages in 1-3 minutes across most chains.
          </li>
        </ol>
      </div>

      {/* Next steps */}
      <div className="mt-12 rounded-xl border border-slate-800 bg-slate-900/40 p-6">
        <h3 className="text-base font-semibold text-white">Next steps</h3>
        <ul className="mt-3 space-y-2 text-sm text-slate-300">
          <li>
            <Link href="/docs/bridges/across" className="text-blue-400 hover:underline">
              Across Bridge
            </Link>{" "}
            -- relayer-based instant bridging, fastest option.
          </li>
          <li>
            <Link href="/docs/bridges/layerzero" className="text-blue-400 hover:underline">
              LayerZero
            </Link>{" "}
            -- the underlying messaging protocol powering Stargate.
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
