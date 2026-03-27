import type { Address } from './types';
/**
 * Derive a Dina address from an Ed25519 public key.
 * Address = SHA-256(pubkey), hex-encoded (32 bytes / 64 hex chars).
 */
export declare function addressFromPublicKey(pubkey: Uint8Array): Address;
/**
 * Format micro-USDC to a human-readable string.
 * 1 USDC = 1_000_000 micro-units.
 * @example formatUSDC(100_500_000n) => "100.500000"
 */
export declare function formatUSDC(microUsdc: bigint): string;
/**
 * Parse a human-readable USDC string to micro-units.
 * @example parseUSDC("100.50") => 100_500_000n
 */
export declare function parseUSDC(usdc: string): bigint;
/** Convert a hex string to Uint8Array. Accepts optional 0x prefix. */
export declare function hexToBytes(hex: string): Uint8Array;
/** Convert Uint8Array to lowercase hex string (no 0x prefix). */
export declare function bytesToHex(bytes: Uint8Array): string;
/** Validate a Dina address (64 hex characters = 32 bytes). */
export declare function isValidAddress(address: string): boolean;
/** Validate a hash (64 hex characters = 32 bytes). */
export declare function isValidHash(hash: string): boolean;
/** Concatenate multiple Uint8Arrays. */
export declare function concatBytes(...arrays: Uint8Array[]): Uint8Array;
/** Encode a number as a little-endian Uint8Array (8 bytes). */
export declare function encodeBigintLE(value: bigint): Uint8Array;
/** Encode a UTF-8 string to Uint8Array. */
export declare function encodeString(str: string): Uint8Array;
//# sourceMappingURL=utils.d.ts.map