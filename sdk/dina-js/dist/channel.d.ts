import type { Address, Hash, SignedState } from './types';
import { DinaClient } from './client';
import { DinaWallet } from './wallet';
/**
 * Payment channel client for off-chain USDC micro-payments.
 *
 * Channels allow two parties to exchange payments instantly without
 * on-chain transactions, then settle the final balances in one tx.
 * Useful for robotics pay-per-use, streaming payments, and IoT metering.
 */
export declare class PaymentChannel {
    private readonly wallet;
    private readonly client;
    private channels;
    constructor(wallet: DinaWallet, client: DinaClient);
    /**
     * Open a new payment channel with a counterparty.
     * Locks `depositAmount` USDC on-chain into the channel contract.
     * @returns The channel ID.
     */
    open(counterparty: Address, depositAmount: bigint): Promise<string>;
    /**
     * Make an off-chain payment within a channel.
     * Updates local state and returns a signed state that the counterparty
     * can verify and hold as proof.
     */
    pay(channelId: string, amount: bigint): Promise<SignedState>;
    /**
     * Receive and validate an incoming payment state from the counterparty.
     * Verifies the signature and updates local state if valid.
     */
    receivePayment(signedState: SignedState): Promise<void>;
    /**
     * Close a channel and settle final balances on-chain.
     * Returns the settlement transaction hash.
     */
    close(channelId: string): Promise<Hash>;
    /** Get the current balance split for a channel. */
    getBalance(channelId: string): {
        mine: bigint;
        theirs: bigint;
    };
    /**
     * Serialize the latest channel state into a compact binary blob
     * suitable for transmission over BLE, QR code, or offline relay.
     *
     * Format: channelId (32B) | nonce (8B) | balanceA (8B) | balanceB (8B) | sig (64B)
     * Total: 120 bytes
     */
    toRelayBlob(channelId: string): Uint8Array;
    private getChannel;
    private hashState;
}
//# sourceMappingURL=channel.d.ts.map