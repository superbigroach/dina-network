"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.DinaRpcError = exports.DinaClient = void 0;
const sha256_1 = require("@noble/hashes/sha256");
const utils_1 = require("./utils");
/**
 * JSON-RPC client for the Dina Network.
 *
 * Communicates with a Dina node over HTTP (queries + transactions)
 * and optionally WebSocket (subscriptions).
 */
class DinaClient {
    /**
     * @param rpcUrl - HTTP endpoint of a Dina node, e.g. "https://rpc.dina.network"
     * @param wsUrl  - Optional WebSocket endpoint for subscriptions.
     *                 If omitted, derived from rpcUrl by replacing http with ws.
     */
    constructor(rpcUrl, wsUrl) {
        this.requestId = 0;
        this.ws = null;
        this.subscriptions = new Map();
        this.rpcUrl = rpcUrl.replace(/\/+$/, '');
        if (wsUrl) {
            this.wsUrl = wsUrl.replace(/\/+$/, '');
        }
        else {
            this.wsUrl = this.rpcUrl
                .replace(/^https:/, 'wss:')
                .replace(/^http:/, 'ws:');
        }
    }
    // ---------------------------------------------------------------------------
    // Low-level RPC
    // ---------------------------------------------------------------------------
    nextId() {
        return ++this.requestId;
    }
    async rpc(method, params = []) {
        const body = {
            jsonrpc: '2.0',
            id: this.nextId(),
            method,
            params,
        };
        const res = await fetch(this.rpcUrl, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body, (_key, value) => typeof value === 'bigint' ? value.toString() : value),
        });
        if (!res.ok) {
            throw new DinaRpcError(-1, `HTTP ${res.status}: ${res.statusText}`, undefined);
        }
        const json = (await res.json());
        if (json.error) {
            throw new DinaRpcError(json.error.code, json.error.message, json.error.data);
        }
        return json.result;
    }
    // ---------------------------------------------------------------------------
    // Queries
    // ---------------------------------------------------------------------------
    /** Get the USDC balance of an address in micro-units. */
    async getBalance(address) {
        const result = await this.rpc('dina_getBalance', [address]);
        return BigInt(result);
    }
    /** Get full account info including nonce. */
    async getAccount(address) {
        const raw = await this.rpc('dina_getAccount', [address]);
        return {
            address: raw.address,
            balance: BigInt(raw.balance),
            nonce: raw.nonce,
        };
    }
    /** Get a block by height. */
    async getBlock(height) {
        return this.rpc('dina_getBlock', [height]);
    }
    /** Get the latest finalized block. */
    async getLatestBlock() {
        return this.rpc('dina_getLatestBlock', []);
    }
    /** Get a transaction receipt by hash. */
    async getTransaction(hash) {
        const raw = await this.rpc('dina_getTransaction', [hash]);
        return {
            ...raw,
            feePaid: BigInt(raw.feePaid),
        };
    }
    /** Get network status information. */
    async getNetworkInfo() {
        return this.rpc('dina_getNetworkInfo', []);
    }
    /** Get registered device info by its public key. */
    async getDevice(pubkey) {
        return this.rpc('dina_getDevice', [pubkey]);
    }
    // ---------------------------------------------------------------------------
    // Transactions
    // ---------------------------------------------------------------------------
    /** Submit a pre-signed transaction blob. Returns the transaction hash. */
    async sendTransaction(signedTx) {
        return this.rpc('dina_sendTransaction', [signedTx]);
    }
    /**
     * Build, sign, and send a USDC transfer.
     * Returns the transaction hash.
     */
    async transfer(wallet, params) {
        const account = await this.getAccount(wallet.address);
        const txPayload = this.buildTransferPayload(wallet.address, params, account.nonce);
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
    async deployContract(wallet, params) {
        const account = await this.getAccount(wallet.address);
        const wasmHex = (0, utils_1.bytesToHex)(params.wasmBytes);
        const txPayload = this.buildDeployPayload(wallet.address, wasmHex, account.nonce);
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
    async callContract(wallet, params) {
        const account = await this.getAccount(wallet.address);
        const txPayload = this.buildCallPayload(wallet.address, params, account.nonce);
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
    onNewBlock(callback) {
        const subId = this.nextId();
        this.ensureWebSocket();
        this.sendWs('dina_subscribe', ['newBlock'], subId);
        this.subscriptions.set(subId, {
            method: 'newBlock',
            callback: callback,
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
    onTransaction(address, callback) {
        const subId = this.nextId();
        this.ensureWebSocket();
        this.sendWs('dina_subscribe', ['transaction', address], subId);
        this.subscriptions.set(subId, {
            method: 'transaction',
            callback: callback,
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
    async estimateFee(txType, params) {
        const result = await this.rpc('dina_estimateFee', [
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
    async waitForTransaction(hash, timeout = 30000) {
        const start = Date.now();
        const pollInterval = 1000;
        while (Date.now() - start < timeout) {
            try {
                const receipt = await this.getTransaction(hash);
                if (receipt)
                    return receipt;
            }
            catch {
                // Transaction not found yet — keep polling.
            }
            await new Promise((r) => setTimeout(r, pollInterval));
        }
        throw new Error(`Transaction ${hash} not confirmed within ${timeout}ms`);
    }
    /** Disconnect WebSocket if open. */
    disconnect() {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
        this.subscriptions.clear();
    }
    // ---------------------------------------------------------------------------
    // Internal helpers
    // ---------------------------------------------------------------------------
    buildTransferPayload(from, params, nonce) {
        return (0, sha256_1.sha256)((0, utils_1.concatBytes)((0, utils_1.encodeString)('transfer'), (0, utils_1.hexToBytes)(from), (0, utils_1.hexToBytes)(params.to), (0, utils_1.encodeBigintLE)(params.amount), (0, utils_1.encodeString)(params.memo ?? ''), (0, utils_1.encodeBigintLE)(BigInt(nonce))));
    }
    buildDeployPayload(from, wasmHex, nonce) {
        return (0, sha256_1.sha256)((0, utils_1.concatBytes)((0, utils_1.encodeString)('deploy'), (0, utils_1.hexToBytes)(from), (0, sha256_1.sha256)((0, utils_1.hexToBytes)(wasmHex)), // hash of wasm to keep payload small
        (0, utils_1.encodeBigintLE)(BigInt(nonce))));
    }
    buildCallPayload(from, params, nonce) {
        return (0, sha256_1.sha256)((0, utils_1.concatBytes)((0, utils_1.encodeString)('call'), (0, utils_1.hexToBytes)(from), (0, utils_1.hexToBytes)(params.contract), (0, utils_1.encodeString)(params.method), (0, utils_1.encodeString)(JSON.stringify(params.args)), (0, utils_1.encodeBigintLE)(params.usdcAttached ?? 0n), (0, utils_1.encodeBigintLE)(BigInt(nonce))));
    }
    ensureWebSocket() {
        if (this.ws)
            return;
        if (!this.wsUrl) {
            throw new Error('No WebSocket URL configured');
        }
        const WS = typeof WebSocket !== 'undefined'
            ? WebSocket
            : require('ws');
        this.ws = new WS(this.wsUrl);
        this.ws.onmessage = (event) => {
            try {
                const data = JSON.parse(typeof event.data === 'string' ? event.data : event.data.toString());
                // Subscription notifications come as { jsonrpc, method, params: { subscription, result } }
                if (data.method === 'dina_subscription' && data.params) {
                    const sub = this.subscriptions.get(data.params.subscription);
                    if (sub) {
                        sub.callback(data.params.result);
                    }
                }
            }
            catch {
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
    sendWs(method, params, id) {
        if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
            // Queue until open
            const check = setInterval(() => {
                if (this.ws && this.ws.readyState === WebSocket.OPEN) {
                    clearInterval(check);
                    this.ws.send(JSON.stringify({ jsonrpc: '2.0', id, method, params }));
                }
            }, 50);
            return;
        }
        this.ws.send(JSON.stringify({ jsonrpc: '2.0', id, method, params }));
    }
}
exports.DinaClient = DinaClient;
/** Structured RPC error from a Dina node. */
class DinaRpcError extends Error {
    constructor(code, message, data) {
        super(message);
        this.code = code;
        this.data = data;
        this.name = 'DinaRpcError';
    }
}
exports.DinaRpcError = DinaRpcError;
//# sourceMappingURL=client.js.map