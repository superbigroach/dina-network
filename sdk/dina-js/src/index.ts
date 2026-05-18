// Core types
export type {
  Address,
  Hash,
  Signature,
  Account,
  Block,
  TransactionReceipt,
  TransferParams,
  ContractCallParams,
  DeployParams,
  NetworkInfo,
  DeviceInfo,
  SignedState,
  SpendingStats,
  RpcRequest,
  RpcResponse,
} from './types';

// Wallet
export { DinaWallet } from './wallet';

// Passkey wallet (browser environment required)
export { PasskeyWallet } from './passkey';
export type { PasskeyRegistration, PasskeyAssertion } from './passkey';

// Client
export { DinaClient, DinaRpcError } from './client';

// Contracts
export {
  DinaContract,
  TokenContract,
  AgentWalletContract,
} from './contract';

// Parallel wallet
export { ParallelWallet } from './parallel';
export type {
  ParallelConfig,
  ParallelStats,
  TransferPriority,
  BatchOptions,
} from './parallel';

// Payment channels
export { PaymentChannel } from './channel';

// Utilities
export {
  addressFromPublicKey,
  formatUSDC,
  parseUSDC,
  hexToBytes,
  bytesToHex,
  isValidAddress,
  isValidHash,
  concatBytes,
  encodeBigintLE,
  encodeString,
} from './utils';
