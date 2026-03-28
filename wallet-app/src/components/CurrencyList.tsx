'use client';
import { useState } from 'react';
import { Currency, CurrencyRegion } from '@/lib/types';
import { CURRENCY_REGIONS } from '@/lib/constants';

interface Props {
  currencies: Currency[];
}

export function CurrencyList({ currencies }: Props) {
  const [search, setSearch] = useState('');

  const filtered = currencies.filter(
    (c) =>
      c.symbol.toLowerCase().includes(search.toLowerCase()) ||
      c.name.toLowerCase().includes(search.toLowerCase()),
  );

  const grouped = CURRENCY_REGIONS.reduce(
    (acc, region) => {
      const items = filtered.filter((c) => c.region === region);
      if (items.length > 0) acc.push({ region, items });
      return acc;
    },
    [] as { region: CurrencyRegion; items: Currency[] }[],
  );

  return (
    <div className="rounded-xl bg-slate-900 border border-slate-800 overflow-hidden">
      <div className="px-4 py-3 border-b border-slate-800 flex items-center justify-between gap-3">
        <h3 className="text-sm font-semibold text-slate-300 uppercase tracking-wider shrink-0">
          Currencies
        </h3>
        <input
          type="text"
          placeholder="Search..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-40 bg-slate-800 border border-slate-700 rounded-lg px-3 py-1.5 text-xs text-white placeholder:text-slate-500 outline-none focus:border-emerald-500 transition-colors"
        />
      </div>

      <div className="max-h-[32rem] overflow-y-auto divide-y divide-slate-800/50">
        {grouped.length === 0 && (
          <p className="px-4 py-6 text-sm text-slate-500 text-center">No currencies found</p>
        )}

        {grouped.map(({ region, items }) => (
          <div key={region}>
            <div className="px-4 py-2 bg-slate-800/40 sticky top-0 z-10">
              <span className="text-[11px] font-semibold text-slate-500 uppercase tracking-widest">
                {region}
              </span>
            </div>

            {items.map((c) => {
              const displayBalance = c.balance / 1_000_000;
              const hasBalance = c.balance > 0;

              return (
                <div
                  key={c.symbol}
                  className="flex items-center justify-between px-4 py-3 hover:bg-slate-800/50 transition-colors"
                >
                  <div className="flex items-center gap-3">
                    <span className="text-xl">{c.icon}</span>
                    <div>
                      <p className="text-sm font-medium text-white">{c.symbol}</p>
                      <p className="text-xs text-slate-500">{c.name}</p>
                    </div>
                  </div>
                  <div className="text-right">
                    {hasBalance ? (
                      <p className="text-sm font-semibold tabular-nums text-white">
                        {displayBalance.toLocaleString('en-US', {
                          minimumFractionDigits: 2,
                          maximumFractionDigits: 2,
                        })}
                      </p>
                    ) : (
                      <button className="text-xs font-semibold text-emerald-400 border border-emerald-400/30 rounded-lg px-3 py-1 hover:bg-emerald-400/10 transition-colors">
                        Add
                      </button>
                    )}
                    <p className="text-xs text-emerald-400 mt-0.5">
                      {(c.yieldRateBps / 100).toFixed(1)}% APY
                    </p>
                  </div>
                </div>
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
}
