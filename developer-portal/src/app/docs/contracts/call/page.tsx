import Link from "next/link";

export const metadata = {
  title: "Call Contract — Dina Network Developer Portal",
  description:
    "How to interact with deployed WASM smart contracts on Dina Network.",
};

export default function CallContractPage() {
  return (
    <div>
      {/* Header */}
      <p className="text-sm font-medium uppercase tracking-wider text-blue-400 mb-3">
        Smart Contracts
      </p>
      <h1 className="text-4xl font-bold tracking-tight text-white mb-4">
        Call Contract
      </h1>
      <p className="text-lg text-slate-400 max-w-3xl leading-relaxed mb-10">
        Interact with deployed smart contracts by calling their methods. Dina
        supports two types of calls: <strong>view calls</strong> (free,
        read-only) and <strong>mutation calls</strong> (gas-consuming,
        state-changing). Both use JSON-based ABI encoding.
      </p>

      {/* Two Call Types */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          View Calls vs Mutation Calls
        </h2>
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="rounded-xl border border-green-500/30 bg-green-500/5 p-6">
            <h3 className="text-lg font-semibold text-green-400 mb-2">
              View Call (Read-Only)
            </h3>
            <ul className="space-y-2">
              {[
                "Free -- no gas cost",
                "No wallet/signature required",
                "Does not modify state",
                "Executes locally on the queried node",
                "Returns immediately",
                "Methods using &self in Rust",
              ].map((item) => (
                <li key={item} className="flex items-start gap-2 text-sm text-slate-300">
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-green-500" />
                  {item}
                </li>
              ))}
            </ul>
          </div>
          <div className="rounded-xl border border-blue-500/30 bg-blue-500/5 p-6">
            <h3 className="text-lg font-semibold text-blue-400 mb-2">
              Mutation Call (State-Changing)
            </h3>
            <ul className="space-y-2">
              {[
                "Costs gas (30,000 base)",
                "Requires wallet signature",
                "Modifies on-chain state",
                "Processed through consensus",
                "Included in a block (100ms)",
                "Methods using &mut self in Rust",
              ].map((item) => (
                <li key={item} className="flex items-start gap-2 text-sm text-slate-300">
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-blue-500" />
                  {item}
                </li>
              ))}
            </ul>
          </div>
        </div>
      </section>

      {/* ABI Encoding */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          ABI Encoding
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Dina contracts use JSON-based ABI encoding. When you call a contract
          method, you specify the method name as a string and the arguments as
          a JSON object. The runtime deserializes these into the corresponding
          Rust structs defined in the contract.
        </p>
        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6 mb-4">
          <h3 className="text-sm font-semibold text-white mb-3">
            Contract defines:
          </h3>
          <pre className="overflow-x-auto rounded-lg bg-slate-800 p-4 text-sm leading-relaxed">
            <code className="text-slate-200">{`#[derive(Serialize, Deserialize)]
struct SetGreetingArgs {
    new_greeting: String,
}

// In dispatch:
"set_greeting" => {
    let a: SetGreetingArgs = serde_json::from_slice(args).expect("bad args");
    s.set_greeting(caller, a.new_greeting);
    serde_json::to_vec("ok").unwrap()
}`}</code>
          </pre>
        </div>
        <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-6">
          <h3 className="text-sm font-semibold text-white mb-3">
            Caller sends:
          </h3>
          <pre className="overflow-x-auto rounded-lg bg-slate-800 p-4 text-sm leading-relaxed">
            <code className="text-slate-200">{`{
  "method": "set_greeting",
  "args": {
    "new_greeting": "Hello from the Machine Economy!"
  }
}`}</code>
          </pre>
        </div>
      </section>

      {/* View Call Examples */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          View Call Examples
        </h2>

        <h3 className="text-lg font-semibold text-white mb-3">JavaScript / TypeScript</h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-6">
          <code className="text-slate-200">{`import { DinaClient, DinaContract } from '@dina-network/sdk';

const client = new DinaClient('https://rpc-testnet.dina.network');
const contract = new DinaContract('dina1abc123...xyz', client);

// Simple view call -- no wallet needed
const greeting = await contract.view('get_greeting', {});
console.log('Greeting:', greeting);
// => "Hello from the Machine Economy!"

// View call with arguments
const balance = await contract.view('balance_of', {
  owner: '0x1a2b3c...addr',
});
console.log('Balance:', balance);
// => 50000000 (50 USDC)

// Using typed DRC helpers
const token = DinaContract.token('dina1token...xyz', client);
const totalSupply = await token.totalSupply();
const name = await token.name();
console.log(\`\${name}: \${totalSupply} total supply\`);`}</code>
        </pre>

        <h3 className="text-lg font-semibold text-white mb-3">Python</h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-6">
          <code className="text-slate-200">{`from dina import DinaClient

client = DinaClient("https://rpc-testnet.dina.network")

# View call -- free, no wallet needed
greeting = client.view_contract(
    contract="dina1abc123...xyz",
    method="get_greeting",
    args={},
)
print(f"Greeting: {greeting}")

# View with arguments
balance = client.view_contract(
    contract="dina1token...xyz",
    method="balance_of",
    args={"owner": "0x1a2b3c...addr"},
)
print(f"Balance: {balance / 1_000_000} USDC")`}</code>
        </pre>

        <h3 className="text-lg font-semibold text-white mb-3">CLI</h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`# View call (free)
dina contract view \\
  --address dina1abc123...xyz \\
  --method get_greeting \\
  --args '{}' \\
  --network testnet

# Output: "Hello from the Machine Economy!"`}</code>
        </pre>
      </section>

      {/* Mutation Call Examples */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Mutation Call Examples
        </h2>

        <h3 className="text-lg font-semibold text-white mb-3">JavaScript / TypeScript</h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-6">
          <code className="text-slate-200">{`import { DinaClient, DinaContract, DinaWallet } from '@dina-network/sdk';

const client = new DinaClient('https://rpc-testnet.dina.network');
const wallet = DinaWallet.fromKeyFile('./my-wallet.json');
const contract = new DinaContract('dina1abc123...xyz', client);

// State-changing call -- requires wallet for signing
const txHash = await contract.call(
  'set_greeting',
  { new_greeting: 'Updated greeting!' },
  wallet,
);

console.log('TX hash:', txHash);

// Get the receipt
const receipt = await client.getTransactionReceipt(txHash);
console.log('Status:', receipt.status);     // "success"
console.log('Gas used:', receipt.gasUsed);  // 30000
console.log('Events:', receipt.events);     // [ ... ]`}</code>
        </pre>

        <h3 className="text-lg font-semibold text-white mb-3">Python</h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-6">
          <code className="text-slate-200">{`from dina import DinaClient, DinaWallet

client = DinaClient("https://rpc-testnet.dina.network")
wallet = DinaWallet.from_key_file("./my-wallet.json")

# State-changing call
tx = client.call_contract(
    wallet=wallet,
    contract="dina1abc123...xyz",
    method="set_greeting",
    args={"new_greeting": "Updated greeting!"},
)

print(f"TX hash: {tx.hash}")
print(f"Status: {tx.status}")
print(f"Gas used: {tx.gas_used}")`}</code>
        </pre>

        <h3 className="text-lg font-semibold text-white mb-3">CLI</h3>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`# State-changing call (requires signing)
dina contract call \\
  --address dina1abc123...xyz \\
  --method set_greeting \\
  --args '{"new_greeting": "Updated greeting!"}' \\
  --network testnet`}</code>
        </pre>
      </section>

      {/* Attaching USDC */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Attaching USDC to Calls
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Some contract methods accept USDC payments -- for example, funding
          an escrow, purchasing an NFT, or staking tokens. You attach USDC
          by including the amount in both the args and the call metadata.
        </p>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-6">
          <code className="text-slate-200">{`// JavaScript: attach 50 USDC to a contract call
const tx = await contract.call(
  'fund_deal',
  {
    deal_id: 42,
    usdc_attached: 50_000_000,  // included in args for the contract to read
  },
  wallet,
  BigInt(50_000_000),           // actual USDC transfer amount
);

console.log('Funded deal 42 with 50 USDC');`}</code>
        </pre>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed mb-6">
          <code className="text-slate-200">{`# Python: attach USDC
tx = client.call_contract(
    wallet=wallet,
    contract="dina1escrow...xyz",
    method="fund_deal",
    args={"deal_id": 42, "usdc_attached": 50_000_000},
    usdc_attached=50_000_000,
)`}</code>
        </pre>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`# CLI: attach USDC with --usdc flag
dina contract call \\
  --address dina1escrow...xyz \\
  --method fund_deal \\
  --args '{"deal_id": 42, "usdc_attached": 50000000}' \\
  --usdc 50.0 \\
  --network testnet`}</code>
        </pre>
      </section>

      {/* Reading Events */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Reading Return Values and Events
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          Contract methods return JSON-serialized data. For mutations, you
          access the return value through the transaction receipt. Events are
          emitted as part of the receipt and provide structured data about
          what happened during execution.
        </p>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`import { DinaClient, DinaContract, DinaWallet } from '@dina-network/sdk';

const client = new DinaClient('https://rpc-testnet.dina.network');
const wallet = DinaWallet.fromKeyFile('./my-wallet.json');
const contract = new DinaContract('dina1market...xyz', client);

// Call a method that returns data
const txHash = await contract.call(
  'create_listing',
  {
    service_type: 'compute',
    price: 5_000_000,  // 5 USDC
    description: 'GPU compute, 1 hour',
  },
  wallet,
);

// Get the receipt with return value and events
const receipt = await client.getTransactionReceipt(txHash);

// Parse the return value (JSON bytes -> object)
const listingId = receipt.returnValue; // e.g., 7
console.log('Created listing ID:', listingId);

// Read events emitted by the contract
for (const event of receipt.events) {
  console.log('Event:', event.name);   // "ListingCreated"
  console.log('Data:', event.data);    // { listing_id: 7, provider: "0x...", price: 5000000 }
}

// Subscribe to future events via WebSocket
const ws = client.subscribeEvents('dina1market...xyz');
ws.on('ListingCreated', (event) => {
  console.log('New listing:', event.data.listing_id);
});
ws.on('ListingPurchased', (event) => {
  console.log('Purchased:', event.data.listing_id, 'by', event.data.buyer);
});`}</code>
        </pre>
      </section>

      {/* JSON-RPC Raw Call */}
      <section className="mb-12">
        <h2 className="text-2xl font-semibold text-white mb-4">
          Raw JSON-RPC Contract Call
        </h2>
        <p className="text-sm text-slate-400 leading-relaxed mb-4">
          If you are not using an SDK, you can call contracts directly via
          the JSON-RPC API:
        </p>
        <pre className="overflow-x-auto rounded-xl bg-slate-800 p-5 text-sm leading-relaxed">
          <code className="text-slate-200">{`// View call (free, no signing)
const viewResponse = await fetch('https://rpc-testnet.dina.network', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    jsonrpc: '2.0',
    method: 'dina_callContract',
    params: [{
      contract: 'dina1abc123...xyz',
      method: 'get_greeting',
      args: {},
      readonly: true,
    }],
    id: 1,
  }),
});

const viewResult = await viewResponse.json();
console.log('Greeting:', viewResult.result);
// => "Hello from the Machine Economy!"

// Mutation call (requires signed transaction)
const mutationResponse = await fetch('https://rpc-testnet.dina.network', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    jsonrpc: '2.0',
    method: 'dina_sendTransaction',
    params: [{
      from: '0x1a2b3c...sender',
      to: 'dina1abc123...xyz',   // contract address
      amount: 0,
      fee: 1429,                 // 30,000 gas worth
      nonce: 43,
      data: {
        method: 'set_greeting',
        args: { new_greeting: 'Raw RPC call!' },
      },
      signature: '0x...',       // Ed25519 signature
    }],
    id: 2,
  }),
});`}</code>
        </pre>
      </section>

      {/* Next Steps */}
      <div className="mt-10 flex flex-wrap gap-4">
        <Link
          href="/docs/contracts/deploy"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          &larr; Deploy Contract
        </Link>
        <Link
          href="/docs/contracts/standards"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          DRC Standards &rarr;
        </Link>
        <Link
          href="/docs/contracts/wasm"
          className="rounded-lg border border-slate-800 bg-slate-900/30 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:border-blue-500/40 hover:text-white"
        >
          WASM Runtime &rarr;
        </Link>
      </div>
    </div>
  );
}
