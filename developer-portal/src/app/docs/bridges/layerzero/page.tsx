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

export default function LayerZeroPage() {
  return (
    <div>
      <h1 className="text-4xl font-bold tracking-tight text-white">
        LayerZero Integration
      </h1>
      <p className="mt-4 text-lg text-slate-300">
        LayerZero connects Dina Network to 40+ chains using Ultra Light Nodes
        (ULNs) with an independent Oracle and Relayer security model. It
        supports the OFT (Omnichain Fungible Token) standard for seamless
        cross-chain token transfers.
      </p>

      {/* How LayerZero Works */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">How LayerZero Works</h2>
        <p className="mt-3 text-sm text-slate-300">
          LayerZero uses a dual-verification system: an Oracle provides block
          headers, and a Relayer delivers transaction proofs. Neither party can
          forge a message alone -- both must agree for a message to be valid.
        </p>

        <CodeBlock title="LayerZero Architecture">
          {`  Source Chain               LayerZero Protocol             Dina Network
  ============               ==================             ============

  1. User calls
     OFT.send() on
     source chain
         |
         v
  2. LayerZero Endpoint
     emits a Packet     ----+
                            |
                            +---> 3. Oracle reads the
                            |        block header from
                            |        source chain
                            |             |
                            |             v
                            |     4. Oracle submits
                            |        block header to  ---> 6. Dina Endpoint
                            |        Dina Endpoint          validates:
                            |                               - block header
                            +---> 5. Relayer reads          - tx proof
                                     tx proof from          match?
                                     source chain    --->      |
                                                               v
                                                          7. OFT.receive()
                                                             mints Bridged
                                                             USDC on Dina

  Security: Oracle and Relayer are independent -- collusion required to forge`}
        </CodeBlock>
      </div>

      {/* OFT Standard */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">OFT (Omnichain Fungible Token)</h2>
        <p className="mt-3 text-sm text-slate-300">
          OFT is LayerZero&apos;s standard for cross-chain tokens. Instead of
          wrapping, OFT burns tokens on the source chain and mints on the
          destination, maintaining a unified supply across all chains.
        </p>

        <div className="mt-6 space-y-4">
          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">OFT (burn and mint)</h3>
            <p className="mt-2 text-sm text-slate-300">
              Used when the token originates on the chain. The source OFT
              contract burns tokens and sends a LayerZero message. The
              destination OFT contract mints tokens upon receiving the message.
            </p>
          </div>

          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">OFTAdapter (lock and mint)</h3>
            <p className="mt-2 text-sm text-slate-300">
              Used when the token already exists on a chain (e.g., native USDC on
              Ethereum). The adapter locks tokens on the source side and signals
              the destination OFT to mint.
            </p>
          </div>
        </div>
      </div>

      {/* Bridge USDC to Dina */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Bridge USDC to Dina via LayerZero</h2>

        <CodeBlock title="layerzero-bridge.ts">
          {`import { DinaClient, DinaWallet } from 'dina-js';
import { ethers } from 'ethers';
import { Options } from '@layerzerolabs/lz-v2-utilities';

// --- Configuration ---
const DINA_LZ_ENDPOINT_ID = 30099;  // LayerZero v2 endpoint ID for Dina
const BASE_LZ_ENDPOINT_ID = 30184;  // LayerZero v2 endpoint ID for Base
const OFT_ADAPTER_BASE    = "0xOFTAdapterOnBase...";
const OFT_DINA             = "dina1oft_usdc...";

// --- Step 1: Send from Base to Dina ---
async function sendFromBase(
  signer: ethers.Signer,
  dinaRecipient: string,
  usdcAmount: bigint
) {
  const adapter = new ethers.Contract(OFT_ADAPTER_BASE, [
    "function send((uint32,bytes32,uint256,uint256,bytes,bytes,bytes),bytes,(address,uint256))",
    "function quoteSend((uint32,bytes32,uint256,uint256,bytes,bytes,bytes),bool) view returns (uint256,uint256)",
  ], signer);

  // Build send parameters
  const sendParam = {
    dstEid:        DINA_LZ_ENDPOINT_ID,
    to:            ethers.zeroPadValue(dinaRecipient, 32),
    amountLD:      usdcAmount,
    minAmountLD:   usdcAmount * 99n / 100n,  // 1% slippage
    extraOptions:  Options.newOptions()
                     .addExecutorLzReceiveOption(200000, 0)
                     .toHex(),
    composeMsg:    "0x",
    oftCmd:        "0x",
  };

  // Quote the cross-chain fee
  const [nativeFee] = await adapter.quoteSend(sendParam, false);
  console.log("LayerZero fee:", ethers.formatEther(nativeFee), "ETH");

  // Send the transaction
  const tx = await adapter.send(
    sendParam,
    { nativeFee, lzTokenFee: 0 },
    { fee: nativeFee },
    { value: nativeFee }
  );

  const receipt = await tx.wait();
  console.log("Send tx:", receipt.hash);
  return receipt;
}

// --- Step 2: Receive on Dina (automatic) ---
// LayerZero's Relayer and Oracle deliver the message automatically.
// The OFT contract on Dina mints Bridged USDC to the recipient.

// --- Step 3: Verify on Dina ---
async function verifyOnDina(dinaRecipient: string) {
  const client = new DinaClient("https://testnet.dina.network");
  const balance = await client.getBalance(dinaRecipient);
  console.log("Dina balance:", balance.formatted);
}`}
        </CodeBlock>
      </div>

      {/* Send from Dina */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Bridge from Dina via LayerZero</h2>

        <CodeBlock title="layerzero-send-from-dina.ts">
          {`import { DinaClient, DinaWallet } from 'dina-js';

const client = new DinaClient("https://testnet.dina.network");
const wallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

// Quote the cross-chain fee
const quote = await client.queryContract({
  contract: OFT_DINA,
  method: "quote_send",
  args: {
    dst_eid:     BASE_LZ_ENDPOINT_ID,
    to:          "0xYourBaseAddress...",
    amount:      100_000000,   // 100 USDC
    min_amount:  99_000000,    // 1% slippage
  },
});

console.log("Fee:", quote.nativeFee, "USDC");

// Send USDC from Dina to Base
const tx = await client.callContract(wallet, {
  contract: OFT_DINA,
  method: "send",
  args: {
    dst_eid:       BASE_LZ_ENDPOINT_ID,
    to:            "0xYourBaseAddress...",
    amount:        100_000000,
    min_amount:    99_000000,
    extra_options: "",
    compose_msg:   "",
    oft_cmd:       "",
  },
  value: quote.nativeFee,
});

console.log("LayerZero send initiated:", tx.hash);
// Bridged USDC is burned on Dina, USDC unlocked on Base (~10 min)`}
        </CodeBlock>
      </div>

      {/* Trusted Remote Configuration */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Trusted Remote Configuration</h2>
        <p className="mt-3 text-sm text-slate-300">
          LayerZero requires each OFT deployment to set trusted remote peers --
          the contract addresses on other chains that are authorized to send
          messages. This prevents unauthorized contracts from minting tokens.
        </p>

        <CodeBlock title="trusted-remote.ts">
          {`import { DinaClient, DinaWallet } from 'dina-js';

const client = new DinaClient("https://testnet.dina.network");
const wallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

// Set the trusted peer for Base on the Dina OFT contract
// (only callable by the contract owner)
const tx = await client.callContract(wallet, {
  contract: OFT_DINA,
  method: "set_peer",
  args: {
    eid:  BASE_LZ_ENDPOINT_ID,        // 30184
    peer: "0xOFTAdapterOnBase...",     // 32-byte padded address
  },
});

console.log("Peer set:", tx.hash);

// Query current peers
const peer = await client.queryContract({
  contract: OFT_DINA,
  method: "get_peer",
  args: { eid: BASE_LZ_ENDPOINT_ID },
});

console.log("Base peer:", peer);  // 0xOFTAdapterOnBase...`}
        </CodeBlock>

        <CodeBlock title="Security Architecture">
          {`  Trusted Remote Peers
  ====================

  Dina OFT Contract
  +-------------------------------------------+
  | Peer Table:                               |
  |   30184 (Base)      -> 0xOFTAdapter...    |
  |   30101 (Ethereum)  -> 0xOFTAdapter...    |
  |   30110 (Arbitrum)  -> 0xOFTAdapter...    |
  |   30111 (Optimism)  -> 0xOFTAdapter...    |
  |                                           |
  | Rule: Only accept messages from peers     |
  |       registered in this table.           |
  +-------------------------------------------+

  Messages from unknown contracts are REJECTED.`}
        </CodeBlock>
      </div>

      {/* Supported Chains */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Supported Chains</h2>
        <div className="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4">
          {[
            "Ethereum", "Base", "Arbitrum", "Optimism", "Polygon", "Avalanche",
            "BNB Chain", "Fantom", "Celo", "Moonbeam", "Gnosis", "Mantle",
            "Scroll", "Linea", "zkSync Era", "Polygon zkEVM", "Manta",
            "Blast", "Mode", "Fraxtal", "Metis", "Kava", "Core DAO",
            "Harmony", "Tenet", "Loot", "Astar", "Sei", "Injective",
            "Neutron", "Osmosis", "Telos", "Fuse", "Canto", "Klaytn",
            "Aurora", "OKX Chain", "Cronos", "Boba", "DFK Chain",
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

      {/* Next steps */}
      <div className="mt-12 rounded-xl border border-slate-800 bg-slate-900/40 p-6">
        <h3 className="text-base font-semibold text-white">Next steps</h3>
        <ul className="mt-3 space-y-2 text-sm text-slate-300">
          <li>
            <Link href="/docs/bridges/cctp" className="text-blue-400 hover:underline">
              Circle CCTP
            </Link>{" "}
            -- the recommended bridge for USDC specifically.
          </li>
          <li>
            <Link href="/docs/bridges/usdc" className="text-blue-400 hover:underline">
              Bridged USDC Standard
            </Link>{" "}
            -- how LayerZero OFT tokens map to unified Bridged USDC on Dina.
          </li>
          <li>
            <Link href="/docs/contracts/deploy" className="text-blue-400 hover:underline">
              Deploy a Contract
            </Link>{" "}
            -- deploy your own OFT contract on Dina.
          </li>
        </ul>
      </div>
    </div>
  );
}
