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

export default function AxelarPage() {
  return (
    <div>
      <h1 className="text-4xl font-bold tracking-tight text-white">
        Axelar Integration
      </h1>
      <p className="mt-4 text-lg text-slate-300">
        Axelar connects Dina Network to 60+ chains using its decentralized
        validator network and General Message Passing (GMP) protocol. It also
        offers the Interchain Token Service (ITS) for deploying tokens that
        exist natively on multiple chains.
      </p>

      {/* How Axelar Works */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">How Axelar Works</h2>
        <p className="mt-3 text-sm text-slate-300">
          Axelar runs its own Proof-of-Stake blockchain with a decentralized
          validator set. Validators collectively verify cross-chain messages
          using threshold cryptography -- no single validator can forge a
          message.
        </p>

        <CodeBlock title="Axelar Architecture">
          {`  Source Chain              Axelar Network                Dina Network
  ============              ==============                ============

  1. User calls
     Gateway.sendToken()
     or
     Gateway.callContract()
         |
         v
  2. Axelar Gateway          3. Axelar validators
     emits event     ------->   observe the event
                                     |
                                     v
                             4. Validators reach
                                consensus via
                                threshold signing
                                     |
                                     v
                             5. Axelar relayer      ---> 6. Axelar Gateway
                                submits approved          on Dina executes
                                message to Dina           the command
                                                               |
                                                               v
                                                          7. Bridged USDC
                                                             minted (or
                                                             GMP executed)

  Security: Decentralized PoS validators with quadratic voting`}
        </CodeBlock>
      </div>

      {/* Key Concepts */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Key Concepts</h2>

        <div className="mt-6 space-y-4">
          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">General Message Passing (GMP)</h3>
            <p className="mt-2 text-sm text-slate-300">
              GMP lets you send arbitrary data and function calls across chains.
              Unlike simple token bridges, GMP enables composable cross-chain
              applications -- for example, bridge USDC and simultaneously stake
              it in a Dina contract, all in one transaction.
            </p>
          </div>

          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">Interchain Token Service (ITS)</h3>
            <p className="mt-2 text-sm text-slate-300">
              ITS allows deploying a single token that exists natively on
              multiple chains. The token maintains a unified supply through
              Axelar&apos;s cross-chain messaging. On Dina, USDC bridged via ITS
              is unified with the Bridged USDC standard.
            </p>
          </div>

          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">Gateway Contract</h3>
            <p className="mt-2 text-sm text-slate-300">
              Each connected chain has an Axelar Gateway contract that serves as
              the entry/exit point for cross-chain messages. On Dina, the Gateway
              validates messages signed by the Axelar validator set.
            </p>
          </div>

          <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-5">
            <h3 className="font-semibold text-white">Gas Service</h3>
            <p className="mt-2 text-sm text-slate-300">
              The Axelar Gas Service allows you to prepay destination chain gas
              on the source chain. This means users do not need native tokens on
              the destination chain to complete a transfer.
            </p>
          </div>
        </div>
      </div>

      {/* Bridge USDC via Axelar */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Bridge USDC to Dina via Axelar</h2>

        <CodeBlock title="axelar-bridge.ts">
          {`import { DinaClient, DinaWallet } from 'dina-js';
import { ethers } from 'ethers';
import {
  AxelarGMPRecoveryAPI,
  AxelarQueryAPI,
  Environment,
  EvmChain,
  GasToken,
} from '@axelar-network/axelarjs-sdk';

// --- Configuration ---
const AXELAR_GATEWAY_BASE   = "0xe432150cce91c13a887f7D836923d5597adD8E31";
const AXELAR_GAS_SERVICE    = "0x2d5d7d31F671F86C782533cc367F14109a082712";
const AXELAR_GATEWAY_DINA   = "dina1axelar_gateway...";
const BASE_USDC             = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
const DINA_CHAIN_NAME       = "dina";

// --- Step 1: Estimate cross-chain gas ---
async function estimateGas(
  sourceChain: string,
  amount: bigint
): Promise<bigint> {
  const api = new AxelarQueryAPI({ environment: Environment.MAINNET });

  const gasFee = await api.estimateGasFee(
    sourceChain,
    DINA_CHAIN_NAME,
    GasToken.ETH,
    700000,  // destination gas limit
  );

  console.log("Estimated gas fee:", ethers.formatEther(gasFee.toString()), "ETH");
  return BigInt(gasFee.toString());
}

// --- Step 2: Send USDC from Base to Dina ---
async function sendFromBase(
  signer: ethers.Signer,
  dinaRecipient: string,
  usdcAmount: bigint
) {
  // Approve Gateway to spend USDC
  const usdc = new ethers.Contract(BASE_USDC, [
    "function approve(address,uint256) returns (bool)",
  ], signer);
  await (await usdc.approve(AXELAR_GATEWAY_BASE, usdcAmount)).wait();

  // Pay for destination gas
  const gasService = new ethers.Contract(AXELAR_GAS_SERVICE, [
    "function payNativeGasForContractCallWithToken(address,string,string,bytes,string,uint256,address) payable",
  ], signer);

  const gasFee = await estimateGas(EvmChain.BASE, usdcAmount);

  await (await gasService.payNativeGasForContractCallWithToken(
    await signer.getAddress(),
    DINA_CHAIN_NAME,
    dinaRecipient,
    "0x",                    // no additional payload
    "USDC",
    usdcAmount,
    await signer.getAddress(),
    { value: gasFee }
  )).wait();

  // Send tokens through Gateway
  const gateway = new ethers.Contract(AXELAR_GATEWAY_BASE, [
    "function sendToken(string,string,string,uint256)",
  ], signer);

  const tx = await gateway.sendToken(
    DINA_CHAIN_NAME,         // destination chain
    dinaRecipient,           // destination address
    "USDC",                  // token symbol
    usdcAmount               // amount
  );

  const receipt = await tx.wait();
  console.log("Axelar send tx:", receipt.hash);
  return receipt;
}

// --- Step 3: Monitor transfer ---
async function monitorTransfer(txHash: string) {
  const api = new AxelarGMPRecoveryAPI({
    environment: Environment.MAINNET,
  });

  // Poll for status
  let status;
  do {
    status = await api.queryTransactionStatus(txHash);
    console.log("Transfer status:", status.status);

    if (status.status !== "destination_executed") {
      await new Promise(r => setTimeout(r, 30_000));
    }
  } while (status.status !== "destination_executed");

  console.log("Transfer complete!");
}`}
        </CodeBlock>
      </div>

      {/* GMP Example */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">General Message Passing (GMP)</h2>
        <p className="mt-3 text-sm text-slate-300">
          GMP lets you send arbitrary data along with tokens. This example
          bridges USDC and immediately deposits it into a staking contract on
          Dina.
        </p>

        <CodeBlock title="axelar-gmp.ts">
          {`import { ethers } from 'ethers';

// Send USDC + call a function on Dina in one transaction
async function bridgeAndStake(
  signer: ethers.Signer,
  dinaStakingContract: string,
  usdcAmount: bigint
) {
  const gateway = new ethers.Contract(AXELAR_GATEWAY_BASE, [
    "function callContractWithToken(string,string,bytes,string,uint256)",
  ], signer);

  // Encode the function call payload
  const payload = ethers.AbiCoder.defaultAbiCoder().encode(
    ["string", "uint256"],
    ["stake", usdcAmount]
  );

  const tx = await gateway.callContractWithToken(
    DINA_CHAIN_NAME,             // destination chain
    dinaStakingContract,         // destination contract
    payload,                     // arbitrary data
    "USDC",                      // token
    usdcAmount                   // amount
  );

  const receipt = await tx.wait();
  console.log("GMP tx:", receipt.hash);
  // On Dina, the staking contract receives USDC + the payload
  // and executes the stake logic automatically
}`}
        </CodeBlock>
      </div>

      {/* Send from Dina */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Bridge from Dina via Axelar</h2>

        <CodeBlock title="axelar-send-from-dina.ts">
          {`import { DinaClient, DinaWallet } from 'dina-js';

const client = new DinaClient("https://testnet.dina.network");
const wallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

// Send USDC from Dina to Ethereum via Axelar Gateway
const tx = await client.callContract(wallet, {
  contract: AXELAR_GATEWAY_DINA,
  method: "send_token",
  args: {
    destination_chain:   "ethereum",
    destination_address: "0xYourEthAddress...",
    symbol:              "USDC",
    amount:              100_000000,  // 100 USDC
  },
});

console.log("Axelar transfer initiated:", tx.hash);
// Axelar validators confirm and relay (~20 min)
// USDC is released on Ethereum`}
        </CodeBlock>
      </div>

      {/* Supported Chains */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Supported Chains</h2>
        <p className="mt-3 text-sm text-slate-300">
          Axelar connects the most chains of any bridge protocol, spanning
          EVM, Cosmos, and other ecosystems.
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
                  Ethereum, Base, Arbitrum, Optimism, Polygon, Avalanche, BNB Chain, Fantom, Celo, Moonbeam, Linea, Scroll, Mantle, Blast, Filecoin
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Cosmos</td>
                <td className="px-4 py-3">
                  Osmosis, Cosmos Hub, Juno, Kujira, Sei, Neutron, Celestia, dYdX, Injective, Stargaze, Axelar, Stride, Persistence, Secret Network
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3 font-medium text-white">Other</td>
                <td className="px-4 py-3">
                  Sui, Near, Polkadot (Moonbeam), Hedera, Flow
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Gateway and Gas Service */}
      <div className="mt-10">
        <h2 className="text-2xl font-semibold text-white">Contract Addresses</h2>
        <div className="mt-4 overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-slate-700">
                <th className="px-4 py-3 font-semibold text-white">Contract</th>
                <th className="px-4 py-3 font-semibold text-white">Chain</th>
                <th className="px-4 py-3 font-semibold text-white">Address</th>
              </tr>
            </thead>
            <tbody className="text-slate-300">
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Gateway</td>
                <td className="px-4 py-3">Dina</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-blue-400">dina1axelar_gateway...</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Gas Service</td>
                <td className="px-4 py-3">Dina</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-blue-400">dina1axelar_gas_service...</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">ITS</td>
                <td className="px-4 py-3">Dina</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-blue-400">dina1axelar_its...</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Gateway</td>
                <td className="px-4 py-3">Ethereum</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">0x4F4495243837681061C4743b74B3eEdf548D56A5</code>
                </td>
              </tr>
              <tr className="border-b border-slate-800">
                <td className="px-4 py-3">Gateway</td>
                <td className="px-4 py-3">Base</td>
                <td className="px-4 py-3">
                  <code className="text-xs text-slate-400">0xe432150cce91c13a887f7D836923d5597adD8E31</code>
                </td>
              </tr>
            </tbody>
          </table>
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
            -- the recommended bridge for USDC (burn-and-mint).
          </li>
          <li>
            <Link href="/docs/bridges/usdc" className="text-blue-400 hover:underline">
              Bridged USDC Standard
            </Link>{" "}
            -- how Axelar-bridged USDC maps to the unified token on Dina.
          </li>
          <li>
            <Link href="/docs/contracts/call" className="text-blue-400 hover:underline">
              Call Contracts
            </Link>{" "}
            -- interact with the Axelar Gateway from your Dina contracts.
          </li>
        </ul>
      </div>
    </div>
  );
}
