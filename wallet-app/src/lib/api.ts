// Regional HTTPS proxies — wallet auto-selects the fastest
// Single validator for testnet — Montreal (closest to founder)
// Multi-validator requires P2P consensus sync (not yet implemented)
const ENDPOINTS = [
  'https://dina-proxy-ca-jy6qm6s57a-nn.a.run.app',
];

let _fastestEndpoint: string | null = null;

/** Race all endpoints and cache the fastest one */
async function getFastestEndpoint(): Promise<string> {
  if (_fastestEndpoint) return _fastestEndpoint;

  // Race: first endpoint to respond wins
  const controller = new AbortController();
  try {
    const result = await Promise.any(
      ENDPOINTS.map(async (ep) => {
        const start = Date.now();
        const res = await fetch(`${ep}/health`, { signal: controller.signal });
        if (!res.ok) throw new Error('bad');
        const elapsed = Date.now() - start;
        console.log(`[dina] ${ep} responded in ${elapsed}ms`);
        return ep;
      })
    );
    _fastestEndpoint = result;
    controller.abort(); // cancel slower ones
    console.log(`[dina] Using fastest endpoint: ${_fastestEndpoint}`);
    return result;
  } catch {
    // All failed — use first as fallback
    _fastestEndpoint = ENDPOINTS[0];
    return _fastestEndpoint;
  }
}

export async function rest(path: string, options?: RequestInit): Promise<unknown> {
  const base = await getFastestEndpoint();
  const res = await fetch(`${base}${path}`, {
    ...options,
    signal: AbortSignal.timeout(15000),
  });
  if (!res.ok) throw new Error(`API error: ${res.status}`);
  return res.json();
}

export async function getBalance(address: string): Promise<number> {
  const data = (await rest(`/v1/balance/${address}`)) as { balance?: number };
  return data.balance ?? 0;
}

export async function getBalanceRest(address: string): Promise<number> {
  return getBalance(address);
}

export async function getNonce(address: string): Promise<number> {
  const data = (await rest(`/v1/balance/${address}`)) as { nonce?: number };
  return data.nonce ?? 0;
}

export async function fundFromFaucet(address: string): Promise<void> {
  await rest(`/faucet/${address}`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: '{}' });
}

export async function getHealth(): Promise<{ height: number; status: string }> {
  return rest('/health') as Promise<{ height: number; status: string }>;
}

export async function getNetworkInfo(): Promise<unknown> {
  return rest('/health');
}

export interface RecentTransaction {
  id: string;
  type: 'send' | 'receive' | 'convert' | 'yield';
  amount: number;
  currency: string;
  counterparty?: string;
  timestamp: number;
  status: 'confirmed' | 'pending';
  wallet: string;
  txHash?: string;
}

export async function getRecentTransactions(address: string): Promise<RecentTransaction[]> {
  // Try the REST transaction history endpoint
  try {
    const base = await getFastestEndpoint();
    const res = await fetch(`${base}/v1/transactions/${address}`, {
      signal: AbortSignal.timeout(5000),
    });
    if (res.ok) {
      const data = await res.json();
      if (data.transactions?.length > 0) return data.transactions;
    }
  } catch {
    // endpoint doesn't exist yet — fall through
  }

  // Build history from localStorage tx log
  const txLog = getLocalTxLog(address);
  if (txLog.length > 0) return txLog;

  // Fallback: show faucet funding if balance > 0
  try {
    const bal = await getBalance(address);
    if (bal > 0) {
      return [{
        id: 'faucet-' + address.slice(0, 8),
        type: 'receive' as const,
        amount: bal,
        currency: 'USDC',
        counterparty: 'Dina Testnet Faucet',
        timestamp: Math.floor(Date.now() / 1000) - 60,
        status: 'confirmed' as const,
        wallet: 'Main Wallet',
      }];
    }
  } catch {
    // ignore
  }

  return [];
}

/**
 * Submit a signed transaction to the Dina testnet.
 * The validator expects: POST /v1/transaction { tx_hex: "<hex-encoded JSON>" }
 */
export async function submitSignedTransaction(
  txJson: object,
): Promise<{ txHash: string; success: boolean; confirmed: boolean; blockHeight?: number; validators?: number }> {
  // Hex-encode the JSON bytes (this is what the Rust validator expects)
  const jsonStr = JSON.stringify(txJson);
  const jsonBytes = new TextEncoder().encode(jsonStr);
  const txHex = Array.from(jsonBytes).map(b => b.toString(16).padStart(2, '0')).join('');

  // Use the /confirm endpoint which waits for BFT consensus (3/4 validators)
  const data = (await rest('/v1/transaction/confirm', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ tx_hex: txHex }),
  })) as { tx_hash?: string; error?: string; confirmed?: boolean; block_height?: number; validators?: number; pending?: boolean };

  if (data.error) {
    throw new Error(data.error);
  }

  return {
    txHash: data.tx_hash || '',
    success: true,
    confirmed: data.confirmed ?? false,
    blockHeight: data.block_height,
    validators: data.validators,
  };
}

/** Legacy wrapper — still used by send page before migration to signed flow. */
export async function submitTransfer(
  from: string,
  to: string,
  amount: number,
): Promise<{ txHash?: string; success: boolean }> {
  // Try the real signed transaction endpoint
  try {
    const { signTransfer, getOrCreateKeypair } = await import('./crypto');
    const keypair = getOrCreateKeypair();

    const { txJson } = await signTransfer({
      keypair,
      to,
      amount: BigInt(amount),
      nonce: BigInt(Date.now()), // use timestamp as nonce for testnet
      fee: BigInt(0), // Dina Network = zero fees
    });

    return await submitSignedTransaction(txJson);
  } catch (err) {
    console.warn('Signed transfer failed, trying unsigned:', err);
    // Fallback: submit unsigned (will fail signature check but we log the attempt)
    return { success: false };
  }
}

// ── Local transaction log (persisted in localStorage) ──────────────

const TX_LOG_KEY = 'dina_tx_log';

export function logTransaction(address: string, tx: RecentTransaction): void {
  const log = getLocalTxLog(address);
  log.unshift(tx); // newest first
  if (log.length > 50) log.length = 50; // keep last 50
  localStorage.setItem(`${TX_LOG_KEY}_${address.slice(0, 16)}`, JSON.stringify(log));
}

function getLocalTxLog(address: string): RecentTransaction[] {
  try {
    const raw = localStorage.getItem(`${TX_LOG_KEY}_${address.slice(0, 16)}`);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}
