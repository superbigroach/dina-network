'use client';
import { Navbar } from '@/components/Navbar';
import { BalanceStream } from '@/components/BalanceStream';
import { WalletCard } from '@/components/WalletCard';
import { YieldDisplay } from '@/components/YieldDisplay';
import { TransactionList } from '@/components/TransactionList';
import { CurrencyList } from '@/components/CurrencyList';
import { MOCK_WALLETS, MOCK_CURRENCIES, MOCK_TRANSACTIONS } from '@/lib/constants';
import Link from 'next/link';

export default function DashboardPage() {
  const activeWallets = MOCK_WALLETS.filter(w => w.isSetUp && w.balance > 0);
  const totalBalance = activeWallets.reduce((sum, w) => sum + w.balance, 0);
  const weightedYieldBps = totalBalance > 0
    ? Math.round(activeWallets.reduce((sum, w) => sum + (w.balance / totalBalance) * w.yieldRateBps, 0))
    : 0;
  const earliestUpdate = Math.min(...activeWallets.map(w => w.lastYieldUpdate));

  return (
    <div className="min-h-screen bg-slate-950">
      <Navbar />
      <main className="max-w-6xl mx-auto px-4 py-8">
        {/* Hero: Total Balance */}
        <div className="text-center mb-8">
          <p className="text-sm text-slate-400 uppercase tracking-wider mb-2">Total Balance</p>
          <BalanceStream
            baseBalance={totalBalance}
            yieldRateBps={weightedYieldBps}
            lastUpdate={earliestUpdate}
            size="lg"
            className="text-white"
          />
          <p className="mt-2 text-sm text-emerald-400">
            Earning {(weightedYieldBps / 100).toFixed(2)}% APY across all wallets
          </p>
        </div>

        {/* Yield Stats */}
        <div className="mb-8">
          <YieldDisplay wallets={MOCK_WALLETS} />
        </div>

        {/* Action Buttons */}
        <div className="flex justify-center gap-4 mb-8">
          <Link
            href="/send"
            className="px-6 py-3 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white font-semibold transition-colors flex items-center gap-2"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 12L3.269 3.126A59.768 59.768 0 0121.485 12 59.77 59.77 0 013.27 20.876L5.999 12zm0 0h7.5"/>
            </svg>
            Send
          </Link>
          <Link
            href="/receive"
            className="px-6 py-3 rounded-xl bg-slate-800 hover:bg-slate-700 text-white font-semibold transition-colors flex items-center gap-2 border border-slate-700"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3"/>
            </svg>
            Receive
          </Link>
          <Link
            href="/convert"
            className="px-6 py-3 rounded-xl bg-slate-800 hover:bg-slate-700 text-white font-semibold transition-colors flex items-center gap-2 border border-slate-700"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M7.5 21L3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5"/>
            </svg>
            Convert
          </Link>
        </div>

        {/* Wallet Grid */}
        <div className="mb-8">
          <h2 className="text-sm font-semibold text-slate-400 uppercase tracking-wider mb-4">Wallets</h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {MOCK_WALLETS.map((wallet) => (
              <WalletCard key={wallet.id} wallet={wallet} />
            ))}
          </div>
        </div>

        {/* Bottom sections */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <TransactionList transactions={MOCK_TRANSACTIONS} />
          <CurrencyList currencies={MOCK_CURRENCIES} />
        </div>
      </main>
    </div>
  );
}
