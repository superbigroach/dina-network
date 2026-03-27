"use client";

import { CodeBlock, LanguageTabs } from "@/components/code-block";

/* ------------------------------------------------------------------ */
/*  Section heading helper                                             */
/* ------------------------------------------------------------------ */
function H2({ id, children }: { id: string; children: React.ReactNode }) {
  return (
    <h2
      id={id}
      className="mb-4 mt-12 scroll-mt-24 border-b border-slate-800/60 pb-2 text-2xl font-bold tracking-tight"
    >
      {children}
    </h2>
  );
}

function H3({ id, children }: { id: string; children: React.ReactNode }) {
  return (
    <h3
      id={id}
      className="mb-3 mt-8 scroll-mt-24 text-xl font-semibold tracking-tight"
    >
      {children}
    </h3>
  );
}

function Badge({ children }: { children: React.ReactNode }) {
  return (
    <span className="ml-2 rounded-full bg-blue-600/15 px-2 py-0.5 text-xs font-medium text-blue-400">
      {children}
    </span>
  );
}

function MethodSignature({ sig, returns, description }: { sig: string; returns: string; description: string }) {
  return (
    <div className="my-3 rounded-lg border border-slate-800/60 bg-slate-900/40 p-4">
      <code className="text-sm text-blue-400">{sig}</code>
      <span className="ml-2 text-sm text-slate-500">{"=>"} {returns}</span>
      <p className="mt-1.5 text-sm text-slate-400">{description}</p>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Page                                                               */
/* ------------------------------------------------------------------ */
export default function JavaScriptSdkPage() {
  return (
    <>
      {/* Header */}
      <div className="mb-2 flex items-center gap-2 text-sm text-slate-500">
        SDKs
        <span className="text-slate-700">/</span>
        JavaScript &amp; TypeScript
      </div>
      <h1 className="text-4xl font-extrabold tracking-tight">
        JavaScript / TypeScript SDK
        <Badge>dina-js</Badge>
      </h1>
      <p className="mt-4 text-lg leading-relaxed text-slate-400">
        The official JavaScript and TypeScript SDK for the Dina Network. Works in
        Node.js, Deno, Bun, and modern browsers.
      </p>

      {/* ---- Installation ---- */}
      <H2 id="installation">Installation</H2>

      <LanguageTabs
        tabs={[
          { label: "npm", language: "bash", code: "npm install dina-js" },
          { label: "yarn", language: "bash", code: "yarn add dina-js" },
          { label: "pnpm", language: "bash", code: "pnpm add dina-js" },
          { label: "bun", language: "bash", code: "bun add dina-js" },
        ]}
      />

      {/* ---- Quick Start ---- */}
      <H2 id="quick-start">Quick Start</H2>

      <CodeBlock
        language="typescript"
        filename="quickstart.ts"
        code={`import { DinaWallet, DinaClient } from 'dina-js';

// Generate a new wallet
const wallet = DinaWallet.generate();
console.log('Address:', wallet.address);

// Connect to testnet
const client = new DinaClient('https://rpc.dina.network');

// Check balance (returns USDC string, e.g. "100.50")
const balance = await client.getBalance(wallet.address);
console.log('Balance:', balance, 'USDC');

// Send a transaction
const tx = await client.sendTransaction({
  from:   wallet,
  to:     '0x742d35Cc6634C0532925a3b844Bc9e7595f2bD38',
  amount: '10.00',  // 10 USDC
});

console.log('TX Hash:', tx.hash);
const receipt = await client.waitForTransaction(tx.hash);
console.log('Status:', receipt.status); // "confirmed"`}
      />

      {/* ---- API Reference ---- */}
      <H2 id="api-reference">API Reference</H2>

      {/* DinaWallet */}
      <H3 id="dina-wallet">DinaWallet</H3>
      <p className="mb-4 text-sm text-slate-400">
        Key management and transaction signing. Private keys never leave the
        client.
      </p>

      <MethodSignature
        sig="DinaWallet.generate()"
        returns="DinaWallet"
        description="Generate a new random wallet with a fresh private key."
      />
      <MethodSignature
        sig="DinaWallet.fromPrivateKey(key: string)"
        returns="DinaWallet"
        description="Restore a wallet from a hex-encoded private key."
      />
      <MethodSignature
        sig="DinaWallet.fromMnemonic(mnemonic: string, index?: number)"
        returns="DinaWallet"
        description="Derive a wallet from a BIP-39 mnemonic phrase. Optional index for HD derivation (default 0)."
      />
      <MethodSignature
        sig="wallet.sign(message: Uint8Array)"
        returns="Uint8Array"
        description="Sign arbitrary bytes with the wallet's private key (Ed25519)."
      />
      <MethodSignature
        sig="wallet.verify(message: Uint8Array, signature: Uint8Array)"
        returns="boolean"
        description="Verify a signature against the wallet's public key."
      />
      <MethodSignature
        sig="wallet.address"
        returns="string"
        description="The wallet's Dina address (read-only property)."
      />
      <MethodSignature
        sig="wallet.publicKey"
        returns="string"
        description="Hex-encoded public key (read-only property)."
      />

      <CodeBlock
        language="typescript"
        filename="wallet-example.ts"
        code={`import { DinaWallet } from 'dina-js';

// Generate a new wallet
const wallet = DinaWallet.generate();
console.log('Address:',    wallet.address);
console.log('Public Key:', wallet.publicKey);

// Sign and verify a message
const message = new TextEncoder().encode('Hello Dina');
const signature = wallet.sign(message);
const valid = wallet.verify(message, signature);
console.log('Signature valid:', valid); // true

// Restore from mnemonic
const restored = DinaWallet.fromMnemonic(
  'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about'
);`}
      />

      {/* DinaClient */}
      <H3 id="dina-client">DinaClient</H3>
      <p className="mb-4 text-sm text-slate-400">
        RPC client for querying the blockchain and submitting transactions.
      </p>

      <MethodSignature
        sig="new DinaClient(rpcUrl: string)"
        returns="DinaClient"
        description="Create a client connected to a Dina Network RPC endpoint."
      />
      <MethodSignature
        sig="client.getBalance(address: string)"
        returns="Promise&lt;string&gt;"
        description="Get the USDC balance for an address. Returns a human-readable string (e.g. '100.50')."
      />
      <MethodSignature
        sig="client.getAccount(address: string)"
        returns="Promise&lt;Account&gt;"
        description="Get full account details: balance, nonce, contract code hash, and storage root."
      />
      <MethodSignature
        sig="client.getBlock(number: number | 'latest')"
        returns="Promise&lt;Block&gt;"
        description="Fetch a block by number or 'latest'. Includes transaction hashes."
      />
      <MethodSignature
        sig="client.sendTransaction(params: TxParams)"
        returns="Promise&lt;TxResult&gt;"
        description="Sign and broadcast a transaction. Params include from (wallet), to, amount, and optional data."
      />
      <MethodSignature
        sig="client.estimateGas(params: TxParams)"
        returns="Promise&lt;string&gt;"
        description="Estimate gas cost for a transaction in USDC micro-units."
      />
      <MethodSignature
        sig="client.getTransactionReceipt(hash: string)"
        returns="Promise&lt;Receipt | null&gt;"
        description="Get the receipt for a confirmed transaction. Returns null if not yet confirmed."
      />
      <MethodSignature
        sig="client.waitForTransaction(hash: string, timeout?: number)"
        returns="Promise&lt;Receipt&gt;"
        description="Poll until a transaction is confirmed. Default timeout is 30 seconds."
      />

      <CodeBlock
        language="typescript"
        filename="client-example.ts"
        code={`import { DinaClient } from 'dina-js';

const client = new DinaClient('https://rpc.dina.network');

// Query the latest block
const block = await client.getBlock('latest');
console.log('Block #:', block.number);
console.log('Tx count:', block.transactions.length);

// Get account details
const account = await client.getAccount('dina1abc...');
console.log('Balance:', account.balance);
console.log('Nonce:',   account.nonce);

// Estimate gas before sending
const gas = await client.estimateGas({
  from: wallet,
  to:   'dina1xyz...',
  amount: '50.00',
});
console.log('Estimated fee:', gas, 'USDC');`}
      />

      {/* DinaContract */}
      <H3 id="dina-contract">DinaContract</H3>
      <p className="mb-4 text-sm text-slate-400">
        Interact with deployed smart contracts or deploy new ones.
      </p>

      <MethodSignature
        sig="new DinaContract(client: DinaClient, address: string, abi: ABI)"
        returns="DinaContract"
        description="Create a contract instance bound to a deployed address."
      />
      <MethodSignature
        sig="contract.call(method: string, ...args: any[])"
        returns="Promise&lt;any&gt;"
        description="Call a read-only contract method (no gas cost)."
      />
      <MethodSignature
        sig="DinaContract.deploy(client: DinaClient, wallet: DinaWallet, bytecode: string, abi: ABI, ...args: any[])"
        returns="Promise&lt;DinaContract&gt;"
        description="Deploy a new contract. Returns the contract instance once confirmed."
      />

      <CodeBlock
        language="typescript"
        filename="contract-example.ts"
        code={`import { DinaClient, DinaContract, DinaWallet } from 'dina-js';

const client = new DinaClient('https://rpc.dina.network');
const wallet = DinaWallet.fromPrivateKey(process.env.PRIVATE_KEY!);

// Interact with an existing contract
const contract = new DinaContract(client, '0xContractAddr...', MY_ABI);
const result = await contract.call('getScore', wallet.address);
console.log('Score:', result);

// Deploy a new contract
const deployed = await DinaContract.deploy(
  client,
  wallet,
  contractBytecode,
  contractAbi,
  'Constructor Arg 1',
  42
);
console.log('Deployed at:', deployed.address);`}
      />

      {/* TokenContract */}
      <H3 id="token-contract">TokenContract</H3>
      <p className="mb-4 text-sm text-slate-400">
        Pre-built helper for DRC-20 token interactions (USDC and custom tokens).
      </p>

      <MethodSignature
        sig="token.balanceOf(address: string)"
        returns="Promise&lt;string&gt;"
        description="Get the token balance for an address."
      />
      <MethodSignature
        sig="token.transfer(wallet: DinaWallet, to: string, amount: string)"
        returns="Promise&lt;TxResult&gt;"
        description="Transfer tokens from the wallet to a recipient."
      />
      <MethodSignature
        sig="token.approve(wallet: DinaWallet, spender: string, amount: string)"
        returns="Promise&lt;TxResult&gt;"
        description="Approve a spender to transfer tokens on behalf of the wallet."
      />
      <MethodSignature
        sig="token.allowance(owner: string, spender: string)"
        returns="Promise&lt;string&gt;"
        description="Check the remaining allowance for a spender."
      />

      {/* AgentWalletContract */}
      <H3 id="agent-wallet-contract">AgentWalletContract</H3>
      <p className="mb-4 text-sm text-slate-400">
        DRC-101 Agent Wallet management. Create and manage autonomous AI agent wallets
        with spending limits.
      </p>

      <MethodSignature
        sig="agentWallet.createAgent(wallet: DinaWallet, agentId: string, limits: AgentLimits)"
        returns="Promise&lt;TxResult&gt;"
        description="Create a new agent sub-wallet with the specified spending limits."
      />
      <MethodSignature
        sig="agentWallet.setLimits(wallet: DinaWallet, agentId: string, limits: AgentLimits)"
        returns="Promise&lt;TxResult&gt;"
        description="Update spending limits (per-tx max, daily max, approved recipients)."
      />
      <MethodSignature
        sig="agentWallet.revokeAgent(wallet: DinaWallet, agentId: string)"
        returns="Promise&lt;TxResult&gt;"
        description="Revoke an agent immediately, freezing all remaining funds."
      />

      {/* PaymentChannel */}
      <H3 id="payment-channel">PaymentChannel</H3>
      <p className="mb-4 text-sm text-slate-400">
        Off-chain payment channels for instant micro-transactions (5ms latency).
      </p>

      <MethodSignature
        sig="channel.open(wallet: DinaWallet, counterparty: string, deposit: string)"
        returns="Promise&lt;ChannelId&gt;"
        description="Open a payment channel with an initial USDC deposit."
      />
      <MethodSignature
        sig="channel.pay(amount: string)"
        returns="SignedState"
        description="Create an off-chain payment (instant, no gas). Returns signed state to share with counterparty."
      />
      <MethodSignature
        sig="channel.close()"
        returns="Promise&lt;TxResult&gt;"
        description="Cooperatively close the channel and settle balances on-chain."
      />
      <MethodSignature
        sig="channel.settle()"
        returns="Promise&lt;TxResult&gt;"
        description="Force-settle after the dispute period if counterparty is unresponsive."
      />

      {/* Utilities */}
      <H3 id="utilities">Utility Functions</H3>

      <MethodSignature
        sig="formatUSDC(microUnits: bigint)"
        returns="string"
        description="Convert micro-USDC (6 decimals) to a human-readable string. e.g. 1000000n => '1.00'."
      />
      <MethodSignature
        sig="parseUSDC(amount: string)"
        returns="bigint"
        description="Convert a human-readable USDC string to micro-units. e.g. '1.50' => 1500000n."
      />
      <MethodSignature
        sig="addressFromPublicKey(pubkey: string)"
        returns="string"
        description="Derive a Dina address from a hex-encoded public key."
      />
      <MethodSignature
        sig="isValidAddress(address: string)"
        returns="boolean"
        description="Check if a string is a valid Dina network address."
      />

      <CodeBlock
        language="typescript"
        filename="utilities-example.ts"
        code={`import { formatUSDC, parseUSDC, isValidAddress } from 'dina-js';

const micro = parseUSDC('250.75');
console.log(micro); // 250750000n

const display = formatUSDC(250750000n);
console.log(display); // "250.75"

console.log(isValidAddress('dina1abc...')); // true
console.log(isValidAddress('invalid'));      // false`}
      />

      {/* ---- Full Examples ---- */}
      <H2 id="examples">Full Examples</H2>

      <H3 id="example-create-wallet">Create a Wallet and Fund It</H3>
      <CodeBlock
        language="typescript"
        filename="create-and-fund.ts"
        code={`import { DinaWallet, DinaClient } from 'dina-js';

const wallet = DinaWallet.generate();
const client = new DinaClient('https://rpc.dina.network');

// Request testnet USDC from faucet
const res = await fetch(\`https://rpc.dina.network/faucet/\${wallet.address}\`, {
  method: 'POST',
});
const { txHash } = await res.json();
await client.waitForTransaction(txHash);

const balance = await client.getBalance(wallet.address);
console.log('Funded! Balance:', balance, 'USDC');`}
      />

      <H3 id="example-send-tx">Send a Transaction</H3>
      <CodeBlock
        language="typescript"
        filename="send-transaction.ts"
        code={`import { DinaWallet, DinaClient } from 'dina-js';

const sender   = DinaWallet.fromPrivateKey(process.env.PRIVATE_KEY!);
const client   = new DinaClient('https://rpc.dina.network');
const recipient = 'dina1recipient...';

// Estimate fee first
const fee = await client.estimateGas({
  from:   sender,
  to:     recipient,
  amount: '100.00',
});
console.log('Fee:', fee, 'USDC');

// Send 100 USDC
const tx = await client.sendTransaction({
  from:   sender,
  to:     recipient,
  amount: '100.00',
});

// Wait for confirmation (100ms finality)
const receipt = await client.waitForTransaction(tx.hash);
console.log('Confirmed in block:', receipt.blockNumber);
console.log('Gas used:', receipt.gasUsed);`}
      />

      <H3 id="example-deploy-contract">Deploy a Smart Contract</H3>
      <CodeBlock
        language="typescript"
        filename="deploy-contract.ts"
        code={`import { DinaWallet, DinaClient, DinaContract } from 'dina-js';
import { readFileSync } from 'fs';

const wallet = DinaWallet.fromPrivateKey(process.env.PRIVATE_KEY!);
const client = new DinaClient('https://rpc.dina.network');

// Load compiled WASM contract
const bytecode = readFileSync('./my_contract.wasm');

const abi = [
  { name: 'initialize', inputs: [{ name: 'name', type: 'string' }], outputs: [] },
  { name: 'getValue',   inputs: [],                                  outputs: [{ type: 'u64' }] },
  { name: 'setValue',   inputs: [{ name: 'val', type: 'u64' }],     outputs: [] },
];

// Deploy
const contract = await DinaContract.deploy(
  client,
  wallet,
  bytecode.toString('hex'),
  abi,
  'My Contract'  // constructor arg
);

console.log('Contract deployed at:', contract.address);

// Call a read method
const value = await contract.call('getValue');
console.log('Current value:', value);`}
      />

      {/* ---- Error Handling ---- */}
      <H2 id="error-handling">Error Handling</H2>
      <p className="mb-4 text-sm text-slate-400">
        The SDK throws typed errors that you can catch and handle individually.
      </p>

      <CodeBlock
        language="typescript"
        filename="error-handling.ts"
        code={`import {
  DinaClient,
  DinaWallet,
  InsufficientFundsError,
  TransactionTimeoutError,
  RpcError,
  InvalidAddressError,
} from 'dina-js';

const client = new DinaClient('https://rpc.dina.network');
const wallet = DinaWallet.fromPrivateKey(process.env.PRIVATE_KEY!);

try {
  const tx = await client.sendTransaction({
    from:   wallet,
    to:     'dina1recipient...',
    amount: '1000000.00',
  });
  await client.waitForTransaction(tx.hash, 10_000); // 10s timeout
} catch (err) {
  if (err instanceof InsufficientFundsError) {
    console.error('Not enough USDC. Have:', err.balance, 'Need:', err.required);
  } else if (err instanceof TransactionTimeoutError) {
    console.error('Tx not confirmed within timeout. Hash:', err.txHash);
  } else if (err instanceof InvalidAddressError) {
    console.error('Bad address format:', err.address);
  } else if (err instanceof RpcError) {
    console.error('RPC error:', err.code, err.message);
  } else {
    throw err;
  }
}`}
      />

      {/* ---- TypeScript Types ---- */}
      <H2 id="types">TypeScript Types</H2>
      <CodeBlock
        language="typescript"
        filename="types.d.ts"
        code={`// Core types exported by dina-js

interface Account {
  address:      string;
  balance:      string;   // USDC as human-readable string
  nonce:        number;
  codeHash?:    string;   // present if contract account
  storageRoot?: string;
}

interface Block {
  number:       number;
  hash:         string;
  parentHash:   string;
  timestamp:    number;
  transactions: string[];  // tx hashes
  validator:    string;
  gasUsed:      string;
}

interface TxParams {
  from:    DinaWallet;
  to:      string;
  amount:  string;
  data?:   string;   // hex-encoded contract call data
  nonce?:  number;   // auto-fetched if omitted
}

interface TxResult {
  hash:  string;
  nonce: number;
}

interface Receipt {
  txHash:      string;
  status:      'confirmed' | 'failed';
  blockNumber: number;
  gasUsed:     string;
  events:      Event[];
}

interface AgentLimits {
  maxPerTransaction: string;  // USDC
  maxDaily:          string;  // USDC
  allowedRecipients: string[];
  expiresAt?:        number;  // unix timestamp
}`}
      />
    </>
  );
}
