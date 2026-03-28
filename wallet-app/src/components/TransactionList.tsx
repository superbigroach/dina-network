'use client';
import { Transaction } from '@/lib/types';
import { formatUsdc } from '@/lib/yield';

interface Props {
  transactions: Transaction[];
}

const typeConfig: Record<string, { label: string; color: string; sign: string }> = {
  receive: { label: 'Received', color: 'text-emerald-400', sign: '+' },
  send: { label: 'Sent', color: 'text-red-400', sign: '-' },
  convert: { label: 'Converted', color: 'text-blue-400', sign: '' },
  yield: { label: 'Yield', color: 'text-emerald-400', sign: '+' },
};

function timeAgo(timestamp: number): string {
  const seconds = Math.floor(Date.now() / 1000 - timestamp);
  if (seconds < 60) return 'just now';
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

export function TransactionList({ transactions }: Props) {
  return (
    <div className="rounded-xl bg-slate-900 border border-slate-800 overflow-hidden">
      <div className="px-4 py-3 border-b border-slate-800">
        <h3 className="text-sm font-semibold text-slate-300 uppercase tracking-wider">Recent Activity</h3>
      </div>
      <div className="divide-y divide-slate-800">
        {transactions.map((tx) => {
          const cfg = typeConfig[tx.type];
          return (
            <div key={tx.id} className="flex items-center justify-between px-4 py-3 hover:bg-slate-800/50 transition-colors">
              <div>
                <p className="text-sm font-medium text-white">{cfg.label}</p>
                <p className="text-xs text-slate-500">
                  {tx.counterparty || tx.wallet} &middot; {timeAgo(tx.timestamp)}
                </p>
              </div>
              <div className="text-right">
                <p className={`text-sm font-semibold tabular-nums ${cfg.color}`}>
                  {cfg.sign}{formatUsdc(tx.amount)}
                </p>
                <p className="text-xs text-slate-500">{tx.currency}</p>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
