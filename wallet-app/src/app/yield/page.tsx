'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { useAuth } from '@/components/AuthProvider';
import { Navbar } from '@/components/Navbar';
import { YieldCard } from '@/components/YieldCard';
import { formatUsdc } from '@/lib/yield';
import { MOCK_YIELD_POSITIONS, MOCK_RATE_STATS } from '@/lib/constants';
import { getBalanceRest } from '@/lib/api';
import Link from 'next/link';

export default function YieldPage() {
  const { user, loading } = useAuth();
  const router = useRouter();
  const [realBalance, setRealBalance] = useState<number>(0);

  useEffect(() => {
    if (!loading && !user) {
      router.push('/');
    }
  }, [user, loading, router]);

  // Fetch real testnet balance
  useEffect(() => {
    const address = typeof window !== 'undefined' ? localStorage.getItem('dina_address') : null;
    if (!address) return;
    getBalanceRest(address).then(b => setRealBalance(b || 0)).catch(() => {});
  }, []);

  const estimatedAnnualYield = Math.floor((realBalance * 450) / 10_000);

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
      <main className="max-w-3xl mx-auto px-4 py-8">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-2xl font-bold text-white mb-1">Your Yield</h1>
          <p className="text-sm text-slate-400">
            Your USDC on Dina earns 4.5% APY from US Treasury bill yield.
          </p>
        </div>

        {/* Real balance + yield estimate */}
        <div className="rounded-2xl bg-gradient-to-br from-slate-900 to-emerald-950/20 border border-emerald-800/30 p-6 mb-8">
          <p className="text-xs text-slate-400 uppercase tracking-wider mb-3">
            How Dina Yield Works
          </p>
          <p className="text-sm text-slate-300 leading-relaxed mb-4">
            When you hold USDC on Dina, it earns 4.5% APY backed by US Treasury bills.
            When you hold other currencies, yield accrues in USDC and converts at the
            live market rate when you claim.
          </p>

          <div className="grid grid-cols-2 gap-4 mb-4">
            <div className="rounded-xl bg-slate-800/50 p-4">
              <p className="text-[11px] text-slate-500 uppercase tracking-wider mb-1">
                Current Testnet Balance
              </p>
              <p className="text-lg font-bold text-white tabular-nums">
                {formatUsdc(realBalance)}
              </p>
              <p className="text-[10px] text-slate-500">USDC (streaming...)</p>
            </div>
            <div className="rounded-xl bg-slate-800/50 p-4">
              <p className="text-[11px] text-slate-500 uppercase tracking-wider mb-1">
                Estimated Annual Yield
              </p>
              <p className="text-lg font-bold text-emerald-400 tabular-nums">
                {formatUsdc(estimatedAnnualYield)}
              </p>
              <p className="text-[10px] text-slate-500">at 4.50% APY</p>
            </div>
          </div>

          <div className="rounded-lg bg-amber-900/20 border border-amber-800/30 px-4 py-3">
            <p className="text-xs text-amber-300 font-medium mb-1">Testnet Preview</p>
            <p className="text-xs text-amber-200/70 leading-relaxed">
              Yield claiming will be available when the YieldVault contract is deployed to testnet.
              The balance counter on your dashboard streams an estimate of accrued yield in real time.
            </p>
          </div>

          <div className="mt-4 flex justify-center">
            <Link
              href="/dashboard"
              className="px-5 py-2.5 rounded-xl bg-amber-600 hover:bg-amber-500 text-white font-semibold text-sm transition-colors"
            >
              Get Test USDC from Faucet
            </Link>
          </div>
        </div>

        {/* Rate analytics section — simulated data preview */}
        <div className="mb-4">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-semibold text-slate-400 uppercase tracking-wider">
              Rate Analytics Preview
            </h2>
            <span className="text-[10px] text-slate-600 bg-slate-800 px-2 py-0.5 rounded-full">
              Simulated data
            </span>
          </div>
          <p className="text-xs text-slate-600 mt-1 mb-4">
            Rate data shown is simulated. Live Pyth oracle feed will be connected on mainnet.
          </p>
        </div>

        <div className="space-y-4">
          {MOCK_YIELD_POSITIONS.map((position) => (
            <YieldCard
              key={position.currency}
              position={position}
              rateStats={MOCK_RATE_STATS[position.currency]}
            />
          ))}
        </div>

        <div className="mt-8 text-center">
          <p className="text-xs text-slate-600">
            The yield positions above are simulated to preview what the UI will look like on mainnet.
            <br />
            Rates will update every 5 minutes from Pyth oracle feeds when connected.
          </p>
        </div>
      </main>
    </div>
  );
}
