'use client';

import { useState, useEffect, useRef } from 'react';
import { YieldInfo, RateStats } from '@/lib/types';
import { RateChart } from './RateChart';
import { formatUsdc } from '@/lib/yield';

interface Props {
  position: YieldInfo;
  rateStats: RateStats | undefined;
}

/** Format a micro-unit amount into local currency display */
function formatLocal(microUnits: number, symbol: string, rate: number): string {
  // rate is micro-units per 1 USDC (6 decimals)
  // For high-value currencies like JPY/KRW, show no decimals
  const val = microUnits / 1_000_000;
  const perUsdc = rate / 1_000_000;

  if (perUsdc >= 100) {
    return `${symbol}${val.toLocaleString('en-US', { minimumFractionDigits: 0, maximumFractionDigits: 0 })}`;
  }
  return `${symbol}${val.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

/** Streaming yield counter -- ticks up in real time */
function useStreamingYield(baseUsdc: number, backingUsdc: number, yieldBps: number): number {
  const [current, setCurrent] = useState(baseUsdc);
  const startRef = useRef(Date.now());
  const frameRef = useRef<number>(0);

  useEffect(() => {
    startRef.current = Date.now();
    const update = () => {
      const elapsedSeconds = (Date.now() - startRef.current) / 1000;
      // Yield per second = backing * bps / (10000 * seconds_per_year)
      const yieldPerSec = (backingUsdc * yieldBps) / (10_000 * 31_536_000);
      setCurrent(baseUsdc + Math.floor(yieldPerSec * elapsedSeconds));
      frameRef.current = requestAnimationFrame(update);
    };
    frameRef.current = requestAnimationFrame(update);
    return () => cancelAnimationFrame(frameRef.current);
  }, [baseUsdc, backingUsdc, yieldBps]);

  return current;
}

export function YieldCard({ position, rateStats }: Props) {
  const [expanded, setExpanded] = useState(true);
  const [claiming, setClaiming] = useState(false);
  const [claimed, setClaimed] = useState(false);

  // Stream the pending yield in real time
  const streamedYieldUsdc = useStreamingYield(
    position.pendingYieldUsdc,
    position.usdcBacking,
    450, // 4.5% APY
  );

  // Convert streaming USDC yield to local currency
  const streamedYieldLocal = Math.floor(
    (streamedYieldUsdc * position.currentRate) / 1_000_000
  );

  const handleClaim = () => {
    setClaiming(true);
    // Simulate claim transaction
    setTimeout(() => {
      setClaiming(false);
      setClaimed(true);
      setTimeout(() => setClaimed(false), 3000);
    }, 2000);
  };

  // Rate gain/loss since deposit
  const rateChange = position.currentRate - position.depositRate;
  const rateChangePct = ((rateChange / position.depositRate) * 100).toFixed(2);
  const rateIsUp = rateChange >= 0;

  return (
    <div className="rounded-2xl bg-slate-900 border border-slate-800 overflow-hidden transition-all">
      {/* Header -- always visible */}
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full px-5 py-4 flex items-center justify-between hover:bg-slate-800/50 transition-colors"
      >
        <div className="flex items-center gap-3">
          <span className="text-2xl">{position.flag}</span>
          <div className="text-left">
            <p className="text-sm font-semibold text-white">
              {position.currency}
            </p>
            <p className="text-xs text-slate-400">
              {formatLocal(position.localBalance, position.currencySymbol, position.currentRate)} balance
            </p>
          </div>
        </div>
        <div className="flex items-center gap-3">
          <div className="text-right">
            <p className="text-sm font-bold text-emerald-400 tabular-nums">
              +{formatUsdc(streamedYieldUsdc)}
            </p>
            <p className="text-[11px] text-slate-500">unclaimed</p>
          </div>
          <svg
            className={`w-4 h-4 text-slate-500 transition-transform ${expanded ? 'rotate-180' : ''}`}
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            strokeWidth={2}
          >
            <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
          </svg>
        </div>
      </button>

      {/* Expanded content */}
      {expanded && (
        <div className="px-5 pb-5 space-y-4 border-t border-slate-800/50">
          {/* Balance details */}
          <div className="grid grid-cols-2 gap-3 pt-4">
            <div>
              <p className="text-[11px] text-slate-500 uppercase tracking-wider mb-0.5">
                Local Balance
              </p>
              <p className="text-sm font-semibold text-white tabular-nums">
                {formatLocal(position.localBalance, position.currencySymbol, position.currentRate)}
              </p>
            </div>
            <div>
              <p className="text-[11px] text-slate-500 uppercase tracking-wider mb-0.5">
                USDC Backing
              </p>
              <p className="text-sm font-semibold text-white tabular-nums">
                {formatUsdc(position.usdcBacking)}
              </p>
            </div>
            <div>
              <p className="text-[11px] text-slate-500 uppercase tracking-wider mb-0.5">
                Pending Yield
              </p>
              <p className="text-sm font-bold text-emerald-400 tabular-nums">
                {formatUsdc(streamedYieldUsdc)}
              </p>
              <p className="text-[10px] text-slate-500 tabular-nums">
                = {formatLocal(streamedYieldLocal, position.currencySymbol, position.currentRate)}
              </p>
            </div>
            <div>
              <p className="text-[11px] text-slate-500 uppercase tracking-wider mb-0.5">
                Total Claimed
              </p>
              <p className="text-sm font-semibold text-white tabular-nums">
                {formatUsdc(position.totalClaimedUsdc)}
              </p>
              <p className="text-[10px] text-slate-500 tabular-nums">
                = {formatLocal(position.totalClaimedLocal, position.currencySymbol, position.currentRate)}
              </p>
            </div>
          </div>

          {/* Rate since deposit */}
          <div className="rounded-lg bg-slate-800/50 px-3 py-2 flex items-center justify-between">
            <span className="text-xs text-slate-400">Rate since deposit</span>
            <span className={`text-xs font-medium tabular-nums ${rateIsUp ? 'text-emerald-400' : 'text-red-400'}`}>
              {rateIsUp ? '\u25B2' : '\u25BC'} {rateIsUp ? '+' : ''}{rateChangePct}%
            </span>
          </div>

          {/* Rate chart */}
          {rateStats && (
            <RateChart
              current={rateStats.current}
              high30d={rateStats.high30d}
              low30d={rateStats.low30d}
              avg30d={rateStats.avg30d}
              change24hBps={rateStats.change24hBps}
              change7dBps={rateStats.change7dBps}
              change30dBps={rateStats.change30dBps}
              currencySymbol={position.currency}
            />
          )}

          {/* Best claim info */}
          {rateStats && (
            <div className="rounded-lg bg-slate-800/30 px-3 py-2 text-xs text-slate-400 space-y-1">
              <div className="flex justify-between">
                <span>Best rate in 30d</span>
                <span className="text-white tabular-nums">
                  {(rateStats.high30d / 1_000_000 >= 100)
                    ? (rateStats.high30d / 1_000_000).toFixed(1)
                    : (rateStats.high30d / 1_000_000).toFixed(4)}
                </span>
              </div>
              {rateStats.current < rateStats.high30d && (
                <div className="flex justify-between">
                  <span>Current vs best</span>
                  <span className="text-amber-400 tabular-nums">
                    -{(((rateStats.high30d - rateStats.current) / rateStats.high30d) * 100).toFixed(1)}%
                  </span>
                </div>
              )}
            </div>
          )}

          {/* Action buttons */}
          <div className="flex gap-3">
            <button
              onClick={handleClaim}
              disabled={claiming || claimed || streamedYieldUsdc <= 0}
              className={`flex-1 py-2.5 rounded-xl font-semibold text-sm transition-all flex items-center justify-center gap-2 ${
                claimed
                  ? 'bg-emerald-600/20 text-emerald-400 border border-emerald-600/30'
                  : claiming
                    ? 'bg-slate-800 text-slate-400'
                    : 'bg-emerald-600 hover:bg-emerald-500 text-white'
              }`}
            >
              {claiming ? (
                <>
                  <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                  Claiming...
                </>
              ) : claimed ? (
                <>
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
                    <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
                  </svg>
                  Claimed!
                </>
              ) : (
                <>
                  Claim {formatLocal(streamedYieldLocal, position.currencySymbol, position.currentRate)} Now
                </>
              )}
            </button>
            <button
              className="px-4 py-2.5 rounded-xl text-sm font-medium text-slate-300 bg-slate-800 hover:bg-slate-700 border border-slate-700 transition-colors"
              title="Set an auto-claim threshold"
            >
              Auto-claim
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
