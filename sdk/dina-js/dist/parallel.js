"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ParallelWallet = void 0;
const wallet_1 = require("./wallet");
// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
/** Estimated fee per on-chain transaction in micro-USDC */
const FEE_PER_TX = 100n; // $0.0001
/** Max recipients per batch transaction (DRC-19 limit) */
const MAX_BATCH_SIZE = 100;
/** Estimated time per block in milliseconds */
const BLOCK_TIME_MS = 100;
// ---------------------------------------------------------------------------
// ParallelWallet
// ---------------------------------------------------------------------------
/**
 * Auto-scaling parallel wallet system for the Dina Network.
 *
 * Manages N sub-wallets under a single master wallet, enabling truly parallel
 * on-chain transactions by giving each sub-wallet an independent nonce.
 *
 * Supports 4 transaction modes:
 *   - single:         1 wallet, 1 transfer (trivial case)
 *   - batch:          1 wallet, 1 batch-transfer to many recipients
 *   - parallel:       N wallets, each sends 1 transfer concurrently
 *   - parallel-batch: N wallets, each sends 1 batch-transfer concurrently
 */
class ParallelWallet {
    constructor(masterWallet, client, config) {
        this.masterWallet = masterWallet;
        this.client = client;
        this.config = {
            maxWallets: config?.maxWallets ?? 100,
            autoScale: config?.autoScale ?? true,
            minBalancePerWallet: config?.minBalancePerWallet ?? 1000000n,
        };
        this.subWallets = [];
    }
    // -------------------------------------------------------------------------
    // Factory presets
    // -------------------------------------------------------------------------
    /** Solo mode -- single wallet, no parallelism. */
    static solo(wallet, client) {
        return new ParallelWallet(wallet, client, {
            maxWallets: 1,
            autoScale: false,
            minBalancePerWallet: 0n,
        });
    }
    /** Standard -- up to 10 sub-wallets with auto-scaling. */
    static standard(wallet, client) {
        return new ParallelWallet(wallet, client, {
            maxWallets: 10,
            autoScale: true,
            minBalancePerWallet: 1000000n,
        });
    }
    /** Pro -- up to 100 sub-wallets. */
    static pro(wallet, client) {
        return new ParallelWallet(wallet, client, {
            maxWallets: 100,
            autoScale: true,
            minBalancePerWallet: 1000000n,
        });
    }
    /** Enterprise -- up to 10,000 sub-wallets for maximum throughput. */
    static enterprise(wallet, client) {
        return new ParallelWallet(wallet, client, {
            maxWallets: 10000,
            autoScale: true,
            minBalancePerWallet: 1000000n,
        });
    }
    // -------------------------------------------------------------------------
    // Core operations
    // -------------------------------------------------------------------------
    /**
     * Send a single USDC transfer.
     *
     * Uses the master wallet directly for a single transfer.
     */
    async transfer(to, amount) {
        return this.client.transfer(this.masterWallet, { to, amount });
    }
    /**
     * Send multiple payments, automatically selecting the optimal strategy.
     *
     * Depending on the payment count and options, this will use:
     *   - single mode for 1 payment
     *   - batch mode for 2-100 payments when optimizing for cost
     *   - parallel mode for 2-100 payments when optimizing for speed
     *   - parallel-batch mode for 100+ payments
     */
    async batchTransfer(payments, options) {
        if (payments.length === 0) {
            return [];
        }
        // Single payment -- just send it directly
        if (payments.length === 1) {
            const hash = await this.transfer(payments[0].to, payments[0].amount);
            return [hash];
        }
        const strategy = this.optimizeStrategy(payments.length, options);
        switch (strategy.mode) {
            case 'single':
                return this.executeSingle(payments);
            case 'batch':
                return this.executeBatch(payments, strategy.batchSize);
            case 'parallel':
                await this.ensureWallets(strategy.walletsNeeded);
                return this.executeParallel(payments);
            case 'parallel-batch':
                await this.ensureWallets(strategy.walletsNeeded);
                return this.executeParallelBatch(payments, strategy);
            default:
                return this.executeSingle(payments);
        }
    }
    // -------------------------------------------------------------------------
    // Management
    // -------------------------------------------------------------------------
    /**
     * Create N new sub-wallets by generating Ed25519 keypairs.
     * Returns the addresses of the new wallets.
     */
    async createWallets(count) {
        const toCreate = Math.min(count, this.config.maxWallets - this.subWallets.length);
        if (toCreate <= 0) {
            return [];
        }
        const newAddresses = [];
        for (let i = 0; i < toCreate; i++) {
            const wallet = wallet_1.DinaWallet.generate();
            this.subWallets.push(wallet);
            newAddresses.push(wallet.address);
        }
        return newAddresses;
    }
    /**
     * Fund all sub-wallets by distributing totalAmount evenly from the master
     * wallet. Sends one transfer per sub-wallet.
     */
    async fundAll(totalAmount) {
        if (this.subWallets.length === 0) {
            throw new Error('ParallelWallet: no sub-wallets to fund');
        }
        return this.distribute(totalAmount);
    }
    /**
     * Consolidate: transfer all balances from sub-wallets back to the master
     * wallet. Returns one transaction hash per sub-wallet that had a balance.
     */
    async consolidate() {
        const hashes = [];
        const balanceChecks = await Promise.all(this.subWallets.map((w) => this.client.getBalance(w.address)));
        const transferPromises = [];
        for (let i = 0; i < this.subWallets.length; i++) {
            const balance = balanceChecks[i];
            if (balance > 0n) {
                transferPromises.push(this.client.transfer(this.subWallets[i], {
                    to: this.masterWallet.address,
                    amount: balance,
                }));
            }
        }
        const results = await Promise.all(transferPromises);
        hashes.push(...results);
        return hashes;
    }
    /**
     * Return stats about the parallel wallet system.
     */
    async stats() {
        if (this.subWallets.length === 0) {
            return {
                activeWallets: 0,
                totalBalance: 0n,
                avgBalance: 0n,
                totalTransactions: 0,
            };
        }
        const accounts = await Promise.all(this.subWallets.map((w) => this.client.getAccount(w.address)));
        let totalBalance = 0n;
        let totalNonce = 0;
        for (const account of accounts) {
            totalBalance += account.balance;
            totalNonce += account.nonce;
        }
        const activeWallets = this.subWallets.length;
        return {
            activeWallets,
            totalBalance,
            avgBalance: activeWallets > 0 ? totalBalance / BigInt(activeWallets) : 0n,
            totalTransactions: totalNonce,
        };
    }
    // -------------------------------------------------------------------------
    // Internal — wallet management
    // -------------------------------------------------------------------------
    /**
     * Ensure at least `count` sub-wallets exist. Creates new ones if needed
     * (when autoScale is enabled).
     */
    async ensureWallets(count) {
        if (this.subWallets.length >= count) {
            return;
        }
        if (!this.config.autoScale) {
            throw new Error(`ParallelWallet: need ${count} wallets but only have ${this.subWallets.length} and autoScale is disabled`);
        }
        const needed = count - this.subWallets.length;
        await this.createWallets(needed);
    }
    /**
     * Distribute an amount evenly across all sub-wallets via the master wallet.
     */
    async distribute(amount) {
        const count = BigInt(this.subWallets.length);
        const perWallet = amount / count;
        const remainder = amount % count;
        // Use batch transfer for efficiency: build a list of transfers
        // and send them from the master wallet
        const transfers = [];
        for (let i = 0; i < this.subWallets.length; i++) {
            const extra = BigInt(i) < remainder ? 1n : 0n;
            const walletAmount = perWallet + extra;
            if (walletAmount > 0n) {
                transfers.push(this.client.transfer(this.masterWallet, {
                    to: this.subWallets[i].address,
                    amount: walletAmount,
                }));
            }
        }
        // Send all funding transfers. Return the first hash as representative.
        const hashes = await Promise.all(transfers);
        return hashes[0];
    }
    // -------------------------------------------------------------------------
    // Internal — execution modes
    // -------------------------------------------------------------------------
    /** Sequential single transfers from the master wallet. */
    async executeSingle(payments) {
        const hashes = [];
        for (const payment of payments) {
            const hash = await this.client.transfer(this.masterWallet, payment);
            hashes.push(hash);
        }
        return hashes;
    }
    /**
     * Batch mode: split payments into chunks of batchSize and send each chunk
     * as a batch transaction from the master wallet.
     *
     * In a real DRC-19 batch transfer, a single transaction can pay up to 100
     * recipients. Here we simulate by sending individual transfers per chunk
     * since the DinaClient.transfer API handles single recipients. A production
     * implementation would call a `batchTransfer` RPC method.
     */
    async executeBatch(payments, batchSize) {
        const allHashes = [];
        for (let i = 0; i < payments.length; i += batchSize) {
            const chunk = payments.slice(i, i + batchSize);
            // Each chunk is sent as parallel transfers from the master wallet.
            // In production this would be a single batch transaction.
            const chunkHashes = await Promise.all(chunk.map((p) => this.client.transfer(this.masterWallet, p)));
            allHashes.push(...chunkHashes);
        }
        return allHashes;
    }
    /**
     * Parallel mode: assign each payment to a different sub-wallet and send
     * all transfers concurrently (each sub-wallet has its own nonce).
     */
    async executeParallel(payments) {
        const promises = payments.map((payment, i) => {
            const wallet = this.subWallets[i % this.subWallets.length];
            return this.client.transfer(wallet, payment);
        });
        return Promise.all(promises);
    }
    /**
     * Parallel-batch mode: distribute payments across sub-wallets in chunks.
     * Each sub-wallet sends one batch transaction containing batchSize payments.
     */
    async executeParallelBatch(payments, strategy) {
        const { walletsNeeded, batchSize } = strategy;
        const allHashes = [];
        // Split payments into groups, one per sub-wallet
        const groups = Array.from({ length: walletsNeeded }, () => []);
        for (let i = 0; i < payments.length; i++) {
            groups[i % walletsNeeded].push(payments[i]);
        }
        // Each sub-wallet processes its group in batches
        const walletPromises = groups.map(async (group, walletIdx) => {
            const wallet = this.subWallets[walletIdx % this.subWallets.length];
            const hashes = [];
            for (let i = 0; i < group.length; i += batchSize) {
                const chunk = group.slice(i, i + batchSize);
                // In production each chunk would be a single batch tx.
                // Here we send them in parallel from this sub-wallet.
                const chunkHashes = await Promise.all(chunk.map((p) => this.client.transfer(wallet, p)));
                hashes.push(...chunkHashes);
            }
            return hashes;
        });
        const results = await Promise.all(walletPromises);
        for (const r of results) {
            allHashes.push(...r);
        }
        return allHashes;
    }
    // -------------------------------------------------------------------------
    // Internal — strategy optimization
    // -------------------------------------------------------------------------
    /**
     * Determine the optimal execution strategy given payment count and options.
     *
     * Decision matrix:
     *   - 1 payment:                           single (1 wallet)
     *   - 2-100 payments + priority=cost:      batch  (1 wallet, batch tx)
     *   - 2-100 payments + priority=speed:     parallel (N wallets)
     *   - 100+ payments + priority=cost:       batch  (1 wallet, sequential batches)
     *   - 100+ payments + priority=speed:      parallel-batch (N wallets x batch)
     *   - priority=auto:                       batch for < 100, parallel-batch for >= 100
     */
    optimizeStrategy(paymentCount, options) {
        const priority = options?.priority ?? 'auto';
        const forceParallel = options?.parallel === true;
        // Single payment
        if (paymentCount <= 1) {
            return {
                mode: 'single',
                walletsNeeded: 1,
                batchSize: 1,
                estimatedFee: FEE_PER_TX,
                estimatedTime: BLOCK_TIME_MS,
            };
        }
        // Force parallel if requested
        if (forceParallel) {
            const walletsNeeded = Math.min(paymentCount, this.config.maxWallets);
            if (paymentCount <= walletsNeeded) {
                return {
                    mode: 'parallel',
                    walletsNeeded,
                    batchSize: 1,
                    estimatedFee: FEE_PER_TX * BigInt(walletsNeeded),
                    estimatedTime: BLOCK_TIME_MS,
                };
            }
            const batchSize = Math.min(MAX_BATCH_SIZE, Math.ceil(paymentCount / walletsNeeded));
            return {
                mode: 'parallel-batch',
                walletsNeeded,
                batchSize,
                estimatedFee: FEE_PER_TX * BigInt(Math.ceil(paymentCount / batchSize)),
                estimatedTime: BLOCK_TIME_MS * Math.ceil(paymentCount / (walletsNeeded * batchSize)),
            };
        }
        // Small payment counts (2-99)
        if (paymentCount < MAX_BATCH_SIZE) {
            if (priority === 'cost' || priority === 'auto') {
                // Batch mode: 1 wallet, 1 batch transaction
                return {
                    mode: 'batch',
                    walletsNeeded: 1,
                    batchSize: paymentCount,
                    estimatedFee: FEE_PER_TX, // 1 batch tx = 1 fee
                    estimatedTime: BLOCK_TIME_MS,
                };
            }
            // priority === 'speed': use parallel
            const walletsNeeded = Math.min(paymentCount, this.config.maxWallets);
            return {
                mode: 'parallel',
                walletsNeeded,
                batchSize: 1,
                estimatedFee: FEE_PER_TX * BigInt(walletsNeeded),
                estimatedTime: BLOCK_TIME_MS,
            };
        }
        // Large payment counts (100+)
        if (priority === 'cost') {
            // Sequential batches from 1 wallet
            const batchCount = Math.ceil(paymentCount / MAX_BATCH_SIZE);
            return {
                mode: 'batch',
                walletsNeeded: 1,
                batchSize: MAX_BATCH_SIZE,
                estimatedFee: FEE_PER_TX * BigInt(batchCount),
                estimatedTime: BLOCK_TIME_MS * batchCount,
            };
        }
        // priority === 'speed' or 'auto' for 100+ payments: parallel-batch
        const walletsNeeded = Math.min(Math.ceil(paymentCount / MAX_BATCH_SIZE), this.config.maxWallets);
        const batchSize = MAX_BATCH_SIZE;
        const totalBatches = Math.ceil(paymentCount / batchSize);
        const blocksNeeded = Math.ceil(totalBatches / walletsNeeded);
        return {
            mode: 'parallel-batch',
            walletsNeeded,
            batchSize,
            estimatedFee: FEE_PER_TX * BigInt(totalBatches),
            estimatedTime: BLOCK_TIME_MS * blocksNeeded,
        };
    }
}
exports.ParallelWallet = ParallelWallet;
//# sourceMappingURL=parallel.js.map