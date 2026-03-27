import { sha256 } from '@noble/hashes/sha256';
import type { Address, Hash } from './types';

/**
 * Derive a Dina address from an Ed25519 public key.
 * Address = SHA-256(pubkey), hex-encoded (32 bytes / 64 hex chars).
 */
export function addressFromPublicKey(pubkey: Uint8Array): Address {
  const hash = sha256(pubkey);
  return bytesToHex(hash);
}

/**
 * Format micro-USDC to a human-readable string.
 * 1 USDC = 1_000_000 micro-units.
 * @example formatUSDC(100_500_000n) => "100.500000"
 */
export function formatUSDC(microUsdc: bigint): string {
  const negative = microUsdc < 0n;
  const abs = negative ? -microUsdc : microUsdc;
  const whole = abs / 1_000_000n;
  const frac = abs % 1_000_000n;
  const fracStr = frac.toString().padStart(6, '0');
  // Trim trailing zeros but keep at least 2 decimal places
  let trimmed = fracStr.replace(/0+$/, '');
  if (trimmed.length < 2) trimmed = fracStr.slice(0, 2);
  return `${negative ? '-' : ''}${whole}.${trimmed}`;
}

/**
 * Parse a human-readable USDC string to micro-units.
 * @example parseUSDC("100.50") => 100_500_000n
 */
export function parseUSDC(usdc: string): bigint {
  const trimmed = usdc.trim();
  if (!/^-?\d+(\.\d+)?$/.test(trimmed)) {
    throw new Error(`Invalid USDC amount: "${usdc}"`);
  }

  const negative = trimmed.startsWith('-');
  const abs = negative ? trimmed.slice(1) : trimmed;
  const parts = abs.split('.');
  const whole = BigInt(parts[0]) * 1_000_000n;

  let frac = 0n;
  if (parts[1]) {
    const fracStr = parts[1].slice(0, 6).padEnd(6, '0');
    frac = BigInt(fracStr);
  }

  const result = whole + frac;
  return negative ? -result : result;
}

/** Convert a hex string to Uint8Array. Accepts optional 0x prefix. */
export function hexToBytes(hex: string): Uint8Array {
  const clean = hex.startsWith('0x') ? hex.slice(2) : hex;
  if (clean.length % 2 !== 0) {
    throw new Error(`Invalid hex string length: ${clean.length}`);
  }
  if (!/^[0-9a-fA-F]*$/.test(clean)) {
    throw new Error('Invalid hex characters');
  }
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < clean.length; i += 2) {
    bytes[i / 2] = parseInt(clean.slice(i, i + 2), 16);
  }
  return bytes;
}

/** Convert Uint8Array to lowercase hex string (no 0x prefix). */
export function bytesToHex(bytes: Uint8Array): string {
  let hex = '';
  for (let i = 0; i < bytes.length; i++) {
    hex += bytes[i].toString(16).padStart(2, '0');
  }
  return hex;
}

/** Validate a Dina address (64 hex characters = 32 bytes). */
export function isValidAddress(address: string): boolean {
  return typeof address === 'string' &&
    address.length === 64 &&
    /^[0-9a-fA-F]{64}$/.test(address);
}

/** Validate a hash (64 hex characters = 32 bytes). */
export function isValidHash(hash: string): boolean {
  return typeof hash === 'string' &&
    hash.length === 64 &&
    /^[0-9a-fA-F]{64}$/.test(hash);
}

/** Concatenate multiple Uint8Arrays. */
export function concatBytes(...arrays: Uint8Array[]): Uint8Array {
  let totalLen = 0;
  for (const arr of arrays) totalLen += arr.length;
  const result = new Uint8Array(totalLen);
  let offset = 0;
  for (const arr of arrays) {
    result.set(arr, offset);
    offset += arr.length;
  }
  return result;
}

/** Encode a number as a little-endian Uint8Array (8 bytes). */
export function encodeBigintLE(value: bigint): Uint8Array {
  const buf = new Uint8Array(8);
  let v = value < 0n ? -value : value;
  for (let i = 0; i < 8; i++) {
    buf[i] = Number(v & 0xffn);
    v >>= 8n;
  }
  return buf;
}

/** Encode a UTF-8 string to Uint8Array. */
export function encodeString(str: string): Uint8Array {
  return new TextEncoder().encode(str);
}
