/** Hex-encoded 32-byte address */
export type Address = string;

/** Hex-encoded 32-byte hash */
export type Hash = string;

/** Hex-encoded 64-byte Ed25519 signature */
export type Signature = string;

export interface Account {
  address: Address;
  /** USDC balance in micro-units (1 USDC = 1_000_000 micro) */
  balance: bigint;
  nonce: number;
}

export interface Block {
  height: number;
  hash: Hash;
  parentHash: Hash;
  timestamp: number;
  proposer: Address;
  transactionCount: number;
  stateRoot: Hash;
}

export interface TransactionReceipt {
  txHash: Hash;
  blockHeight: number;
  success: boolean;
  gasUsed: number;
  feePaid: bigint;
  error?: string;
}

export interface TransferParams {
  to: Address;
  amount: bigint;
  memo?: string;
}

export interface ContractCallParams {
  contract: Address;
  method: string;
  args: unknown;
  usdcAttached?: bigint;
}

export interface DeployParams {
  wasmBytes: Uint8Array;
  initArgs: unknown;
}

export interface NetworkInfo {
  chainId: string;
  blockHeight: number;
  peerCount: number;
  version: string;
  epoch: number;
}

export interface DeviceInfo {
  pubkey: string;
  deviceType: string;
  owner: Address;
  registered: number;
  lastSeen: number;
  status: 'active' | 'inactive' | 'suspended';
}

export interface SignedState {
  channelId: string;
  nonce: number;
  balanceA: bigint;
  balanceB: bigint;
  signature: Signature;
}

export interface SpendingStats {
  totalSpent: bigint;
  transactionCount: number;
  lastTransaction: number;
  dailyLimit: bigint;
  dailySpent: bigint;
}

export interface RpcRequest {
  jsonrpc: '2.0';
  id: number;
  method: string;
  params: unknown[];
}

export interface RpcResponse<T = unknown> {
  jsonrpc: '2.0';
  id: number;
  result?: T;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}
