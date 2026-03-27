import { sha256 } from '@noble/hashes/sha256';
import type {
  Account,
  Address,
  Block,
  ContractCallParams,
  DeployParams,
  DeviceInfo,
  Hash,
  NetworkInfo,
  RpcRequest,
  RpcResponse,
  TransactionReceipt,
  TransferParams,
} from './types';
import { DinaWallet } from './wallet';
import {
  bytesToHex,
  concatBytes,
  encodeBigintLE,
  encodeString,
  hexToBytes,
} from './utils';

/**
 * JSON-RPC client for the Dina Network.
 *
 * Communicates with a Dina node over HTTP (queries + transactions)
 * and optionally WebSocket (subscriptions).
 */
export class DinaClient {
  private readonly rpcUrl: string;
  private readonly wsUrl: string | null;
  private requestId = 0;
  private ws: WebSocket | null = null;
  private subscriptions = new Map<
    number,
    { method: string; callback: (data: unknown) => void }
  >();

  /**
   * @param rpcUrl - HTTP endpoint of a Dina node, e.g. "https://rpc.dina.network"
   * @param wsUrl  - Optional WebSocket endpoint for subscriptions.
   *                 If omitted, derived from rpcUrl by replacing http with ws.
   */
  constructor(rpcUrl: string, wsUrl?: string) {
    this.rpcUrl = rpcUrl.replace(/\/+$/, '');
    if (wsUrl) {
      this.wsUrl = wsUrl.replace(/\/+$/, '');
    } else {
      this.wsUrl = this.rpcUrl
        .replace(/^https:/, 'wss:')
        .replace(/^http:/, 'ws:');
    }
  }

  // ---------------------------------------------------------------------------
  // Low-level RPC
  // ---------------------------------------------------------------------------

  private nextId(): number {
    return ++this.requestId;
  }

