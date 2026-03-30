/**
 * Ed25519 keypair management and transaction signing for Dina Network.
 *
 * Matches the Rust validator's bincode-serialized signing payload format.
 * Uses @noble/ed25519 for cryptographic operations.
 */
import * as ed from '@noble/ed25519';
import { sha256, sha512 } from '@noble/hashes/sha2.js';

/** Ensure sha512 is configured before any ed25519 operation */
export function ensureSha512() {
  if (!ed.hashes.sha512) {
    ed.hashes.sha512 = (...msgs: Uint8Array[]) => {
      const h = sha512.create();
      for (const m of msgs) h.update(m);
      return h.digest();
    };
  }
}

const STORAGE_KEY_PRIVKEY = 'dina_privkey';
const STORAGE_KEY_PUBKEY = 'dina_pubkey';
const STORAGE_KEY_ADDRESS = 'dina_address';

export interface DinaKeypair {
  privateKey: Uint8Array; // 32 bytes
  publicKey: Uint8Array;  // 32 bytes
  address: string;        // 64-char hex (SHA-256 of pubkey)
}

/** Derive a Dina address from an Ed25519 public key (SHA-256 hash). */
export function addressFromPubkey(pubkey: Uint8Array): string {
  const hash = sha256(pubkey);
  return Array.from(hash).map(b => b.toString(16).padStart(2, '0')).join('');
}

/** Load or generate a persistent Ed25519 keypair from localStorage. */
export function getOrCreateKeypair(): DinaKeypair {
  ensureSha512();
  const existingPriv = localStorage.getItem(STORAGE_KEY_PRIVKEY);

  if (existingPriv) {
    const privateKey = hexToBytes(existingPriv);
    const publicKey = ed.getPublicKey(privateKey);
    const address = addressFromPubkey(publicKey);

    // Migrate: if old random address exists, update it
    const storedAddr = localStorage.getItem(STORAGE_KEY_ADDRESS);
    if (storedAddr !== address) {
      localStorage.setItem(STORAGE_KEY_ADDRESS, address);
      localStorage.setItem(STORAGE_KEY_PUBKEY, bytesToHex(publicKey));
    }

    return { privateKey, publicKey, address };
  }

  // Generate new keypair
  const privateKey = ed.utils.randomSecretKey();
  const publicKey = ed.getPublicKey(privateKey);
  const address = addressFromPubkey(publicKey);

  localStorage.setItem(STORAGE_KEY_PRIVKEY, bytesToHex(privateKey));
  localStorage.setItem(STORAGE_KEY_PUBKEY, bytesToHex(publicKey));
  localStorage.setItem(STORAGE_KEY_ADDRESS, address);

  return { privateKey, publicKey, address };
}

/**
 * Build the signing payload for a Transfer transaction.
 *
 * Must exactly match Rust's bincode serialization of TransferPayload:
 *   struct TransferPayload { tag: u8, from: Address, to: Address, amount: u64,
 *     memo: Option<Vec<u8>>, device_witness: Option<WitnessProof>, nonce: u64, fee: u64 }
 *
 * Bincode 1.3 default config: little-endian, fixed-size integers,
 * varint-encoded lengths for Vec, Option as 0/1 byte tag.
 */
export function buildTransferSigningPayload(
  from: Uint8Array, // 32 bytes
  to: Uint8Array,   // 32 bytes
  amount: bigint,
  nonce: bigint,
  fee: bigint,
  memo?: Uint8Array,
): Uint8Array {
  const parts: Uint8Array[] = [];

  // tag: u8 = 0 (Transfer)
  parts.push(new Uint8Array([0]));

  // from: Address ([u8; 32])
  parts.push(from);

  // to: Address ([u8; 32])
  parts.push(to);

  // amount: u64 LE
  parts.push(u64LE(amount));

  // memo: Option<Vec<u8>>
  if (memo && memo.length > 0) {
    parts.push(new Uint8Array([1])); // Some
    parts.push(u64LE(BigInt(memo.length))); // bincode Vec length as u64
    parts.push(memo);
  } else {
    parts.push(new Uint8Array([0])); // None
  }

  // device_witness: Option<WitnessProof> = None
  parts.push(new Uint8Array([0]));

  // nonce: u64 LE
  parts.push(u64LE(nonce));

  // fee: u64 LE
  parts.push(u64LE(fee));

  // Concatenate all parts
  const totalLen = parts.reduce((s, p) => s + p.length, 0);
  const result = new Uint8Array(totalLen);
  let offset = 0;
  for (const part of parts) {
    result.set(part, offset);
    offset += part.length;
  }
  return result;
}

/** Sign a Transfer transaction and return the full JSON-serializable transaction object. */
export function signTransfer(params: {
  keypair: DinaKeypair;
  to: string;       // hex address
  amount: bigint;   // micro-USDC
  nonce: bigint;
  fee: bigint;
  memo?: string;
}): { txJson: object; txHash: string } {
  ensureSha512();
  const { keypair, to, amount, nonce, fee, memo } = params;

  const fromBytes = hexToBytes(keypair.address);
  const toBytes = hexToBytes(to);
  const memoBytes = memo ? new TextEncoder().encode(memo) : undefined;

  const payload = buildTransferSigningPayload(fromBytes, toBytes, amount, nonce, fee, memoBytes);
  const signature = ed.sign(payload, keypair.privateKey);

  // Build the Transaction::Transfer JSON matching Rust's serde format
  const txJson = {
    Transfer: {
      from: Array.from(fromBytes),
      to: Array.from(toBytes),
      amount: Number(amount),
      memo: memoBytes ? Array.from(memoBytes) : null,
      device_witness: null,
      nonce: Number(nonce),
      fee: Number(fee),
      pub_key: Array.from(keypair.publicKey),
      signature: Array.from(signature),
    },
  };

  // Compute tx hash (SHA-256 of bincode of full transaction — approximate with JSON for now)
  const txBytes = new TextEncoder().encode(JSON.stringify(txJson));
  const hash = sha256(txBytes);
  const txHash = bytesToHex(hash);

  return { txJson, txHash };
}

// ── Helpers ──────────────────────────────────────────────────────────

function u64LE(value: bigint): Uint8Array {
  const buf = new Uint8Array(8);
  const view = new DataView(buf.buffer);
  view.setBigUint64(0, value, true); // little-endian
  return buf;
}

export function hexToBytes(hex: string): Uint8Array {
  const clean = hex.startsWith('0x') ? hex.slice(2) : hex;
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(clean.substring(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
}
