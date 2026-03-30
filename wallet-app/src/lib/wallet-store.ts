/**
 * Persistent wallet store — each of 9 wallets gets its own testnet address + balance.
 * Stored in localStorage so it survives page reloads.
 */
import { getBalanceRest, fundFromFaucet } from './api';

export interface StoredWallet {
  id: string;
  name: string;
  type: 'main' | 'savings' | 'backup' | 'agent' | 'speed';
  icon: string;
  address: string;
  privateKey: string; // hex-encoded Ed25519 private key
  balance: number;
  isSetUp: boolean;
}

const STORE_KEY = 'dina_wallets';

const WALLET_DEFS = [
  { id: 'smart1', name: 'Smart 1', type: 'main' as const, icon: '🏦' },
  { id: 'smart2', name: 'Smart 2', type: 'savings' as const, icon: '🏦' },
  { id: 'smart3', name: 'Smart 3', type: 'backup' as const, icon: '🏦' },
  { id: 'agent1', name: 'Agent 1', type: 'agent' as const, icon: '🤖' },
  { id: 'agent2', name: 'Agent 2', type: 'agent' as const, icon: '🤖' },
  { id: 'agent3', name: 'Agent 3', type: 'agent' as const, icon: '🤖' },
  { id: 'parallel1', name: 'Parallel 1', type: 'speed' as const, icon: '⚡' },
  { id: 'parallel2', name: 'Parallel 2', type: 'speed' as const, icon: '⚡' },
  { id: 'parallel3', name: 'Parallel 3', type: 'speed' as const, icon: '⚡' },
];

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
}

async function generateKeypair(): Promise<{ address: string; privateKey: string }> {
  // Generate a real Ed25519 keypair — address = SHA-256(pubkey)
  const { ensureSha512, addressFromPubkey, bytesToHex } = await import('./crypto');
  const ed = await import('@noble/ed25519');
  ensureSha512();
  const privKey = ed.utils.randomSecretKey();
  const pubKey = ed.getPublicKey(privKey);
  const address = addressFromPubkey(pubKey);
  console.log('[dina] Generated Ed25519 keypair:', address.slice(0, 12) + '...', 'key length:', bytesToHex(privKey).length);
  return { address, privateKey: bytesToHex(privKey) };
}

/** Load wallets from localStorage, initializing if needed. */
export function loadWallets(): StoredWallet[] {
  try {
    const raw = localStorage.getItem(STORE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as StoredWallet[];
      // Accept if 9 wallets and IDs match (migrate missing fields gracefully)
      if (parsed.length === 9 && parsed[0]?.id === WALLET_DEFS[0].id) {
        // Ensure privateKey field exists on all wallets (migration)
        for (const w of parsed) {
          if (w.privateKey === undefined) w.privateKey = '';
        }
        saveWallets(parsed);
        return parsed;
      }
    }
  } catch { /* corrupt data, reinitialize */ }

  // Initialize: generate a proper keypair for main wallet if none exists
  let mainAddr = localStorage.getItem('dina_address') || '';
  let mainPrivKey = localStorage.getItem('dina_privkey') || '';

  // If no valid keypair, generate random bytes as placeholder
  // The async initMainKeypair() below will fix it properly
  if (!mainAddr) {
    mainAddr = toHex(crypto.getRandomValues(new Uint8Array(32)));
  }
  localStorage.setItem('dina_address', mainAddr);

  const wallets: StoredWallet[] = WALLET_DEFS.map((def, i) => ({
    ...def,
    address: i === 0 ? mainAddr : '',
    privateKey: i === 0 ? mainPrivKey : '',
    balance: 0,
    isSetUp: i === 0,
  }));

  saveWallets(wallets);
  return wallets;
}

/** Ensure all set-up wallets have valid Ed25519 keypairs. Generates keys for any wallet missing one. */
export async function ensureKeypairs(): Promise<StoredWallet[]> {
  const wallets = loadWallets();
  let changed = false;

  for (const w of wallets) {
    if (w.isSetUp && (!w.privateKey || w.privateKey.length < 64)) {
      // Wallet was set up but has no valid private key — generate one.
      // This replaces the address since the old one was random bytes (not derived from a real key).
      const kp = await generateKeypair();
      w.address = kp.address;
      w.privateKey = kp.privateKey;
      // Also update legacy localStorage for main wallet
      if (w.id === 'smart1') {
        localStorage.setItem('dina_address', kp.address);
        localStorage.setItem('dina_privkey', kp.privateKey);
      }
      changed = true;
      console.log(`[dina] Generated missing keypair for wallet ${w.id}: ${kp.address.slice(0, 12)}...`);
    }
  }

  if (changed) {
    saveWallets(wallets);
  }
  return wallets;
}

/** Save wallets to localStorage. */
export function saveWallets(wallets: StoredWallet[]): void {
  localStorage.setItem(STORE_KEY, JSON.stringify(wallets));
}

/** Set up a wallet — generate address and fund from faucet. */
export async function setupWallet(walletId: string): Promise<StoredWallet[]> {
  const wallets = loadWallets();
  const idx = wallets.findIndex(w => w.id === walletId);
  if (idx < 0) return wallets;

  const wallet = wallets[idx];
  if (wallet.isSetUp) return wallets;

  // Generate a real Ed25519 keypair
  const kp = await generateKeypair();
  wallet.address = kp.address;
  wallet.privateKey = kp.privateKey;
  wallet.isSetUp = true;

  // Save — wallet starts at $0, user funds manually via faucet or send
  wallet.balance = 0;
  wallets[idx] = wallet;
  saveWallets(wallets);
  return wallets;
}

/** Refresh balances for all set-up wallets. */
export async function refreshAllBalances(wallets: StoredWallet[]): Promise<StoredWallet[]> {
  const updated = [...wallets];
  for (const w of updated) {
    if (w.isSetUp && w.address) {
      try {
        const bal = await getBalanceRest(w.address);
        w.balance = bal || 0;
      } catch {
        // keep existing balance
      }
    }
  }
  saveWallets(updated);
  return updated;
}

/** Get total balance across all wallets. */
export function totalBalance(wallets: StoredWallet[]): number {
  return wallets.reduce((sum, w) => sum + w.balance, 0);
}

/** Get the number of funded wallets. */
export function fundedCount(wallets: StoredWallet[]): number {
  return wallets.filter(w => w.isSetUp).length;
}
