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

function Step({
  number,
  title,
  children,
}: {
  number: number;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="relative pl-12">
      <div className="absolute left-0 top-0 flex h-8 w-8 items-center justify-center rounded-full bg-blue-600 text-sm font-bold text-white">
        {number}
      </div>
      <h3 className="text-lg font-semibold text-white">{title}</h3>
      <div className="mt-2 text-sm leading-relaxed text-slate-300">
        {children}
      </div>
    </div>
  );
}

export default function QuickstartPage() {
  return (
    <div>
      <h1 className="text-4xl font-bold tracking-tight text-white">
        Quickstart
      </h1>
      <p className="mt-4 text-lg text-slate-300">
        Go from zero to your first on-chain transaction in five steps. This
        guide uses the{" "}
        <code className="rounded bg-slate-800 px-1.5 py-0.5 text-sm text-blue-400">
          dina-js
        </code>{" "}
        SDK but the same concepts apply to Python and Rust.
      </p>

      <div className="mt-10 space-y-10">
        {/* Step 1 */}
        <Step number={1} title="Install the SDK">
          <p>
            Add the Dina JavaScript SDK to your project using npm, yarn, or
            pnpm.
          </p>
          <CodeBlock title="Terminal">
            {`npm install dina-js`}
          </CodeBlock>
          <p className="mt-3 text-slate-400">
            The package includes TypeScript type definitions out of the box.
          </p>
        </Step>

        {/* Step 2 */}
        <Step number={2} title="Create a wallet">
          <p>
            Generate a new Ed25519 keypair. The wallet gives you an address on
            the Dina testnet.
          </p>
          <CodeBlock title="create-wallet.ts">
            {`import { DinaWallet } from "dina-js";

// Generate a brand-new keypair
const wallet = DinaWallet.generate();

console.log("Address:", wallet.address);
console.log("Public key:", wallet.publicKeyHex);

// IMPORTANT: back up the secret key securely
console.log("Secret key:", wallet.secretKeyHex);`}
          </CodeBlock>
          <p className="mt-3 text-slate-400">
            You can also restore a wallet from an existing secret key with{" "}
            <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">
              DinaWallet.fromSecretKey(hex)
            </code>
            .
          </p>
        </Step>

        {/* Step 3 */}
        <Step number={3} title="Get testnet USDC">
          <p>
            Request testnet USDC from the faucet. Each request deposits 100
            USDC into your wallet.
          </p>
          <CodeBlock title="Terminal">
            {`curl -X POST https://testnet.dina.network/faucet/YOUR_ADDRESS`}
          </CodeBlock>
          <p className="mt-3">
            Or use the{" "}
            <Link
              href="/faucet"
              className="text-blue-400 underline decoration-blue-400/30 hover:decoration-blue-400"
            >
              web faucet
            </Link>{" "}
            to paste your address and receive funds instantly.
          </p>
        </Step>

        {/* Step 4 */}
        <Step number={4} title="Send your first transaction">
          <p>
            Create a USDC transfer, sign it with your wallet, and broadcast it
            to the network.
          </p>
          <CodeBlock title="send-tx.ts">
            {`import { DinaClient, DinaWallet } from "dina-js";

const client = new DinaClient("https://testnet.dina.network");
const wallet = DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY!);

const tx = await client.transfer({
  from: wallet.address,
  to: "dina1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
  amount: 10_000000, // 10.000000 USDC (6 decimals)
});

// Sign and broadcast
const signed = wallet.sign(tx);
const receipt = await client.send(signed);

console.log("Transaction hash:", receipt.hash);
console.log("Block:", receipt.blockNumber);
console.log("Status:", receipt.status); // "confirmed"`}
          </CodeBlock>
          <p className="mt-3 text-slate-400">
            Transactions finalize in ~100ms. The receipt is available
            immediately after the call resolves.
          </p>
        </Step>

        {/* Step 5 */}
        <Step number={5} title="Check the explorer">
          <p>
            View your transaction on the Dina block explorer. Paste your
            transaction hash or address to see balances, transfers, and
            contract interactions.
          </p>
          <CodeBlock title="Terminal">
            {`# Open the explorer in your browser
open https://explorer.dina.network/tx/YOUR_TX_HASH`}
          </CodeBlock>
          <p className="mt-3">
            Or use the{" "}
            <Link
              href="/explorer"
              className="text-blue-400 underline decoration-blue-400/30 hover:decoration-blue-400"
            >
              built-in explorer
            </Link>{" "}
            on this portal.
          </p>
        </Step>
      </div>

      {/* Full working example */}
      <div className="mt-14">
        <h2 className="text-2xl font-semibold text-white">
          Full working example
        </h2>
        <p className="mt-3 text-sm text-slate-300">
          Copy this script, set your secret key, and run it with{" "}
          <code className="rounded bg-slate-800 px-1.5 py-0.5 text-blue-400">
            npx tsx quickstart.ts
          </code>{" "}
          to see everything end to end.
        </p>
        <CodeBlock title="quickstart.ts">
          {`import { DinaClient, DinaWallet } from "dina-js";

async function main() {
  // 1. Connect to testnet
  const client = new DinaClient("https://testnet.dina.network");
  const info = await client.networkInfo();
  console.log("Chain:", info.chainId, "| Block:", info.blockHeight);

  // 2. Create or restore wallet
  const wallet = process.env.DINA_SECRET_KEY
    ? DinaWallet.fromSecretKey(process.env.DINA_SECRET_KEY)
    : DinaWallet.generate();
  console.log("Wallet:", wallet.address);

  // 3. Check balance
  const balance = await client.getBalance(wallet.address);
  console.log("Balance:", balance.formatted); // e.g. "100.000000 USDC"

  // 4. Send a transfer
  if (balance.raw >= 1_000000) {
    const tx = await client.transfer({
      from: wallet.address,
      to: "dina1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
      amount: 1_000000, // 1 USDC
    });

    const signed = wallet.sign(tx);
    const receipt = await client.send(signed);
    console.log("Sent! Hash:", receipt.hash);
    console.log("Confirmed in block:", receipt.blockNumber);
  } else {
    console.log("Not enough balance. Request testnet USDC from the faucet:");
    console.log(\`  curl -X POST https://testnet.dina.network/faucet/\${wallet.address}\`);
  }
}

main().catch(console.error);`}
        </CodeBlock>
      </div>

      {/* Next steps */}
      <div className="mt-12 rounded-xl border border-slate-800 bg-slate-900/40 p-6">
        <h3 className="text-base font-semibold text-white">Next steps</h3>
        <ul className="mt-3 space-y-2 text-sm text-slate-300">
          <li>
            <Link href="/docs/architecture" className="text-blue-400 hover:underline">
              Architecture overview
            </Link>{" "}
            -- learn how parallel execution and TurboBFT consensus work under
            the hood.
          </li>
          <li>
            <Link href="/docs/wallets" className="text-blue-400 hover:underline">
              Wallets deep dive
            </Link>{" "}
            -- explore agent wallets, swarm wallets, and HD key derivation.
          </li>
          <li>
            <Link href="/docs/contracts/deploy" className="text-blue-400 hover:underline">
              Deploy a smart contract
            </Link>{" "}
            -- compile and deploy WASM contracts using the DRC standard
            library.
          </li>
        </ul>
      </div>
    </div>
  );
}
