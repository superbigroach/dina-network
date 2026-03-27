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

export default function BridgesOverviewPage() {
  return (
    <div>
      <h1 className="text-4xl font-bold tracking-tight text-white">
        Bridge USDC to Dina Network
      </h1>
      <p className="mt-4 text-lg text-slate-300">
        Dina Network supports 5 production bridges for moving USDC and other
        assets between Dina and external chains. All bridged assets on Dina use
        the{" "}
        <Link
          href="/docs/bridges/usdc"
          className="text-blue-400 underline decoration-blue-400/30 hover:decoration-blue-400"
        >
          Bridged USDC
        </Link>{" "}
        standard -- a 1:1 backed representation that can be
        upgraded to native USDC in the future.
      </p>

      {/* Status Banner */}
      <div className="mt-6 rounded-lg border border-yellow-600/30 bg-yellow-900/10 p-4">
        <h3 className="text-lg font-semibold text-yellow-400">Current Status</h3>
        <ul className="mt-2 space-y-1 text-sm text-slate-300">
          <li><span className="font-mono text-green-400">READY</span> — Base ↔ Dina bridge (no third-party approval needed)</li>
          <li><span className="font-mono text-slate-500">NOT ACTIVE</span> — CCTP, Wormhole, LayerZero, Axelar, Across, Stargate (Dina-side contracts deployed, waiting for third-party approval)</li>
        </ul>
        <p className="mt-2 text-xs text-slate-400">
          Only the Base bridge works today. Other bridges activate when the third-party protocol adds Dina as a supported chain. Application guides are in the repo at bridges/third-party/.
        </p>
      </div>

      {/* ASCII Architecture Diagram */}
      <div className="mt-8">
        <h2 className="text-2xl font-semibold text-white">How Bridging Works</h2>
        <CodeBlock title="Bridge Architecture">
          {`                         +--------------------+
                         |   Dina Network     |
                         |                    |
                         |  Bridged USDC      |
                         |  (1:1 backed)      |
                         +--------+-----------+
                                  |
              +-------------------+-------------------+
              |          |          |         |        |
        +-----+---+ +---+----+ +--+---+ +---+---+ +--+----+
        |  CCTP   | |Wormhole| |Layer | |Axelar | | Base  |
        | (Circle)| |        | | Zero | |       | |Bridge |
        +---------+ +--------+ +------+ +-------+ +-------+
              |          |          |         |        |
     +--------+--+  +----+---+  +--+---+  +--+---+  +-+----+
     | Ethereum  |  | Solana |  | Arb  |  | Cosmos|  | Base |
     | Base      |  | Sui    |  | OP   |  | Avax  |  |      |
     | Polygon   |  | Aptos  |  | BNB  |  | Polygon  |      |
     | Avalanche |  | ...    |  | ...  |  | ...   |  |      |
     +-----------+  +--------+  +------+  +-------+  +------+`}
        </CodeBlock>
      </div>

      {/* Comparison Table */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Bridge Comparison</h2>
        <p className="mt-3 text-sm text-slate-300">
          Each bridge has different trade-offs in speed, security, and chain
          coverage. Choose the one that fits your use case.
        </p>

        <div className="mt-6 overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-slate-700">
                <th className="px-4 py-3 font-semibold text-white">Bridge</th>
                <th className="px-4 py-3 font-semibold text-white">Type</th>
                <th className="px-4 py-3 font-semibold text-white">Chains Connected</th>
                <th className="px-4 py-3 font-semibold text-white">Speed</th>
                <th className="px-4 py-3 font-semibold text-white">Security Model</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">
                  <Link href="/docs/bridges/cctp" className="text-blue-400 hover:underline">
                    Circle CCTP
                  </Link>
                </td>
                <td className="px-4 py-3">Burn / Mint</td>
                <td className="px-4 py-3">21+ chains</td>
                <td className="px-4 py-3">~20 min</td>
                <td className="px-4 py-3">Circle attestation service</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">
                  <Link href="/docs/bridges/wormhole" className="text-blue-400 hover:underline">
                    Wormhole
                  </Link>
                </td>
                <td className="px-4 py-3">Lock / Mint</td>
                <td className="px-4 py-3">30+ chains</td>
                <td className="px-4 py-3">~15 min</td>
                <td className="px-4 py-3">Guardian multisig (19 of 13)</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">
                  <Link href="/docs/bridges/layerzero" className="text-blue-400 hover:underline">
                    LayerZero
                  </Link>
                </td>
                <td className="px-4 py-3">OFT</td>
                <td className="px-4 py-3">40+ chains</td>
                <td className="px-4 py-3">~10 min</td>
                <td className="px-4 py-3">Oracle + Relayer</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">
                  <Link href="/docs/bridges/axelar" className="text-blue-400 hover:underline">
                    Axelar
                  </Link>
                </td>
                <td className="px-4 py-3">ITS</td>
                <td className="px-4 py-3">60+ chains</td>
                <td className="px-4 py-3">~20 min</td>
                <td className="px-4 py-3">Validator consensus</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Base Bridge</td>
                <td className="px-4 py-3">Lock / Mint</td>
                <td className="px-4 py-3">Base only</td>
                <td className="px-4 py-3">~10 min</td>
                <td className="px-4 py-3">Relayer</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Which Bridge to Use */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Which Bridge Should You Use?</h2>

        <div className="mt-6 space-y-4">
          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">For USDC transfers (recommended)</h3>
            <p className="mt-2 text-sm text-slate-300">
              Use{" "}
              <Link href="/docs/bridges/cctp" className="text-blue-400 hover:underline">
                Circle CCTP
              </Link>
              . It is the official Circle bridge for USDC, uses a burn-and-mint
              mechanism (no wrapped tokens), and provides the clearest upgrade
              path to native USDC on Dina.
            </p>
          </div>

          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">For maximum chain coverage</h3>
            <p className="mt-2 text-sm text-slate-300">
              Use{" "}
              <Link href="/docs/bridges/axelar" className="text-blue-400 hover:underline">
                Axelar
              </Link>{" "}
              (60+ chains) or{" "}
              <Link href="/docs/bridges/layerzero" className="text-blue-400 hover:underline">
                LayerZero
              </Link>{" "}
              (40+ chains). Both support EVM, Cosmos, and non-EVM chains.
            </p>
          </div>

          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">For Solana, Sui, or Aptos</h3>
            <p className="mt-2 text-sm text-slate-300">
              Use{" "}
              <Link href="/docs/bridges/wormhole" className="text-blue-400 hover:underline">
                Wormhole
              </Link>
              . It has the strongest support for non-EVM chains including Solana,
              Sui, Aptos, and Near.
            </p>
          </div>

          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">For fastest transfers</h3>
            <p className="mt-2 text-sm text-slate-300">
              Use{" "}
              <Link href="/docs/bridges/layerzero" className="text-blue-400 hover:underline">
                LayerZero
              </Link>
              . Its Ultra Light Node architecture typically delivers the fastest
              cross-chain messages at ~10 minutes.
            </p>
          </div>

          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">For Base-only transfers</h3>
            <p className="mt-2 text-sm text-slate-300">
              Use the Base Bridge for direct Base-to-Dina transfers. This is the
              simplest option if your assets are already on Base.
            </p>
          </div>
        </div>
      </div>

      {/* Bridged USDC */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Bridged USDC on Dina</h2>
        <p className="mt-3 text-sm text-slate-300">
          Regardless of which bridge you use, all USDC on Dina Network is
          represented as{" "}
          <Link href="/docs/bridges/usdc" className="text-blue-400 hover:underline">
            Bridged USDC
          </Link>
          . This is a single, unified token that is 1:1 backed by USDC locked or
          burned on the source chain. There are no competing wrapped versions --
          every bridge mints the same Bridged USDC token.
        </p>
        <p className="mt-3 text-sm text-slate-300">
          When Circle enables native USDC on Dina, all Bridged USDC will be
          automatically upgraded at a 1:1 ratio with zero action required from
          holders.
        </p>
      </div>

      {/* Quick Start */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Quick Start: Bridge from Base</h2>
        <p className="mt-3 text-sm text-slate-300">
          The fastest way to get USDC on Dina is via CCTP from Base.
        </p>
        <CodeBlock title="bridge-from-base.ts">
          {`import { DinaClient, DinaWallet } from 'dina-js';

const client = new DinaClient("https://testnet.dina.network");
const wallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

// 1. Approve CCTP contract to spend your USDC on Base
// (done on the Base side using ethers.js or viem)

// 2. Call depositForBurn on Base CCTP contract
// This burns USDC on Base and emits a message

// 3. Wait for Circle attestation (~20 min)
// Poll: https://iris-api.circle.com/attestations/{messageHash}

// 4. Receive minted Bridged USDC on Dina
const balance = await client.getBalance(wallet.address);
console.log("Dina balance:", balance.formatted);`}
        </CodeBlock>
      </div>

      {/* Next steps */}
      <div className="mt-12 rounded-xl border border-slate-800 bg-slate-900/40 p-6">
        <h3 className="text-base font-semibold text-white">Next steps</h3>
        <ul className="mt-3 space-y-2 text-sm text-slate-300">
          <li>
            <Link href="/docs/bridges/cctp" className="text-blue-400 hover:underline">
              Circle CCTP Integration
            </Link>{" "}
            -- the recommended bridge for USDC with full code examples.
          </li>
          <li>
            <Link href="/docs/bridges/usdc" className="text-blue-400 hover:underline">
              Bridged USDC Standard
            </Link>{" "}
            -- understand the token contract and upgrade path.
          </li>
          <li>
            <Link href="/docs/bridges/wormhole" className="text-blue-400 hover:underline">
              Wormhole Integration
            </Link>{" "}
            -- bridge from Solana, Sui, and 30+ other chains.
          </li>
        </ul>
      </div>
    </div>
  );
}
