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

export default function WormholePage() {
  return (
    <div>
      <h1 className="text-4xl font-bold tracking-tight text-white">
        Wormhole Integration
      </h1>
      <p className="mt-4 text-lg text-slate-300">
        Wormhole connects Dina Network to 30+ chains including Solana, Sui,
        Aptos, and all major EVM networks. It uses a Guardian network of 19
        validators to verify cross-chain messages via Verifiable Action
        Approvals (VAAs).
      </p>

      {/* How Wormhole Works */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">How Wormhole Works</h2>
        <p className="mt-3 text-sm text-slate-300">
          Wormhole&apos;s Guardian network observes events on connected chains
          and produces signed VAAs (Verifiable Action Approvals) that can be
          verified on any destination chain.
        </p>

        <CodeBlock title="Wormhole Architecture">
          {`  Source Chain                 Guardian Network              Dina Network
  ============                 ================              ============

  1. User locks USDC
     in Wormhole
     Token Bridge
          |
          v
  2. Wormhole Core             3. 19 Guardians observe
     emits a message   ------->   the source chain
                                       |
                                       v
                               4. 13-of-19 Guardians
                                  sign the VAA
                                  (off-chain consensus)
                                       |
                                       v
                               5. Signed VAA      ------> 6. VAA submitted to
                                  published to              Dina Wormhole
                                  Guardian API              Core contract
                                                                 |
                                                                 v
                                                           7. Bridged USDC
                                                              minted to user

  Security: 13-of-19 Guardian multisig (no single point of failure)`}
        </CodeBlock>
      </div>

      {/* Key Concepts */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Key Concepts</h2>

        <div className="mt-6 space-y-4">
          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">VAA (Verifiable Action Approval)</h3>
            <p className="mt-2 text-sm text-slate-300">
              A signed message from the Guardian network proving that an event
              happened on a source chain. Contains the payload, emitter chain,
              sequence number, and 13+ Guardian signatures.
            </p>
          </div>

          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">Guardian Network</h3>
            <p className="mt-2 text-sm text-slate-300">
              19 independent validators run by organizations like Jump Crypto,
              Figment, Chorus One, and others. A VAA requires signatures from at
              least 13 Guardians (supermajority) to be valid.
            </p>
          </div>

          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">Token Bridge</h3>
            <p className="mt-2 text-sm text-slate-300">
              The Wormhole Token Bridge uses a lock-and-mint model. Tokens are
              locked on the source chain and wrapped tokens are minted on the
              destination. On Dina, Wormhole-bridged USDC is unified with the
              Bridged USDC standard.
            </p>
          </div>
        </div>
      </div>

      {/* Send USDC to Dina via Wormhole */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Bridge USDC to Dina via Wormhole</h2>

        <CodeBlock title="wormhole-bridge.ts">
          {`import { DinaClient, DinaWallet } from 'dina-js';
import {
  wormhole,
  TokenTransfer,
  Wormhole,
  Chain,
  amount,
} from '@wormhole-foundation/sdk';

// --- Configuration ---
const WORMHOLE_CORE_CONTRACT = "dina1wormhole_core...";
const WORMHOLE_TOKEN_BRIDGE  = "dina1wormhole_token_bridge...";
const DINA_CHAIN_ID = 99;  // Wormhole chain ID for Dina

// --- Step 1: Initiate transfer on source chain ---
async function sendFromSolana(
  sourceWallet: any,
  dinaRecipient: string,
  usdcAmount: string
) {
  const wh = await wormhole("Mainnet", []);

  // Get chain contexts
  const sourceChain = wh.getChain("Solana");
  const destChain   = wh.getChain("Dina");

  // Create a token transfer
  const xfer = await wh.tokenTransfer(
    "native",                              // token type
    amount.units(amount.parse(usdcAmount, 6)),  // amount in base units
    {
      chain: "Solana",
      address: sourceWallet.publicKey.toString(),
    },
    {
      chain: "Dina",
      address: dinaRecipient,
    },
    false, // not automatic relay
  );

  // Initiate the transfer (locks tokens on Solana)
  const srcTxIds = await xfer.initiateTransfer(sourceWallet);
  console.log("Source tx:", srcTxIds);

  return xfer;
}

// --- Step 2: Wait for Guardian attestation ---
async function waitForVAA(xfer: TokenTransfer) {
  // Poll for the signed VAA from Guardians
  const timeout = 15 * 60 * 1000; // 15 minutes
  const attestIds = await xfer.fetchAttestation(timeout);
  console.log("VAA attested:", attestIds);
  return attestIds;
}

// --- Step 3: Complete transfer on Dina ---
async function redeemOnDina(
  xfer: TokenTransfer,
  dinaWallet: DinaWallet
) {
  const client = new DinaClient("https://testnet.dina.network");

  // Submit the VAA to Dina's Wormhole core contract
  const tx = await client.callContract(dinaWallet, {
    contract: WORMHOLE_TOKEN_BRIDGE,
    method: "complete_transfer",
    args: {
      vaa: xfer.getTransferVAA(),
    },
  });

  console.log("Redeem tx on Dina:", tx.hash);

  // Verify balance
  const balance = await client.getBalance(dinaWallet.address);
  console.log("Dina balance:", balance.formatted);
}

// --- Full Flow ---
async function bridgeViaSolana() {
  const dinaWallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

  const xfer = await sendFromSolana(
    solanaWallet,
    dinaWallet.address,
    "100" // 100 USDC
  );

  await waitForVAA(xfer);
  await redeemOnDina(xfer, dinaWallet);
}`}
        </CodeBlock>
      </div>

      {/* Receive from Dina */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Bridge from Dina via Wormhole</h2>
        <p className="mt-3 text-sm text-slate-300">
          To send USDC from Dina to another chain, lock Bridged USDC in the
          Wormhole Token Bridge on Dina and redeem on the destination.
        </p>

        <CodeBlock title="wormhole-send-from-dina.ts">
          {`import { DinaClient, DinaWallet } from 'dina-js';

const client = new DinaClient("https://testnet.dina.network");
const wallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

// Lock Bridged USDC and emit a Wormhole message
const tx = await client.callContract(wallet, {
  contract: WORMHOLE_TOKEN_BRIDGE,
  method: "transfer_tokens",
  args: {
    token:       "dina1bridged_usdc...",
    amount:      100_000000,        // 100 USDC
    recipientChain: 1,              // Solana = 1
    recipient:   "SolanaAddress...",
    nonce:       0,
  },
});

console.log("Wormhole transfer initiated:", tx.hash);

// The VAA will be signed by Guardians (~15 min)
// Then redeem on Solana using the Wormhole SDK`}
        </CodeBlock>
      </div>

      {/* Supported Chains */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Supported Chains</h2>
        <p className="mt-3 text-sm text-slate-300">
          Wormhole connects Dina to 30+ chains across EVM, Solana, Move, and
          Cosmos ecosystems.
        </p>

        <div className="mt-4 overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-slate-700">
                <th className="px-4 py-3 font-semibold text-white">Ecosystem</th>
                <th className="px-4 py-3 font-semibold text-white">Chains</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">EVM</td>
                <td className="px-4 py-3">
                  Ethereum, Base, Arbitrum, Optimism, Polygon, Avalanche, BNB Chain, Celo, Fantom, Moonbeam, Klaytn
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Solana</td>
                <td className="px-4 py-3">Solana</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Move</td>
                <td className="px-4 py-3">Sui, Aptos</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Cosmos</td>
                <td className="px-4 py-3">Osmosis, Injective, Sei, Terra</td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Other</td>
                <td className="px-4 py-3">Near, Algorand, Karura</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Guardian Verification */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Guardian Verification</h2>
        <p className="mt-3 text-sm text-slate-300">
          You can verify a VAA on-chain or off-chain by checking the Guardian
          signatures.
        </p>

        <CodeBlock title="verify-vaa.ts">
          {`import { DinaClient } from 'dina-js';

const client = new DinaClient("https://testnet.dina.network");

// Verify a VAA against the Wormhole core contract
const isValid = await client.queryContract({
  contract: WORMHOLE_CORE_CONTRACT,
  method: "verify_vaa",
  args: {
    vaa: vaaBytes,
  },
});

console.log("VAA valid:", isValid);
// true = at least 13-of-19 Guardians signed this message

// You can also query the current Guardian set
const guardianSet = await client.queryContract({
  contract: WORMHOLE_CORE_CONTRACT,
  method: "get_guardian_set",
  args: {},
});

console.log("Guardian set index:", guardianSet.index);
console.log("Guardians:", guardianSet.keys.length); // 19`}
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
            -- the recommended bridge for USDC (burn-and-mint, no wrapped tokens).
          </li>
          <li>
            <Link href="/docs/bridges/usdc" className="text-blue-400 hover:underline">
              Bridged USDC Standard
            </Link>{" "}
            -- how Wormhole-bridged USDC is unified with other bridge tokens on Dina.
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
