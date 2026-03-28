'use client';
import { Wallet } from '@/lib/types';
import { BalanceStream } from './BalanceStream';
import { formatUsdc } from '@/lib/yield';

interface Props {
  wallet: Wallet;
}

export function WalletCard({ wallet }: Props) {
  if (!wallet.isSetUp) {
    return (
      <div className="rounded-xl border border-dashed border-slate-700 bg-slate-900/50 p-4 flex flex-col items-center justify-center min-h-[140px] gap-2">
        <span className="text-2xl opacity-40">{wallet.icon}</span>
        <span className="text-sm text-slate-500">{wallet.name}</span>
        <button className="mt-1 px-4 py-1.5 text-xs font-medium rounded-lg bg-slate-800 text-slate-300 hover:bg-slate-700 transition-colors">
          Set up
        </button>
      </div>
    );
  }

  const typeColors: Record<string, string> = {
    main: 'border-emerald-800/50 bg-gradient-to-br from-slate-900 to-emerald-950/30',
    savings: 'border-blue-800/50 bg-gradient-to-br from-slate-900 to-blue-950/30',
    backup: 'border-amber-800/50 bg-gradient-to-br from-slate-900 to-amber-950/30',
    agent: 'border-purple-800/50 bg-gradient-to-br from-slate-900 to-purple-950/30',
    speed: 'border-cyan-800/50 bg-gradient-to-br from-slate-900 to-cyan-950/30',
  };

  return (
    <div className={`rounded-xl border p-4 min-h-[140px] flex flex-col justify-between ${typeColors[wallet.type] || 'border-slate-800 bg-slate-900'}`}>
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-2">
          <span className="text-xl">{wallet.icon}</span>
          <span className="text-sm font-medium text-slate-300">{wallet.name}</span>
        </div>
        {wallet.isDefault && (
          <span className="text-[10px] font-semibold uppercase tracking-wider px-2 py-0.5 rounded-full bg-emerald-900/60 text-emerald-400 border border-emerald-800/50">
            Default
          </span>
        )}
      </div>

      <div className="mt-3">
        <BalanceStream
          baseBalance={wallet.balance}
          yieldRateBps={wallet.yieldRateBps}
          lastUpdate={wallet.lastYieldUpdate}
          size="sm"
        />
      </div>

      <div className="mt-2 flex items-center justify-between">
        <span className="text-xs text-emerald-400">
          {(wallet.yieldRateBps / 100).toFixed(1)}% APY
        </span>
        {wallet.dailyLimit && (
          <span className="text-xs text-slate-500">
            Limit: {formatUsdc(wallet.dailyLimit)}/day
          </span>
        )}
      </div>
    </div>
  );
}
