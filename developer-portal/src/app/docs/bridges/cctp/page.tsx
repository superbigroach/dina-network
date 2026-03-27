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

export default function CCTPPage() {
  return (
    <div>
      <h1 className="text-4xl font-bold tracking-tight text-white">
        Circle CCTP Integration
      </h1>
      <p className="mt-4 text-lg text-slate-300">
        Circle&apos;s Cross-Chain Transfer Protocol (CCTP) is the recommended
        way to bridge USDC to and from Dina Network. CCTP uses a native
        burn-and-mint mechanism -- USDC is burned on the source chain and
        freshly minted on the destination chain, eliminating wrapped token risk.
      </p>

      {/* How CCTP Works */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">How CCTP Works</h2>
        <p className="mt-3 text-sm text-slate-300">
          CCTP transfers follow a three-step process: burn on source, attest via
          Circle, and mint on destination.
        </p>

        <CodeBlock title="CCTP Flow Diagram">
          {`  Source Chain (e.g. Base)              Circle               Dina Network
  ========================          ==========          ================

  1. User calls
     depositForBurn()
     on TokenMessenger
           |
           v
  2. USDC is burned
     MessageSent event --------> 3. Circle observes
                                    the burn event
                                         |
                                         v
                                 4. Circle signs an
                                    attestation
                                    (off-chain)
                                         |
                                         v
                                 5. Attestation    ---> 6. receiveMessage()
                                    available via       called on Dina
                                    Circle API               |
                                                             v
                                                       7. Bridged USDC
                                                          minted to
                                                          recipient`}
        </CodeBlock>
      </div>

      {/* Domain IDs */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Domain IDs</h2>
        <p className="mt-3 text-sm text-slate-300">
          Each chain in the CCTP network is assigned a unique domain ID. Use
          domain <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">99</code>{" "}
          for Dina Network.
        </p>

        <div className="mt-6 overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-slate-700">
                <th className="px-4 py-3 font-semibold text-white">Chain</th>
                <th className="px-4 py-3 font-semibold text-white">Domain ID</th>
                <th className="px-4 py-3 font-semibold text-white">TokenMessenger Contract</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Dina Network</td>
                <td className="px-4 py-3">99</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-blue-400">dina1cctp_token_messenger...</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Ethereum</td>
                <td className="px-4 py-3">0</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">0xBd3fa81B58Ba92a82136038B25aDec7066af3155</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Avalanche</td>
                <td className="px-4 py-3">1</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">0x6B25532e1060CE10cc3B0A99e5683b91BFDe6982</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Optimism</td>
                <td className="px-4 py-3">2</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">0x2B4069517957735bE00ceE0fadAE88a26365528f</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Arbitrum</td>
                <td className="px-4 py-3">3</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">0x19330d10D9Cc8751218eaf51E8885D058642E08A</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Base</td>
                <td className="px-4 py-3">6</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">0x1682Ae6375C4E4A97e4B583BC394c861A46D8962</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Polygon</td>
                <td className="px-4 py-3">7</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">0x9daF8c91AEFAE50b9c0E69629D3F6Ca40cA3B3FE</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Solana</td>
                <td className="px-4 py-3">5</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">CCTPmbSD7gX1bxKPAmg77w8oFzNFpaQiQUWD43TKaecd</code>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Bridge USDC to Dina */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Bridge USDC from Base to Dina</h2>
        <p className="mt-3 text-sm text-slate-300">
          This example shows the full end-to-end flow for bridging USDC from
          Base to Dina using CCTP.
        </p>

        <CodeBlock title="bridge-cctp.ts">
          {`import { DinaClient, DinaWallet } from 'dina-js';
import { ethers } from 'ethers';

// --- Configuration ---
const CCTP_CONTRACT = "dina1cctp_token_messenger...";
const BASE_TOKEN_MESSENGER = "0x1682Ae6375C4E4A97e4B583BC394c861A46D8962";
const BASE_USDC = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
const DINA_DOMAIN = 99;
const BASE_DOMAIN = 6;

// Utility: convert USDC amount to 6-decimal integer
function parseUSDC(amount: string): bigint {
  return BigInt(Math.round(parseFloat(amount) * 1_000_000));
}

// --- Step 1: Burn USDC on Base ---
async function burnOnBase(
  signer: ethers.Signer,
  amount: string,
  dinaRecipient: string
) {
  const usdc = new ethers.Contract(BASE_USDC, [
    "function approve(address,uint256) returns (bool)",
  ], signer);

  const messenger = new ethers.Contract(BASE_TOKEN_MESSENGER, [
    "function depositForBurn(uint256,uint32,bytes32,address) returns (uint64)",
  ], signer);

  // Approve TokenMessenger to spend USDC
  const approveTx = await usdc.approve(
    BASE_TOKEN_MESSENGER,
    parseUSDC(amount)
  );
  await approveTx.wait();

  // Burn USDC -- this emits a MessageSent event
  const burnTx = await messenger.depositForBurn(
    parseUSDC(amount),
    DINA_DOMAIN,                                    // destinationDomain
    ethers.zeroPadValue(dinaRecipient, 32),          // mintRecipient (32 bytes)
    BASE_USDC                                        // burnToken
  );
  const receipt = await burnTx.wait();

  // Extract message hash from MessageSent event
  const messageHash = receipt.logs[receipt.logs.length - 1].data;
  console.log("Burn tx:", receipt.hash);
  console.log("Message hash:", messageHash);

  return messageHash;
}

// --- Step 2: Wait for Circle Attestation ---
async function waitForAttestation(messageHash: string): Promise<string> {
  const url = \`https://iris-api.circle.com/attestations/\${messageHash}\`;

  while (true) {
    const res = await fetch(url);
    const data = await res.json();

    if (data.status === "complete") {
      console.log("Attestation received!");
      return data.attestation;
    }

    console.log("Waiting for attestation... (status:", data.status, ")");
    await new Promise(r => setTimeout(r, 30_000)); // poll every 30s
  }
}

// --- Step 3: Mint Bridged USDC on Dina ---
async function mintOnDina(
  client: DinaClient,
  wallet: DinaWallet,
  messageBytes: string,
  attestation: string
) {
  const tx = await client.callContract(wallet, {
    contract: CCTP_CONTRACT,
    method: "receive_message",
    args: {
      message: messageBytes,
      attestation: attestation,
    },
  });

  console.log("Mint tx:", tx.hash);
  console.log("Bridged USDC minted on Dina!");

  return tx;
}

// --- Full Flow ---
async function bridgeFromBase() {
  // Base side
  const provider = new ethers.JsonRpcProvider("https://mainnet.base.org");
  const signer = new ethers.Wallet(process.env.BASE_PRIVATE_KEY!, provider);

  // Dina side
  const dinaClient = new DinaClient("https://testnet.dina.network");
  const dinaWallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

  // Bridge 100 USDC
  const messageHash = await burnOnBase(signer, "100", dinaWallet.address);
  const attestation = await waitForAttestation(messageHash);
  await mintOnDina(dinaClient, dinaWallet, messageHash, attestation);

  // Verify
  const balance = await dinaClient.getBalance(dinaWallet.address);
  console.log("Dina balance:", balance.formatted);
}`}
        </CodeBlock>
      </div>

      {/* Bridge USDC from Dina */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Bridge USDC from Dina to Base</h2>
        <p className="mt-3 text-sm text-slate-300">
          To bridge back, burn Bridged USDC on Dina and mint native USDC on the
          destination chain.
        </p>

        <CodeBlock title="bridge-to-base.ts">
          {`import { DinaClient, DinaWallet } from 'dina-js';

const client = new DinaClient("https://testnet.dina.network");
const wallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

// Burn Bridged USDC on Dina
const tx = await client.callContract(wallet, {
  contract: CCTP_CONTRACT,
  method: 'deposit_for_burn',
  args: {
    amount: parseUSDC("100"),
    destinationDomain: 6,   // Base
    mintRecipient: "0xYourBaseAddress...",
    burnToken: "dina1bridged_usdc...",
  },
});

console.log("Burn tx on Dina:", tx.hash);

// Then wait for attestation and call receiveMessage on Base
// (same pattern as above, reversed)`}
        </CodeBlock>
      </div>

      {/* Upgrade Path */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">
          Upgrade Path: Bridged USDC to Native USDC
        </h2>
        <p className="mt-3 text-sm text-slate-300">
          Circle&apos;s standard process for new chains is:
        </p>
        <ol className="mt-4 list-inside list-decimal space-y-3 text-sm text-slate-300">
          <li>
            <span className="font-medium text-white">Phase 1 -- Bridged USDC:</span>{" "}
            CCTP bridges native USDC from supported chains. On Dina, users
            receive Bridged USDC, which is 1:1 backed and fully redeemable.
          </li>
          <li>
            <span className="font-medium text-white">Phase 2 -- Circle Review:</span>{" "}
            Circle evaluates the chain for security, volume, and ecosystem
            maturity. This typically takes 3-6 months.
          </li>
          <li>
            <span className="font-medium text-white">Phase 3 -- Native USDC:</span>{" "}
            Circle deploys their native USDC contract on Dina. All Bridged USDC
            is atomically upgraded to native USDC at a 1:1 ratio. No user
            action required.
          </li>
        </ol>
        <p className="mt-4 text-sm text-slate-400">
          During the upgrade, the Bridged USDC contract owner transfers
          ownership to Circle, and Circle assumes minting authority. The token
          address and balances remain unchanged.
        </p>
      </div>

      {/* Supported Chains */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Supported Chains</h2>
        <p className="mt-3 text-sm text-slate-300">
          CCTP V2 supports 21+ chains. You can bridge USDC to Dina from any of
          these:
        </p>
        <div className="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4">
          {[
            "Ethereum", "Avalanche", "Optimism", "Arbitrum", "Base", "Polygon",
            "Solana", "Noble (Cosmos)", "Sui", "Aptos", "Polkadot", "Near",
            "Unichain", "Linea", "Sonic", "Hedera", "Mantle", "Sei",
            "ZKsync", "Celo", "Abstract",
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

      {/* Error Handling */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Error Handling</h2>
        <CodeBlock title="error-handling.ts">
          {`// Common CCTP errors and how to handle them

try {
  const tx = await client.callContract(wallet, {
    contract: CCTP_CONTRACT,
    method: 'deposit_for_burn',
    args: { amount, destinationDomain, mintRecipient, burnToken },
  });
} catch (error) {
  if (error.code === 'INSUFFICIENT_BALANCE') {
    // User does not have enough Bridged USDC
    console.error("Not enough USDC. Balance:", await client.getBalance(wallet.address));
  } else if (error.code === 'INVALID_DOMAIN') {
    // Destination domain is not supported
    console.error("Invalid destination domain:", destinationDomain);
  } else if (error.code === 'ATTESTATION_TIMEOUT') {
    // Circle attestation took too long (retry after delay)
    console.error("Attestation timed out. The burn is safe -- retry receiving.");
  } else {
    throw error;
  }
}`}
        </CodeBlock>
      </div>

      {/* Next steps */}
      <div className="mt-12 rounded-xl border border-slate-800 bg-slate-900/40 p-6">
        <h3 className="text-base font-semibold text-white">Next steps</h3>
        <ul className="mt-3 space-y-2 text-sm text-slate-300">
          <li>
            <Link href="/docs/bridges/usdc" className="text-blue-400 hover:underline">
              Bridged USDC Standard
            </Link>{" "}
            -- understand the token contract and how to interact with it.
          </li>
          <li>
            <Link href="/docs/bridges" className="text-blue-400 hover:underline">
              Bridge Overview
            </Link>{" "}
            -- compare CCTP with other bridges.
          </li>
          <li>
            <Link href="/docs/transactions/transfer" className="text-blue-400 hover:underline">
              Send USDC on Dina
            </Link>{" "}
            -- once bridged, learn how to transfer USDC within Dina.
          </li>
        </ul>
      </div>
    </div>
  );
}
