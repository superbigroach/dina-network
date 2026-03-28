'use client';

interface Props {
  current: number;
  high30d: number;
  low30d: number;
  avg30d: number;
  change24hBps: number;
  change7dBps: number;
  change30dBps: number;
  currencySymbol: string;
}

export function RateChart({
  current,
  high30d,
  low30d,
  avg30d,
  change24hBps,
  change7dBps,
  change30dBps,
  currencySymbol,
}: Props) {
  // Calculate position of current rate within 30d range
  const range = high30d - low30d;
  const position = range > 0 ? ((current - low30d) / range) * 100 : 50;

  // Format rate — auto-detect decimals based on magnitude
  const fmt = (r: number) => {
    const val = r / 1_000_000;
    if (val >= 1000) return val.toFixed(0);
    if (val >= 100) return val.toFixed(1);
    if (val >= 10) return val.toFixed(2);
    return val.toFixed(4);
  };

  // Change indicator helpers
  const changeColor = (bps: number) =>
    bps >= 0 ? 'text-emerald-400' : 'text-red-400';
  const changeArrow = (bps: number) => (bps >= 0 ? '\u25B2' : '\u25BC');
  const changePct = (bps: number) => (Math.abs(bps) / 100).toFixed(1);

  return (
    <div className="space-y-3">
      {/* Current rate */}
      <div className="flex items-baseline gap-4">
        <span className="text-lg font-bold text-white tabular-nums">
          1 USD = {fmt(current)} {currencySymbol}
        </span>
      </div>

      {/* 24h / 7d / 30d changes */}
      <div className="flex gap-4 text-xs">
        <span className={changeColor(change24hBps)}>
          24h: {changeArrow(change24hBps)} {changePct(change24hBps)}%
        </span>
        <span className={changeColor(change7dBps)}>
          7d: {changeArrow(change7dBps)} {changePct(change7dBps)}%
        </span>
        <span className={changeColor(change30dBps)}>
          30d: {changeArrow(change30dBps)} {changePct(change30dBps)}%
        </span>
      </div>

      {/* Visual range bar */}
      <div className="relative pt-1 pb-5">
        <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
          <div
            className="h-full bg-gradient-to-r from-red-500 via-amber-400 to-emerald-500 rounded-full"
            style={{ width: '100%' }}
          />
        </div>
        {/* Current position marker */}
        <div
          className="absolute top-0 w-3 h-3 bg-white rounded-full border-2 border-emerald-400 -translate-x-1/2 mt-[-1px]"
          style={{ left: `${Math.min(Math.max(position, 2), 98)}%` }}
        />
        {/* Average position tick */}
        {range > 0 && (
          <div
            className="absolute top-0 w-0.5 h-4 bg-slate-500/60 -translate-x-1/2"
            style={{
              left: `${Math.min(Math.max(((avg30d - low30d) / range) * 100, 2), 98)}%`,
            }}
          />
        )}
        {/* Labels */}
        <div className="flex justify-between mt-2 text-[10px] text-slate-500">
          <span>Low: {fmt(low30d)}</span>
          <span>Avg: {fmt(avg30d)}</span>
          <span>High: {fmt(high30d)}</span>
        </div>
      </div>

      {/* Claim advice */}
      <div className="text-xs text-slate-400">
        {current >= avg30d ? (
          <span className="text-emerald-400">
            Rate is above 30d average -- good time to claim
          </span>
        ) : (
          <span className="text-amber-400">
            Rate is{' '}
            {(((avg30d - current) * 100) / avg30d).toFixed(1)}% below 30d
            average
          </span>
        )}
      </div>
    </div>
  );
}
