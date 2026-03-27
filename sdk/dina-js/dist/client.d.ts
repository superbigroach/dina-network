import type { Account, Address, Block, ContractCallParams, DeployParams, DeviceInfo, Hash, NetworkInfo, TransactionReceipt, TransferParams } from './types';
import { DinaWallet } from './wallet';
/**
 * JSON-RPC client for the Dina Network.
 *
 * Communicates with a Dina node over HTTP (queries + transactions)
 * and optionally WebSocket (subscriptions).
 */
export declare class DinaClient {
    private readonly rpcUrl;
    private readonly wsUrl;
    private requestId;
    private ws;
    private subscriptions;
    /**
     * @param rpcUrl - HTTP endpoint of a Dina node, e.g. "https://rpc.dina.network"
     * @param wsUrl  - Optional WebSocket endpoint for subscriptions.
     *                 If omitted, derived from rpcUrl by replacing http with ws.
     */
    constructor(rpcUrl: string, wsUrl?: string);
    private nextId;
    private rpc;
    /** Get the USDC balance of an address in micro-units. */
    getBalance(address: Address): Promise<bigint>;
    /** Get full account info including nonce. */
    getAccount(address: Address): Promise<Account>;
    /** Get a block by height. */
    getBlock(height: number): Promise<Block>;
    /** Get the latest finalized block. */
    getLatestBlock(): Promise<Block>;
    /** Get a transaction receipt by hash. */
    getTransaction(hash: Hash): Promise<TransactionReceipt>;
    /** Get network status information. */
    getNetworkInfo(): Promise<NetworkInfo>;
    /** Get registered device info by its public key. */
    getDevice(pubkey: string): Promise<DeviceInfo>;
    /** Submit a pre-signed transaction blob. Returns the transaction hash. */
    sendTransaction(signedTx: string): Promise<Hash>;
    /**
     * Build, sign, and send a USDC transfer.
     * Returns the transaction hash.
     */
    transfer(wallet: DinaWallet, params: TransferParams): Promise<Hash>;
    /**
     * Deploy a WASM smart contract.
     * Returns the transaction hash. The contract address can be derived from the receipt.
     */
    deployContract(wallet: DinaWallet, params: DeployParams): Promise<Hash>;
    /**
     * Call a method on a deployed smart contract.
     * Returns the transaction hash.
     */
    callContract(wallet: DinaWallet, params: ContractCallParams): Promise<Hash>;
    /**
     * Subscribe to new block events.
     * Returns an unsubscribe function.
     */
    onNewBlock(callback: (block: Block) => void): () => void;
    /**
     * Subscribe to transactions involving a specific address.
     * Returns an unsubscribe function.
     */
    onTransaction(address: Address, callback: (tx: TransactionReceipt) => void): () => void;
    /** Estimate the fee for a transaction type. */
    estimateFee(txType: string, params: unknown): Promise<bigint>;
    /**
     * Poll until a transaction is confirmed or timeout is reached.
     * @param hash    - Transaction hash to wait for.
     * @param timeout - Timeout in milliseconds (default 30000).
     */
    waitForTransaction(hash: Hash, timeout?: number): Promise<TransactionReceipt>;
    /** Disconnect WebSocket if open. */
    disconnect(): void;
    private buildTransferPayload;
    private buildDeployPayload;
    private buildCallPayload;
    private ensureWebSocket;
    private sendWs;
}
/** Structured RPC error from a Dina node. */
export declare class DinaRpcError extends Error {
    readonly code: number;
    readonly data: unknown;
    constructor(code: number, message: string, data: unknown);
}
//# sourceMappingURL=client.d.ts.map