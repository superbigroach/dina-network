import * as ed from "@noble/ed25519";
import { sha512 } from "@noble/hashes/sha512";
import { sha256 } from "@noble/hashes/sha256";

// ed25519 requires a sha512 implementation
ed.etc.sha512Sync = (...m) => sha512(ed.etc.concatBytes(...m));

// --- Hex helpers ---

export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

export function hexToBytes(hex: string): Uint8Array {
  const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(clean.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

// --- Wallet types ---

export interface Wallet {
  address: string;
  publicKey: string;
  privateKey: string;
}

export interface AgentWalletConfig {
  wallet: Wallet;
  ownerAddress: string;
  dailyLimitUsdc: number;
  perTxLimitUsdc: number;
}

export interface SwarmResult {
  authority: Wallet;
  agents: Wallet[];
}

// --- Core functions ---

/** Derive a 0x-prefixed address from a public key (SHA-256 of the raw bytes, take first 20 bytes). */
export function addressFromPublicKey(pubkeyBytes: Uint8Array): string {
  const hash = sha256(pubkeyBytes);
  // Take first 20 bytes (40 hex chars) to form the address, Ethereum-style
  return "0x" + bytesToHex(hash.slice(0, 20));
}

/** Generate a single Ed25519 wallet. */
export function generateWallet(): Wallet {
  const privateKeyBytes = ed.utils.randomPrivateKey();
  const publicKeyBytes = ed.getPublicKey(privateKeyBytes);
  const address = addressFromPublicKey(publicKeyBytes);

  return {
    address,
    publicKey: bytesToHex(publicKeyBytes),
    privateKey: bytesToHex(privateKeyBytes),
  };
}

/** Generate an agent wallet with spending constraints. */
export function generateAgentWallet(
  ownerAddress: string,
  dailyLimitUsdc: number,
  perTxLimitUsdc: number
): AgentWalletConfig {
  return {
    wallet: generateWallet(),
    ownerAddress,
    dailyLimitUsdc,
    perTxLimitUsdc,
  };
}

/** Generate a swarm of N agent wallets under a single authority. */
export function generateSwarm(count: number): SwarmResult {
  const authority = generateWallet();
  const agents: Wallet[] = [];
  for (let i = 0; i < count; i++) {
    agents.push(generateWallet());
  }
  return { authority, agents };
}
