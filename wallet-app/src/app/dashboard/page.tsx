'use client';
import { useEffect, useState, useCallback } from 'react';
import { Navbar } from '@/components/Navbar';
import { BalanceStream } from '@/components/BalanceStream';
// WalletCard replaced with inline wallet grid
import { YieldDisplay } from '@/components/YieldDisplay';
import { TransactionList } from '@/components/TransactionList';
import { CurrencyList } from '@/components/CurrencyList';
import { CHAIN_ID } from '@/lib/constants';
import { getHealth, getBalanceRest, fundFromFaucet, getRecentTransactions } from '@/lib/api';
import { loadWallets, saveWallets, setupWallet, refreshAllBalances, totalBalance, fundedCount, ensureKeypairs, type StoredWallet } from '@/lib/wallet-store';
import { Wallet, Transaction, Currency } from '@/lib/types';
import Link from 'next/link';

interface NetworkStatus {
  connected: boolean;
  height: number;
  status: string;
}

export default function DashboardPage() {
  // Testnet wallet state
  const [wallets, setWallets] = useState<StoredWallet[]>([]);
  const [dinaAddress, setDinaAddress] = useState<string | null>(null);
  const [realBalance, setRealBalance] = useState<number | null>(null);
  const [settingUp, setSettingUp] = useState<string | null>(null);
  const [balanceLastUpdate, setBalanceLastUpdate] = useState<number>(() => {
    if (typeof window !== 'undefined') {
      const stored = localStorage.getItem('dina_balance_ts');
      if (stored) return parseInt(stored, 10);
    }
    return Math.floor(Date.now() / 1000);
  });
  // Persist balance timestamp so yield counter survives page reload
  const updateBalanceTimestamp = useCallback((ts: number) => {
    setBalanceLastUpdate(ts);
    localStorage.setItem('dina_balance_ts', String(ts));
  }, []);
  const [funding, setFunding] = useState(false);
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [fundInfoWallet, setFundInfoWallet] = useState<string | null>(null);
  const [faucetResult, setFaucetResult] = useState<{time: number; address: string; balance: number} | null>(null);

  // No auth on testnet

  const [network, setNetwork] = useState<NetworkStatus>({
    connected: false,
    height: 0,
    status: 'connecting',
  });

  // Fetch network status
  useEffect(() => {
    let cancelled = false;

    async function fetchStatus() {
      try {
        const health = await getHealth();
        if (!cancelled) {
          setNetwork({
            connected: true,
            height: health.height,
            status: health.status || 'ok',
          });
        }
      } catch {
        if (!cancelled) {
          setNetwork({ connected: false, height: 0, status: 'unreachable' });
        }
      }
    }

    fetchStatus();
    const interval = setInterval(fetchStatus, 15_000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  // Initialize wallets from store and refresh balances
  useEffect(() => {
    let cancelled = false;

    async function initWallet() {
      // Ensure all set-up wallets have valid Ed25519 keypairs
      const stored = await ensureKeypairs();
      if (cancelled) return;
      setWallets(stored);

      const mainWallet = stored[0];
      setDinaAddress(mainWallet.address);

      // Refresh all balances from chain
      try {
        const updated = await refreshAllBalances(stored);
        if (!cancelled) {
          setWallets(updated);
          const total = totalBalance(updated);
          setRealBalance(total);
          if (!localStorage.getItem('dina_balance_ts')) {
            updateBalanceTimestamp(Math.floor(Date.now() / 1000));
          }
        }
      } catch {
        if (!cancelled) {
          setRealBalance(totalBalance(stored));
        }
      }
    }

    initWallet();
    return () => { cancelled = true; };
  }, []);

  // Refresh all wallet balances periodically
  useEffect(() => {
    if (!dinaAddress || wallets.length === 0) return;
    const interval = setInterval(async () => {
      try {
        // Always read latest wallets from localStorage (not stale React state)
        const current = loadWallets();
        const updated = await refreshAllBalances(current);
        const newTotal = totalBalance(updated);
        setWallets(updated);
        setRealBalance(newTotal);
      } catch {
        // ignore
      }
    }, 10_000);
    return () => clearInterval(interval);
  }, [dinaAddress]);

  // Fetch real transactions
  useEffect(() => {
    if (!dinaAddress) return;
    getRecentTransactions(dinaAddress).then(setTransactions).catch(() => {});
  }, [dinaAddress, realBalance]);

  // Set up a sub-wallet (generate address + fund from faucet)
  const handleSetupWallet = useCallback(async (walletId: string) => {
    setSettingUp(walletId);
    try {
      const updated = await setupWallet(walletId);
      setWallets(updated);
      setRealBalance(totalBalance(updated));
      updateBalanceTimestamp(Math.floor(Date.now() / 1000));
    } catch {
      // setup failed
    } finally {
      setSettingUp(null);
    }
  }, [updateBalanceTimestamp]);

  // Fund main wallet handler
  const handleFundWallet = useCallback(async () => {
    if (!dinaAddress) return;
    setFunding(true);
    setFaucetResult(null);
    const start = Date.now();
    try {
      await fundFromFaucet(dinaAddress);
      // Refresh all wallets from chain
      const current = loadWallets();
      const updated = await refreshAllBalances(current);
      const elapsed = Date.now() - start;
      setWallets(updated);
      setRealBalance(totalBalance(updated));
      updateBalanceTimestamp(Math.floor(Date.now() / 1000));
      setFaucetResult({ time: elapsed, address: dinaAddress, balance: totalBalance(updated) });
    } catch {
      // faucet failed — do nothing
    } finally {
      setFunding(false);
    }
  }, [dinaAddress, updateBalanceTimestamp]);

  // Build wallet list from persistent store
  const now = Math.floor(Date.now() / 1000);
  const hasRealBalance = realBalance !== null && realBalance > 0;

  const realWallets: Wallet[] = wallets.map((w, i) => ({
    id: w.id,
    name: w.name,
    type: w.type,
    icon: w.icon,
    balance: w.balance,
    yieldRateBps: 450,
    lastYieldUpdate: balanceLastUpdate,
    isDefault: i === 0,
    isSetUp: w.isSetUp,
  }));

  const yieldBps = 450; // 4.5% APY

  // Only USDC — no fake currencies
  const realCurrencies: Currency[] = [{
    symbol: 'USDC',
    name: 'US Dollar',
    balance: hasRealBalance ? realBalance : 0,
    yieldRateBps: 450,
    ratePerUsdc: 1_000_000,
    icon: '\u{1F1FA}\u{1F1F8}',
    region: 'Major',
  }];

  // No loading guard — dashboard renders immediately on testnet

  return (
    <div className="min-h-screen bg-slate-950">
      <Navbar />
      {/* User bar */}
      <div className="max-w-6xl mx-auto px-4 pt-6 flex items-center justify-between">
        <span className="text-sm text-slate-400">Testnet User</span>
        <span className="text-xs text-slate-600">dina-testnet-1</span>
      </div>
      <main className="max-w-6xl mx-auto px-4 py-8">
        {/* Hero: Total Balance */}
        <div className="text-center mb-8">
          <p className="text-sm text-slate-400 uppercase tracking-wider mb-2">
            Testnet Balance
          </p>
          <BalanceStream
            baseBalance={realBalance ?? 0}
            yieldRateBps={yieldBps}
            lastUpdate={balanceLastUpdate}
            size="lg"
            className="text-white"
          />
          <p className="mt-2 text-sm text-emerald-400">
            Earning {(yieldBps / 100).toFixed(2)}% APY on testnet
          </p>
          {dinaAddress && (
            <p className="mt-1 text-xs text-slate-600 font-mono truncate max-w-md mx-auto">
              {dinaAddress}
            </p>
          )}
          {!hasRealBalance && realBalance !== null && (
            <p className="mt-2 text-xs text-amber-400">
              No balance yet. Tap the faucet button below to get test USDC.
            </p>
          )}
        </div>

        {/* Fund Wallet Button */}
        <div className="flex justify-center mb-6">
          <button
            onClick={handleFundWallet}
            disabled={funding || !dinaAddress}
            className="px-6 py-3 rounded-xl bg-amber-600 hover:bg-amber-500 disabled:bg-slate-800 disabled:text-slate-600 text-white font-semibold transition-colors flex items-center gap-2"
          >
            {funding ? (
              <>
                <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                Funding...
              </>
            ) : (
              <>
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M12 6v12m-3-2.818l.879.659c1.171.879 3.07.879 4.242 0 1.172-.879 1.172-2.303 0-3.182C13.536 12.219 12.768 12 12 12c-.725 0-1.45-.22-2.003-.659-1.106-.879-1.106-2.303 0-3.182s2.9-.879 4.006 0l.415.33M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                Get 10,000 Test USDC
              </>
            )}
          </button>
        </div>

        {/* Faucet Result Details */}
        {faucetResult && (
          <div className="flex justify-center mb-6">
            <div className="rounded-xl bg-slate-900 border border-slate-800 p-4 max-w-md w-full">
              <p className="text-xs text-slate-500 uppercase tracking-wider mb-2">Faucet Transaction</p>
              <div className="space-y-1.5 text-sm">
                <div className="flex justify-between">
                  <span className="text-slate-400">Amount</span>
                  <span className="text-emerald-400 font-mono font-semibold">+$10,000.00 USDC</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-slate-400">Speed</span>
                  <span className="text-emerald-400 font-mono">{faucetResult.time}ms</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-slate-400">Fee</span>
                  <span className="text-emerald-400 font-mono">$0.00 (zero fees)</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-slate-400">Network</span>
                  <span className="text-white">Dina Testnet</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-slate-400">Block time</span>
                  <span className="text-white">100ms</span>
                </div>
                <div className="mt-2 pt-2 border-t border-slate-800">
                  <p className="text-[10px] text-slate-500 uppercase">Wallet Address</p>
                  <p className="text-xs text-slate-400 font-mono break-all mt-0.5">{faucetResult.address}</p>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Yield Stats */}
        <div className="mb-8">
          <YieldDisplay wallets={realWallets} />
        </div>

        {/* Action Buttons */}
        <div className="flex justify-center gap-4 mb-8">
          <Link
            href="/send"
            className="px-6 py-3 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white font-semibold transition-colors flex items-center gap-2"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 12L3.269 3.126A59.768 59.768 0 0121.485 12 59.77 59.77 0 013.27 20.876L5.999 12zm0 0h7.5" />
            </svg>
            Send
          </Link>
          <Link
            href="/receive"
            className="px-6 py-3 rounded-xl bg-slate-800 hover:bg-slate-700 text-white font-semibold transition-colors flex items-center gap-2 border border-slate-700"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3" />
            </svg>
            Receive
          </Link>
          <Link
            href="/convert"
            className="px-6 py-3 rounded-xl bg-slate-800 hover:bg-slate-700 text-white font-semibold transition-colors flex items-center gap-2 border border-slate-700"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M7.5 21L3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5" />
            </svg>
            Convert
          </Link>
        </div>

        {/* Activity Log */}
        <div className="rounded-xl bg-slate-900 border border-slate-800 p-4 mb-8">
          <p className="text-xs text-slate-500 uppercase tracking-wider mb-2">Activity Log</p>
          <div className="font-mono text-xs text-slate-400 space-y-1 max-h-48 overflow-y-auto">
            {dinaAddress && (
              <p><span className="text-slate-600">[wallet]</span> {dinaAddress}</p>
            )}
            {realBalance !== null && (
              <p><span className="text-slate-600">[balance]</span> <span className="text-emerald-400">{(realBalance / 1_000_000).toLocaleString('en-US', {minimumFractionDigits: 2, maximumFractionDigits: 2})} USDC</span></p>
            )}
            <p><span className="text-slate-600">[network]</span> Dina Testnet | Chain: {CHAIN_ID}</p>
            <p><span className="text-slate-600">[blocks]</span> 100ms block time | Zero fees</p>
            <p><span className="text-slate-600">[yield]</span> 4.50% APY on all wallets</p>
            {network.connected && (
              <p><span className="text-slate-600">[status]</span> <span className="text-emerald-400">Connected</span> | Block #{network.height.toLocaleString()}</p>
            )}
            {!network.connected && (
              <p><span className="text-slate-600">[status]</span> <span className="text-amber-400">Connecting...</span></p>
            )}
            {faucetResult && (
              <>
                <p><span className="text-emerald-400">[faucet]</span> +$10,000.00 USDC | {faucetResult.time}ms | Fee: $0.00</p>
              </>
            )}
            {transactions.map(tx => (
              <p key={tx.id}>
                <span className={tx.type === 'receive' ? 'text-emerald-400' : 'text-amber-400'}>[{tx.type}]</span>{' '}
                {tx.type === 'receive' ? '+' : '-'}{(tx.amount / 1_000_000).toFixed(2)} USDC
                {tx.counterparty ? ` | ${tx.counterparty}` : ''}
              </p>
            ))}
          </div>
        </div>

        {/* Wallet Grid */}
        <div className="mb-8">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-semibold text-slate-400 uppercase tracking-wider">Wallets</h2>
            <span className="text-xs text-slate-600">{fundedCount(wallets)} of 9 set up on testnet</span>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {realWallets.map((wallet) => {
              const storedW = wallets.find(w => w.id === wallet.id);
              return (
                <div key={wallet.id} className={`rounded-xl border p-4 ${wallet.isSetUp ? 'bg-slate-900 border-slate-800' : 'bg-slate-900/50 border-slate-800/50'}`}>
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <span className="text-lg">{wallet.icon}</span>
                      <span className="text-sm font-semibold text-white">{wallet.name}</span>
                    </div>
                    {wallet.isDefault && (
                      <span className="text-[10px] font-bold bg-emerald-600 text-white px-2 py-0.5 rounded-full">DEFAULT</span>
                    )}
                  </div>
                  {wallet.isSetUp ? (
                    <>
                      <p className="text-lg font-bold text-white tabular-nums mb-1">
                        ${(wallet.balance / 1_000_000).toLocaleString('en-US', {minimumFractionDigits: 2, maximumFractionDigits: 2})}
                      </p>
                      <p className="text-xs text-emerald-400 mb-1">4.5% APY</p>
                      {storedW?.address && (
                        <div className="flex items-center gap-1">
                          <p className="text-[10px] text-slate-600 font-mono truncate flex-1">{storedW.address}</p>
                          <button
                            onClick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(storedW.address); }}
                            className="text-[10px] text-slate-500 hover:text-emerald-400 shrink-0"
                            title="Copy address"
                          >
                            Copy
                          </button>
                        </div>
                      )}
                    </>
                  ) : (
                    <button
                      onClick={() => handleSetupWallet(wallet.id)}
                      disabled={settingUp === wallet.id}
                      className="mt-2 px-4 py-2 text-sm rounded-lg bg-emerald-600 hover:bg-emerald-500 disabled:bg-slate-700 text-white font-medium transition-colors w-full"
                    >
                      {settingUp === wallet.id ? 'Setting up...' : 'Set up'}
                    </button>
                  )}
                </div>
              );
            })}
          </div>
        </div>

        {/* Bottom sections */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <TransactionList transactions={transactions} />
          <CurrencyList currencies={realCurrencies} />
        </div>

        {/* Network Status */}
        <div className="mt-8 rounded-xl bg-slate-900 border border-slate-800 p-4">
          <h3 className="text-xs font-semibold text-slate-500 uppercase tracking-widest mb-3">
            Testnet Status
          </h3>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
            <div>
              <p className="text-[11px] text-slate-500 mb-0.5">Chain</p>
              <p className="text-sm font-mono text-white">{CHAIN_ID}</p>
            </div>
            <div>
              <p className="text-[11px] text-slate-500 mb-0.5">Block</p>
              <p className="text-sm font-mono text-white tabular-nums">
                {network.height > 0
                  ? `#${network.height.toLocaleString()}`
                  : '--'}
              </p>
            </div>
            <div>
              <p className="text-[11px] text-slate-500 mb-0.5">Status</p>
              <p className="text-sm font-medium flex items-center gap-1.5">
                <span
                  className={`inline-block w-2 h-2 rounded-full ${
                    network.connected ? 'bg-emerald-400' : 'bg-amber-400'
                  }`}
                />
                <span className={network.connected ? 'text-emerald-400' : 'text-amber-400'}>
                  {network.connected ? 'Connected' : 'Offline'}
                </span>
              </p>
            </div>
            <div>
              <p className="text-[11px] text-slate-500 mb-0.5">RPC</p>
              <p className="text-sm font-mono text-slate-300">35.184.213.248</p>
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}
