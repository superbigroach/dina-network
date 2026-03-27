import type { Address, Signature } from './types';
/**
 * Ed25519 wallet for the Dina Network.
 *
 * Wraps @noble/ed25519 to provide key generation, signing, and verification.
 * The wallet derives a Dina address from the public key via SHA-256.
 */
export declare class DinaWallet {
    private readonly _privateKey;
    readonly publicKey: Uint8Array;
    readonly address: Address;
    private constructor();
    /** Generate a new random wallet. */
    static generate(): DinaWallet;
    /**
     * Restore a wallet from a private key.
     * @param key - 32-byte private key as Uint8Array or 64-char hex string.
     */
    static fromPrivateKey(key: Uint8Array | string): DinaWallet;
    /**
     * Restore a wallet from a BIP-39 mnemonic phrase.
     *
     * Derives the private key by taking SHA-256 of the mnemonic bytes.
     * A full BIP-39 + SLIP-0010 derivation path implementation would be used
     * in production; this provides a deterministic key from the phrase.
     */
    static fromMnemonic(mnemonic: string): DinaWallet;
    /** Sign a message, returning a hex-encoded 64-byte Ed25519 signature. */
    sign(message: Uint8Array): Signature;
    /** Verify a signature against a message using this wallet's public key. */
    verify(message: Uint8Array, signature: Signature): boolean;
    /** Export public information (never exposes the private key). */
    toJSON(): {
        address: string;
        publicKey: string;
    };
    /** Export the private key as hex. Handle with care. */
    exportPrivateKey(): string;
}
//# sourceMappingURL=wallet.d.ts.map