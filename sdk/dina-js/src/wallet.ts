import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';
import type { Address, Signature } from './types';
import { addressFromPublicKey, bytesToHex, hexToBytes } from './utils';

// ed25519 v2 requires setting the sha512 hash function
ed.etc.sha512Sync = (...m: Uint8Array[]) => {
  const h = sha512.create();
  for (const msg of m) h.update(msg);
  return h.digest();
};

/**
 * Ed25519 wallet for the Dina Network.
 *
 * Wraps @noble/ed25519 to provide key generation, signing, and verification.
 * The wallet derives a Dina address from the public key via SHA-256.
 */
export class DinaWallet {
  private readonly _privateKey: Uint8Array;
  public readonly publicKey: Uint8Array;
  public readonly address: Address;

  private constructor(privateKey: Uint8Array, publicKey: Uint8Array) {
    this._privateKey = privateKey;
    this.publicKey = publicKey;
    this.address = addressFromPublicKey(publicKey);
  }

  /** Generate a new random wallet. */
  static generate(): DinaWallet {
    const privateKey = ed.utils.randomPrivateKey();
    const publicKey = ed.getPublicKey(privateKey);
    return new DinaWallet(privateKey, publicKey);
  }

  /**
   * Restore a wallet from a private key.
   * @param key - 32-byte private key as Uint8Array or 64-char hex string.
   */
  static fromPrivateKey(key: Uint8Array | string): DinaWallet {
    const privateKey = typeof key === 'string' ? hexToBytes(key) : key;
    if (privateKey.length !== 32) {
      throw new Error(`Private key must be 32 bytes, got ${privateKey.length}`);
    }
    const publicKey = ed.getPublicKey(privateKey);
    return new DinaWallet(privateKey, publicKey);
  }

  /**
   * Restore a wallet from a BIP-39 mnemonic phrase.
   *
   * Derives the private key by taking SHA-256 of the mnemonic bytes.
   * A full BIP-39 + SLIP-0010 derivation path implementation would be used
   * in production; this provides a deterministic key from the phrase.
   */
  static fromMnemonic(mnemonic: string): DinaWallet {
    const trimmed = mnemonic.trim();
    const words = trimmed.split(/\s+/);
    if (words.length < 12 || words.length > 24) {
      throw new Error(
        `Mnemonic must be 12-24 words, got ${words.length}`
      );
    }
    // Derive 32-byte seed from mnemonic via SHA-256
    // In production this would use PBKDF2 per BIP-39 spec
    const { sha256 } = require('@noble/hashes/sha256') as typeof import('@noble/hashes/sha256');
    const seed = sha256(new TextEncoder().encode(trimmed));
    return DinaWallet.fromPrivateKey(seed);
  }

  /** Sign a message, returning a hex-encoded 64-byte Ed25519 signature. */
  sign(message: Uint8Array): Signature {
    const sig = ed.sign(message, this._privateKey);
    return bytesToHex(sig);
  }

  /** Verify a signature against a message using this wallet's public key. */
  verify(message: Uint8Array, signature: Signature): boolean {
    const sigBytes = hexToBytes(signature);
    return ed.verify(sigBytes, message, this.publicKey);
  }

  /** Export public information (never exposes the private key). */
  toJSON(): { address: string; publicKey: string } {
    return {
      address: this.address,
      publicKey: bytesToHex(this.publicKey),
    };
  }

  /** Export the private key as hex. Handle with care. */
  exportPrivateKey(): string {
    return bytesToHex(this._privateKey);
  }
}
