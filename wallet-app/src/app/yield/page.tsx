'use client';

import { useEffect, useState } from 'react';
import { Navbar } from '@/components/Navbar';
import { formatUsdc, yieldPerDay } from '@/lib/yield';
import { getBalanceRest } from '@/lib/api';
import Link from 'next/link';

export default function YieldPage() {
  const [realBalance, setRealBalance] = useState<number>(0);
  const [address, setAddress] = useState<string | null>(null);

  // No auth redirect on testnet

  // Fetch real testnet balance
  useEffect(() => {
    const addr = typeof window !== 'undefined' ? localStorage.getItem('dina_address') : null;
    if (!addr) return;
    setAddress(addr);
    getBalanceRest(addr).then(b => setRealBalance(b || 0)).catch(() => {});
  }, []);

  const dailyYield = yieldPerDay(realBalance, 450);
  const monthlyYield = dailyYield * 30;
  const annualYield = Math.floor((realBalance * 450) / 10_000);

  // No loading spinner on testnet

  return (
    <div className="min-h-screen bg-slate-950">
      <Navbar />
      <main className="max-w-3xl mx-auto px-4 py-8">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-2xl font-bold text-white mb-1">Your Yield</h1>
          <p className="text-sm text-slate-400">
            Your USDC on Dina earns 4.5% APY. You keep 100% of the yield.
          </p>
        </div>

        {/* USDC Balance Card */}
        <div className="rounded-2xl bg-gradient-to-br from-slate-900 to-emerald-950/20 border border-emerald-800/30 p-6 mb-6">
          <div className="flex items-center gap-3 mb-4">
            <span className="text-3xl">{'\u{1F1FA}\u{1F1F8}'}</span>
            <div>
              <p className="text-white font-bold text-lg">USDC</p>
              <p className="text-xs text-slate-400">US Dollar Stablecoin</p>
            </div>
            <div className="ml-auto text-right">
              <p className="text-2xl font-bold text-white tabular-nums">{formatUsdc(realBalance)}</p>
              <p className="text-xs text-emerald-400">4.50% APY</p>
            </div>
          </div>

          {/* Yield breakdown */}
          <div className="grid grid-cols-3 gap-3 mb-4">
            <div className="rounded-xl bg-slate-800/50 p-3">
              <p className="text-[10px] text-slate-500 uppercase tracking-wider mb-1">Daily Yield</p>
              <p className="text-lg font-bold text-emerald-400 tabular-nums">+{formatUsdc(dailyYield)}</p>
            </div>
            <div className="rounded-xl bg-slate-800/50 p-3">
              <p className="text-[10px] text-slate-500 uppercase tracking-wider mb-1">Monthly Yield</p>
              <p className="text-lg font-bold text-emerald-400 tabular-nums">+{formatUsdc(monthlyYield)}</p>
            </div>
            <div className="rounded-xl bg-slate-800/50 p-3">
              <p className="text-[10px] text-slate-500 uppercase tracking-wider mb-1">Annual Yield</p>
              <p className="text-lg font-bold text-emerald-400 tabular-nums">+{formatUsdc(annualYield)}</p>
            </div>
          </div>

          {/* How it works */}
          <div className="rounded-xl bg-slate-800/30 border border-slate-700/50 p-4">
            <p className="text-xs text-slate-400 font-semibold uppercase tracking-wider mb-2">How Dina Yield Works</p>
            <div className="space-y-2 text-sm text-slate-300">
              <p>Your USDC on Dina is backed 1:1 by US Treasury bills earning ~4.5% APY.</p>
              <p>Yield accrues continuously — your balance ticks up every second on the dashboard.</p>
              <p>You keep 100% of the yield. Dina takes no cut.</p>
            </div>
          </div>
        </div>

        {/* Yield rate */}
        <div className="rounded-xl bg-slate-900 border border-slate-800 p-4 mb-6">
          <div className="flex justify-between items-center">
            <p className="text-sm text-white">All 9 wallets earn the same rate</p>
            <span className="text-lg text-emerald-400 font-bold">4.5% APY</span>
          </div>
          <p className="text-xs text-slate-500 mt-1">Backed by US Treasury bills. No tiers, no lockups.</p>
        </div>

        {/* Wallet address */}
        {address && (
          <div className="rounded-xl bg-slate-900 border border-slate-800 p-4 mb-6">
            <p className="text-xs text-slate-500 uppercase tracking-wider mb-1">Your Wallet Address</p>
            <p className="text-xs text-slate-400 font-mono break-all">{address}</p>
          </div>
        )}

        {realBalance === 0 && (
          <div className="text-center">
            <Link
              href="/dashboard"
              className="inline-block px-5 py-2.5 rounded-xl bg-amber-600 hover:bg-amber-500 text-white font-semibold text-sm transition-colors"
            >
              Get Test USDC from Faucet
            </Link>
          </div>
        )}

        <div className="mt-6 rounded-lg bg-amber-900/20 border border-amber-800/30 px-4 py-3">
          <p className="text-xs text-amber-300 font-medium mb-1">Testnet Preview</p>
          <p className="text-xs text-amber-200/70 leading-relaxed">
            Yield is calculated in real-time based on your balance. On mainnet, yield will be
            claimable from the YieldVault contract backed by US Treasury bills.
          </p>
        </div>
      </main>
    </div>
  );
}
