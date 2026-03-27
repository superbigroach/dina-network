// Dina Network testnet configuration
export const TESTNET_CONFIG = {
  chainId: "dina-testnet-1",
  validators: [
    { name: "Validator 0", ip: "35.184.213.248", rpcPort: 8545, restPort: 8080 },
    { name: "Validator 1", ip: "35.193.28.189", rpcPort: 8545, restPort: 8080 },
    { name: "Validator 2", ip: "136.115.115.11", rpcPort: 8545, restPort: 8080 },
  ],
  rpcUrl: "http://35.184.213.248:8545",
  restUrl: "http://35.184.213.248:8080",
  blockTimeMs: 100,
  maxTxsPerBlock: 10000,
  currency: "USDC",
  decimals: 6,
  explorerUrl: "https://explorer.dina.network",
};

export const NAV_SECTIONS = [
  {
    title: "Getting Started",
    items: [
      { title: "Overview", href: "/docs" },
      { title: "Quickstart", href: "/docs/quickstart" },
      { title: "Architecture", href: "/docs/architecture" },
      { title: "Authentication", href: "/docs/authentication" },
    ],
  },
  {
    title: "Wallets",
    items: [
      { title: "Overview", href: "/docs/wallets" },
      { title: "Create Wallet", href: "/docs/wallets/create" },
      { title: "Agent Wallets (DRC-101)", href: "/docs/wallets/agent" },
      { title: "Swarm Wallets (DRC-63)", href: "/docs/wallets/swarm" },
      { title: "Key Management", href: "/docs/wallets/keys" },
      { title: "HD Wallets & Mnemonics", href: "/docs/wallets/hd" },
    ],
  },
  {
    title: "Transactions",
    items: [
      { title: "Send USDC", href: "/docs/transactions/transfer" },
      { title: "Batch Transfers (DRC-19)", href: "/docs/transactions/batch" },
      { title: "Gas & Fees", href: "/docs/transactions/fees" },
      { title: "Payment Channels", href: "/docs/transactions/channels" },
      { title: "Transaction Lifecycle", href: "/docs/transactions/lifecycle" },
    ],
  },
  {
    title: "Smart Contracts",
    items: [
      { title: "Deploy Contract", href: "/docs/contracts/deploy" },
      { title: "Call Contract", href: "/docs/contracts/call" },
      { title: "DRC Standards", href: "/docs/contracts/standards" },
      { title: "WASM Runtime", href: "/docs/contracts/wasm" },
    ],
  },
  {
    title: "Parallel Execution",
    items: [
      { title: "How It Works", href: "/docs/parallel" },
      { title: "Lane-Based Processing", href: "/docs/parallel/lanes" },
      { title: "Throughput Benchmarks", href: "/docs/parallel/benchmarks" },
    ],
  },
  {
    title: "API Reference",
    items: [
      { title: "JSON-RPC API", href: "/docs/api/jsonrpc" },
      { title: "REST API", href: "/docs/api/rest" },
      { title: "WebSocket API", href: "/docs/api/websocket" },
      { title: "Error Codes", href: "/docs/api/errors" },
    ],
  },
  {
    title: "SDKs",
    items: [
      { title: "JavaScript / TypeScript", href: "/docs/sdk/javascript" },
      { title: "Python", href: "/docs/sdk/python" },
      { title: "Rust (dina-core)", href: "/docs/sdk/rust" },
      { title: "CLI Reference", href: "/docs/sdk/cli" },
    ],
  },
  {
    title: "Compare",
    items: [
      { title: "Dina vs Other Chains", href: "/docs/compare" },
    ],
  },
  {
    title: "Infrastructure",
    items: [
      { title: "Run a Validator", href: "/docs/validators" },
      { title: "Network Status", href: "/docs/network" },
      { title: "Device Attestation", href: "/docs/devices" },
      { title: "Webhooks", href: "/docs/webhooks" },
    ],
  },
];

export const API_ENDPOINTS = {
  // REST API
  rest: {
    health: { method: "GET", path: "/health", description: "Node health and block height" },
    getAccount: { method: "GET", path: "/accounts/:address", description: "Account balance and nonce" },
    submitTx: { method: "POST", path: "/transactions", description: "Submit a signed transaction" },
    getBlock: { method: "GET", path: "/blocks/:number", description: "Get block by number" },
    getLatestBlock: { method: "GET", path: "/blocks/latest", description: "Get latest block" },
    getTx: { method: "GET", path: "/transactions/:hash", description: "Get transaction by hash" },
    getPeers: { method: "GET", path: "/peers", description: "Connected peer list" },
    getDevices: { method: "GET", path: "/devices", description: "Registered Cognitum devices" },
    faucet: { method: "POST", path: "/faucet/:address", description: "Request testnet USDC" },
  },
  // JSON-RPC methods
  jsonrpc: [
    { method: "dina_blockNumber", params: [], description: "Current block height" },
    { method: "dina_getBalance", params: ["address"], description: "Account USDC balance" },
    { method: "dina_getAccount", params: ["address"], description: "Full account details" },
    { method: "dina_getBlock", params: ["blockNumber"], description: "Block data with transactions" },
    { method: "dina_getTransaction", params: ["txHash"], description: "Transaction details and receipt" },
    { method: "dina_sendTransaction", params: ["signedTxHex"], description: "Broadcast signed transaction" },
    { method: "dina_estimateGas", params: ["txParams"], description: "Estimate gas for transaction" },
    { method: "dina_getTransactionReceipt", params: ["txHash"], description: "Transaction receipt with events" },
    { method: "dina_chainId", params: [], description: "Chain identifier string" },
    { method: "dina_networkInfo", params: [], description: "Network configuration and stats" },
    { method: "dina_peerCount", params: [], description: "Number of connected peers" },
    { method: "dina_syncing", params: [], description: "Sync status" },
  ],
};
