import * as ed from '@noble/ed25519';
import { sha256 } from '@noble/hashes/sha256';
import type { Address, Hash, SignedState } from './types';
import { DinaClient } from './client';
import { DinaWallet } from './wallet';
import {
  bytesToHex,
  concatBytes,
  encodeBigintLE,
  encodeString,
  hexToBytes,
} from './utils';

interface ChannelState {
  channelId: string;
  counterparty: Address;
  counterpartyPubKey: Uint8Array;
  nonce: number;
  myBalance: bigint;
  theirBalance: bigint;
}

/**
 * Payment channel client for off-chain USDC micro-payments.
 *
 * Channels allow two parties to exchange payments instantly without
 * on-chain transactions, then settle the final balances in one tx.
 * Useful for robotics pay-per-use, streaming payments, and IoT metering.
 */
export class PaymentChannel {
  private readonly wallet: DinaWallet;
  private readonly client: DinaClient;
  private channels = new Map<string, ChannelState>();

  constructor(wallet: DinaWallet, client: DinaClient) {
    this.wallet = wallet;
    this.client = client;
  }

  /**
   * Open a new payment channel with a counterparty.
   * Locks `depositAmount` USDC on-chain into the channel contract.
   * @param counterparty - The counterparty's address.
   * @param depositAmount - Amount of USDC to lock in the channel.
   * @param counterpartyPubKey - The counterparty's Ed25519 public key for signature verification.
   * @returns The channel ID.
   */
  async open(counterparty: Address, depositAmount: bigint, counterpartyPubKey: Uint8Array): Promise<string> {
    const txHash = await this.client.callContract(this.wallet, {
      contract: counterparty, // The channel factory contract
      method: 'open_channel',
      args: {
        counterparty,
        deposit: depositAmount.toString(),
      },
      usdcAttached: depositAmount,
    });

    // Derive channel ID from tx hash (deterministic)
    const channelId = bytesToHex(
      sha256(
        concatBytes(
          hexToBytes(txHash),
          hexToBytes(this.wallet.address),
          hexToBytes(counterparty)
        )
      )
    );

    this.channels.set(channelId, {
      channelId,
      counterparty,
      counterpartyPubKey,
      nonce: 0,
      myBalance: depositAmount,
      theirBalance: 0n,
    });

    return channelId;
  }

  /**
   * Make an off-chain payment within a channel.
   * Updates local state and returns a signed state that the counterparty
   * can verify and hold as proof.
   */
  async pay(channelId: string, amount: bigint): Promise<SignedState> {
    const channel = this.getChannel(channelId);

    if (amount <= 0n) {
      throw new Error('Payment amount must be positive');
    }
    if (amount > channel.myBalance) {
      throw new Error(
        `Insufficient channel balance: have ${channel.myBalance}, need ${amount}`
      );
    }

    channel.nonce += 1;
    channel.myBalance -= amount;
    channel.theirBalance += amount;

    const stateHash = this.hashState(channel);
    const signature = this.wallet.sign(stateHash);

    return {
      channelId,
      nonce: channel.nonce,
      balanceA: channel.myBalance,
      balanceB: channel.theirBalance,
      signature,
    };
  }

  /**
   * Receive and validate an incoming payment state from the counterparty.
   * Verifies the signature and updates local state if valid.
   */
  async receivePayment(signedState: SignedState): Promise<void> {
    const channel = this.channels.get(signedState.channelId);
    if (!channel) {
      throw new Error(`Unknown channel: ${signedState.channelId}`);
    }

    if (signedState.nonce <= channel.nonce) {
      throw new Error(
        `Stale state: received nonce ${signedState.nonce}, current ${channel.nonce}`
      );
    }

    // Verify total balance conservation
    const totalBefore = channel.myBalance + channel.theirBalance;
    const totalAfter = signedState.balanceA + signedState.balanceB;
    if (totalAfter !== totalBefore) {
      throw new Error(
        'Invalid state: total channel balance changed'
      );
    }

    // Verify Ed25519 signature from counterparty over the signed state
    const stateHash = sha256(
      concatBytes(
        hexToBytes(signedState.channelId),
        encodeBigintLE(BigInt(signedState.nonce)),
        encodeBigintLE(signedState.balanceA),
        encodeBigintLE(signedState.balanceB)
      )
    );
    const sigBytes = hexToBytes(signedState.signature);
    const validSig = ed.verify(sigBytes, stateHash, channel.counterpartyPubKey);
    if (!validSig) {
      throw new Error(
        'Invalid signature: counterparty signature verification failed'
      );
    }

    // Accept the new state — from counterparty's perspective,
    // balanceA is theirs and balanceB is ours.
    channel.nonce = signedState.nonce;
    channel.theirBalance = signedState.balanceA;
    channel.myBalance = signedState.balanceB;
  }

  /**
   * Close a channel and settle final balances on-chain.
   * Returns the settlement transaction hash.
   */
  async close(channelId: string): Promise<Hash> {
    const channel = this.getChannel(channelId);
    const stateHash = this.hashState(channel);
    const signature = this.wallet.sign(stateHash);

    const txHash = await this.client.callContract(this.wallet, {
      contract: channel.counterparty,
      method: 'close_channel',
      args: {
        channelId,
        nonce: channel.nonce,
        balanceA: channel.myBalance.toString(),
        balanceB: channel.theirBalance.toString(),
        signature,
      },
    });

    this.channels.delete(channelId);
    return txHash;
  }

  /** Get the current balance split for a channel. */
  getBalance(channelId: string): { mine: bigint; theirs: bigint } {
    const channel = this.getChannel(channelId);
    return {
      mine: channel.myBalance,
      theirs: channel.theirBalance,
    };
  }

  /**
   * Serialize the latest channel state into a compact binary blob
   * suitable for transmission over BLE, QR code, or offline relay.
   *
   * Format: channelId (32B) | nonce (8B) | balanceA (8B) | balanceB (8B) | sig (64B)
   * Total: 120 bytes
   */
  toRelayBlob(channelId: string): Uint8Array {
    const channel = this.getChannel(channelId);
    const stateHash = this.hashState(channel);
    const signature = this.wallet.sign(stateHash);

    return concatBytes(
      hexToBytes(channelId),
      encodeBigintLE(BigInt(channel.nonce)),
      encodeBigintLE(channel.myBalance),
      encodeBigintLE(channel.theirBalance),
      hexToBytes(signature)
    );
  }

  // ---------------------------------------------------------------------------
  // Internal helpers
  // ---------------------------------------------------------------------------

  private getChannel(channelId: string): ChannelState {
    const channel = this.channels.get(channelId);
    if (!channel) {
      throw new Error(`Channel not found: ${channelId}`);
    }
    return channel;
  }

  private hashState(channel: ChannelState): Uint8Array {
    return sha256(
      concatBytes(
        hexToBytes(channel.channelId),
        encodeBigintLE(BigInt(channel.nonce)),
        encodeBigintLE(channel.myBalance),
        encodeBigintLE(channel.theirBalance)
      )
    );
  }
}
