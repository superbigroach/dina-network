'use client';
import { useEffect, useState, useCallback } from 'react';
import { useRouter } from 'next/navigation';
import { useAuth } from '@/components/AuthProvider';
import { signOut } from '@/lib/firebase';
import { Navbar } from '@/components/Navbar';
import { BalanceStream } from '@/components/BalanceStream';
import { WalletCard } from '@/components/WalletCard';
import { YieldDisplay } from '@/components/YieldDisplay';
import { TransactionList } from '@/components/TransactionList';
import { CurrencyList } from '@/components/CurrencyList';
import { MOCK_CURRENCIES, CHAIN_ID } from '@/lib/constants';
import { getHealth, getBalanceRest, fundFromFaucet, getRecentTransactions } from '@/lib/api';
import { Wallet, Transaction, Currency } from '@/lib/types';
import Link from 'next/link';

interface NetworkStatus {
  connected: boolean;
  height: number;
  status: string;
}

export default function DashboardPage() {
  const { user, loading } = useAuth();
  const router = useRouter();

  // Testnet wallet state
  const [dinaAddress, setDinaAddress] = useState<string | null>(null);
  const [realBalance, setRealBalance] = useState<number | null>(null);
  const [balanceLastUpdate, setBalanceLastUpdate] = useState<number>(Math.floor(Date.now() / 1000));
  const [funding, setFunding] = useState(false);
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [fundInfoWallet, setFundInfoWallet] = useState<string | null>(null);

  useEffect(() => {
    if (!loading && !user) {
      router.push('/');
    }
  }, [user, loading, router]);

  async function handleSignOut() {
    await signOut();
    router.push('/');
  }

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

  // Initialize testnet wallet — generate address, fund from faucet if empty, fetch balance
  useEffect(() => {
    let cancelled = false;

    async function initWallet() {
      // Load or generate a testnet address
      let address = localStorage.getItem('dina_address');
      if (!address) {
        const bytes = new Uint8Array(32);
        crypto.getRandomValues(bytes);
        address = Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
        localStorage.setItem('dina_address', address);
      }
      if (cancelled) return;
      setDinaAddress(address);

      try {
        // Check current balance
        let balance = await getBalanceRest(address);

        // Auto-fund from faucet if balance is zero
        if (balance === 0 || balance === undefined) {
          await fundFromFaucet(address);
          balance = await getBalanceRest(address);
        }

        if (!cancelled) {
          setRealBalance(balance || 0);
          setBalanceLastUpdate(Math.floor(Date.now() / 1000));
        }
      } catch {
        // Faucet or balance fetch failed
        if (!cancelled) {
          setRealBalance(null);
        }
      }
    }

    initWallet();
    return () => { cancelled = true; };
  }, []);

  // Refresh balance periodically
  useEffect(() => {
    if (!dinaAddress) return;
    const interval = setInterval(async () => {
      try {
        const balance = await getBalanceRest(dinaAddress);
        setRealBalance(balance || 0);
        setBalanceLastUpdate(Math.floor(Date.now() / 1000));
      } catch {
        // ignore — keep last known balance
      }
    }, 30_000);
    return () => clearInterval(interval);
  }, [dinaAddress]);

  // Fetch real transactions
  useEffect(() => {
    if (!dinaAddress) return;
    getRecentTransactions(dinaAddress).then(setTransactions).catch(() => {});
  }, [dinaAddress, realBalance]);

  // Fund wallet handler
  const handleFundWallet = useCallback(async () => {
    if (!dinaAddress) return;
    setFunding(true);
    try {
      await fundFromFaucet(dinaAddress);
      const balance = await getBalanceRest(dinaAddress);
      setRealBalance(balance || 0);
      setBalanceLastUpdate(Math.floor(Date.now() / 1000));
    } catch {
      // faucet failed — do nothing
    } finally {
      setFunding(false);
    }
  }, [dinaAddress]);

  // Build REAL wallet list — one funded main wallet, 8 unfunded sub-wallets
  const now = Math.floor(Date.now() / 1000);
  const hasRealBalance = realBalance !== null && realBalance > 0;
  const totalBalance = hasRealBalance ? realBalance : 0;

  const realWallets: Wallet[] = [
    {
      id: 'main-1',
      name: 'Main Wallet',
      type: 'main',
      icon: '🏦',
      balance: hasRealBalance ? realBalance : 0,
      yieldRateBps: 450,
      lastYieldUpdate: balanceLastUpdate,
      isDefault: true,
      isSetUp: true,
    },
    {
      id: 'savings-1',
      name: 'Savings',
      type: 'savings',
      icon: '🐷',
      balance: 0,
      yieldRateBps: 450,
      lastYieldUpdate: now,
      isSetUp: false,
    },
    {
      id: 'backup-1',
      name: 'Backup',
      type: 'backup',
      icon: '🔒',
      balance: 0,
      yieldRateBps: 450,
      lastYieldUpdate: now,
      isSetUp: false,
    },
    {
      id: 'agent-1',
      name: 'Shopping',
      type: 'agent',
      icon: '🛒',
      balance: 0,
      yieldRateBps: 350,
      lastYieldUpdate: now,
      dailyLimit: 500_000_000,
      isSetUp: false,
    },
    {
      id: 'agent-2',
      name: 'Bills',
      type: 'agent',
      icon: '📄',
      balance: 0,
      yieldRateBps: 350,
      lastYieldUpdate: now,
      dailyLimit: 200_000_000,
      isSetUp: false,
    },
    {
      id: 'agent-3',
      name: 'Agent 3',
      type: 'agent',
      icon: '🤖',
      balance: 0,
      yieldRateBps: 350,
      lastYieldUpdate: now,
      isSetUp: false,
    },
    {
      id: 'speed-1',
      name: 'Business',
      type: 'speed',
      icon: '⚡',
      balance: 0,
      yieldRateBps: 300,
      lastYieldUpdate: now,
      isSetUp: false,
    },
    {
      id: 'speed-2',
      name: 'Speed 2',
      type: 'speed',
      icon: '⚡',
      balance: 0,
      yieldRateBps: 300,
      lastYieldUpdate: now,
      isSetUp: false,
    },
    {
      id: 'speed-3',
      name: 'Speed 3',
      type: 'speed',
      icon: '⚡',
      balance: 0,
      yieldRateBps: 300,
      lastYieldUpdate: now,
      isSetUp: false,
    },
  ];

  const yieldBps = 450; // 4.5% APY

  // Build real currency list — USDC shows real balance, others $0
  const realCurrencies: Currency[] = MOCK_CURRENCIES.map(c => ({
    ...c,
    balance: c.symbol === 'USDC' ? (hasRealBalance ? realBalance : 0) : 0,
  }));

  if (loading || !user) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="w-8 h-8 border-2 border-emerald-400 border-t-transparent rounded-full animate-spin" />
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-slate-950">
      <Navbar />
      {/* User bar */}
      <div className="max-w-6xl mx-auto px-4 pt-6 flex items-center justify-between">
        <div className="flex items-center gap-3">
          {user.photoURL && (
            <img src={user.photoURL} alt="" className="w-8 h-8 rounded-full" />
          )}
          <span className="text-sm text-slate-400">
            {user.displayName || user.email}
          </span>
        </div>
        <button
          onClick={handleSignOut}
          className="text-xs text-slate-500 hover:text-slate-300 transition-colors"
        >
          Sign out
        </button>
      </div>
      <main className="max-w-6xl mx-auto px-4 py-8">
        {/* Hero: Total Balance */}
        <div className="text-center mb-8">
          <p className="text-sm text-slate-400 uppercase tracking-wider mb-2">
            Testnet Balance
          </p>
          <BalanceStream
            baseBalance={totalBalance}
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

        {/* Wallet Grid */}
        <div className="mb-8">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-semibold text-slate-400 uppercase tracking-wider">Wallets</h2>
            <span className="text-xs text-slate-600">1 of 9 funded on testnet</span>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {realWallets.map((wallet) => (
              <div key={wallet.id} className="relative">
                <WalletCard wallet={wallet} />
                {!wallet.isSetUp && (
                  <button
                    onClick={() => setFundInfoWallet(fundInfoWallet === wallet.id ? null : wallet.id)}
                    className="absolute top-2 right-2 text-slate-500 hover:text-slate-300 transition-colors"
                    title="Info"
                  >
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
                      <path strokeLinecap="round" strokeLinejoin="round" d="M11.25 11.25l.041-.02a.75.75 0 011.063.852l-.708 2.836a.75.75 0 001.063.853l.041-.021M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-9-3.75h.008v.008H12V8.25z" />
                    </svg>
                  </button>
                )}
                {fundInfoWallet === wallet.id && (
                  <div className="absolute inset-0 z-10 bg-slate-900/95 rounded-xl border border-slate-700 p-4 flex flex-col items-center justify-center text-center gap-2">
                    <p className="text-xs text-slate-300 leading-relaxed">
                      On mainnet, each wallet will be a separate on-chain account.
                      On testnet, all funds are in your main wallet.
                    </p>
                    <button
                      onClick={() => setFundInfoWallet(null)}
                      className="mt-1 px-3 py-1 text-xs rounded-lg bg-slate-800 text-slate-400 hover:text-white transition-colors"
                    >
                      Got it
                    </button>
                  </div>
                )}
              </div>
            ))}
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
