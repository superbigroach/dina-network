import { DinaWallet } from './wallet';
import { DinaClient } from './client';
import type { Address, Hash, TransferParams } from './types';
export interface ParallelConfig {
    maxWallets?: number;
    autoScale?: boolean;
    minBalancePerWallet?: bigint;
}
export interface ParallelStats {
    activeWallets: number;
    totalBalance: bigint;
    avgBalance: bigint;
    totalTransactions: number;
}
export type TransferPriority = 'cost' | 'speed' | 'auto';
export interface BatchOptions {
    parallel?: boolean;
    priority?: TransferPriority;
    maxFee?: bigint;
    deadlineMs?: number;
}
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
export declare class ParallelWallet {
    private masterWallet;
    private client;
    private config;
    private subWallets;
    constructor(masterWallet: DinaWallet, client: DinaClient, config?: ParallelConfig);
    /** Solo mode -- single wallet, no parallelism. */
    static solo(wallet: DinaWallet, client: DinaClient): ParallelWallet;
    /** Standard -- up to 10 sub-wallets with auto-scaling. */
    static standard(wallet: DinaWallet, client: DinaClient): ParallelWallet;
    /** Pro -- up to 100 sub-wallets. */
    static pro(wallet: DinaWallet, client: DinaClient): ParallelWallet;
    /** Enterprise -- up to 10,000 sub-wallets for maximum throughput. */
    static enterprise(wallet: DinaWallet, client: DinaClient): ParallelWallet;
    /**
     * Send a single USDC transfer.
     *
     * Uses the master wallet directly for a single transfer.
     */
    transfer(to: Address, amount: bigint): Promise<Hash>;
    /**
     * Send multiple payments, automatically selecting the optimal strategy.
     *
     * Depending on the payment count and options, this will use:
     *   - single mode for 1 payment
     *   - batch mode for 2-100 payments when optimizing for cost
     *   - parallel mode for 2-100 payments when optimizing for speed
     *   - parallel-batch mode for 100+ payments
     */
    batchTransfer(payments: TransferParams[], options?: BatchOptions): Promise<Hash[]>;
    /**
     * Create N new sub-wallets by generating Ed25519 keypairs.
     * Returns the addresses of the new wallets.
     */
    createWallets(count: number): Promise<Address[]>;
    /**
     * Fund all sub-wallets by distributing totalAmount evenly from the master
     * wallet. Sends one transfer per sub-wallet.
     */
    fundAll(totalAmount: bigint): Promise<Hash>;
    /**
     * Consolidate: transfer all balances from sub-wallets back to the master
     * wallet. Returns one transaction hash per sub-wallet that had a balance.
     */
    consolidate(): Promise<Hash[]>;
    /**
     * Return stats about the parallel wallet system.
     */
    stats(): Promise<ParallelStats>;
    /**
     * Ensure at least `count` sub-wallets exist. Creates new ones if needed
     * (when autoScale is enabled).
     */
    private ensureWallets;
    /**
     * Distribute an amount evenly across all sub-wallets via the master wallet.
     */
    private distribute;
    /** Sequential single transfers from the master wallet. */
    private executeSingle;
    /**
     * Batch mode: split payments into chunks of batchSize and send each chunk
     * as a batch transaction from the master wallet.
     *
     * In a real DRC-19 batch transfer, a single transaction can pay up to 100
     * recipients. Here we simulate by sending individual transfers per chunk
     * since the DinaClient.transfer API handles single recipients. A production
     * implementation would call a `batchTransfer` RPC method.
     */
    private executeBatch;
    /**
     * Parallel mode: assign each payment to a different sub-wallet and send
     * all transfers concurrently (each sub-wallet has its own nonce).
     */
    private executeParallel;
    /**
     * Parallel-batch mode: distribute payments across sub-wallets in chunks.
     * Each sub-wallet sends one batch transaction containing batchSize payments.
     */
    private executeParallelBatch;
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
    private optimizeStrategy;
}
//# sourceMappingURL=parallel.d.ts.map