  private async rpc<T>(method: string, params: unknown[] = []): Promise<T> {
    const body: RpcRequest = {
      jsonrpc: '2.0',
      id: this.nextId(),
      method,
      params,
    };

    const res = await fetch(this.rpcUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body, (_key, value) =>
        typeof value === 'bigint' ? value.toString() : value
      ),
    });

    if (!res.ok) {
      throw new DinaRpcError(
        -1,
        `HTTP ${res.status}: ${res.statusText}`,
        undefined
      );
    }

    const json = (await res.json()) as RpcResponse<T>;
    if (json.error) {
      throw new DinaRpcError(
        json.error.code,
        json.error.message,
        json.error.data
      );
    }
    return json.result as T;
  }

  // ---------------------------------------------------------------------------
  // Queries
  // ---------------------------------------------------------------------------

  /** Get the USDC balance of an address in micro-units. */
  async getBalance(address: Address): Promise<bigint> {
    const result = await this.rpc<string>('dina_getBalance', [address]);
    return BigInt(result);
  }

  /** Get full account info including nonce. */
  async getAccount(address: Address): Promise<Account> {
    const raw = await this.rpc<{
      address: string;
      balance: string;
      nonce: number;
    }>('dina_getAccount', [address]);
    return {
      address: raw.address,
      balance: BigInt(raw.balance),
      nonce: raw.nonce,
    };
  }

  /** Get a block by height. */
  async getBlock(height: number): Promise<Block> {
    return this.rpc<Block>('dina_getBlock', [height]);
  }

  /** Get the latest finalized block. */
  async getLatestBlock(): Promise<Block> {
    return this.rpc<Block>('dina_getLatestBlock', []);
  }

  /** Get a transaction receipt by hash. */
  async getTransaction(hash: Hash): Promise<TransactionReceipt> {
    const raw = await this.rpc<{
      txHash: string;
      blockHeight: number;
      success: boolean;
      gasUsed: number;
      feePaid: string;
      error?: string;
    }>('dina_getTransaction', [hash]);
    return {
      ...raw,
      feePaid: BigInt(raw.feePaid),
    };
  }

  /** Get network status information. */
  async getNetworkInfo(): Promise<NetworkInfo> {
    return this.rpc<NetworkInfo>('dina_getNetworkInfo', []);
  }

  /** Get registered device info by its public key. */
  async getDevice(pubkey: string): Promise<DeviceInfo> {
    return this.rpc<DeviceInfo>('dina_getDevice', [pubkey]);
  }

  // ---------------------------------------------------------------------------
  // Transactions
  // ---------------------------------------------------------------------------

  /** Submit a pre-signed transaction blob. Returns the transaction hash. */
  async sendTransaction(signedTx: string): Promise<Hash> {
    return this.rpc<Hash>('dina_sendTransaction', [signedTx]);
  }

  /**
   * Build, sign, and send a USDC transfer.
   * Returns the transaction hash.
   */
  async transfer(wallet: DinaWallet, params: TransferParams): Promise<Hash> {
    const account = await this.getAccount(wallet.address);
    const txPayload = this.buildTransferPayload(
      wallet.address,
      params,
      account.nonce
    );
    const signature = wallet.sign(txPayload);
    const signedTx = JSON.stringify({
      type: 'transfer',
      from: wallet.address,
      to: params.to,
      amount: params.amount.toString(),
      memo: params.memo ?? '',
      nonce: account.nonce,
      signature,
    });
    return this.sendTransaction(signedTx);
  }

  /**
   * Deploy a WASM smart contract.
   * Returns the transaction hash. The contract address can be derived from the receipt.
   */
  async deployContract(
    wallet: DinaWallet,
    params: DeployParams
  ): Promise<Hash> {
    const account = await this.getAccount(wallet.address);
    const wasmHex = bytesToHex(params.wasmBytes);
    const txPayload = this.buildDeployPayload(
      wallet.address,
      wasmHex,
      account.nonce
    );
    const signature = wallet.sign(txPayload);
    const signedTx = JSON.stringify({
      type: 'deploy',
      from: wallet.address,
      wasmBytes: wasmHex,
      initArgs: params.initArgs,
      nonce: account.nonce,
      signature,
    });
    return this.sendTransaction(signedTx);
  }

  /**
   * Call a method on a deployed smart contract.
   * Returns the transaction hash.
   */
  async callContract(
    wallet: DinaWallet,
    params: ContractCallParams
  ): Promise<Hash> {
    const account = await this.getAccount(wallet.address);
    const txPayload = this.buildCallPayload(
      wallet.address,
      params,
      account.nonce
    );
    const signature = wallet.sign(txPayload);
    const signedTx = JSON.stringify({
      type: 'call',
      from: wallet.address,
      contract: params.contract,
      method: params.method,
      args: params.args,
      usdcAttached: (params.usdcAttached ?? 0n).toString(),
      nonce: account.nonce,
      signature,
    });
    return this.sendTransaction(signedTx);
  }

  // ---------------------------------------------------------------------------
  // Subscriptions (WebSocket)
  // ---------------------------------------------------------------------------

  /**
   * Subscribe to new block events.
   * Returns an unsubscribe function.
   */
  onNewBlock(callback: (block: Block) => void): () => void {
    const subId = this.nextId();
    this.ensureWebSocket();
    this.sendWs('dina_subscribe', ['newBlock'], subId);
    this.subscriptions.set(subId, {
      method: 'newBlock',
      callback: callback as (data: unknown) => void,
    });
    return () => {
      this.subscriptions.delete(subId);
      this.sendWs('dina_unsubscribe', [subId], this.nextId());
    };
  }

  /**
   * Subscribe to transactions involving a specific address.
   * Returns an unsubscribe function.
   */
  onTransaction(
    address: Address,
    callback: (tx: TransactionReceipt) => void
  ): () => void {
    const subId = this.nextId();
    this.ensureWebSocket();
    this.sendWs('dina_subscribe', ['transaction', address], subId);
    this.subscriptions.set(subId, {
      method: 'transaction',
      callback: callback as (data: unknown) => void,
    });
    return () => {
      this.subscriptions.delete(subId);
      this.sendWs('dina_unsubscribe', [subId], this.nextId());
    };
  }

  // ---------------------------------------------------------------------------
  // Utility
  // ---------------------------------------------------------------------------

  /** Estimate the fee for a transaction type. */
  async estimateFee(txType: string, params: unknown): Promise<bigint> {
    const result = await this.rpc<string>('dina_estimateFee', [
      txType,
      params,
    ]);
    return BigInt(result);
  }

  /**
   * Poll until a transaction is confirmed or timeout is reached.
   * @param hash    - Transaction hash to wait for.
   * @param timeout - Timeout in milliseconds (default 30000).
   */
  async waitForTransaction(
    hash: Hash,
    timeout = 30_000
  ): Promise<TransactionReceipt> {
    const start = Date.now();
    const pollInterval = 1_000;

    while (Date.now() - start < timeout) {
      try {
        const receipt = await this.getTransaction(hash);
        if (receipt) return receipt;
      } catch {
        // Transaction not found yet — keep polling.
      }
      await new Promise((r) => setTimeout(r, pollInterval));
    }

    throw new Error(
      `Transaction ${hash} not confirmed within ${timeout}ms`
    );
  }

  /** Disconnect WebSocket if open. */
  disconnect(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.subscriptions.clear();
  }

  // ---------------------------------------------------------------------------
  // Internal helpers
  // ---------------------------------------------------------------------------

  private buildTransferPayload(
    from: string,
    params: TransferParams,
    nonce: number
  ): Uint8Array {
    return sha256(
      concatBytes(
        encodeString('transfer'),
        hexToBytes(from),
        hexToBytes(params.to),
        encodeBigintLE(params.amount),
        encodeString(params.memo ?? ''),
        encodeBigintLE(BigInt(nonce))
      )
    );
  }

  private buildDeployPayload(
    from: string,
    wasmHex: string,
    nonce: number
  ): Uint8Array {
    return sha256(
      concatBytes(
        encodeString('deploy'),
        hexToBytes(from),
        sha256(hexToBytes(wasmHex)), // hash of wasm to keep payload small
        encodeBigintLE(BigInt(nonce))
      )
    );
  }

  private buildCallPayload(
    from: string,
    params: ContractCallParams,
    nonce: number
  ): Uint8Array {
    return sha256(
      concatBytes(
        encodeString('call'),
        hexToBytes(from),
        hexToBytes(params.contract),
        encodeString(params.method),
        encodeString(JSON.stringify(params.args)),
        encodeBigintLE(params.usdcAttached ?? 0n),
        encodeBigintLE(BigInt(nonce))
      )
    );
  }

  private ensureWebSocket(): void {
    if (this.ws) return;
    if (!this.wsUrl) {
      throw new Error('No WebSocket URL configured');
    }

    const WS =
      typeof WebSocket !== 'undefined'
        ? WebSocket
        : (require('ws') as typeof WebSocket);

    this.ws = new WS(this.wsUrl);

    this.ws.onmessage = (event: MessageEvent) => {
      try {
        const data = JSON.parse(
          typeof event.data === 'string' ? event.data : event.data.toString()
        );

        // Subscription notifications come as { jsonrpc, method, params: { subscription, result } }
        if (data.method === 'dina_subscription' && data.params) {
          const sub = this.subscriptions.get(data.params.subscription);
          if (sub) {
            sub.callback(data.params.result);
          }
        }
      } catch {
        // Ignore malformed messages
      }
    };

    this.ws.onerror = () => {
      this.ws = null;
    };

    this.ws.onclose = () => {
      this.ws = null;
    };
  }

  private sendWs(method: string, params: unknown[], id: number): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      // Queue until open
      const check = setInterval(() => {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
          clearInterval(check);
          this.ws.send(
            JSON.stringify({ jsonrpc: '2.0', id, method, params })
          );
        }
      }, 50);
      return;
    }
    this.ws.send(JSON.stringify({ jsonrpc: '2.0', id, method, params }));
  }
}

/** Structured RPC error from a Dina node. */
export class DinaRpcError extends Error {
  constructor(
    public readonly code: number,
    message: string,
    public readonly data: unknown
  ) {
    super(message);
    this.name = 'DinaRpcError';
  }
}
