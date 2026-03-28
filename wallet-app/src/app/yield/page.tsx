'use client';

import { useEffect } from 'react';
import { useRouter } from 'next/navigation';
import { useAuth } from '@/components/AuthProvider';
import { Navbar } from '@/components/Navbar';
import { YieldCard } from '@/components/YieldCard';
import { formatUsdc } from '@/lib/yield';
import { MOCK_YIELD_POSITIONS, MOCK_RATE_STATS } from '@/lib/constants';

export default function YieldPage() {
  const { user, loading } = useAuth();
  const router = useRouter();

  useEffect(() => {
    if (!loading && !user) {
      router.push('/');
    }
  }, [user, loading, router]);

  // Aggregate totals
  const totalPendingUsdc = MOCK_YIELD_POSITIONS.reduce(
    (sum, p) => sum + p.pendingYieldUsdc,
    0,
  );
  const totalClaimedUsdc = MOCK_YIELD_POSITIONS.reduce(
    (sum, p) => sum + p.totalClaimedUsdc,
    0,
  );
  const totalEarnedUsdc = totalPendingUsdc + totalClaimedUsdc;
  const totalBackingUsdc = MOCK_YIELD_POSITIONS.reduce(
    (sum, p) => sum + p.usdcBacking,
    0,
  );

  // Weighted average APY across all positions (all at 4.5%)
  const blendedApy = 4.5;

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
            Earn yield on every currency. Claim in local currency or USDC.
          </p>
        </div>

        {/* Summary cards */}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 mb-8">
          <div className="rounded-xl bg-slate-900 border border-slate-800 p-4">
            <p className="text-[11px] text-slate-500 uppercase tracking-wider mb-1">
              Total Earned
            </p>
            <p className="text-lg font-bold text-white tabular-nums">
              {formatUsdc(totalEarnedUsdc)}
            </p>
            <p className="text-[10px] text-slate-500">all time</p>
          </div>
          <div className="rounded-xl bg-slate-900 border border-slate-800 p-4">
            <p className="text-[11px] text-slate-500 uppercase tracking-wider mb-1">
              Unclaimed
            </p>
            <p className="text-lg font-bold text-emerald-400 tabular-nums">
              {formatUsdc(totalPendingUsdc)}
            </p>
            <p className="text-[10px] text-slate-500">across {MOCK_YIELD_POSITIONS.length} currencies</p>
          </div>
          <div className="rounded-xl bg-slate-900 border border-slate-800 p-4">
            <p className="text-[11px] text-slate-500 uppercase tracking-wider mb-1">
              Total Backing
            </p>
            <p className="text-lg font-bold text-white tabular-nums">
              {formatUsdc(totalBackingUsdc)}
            </p>
            <p className="text-[10px] text-slate-500">USDC locked</p>
          </div>
          <div className="rounded-xl bg-slate-900 border border-slate-800 p-4">
            <p className="text-[11px] text-slate-500 uppercase tracking-wider mb-1">
              Blended APY
            </p>
            <p className="text-lg font-bold text-emerald-400 tabular-nums">
              {blendedApy.toFixed(2)}%
            </p>
            <p className="text-[10px] text-slate-500">weighted avg</p>
          </div>
        </div>

        {/* Claim all button */}
        {totalPendingUsdc > 0 && (
          <div className="mb-6 flex justify-center">
            <button className="px-6 py-3 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white font-semibold transition-colors text-sm">
              Claim All Yield ({formatUsdc(totalPendingUsdc)})
            </button>
          </div>
        )}

        {/* Per-currency yield cards */}
        <div className="space-y-4">
          <h2 className="text-sm font-semibold text-slate-400 uppercase tracking-wider">
            By Currency
          </h2>
          {MOCK_YIELD_POSITIONS.map((position) => (
            <YieldCard
              key={position.currency}
              position={position}
              rateStats={MOCK_RATE_STATS[position.currency]}
            />
          ))}
        </div>

        {/* Empty state hint */}
        {MOCK_YIELD_POSITIONS.length > 0 && (
          <div className="mt-8 text-center">
            <p className="text-xs text-slate-600">
              Yield accrues in real-time on all currency balances backed by USDC.
              <br />
              Rates update every 5 minutes from oracle feeds.
            </p>
          </div>
        )}
      </main>
    </div>
  );
}
