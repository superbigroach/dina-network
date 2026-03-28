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
  /** Request timeout in milliseconds (default 30 000). */
  private readonly timeout: number;

  /**
   * @param rpcUrl  - HTTP endpoint of a Dina node, e.g. "https://rpc.dina.network"
   * @param options - Optional configuration.
   * @param options.wsUrl   - WebSocket endpoint for subscriptions.
   *                          If omitted, derived from rpcUrl by replacing http with ws.
   * @param options.timeout - Request timeout in milliseconds (default 30 000).
   */
  constructor(
    rpcUrl: string,
    options?: { wsUrl?: string; timeout?: number }
  ) {
    this.rpcUrl = rpcUrl.replace(/\/+$/, '');
    this.timeout = options?.timeout ?? 30_000;
    const wsUrl = options?.wsUrl;
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

    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeout);

    let res: Response;
    try {
      res = await fetch(this.rpcUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body, (_key, value) =>
          typeof value === 'bigint' ? value.toString() : value
        ),
        signal: controller.signal,
      });
    } catch (err) {
      if (err instanceof DOMException && err.name === 'AbortError') {
        throw new DinaRpcError(-1, `Request timed out after ${this.timeout}ms`, undefined);
      }
      throw err;
    } finally {
      clearTimeout(timer);
    }

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
    const result = await this.rpc<string | number>('dina_getBalance', [address]);
    // The server may return a string or a number — normalize to BigInt safely.
    return BigInt(typeof result === 'number' ? Math.trunc(result) : result);
  }

  /** Get full account info including nonce. */
  async getAccount(address: Address): Promise<Account> {
    const raw = await this.rpc<{
      address: string;
      balance: string | number;
      nonce: string | number;
    }>('dina_getAccount', [address]);
    return {
      address: raw.address,
      // Balance may arrive as a string or number depending on the node version.
      balance: BigInt(typeof raw.balance === 'number' ? Math.trunc(raw.balance) : raw.balance),
      nonce: typeof raw.nonce === 'string' ? parseInt(raw.nonce, 10) : raw.nonce,
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
      feePaid: string | number;
      error?: string;
    }>('dina_getTransaction', [hash]);
    return {
      ...raw,
      feePaid: BigInt(typeof raw.feePaid === 'number' ? Math.trunc(raw.feePaid) : raw.feePaid),
    };
  }

  /** Get network status information. */
  async getNetworkInfo(): Promise<NetworkInfo> {
    return this.rpc<NetworkInfo>('dina_networkInfo', []);
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
    // Normalize amount to BigInt in case the caller passes a number by mistake.
    const amount: bigint = typeof params.amount === 'number'
      ? BigInt(Math.trunc(params.amount as unknown as number))
      : BigInt(params.amount);

    if (amount <= 0n) {
      throw new Error('Transfer amount must be positive');
    }
    if (amount > 18_446_744_073_709_551_615n) {
      throw new Error('Amount exceeds u64 max');
    }
    if (params.to === wallet.address) {
      throw new Error('Cannot transfer to self');
    }
    const account = await this.getAccount(wallet.address);
    const nonce: number = typeof account.nonce === 'number'
      ? account.nonce
      : Number(account.nonce);

    // Build the canonical payload for signing using the normalized BigInt amount.
    const normalizedParams: TransferParams = { ...params, amount };
    const txPayload = this.buildTransferPayload(
      wallet.address,
      normalizedParams,
      nonce
    );
    const signature = wallet.sign(txPayload);

    // Serialize — BigInt values are converted to strings to avoid
    // "TypeError: Do not know how to serialize a BigInt".
    const signedTx = JSON.stringify({
      type: 'transfer',
      from: wallet.address,
      to: params.to,
      amount: amount.toString(),
      memo: params.memo ?? '',
      nonce,
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
      nonce: typeof account.nonce === 'number' ? account.nonce : Number(account.nonce),
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

  /** Estimate the gas cost for a transaction type. */
  async estimateGas(txType: string, params: unknown): Promise<bigint> {
    const result = await this.rpc<{ gas_estimate: string | number }>('dina_estimateGas', [
      txType,
      params,
    ]);
    const v = result.gas_estimate;
    return BigInt(typeof v === 'number' ? Math.trunc(v) : v);
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
