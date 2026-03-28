'use client';
import { Wallet } from '@/lib/types';
import { yieldPerDay, formatUsdc } from '@/lib/yield';

interface Props {
  wallets: Wallet[];
}

export function YieldDisplay({ wallets }: Props) {
  const activeWallets = wallets.filter(w => w.isSetUp && w.balance > 0);
  const totalDailyYield = activeWallets.reduce(
    (sum, w) => sum + yieldPerDay(w.balance, w.yieldRateBps),
    0
  );
  const totalBalance = activeWallets.reduce((sum, w) => sum + w.balance, 0);
  const weightedApy = totalBalance > 0
    ? activeWallets.reduce((sum, w) => sum + (w.balance / totalBalance) * w.yieldRateBps, 0)
    : 0;

  const monthlyYield = totalDailyYield * 30;

  return (
    <div className="grid grid-cols-3 gap-4">
      <div className="rounded-xl bg-slate-900 border border-slate-800 p-4">
        <p className="text-xs text-slate-400 uppercase tracking-wider mb-1">Today&apos;s Yield</p>
        <p className="text-lg font-bold text-emerald-400 tabular-nums">
          +{formatUsdc(totalDailyYield)}
        </p>
      </div>
      <div className="rounded-xl bg-slate-900 border border-slate-800 p-4">
        <p className="text-xs text-slate-400 uppercase tracking-wider mb-1">This Month</p>
        <p className="text-lg font-bold text-emerald-400 tabular-nums">
          +{formatUsdc(monthlyYield)}
        </p>
      </div>
      <div className="rounded-xl bg-slate-900 border border-slate-800 p-4">
        <p className="text-xs text-slate-400 uppercase tracking-wider mb-1">Blended APY</p>
        <p className="text-lg font-bold text-emerald-400 tabular-nums">
          {(weightedApy / 100).toFixed(2)}%
        </p>
      </div>
    </div>
  );
}
