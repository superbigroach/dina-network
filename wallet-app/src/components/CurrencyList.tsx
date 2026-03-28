'use client';
import { Currency } from '@/lib/types';

interface Props {
  currencies: Currency[];
}

export function CurrencyList({ currencies }: Props) {
  return (
    <div className="rounded-xl bg-slate-900 border border-slate-800 overflow-hidden">
      <div className="px-4 py-3 border-b border-slate-800">
        <h3 className="text-sm font-semibold text-slate-300 uppercase tracking-wider">Currencies</h3>
      </div>
      <div className="divide-y divide-slate-800">
        {currencies.map((c) => {
          const displayBalance = c.balance / 1_000_000;
          return (
            <div key={c.symbol} className="flex items-center justify-between px-4 py-3 hover:bg-slate-800/50 transition-colors">
              <div className="flex items-center gap-3">
                <span className="text-xl">{c.icon}</span>
                <div>
                  <p className="text-sm font-medium text-white">{c.symbol}</p>
                  <p className="text-xs text-slate-500">{c.name}</p>
                </div>
              </div>
              <div className="text-right">
                <p className="text-sm font-semibold tabular-nums text-white">
                  {displayBalance.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                </p>
                <p className="text-xs text-emerald-400">{(c.yieldRateBps / 100).toFixed(1)}% APY</p>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
