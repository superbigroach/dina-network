'use client';
import { useState } from 'react';
import { Navbar } from '@/components/Navbar';
import { MOCK_CURRENCIES } from '@/lib/constants';
import Link from 'next/link';

export default function ConvertPage() {
  const [fromCurrency, setFromCurrency] = useState('USDC');
  const [toCurrency, setToCurrency] = useState('EURC');
  const [amount, setAmount] = useState('');
  const [converted, setConverted] = useState(false);

  const fromCurr = MOCK_CURRENCIES.find(c => c.symbol === fromCurrency)!;
  const toCurr = MOCK_CURRENCIES.find(c => c.symbol === toCurrency)!;

  const amountNum = parseFloat(amount) || 0;
  const rate = toCurr.ratePerUsdc / fromCurr.ratePerUsdc;
  const receiveAmount = amountNum * rate;

  const handleConvert = () => {
    setConverted(true);
  };

  const handleSwap = () => {
    setFromCurrency(toCurrency);
    setToCurrency(fromCurrency);
  };

  if (converted) {
    return (
      <div className="min-h-screen bg-slate-950">
        <Navbar />
        <main className="max-w-lg mx-auto px-4 py-16 text-center">
          <div className="w-20 h-20 rounded-full bg-emerald-600/20 flex items-center justify-center mx-auto mb-6">
            <svg className="w-10 h-10 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M4.5 12.75l6 6 9-13.5"/>
            </svg>
          </div>
          <h1 className="text-2xl font-bold text-white mb-2">Converted!</h1>
          <p className="text-slate-400 mb-1">
            {amountNum.toFixed(2)} {fromCurrency} &rarr; {receiveAmount.toFixed(2)} {toCurrency}
          </p>
          <p className="text-xs text-slate-500 mb-8">Zero fees applied</p>
          <Link
            href="/dashboard"
            className="inline-block px-6 py-3 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white font-semibold transition-colors"
          >
            Back to Dashboard
          </Link>
        </main>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-slate-950">
      <Navbar />
      <main className="max-w-lg mx-auto px-4 py-12">
        <h1 className="text-2xl font-bold text-white mb-8 text-center">Convert Currency</h1>

        {/* From */}
        <div className="rounded-xl bg-slate-900 border border-slate-800 p-4 mb-2">
          <label className="block text-xs text-slate-500 uppercase tracking-wider mb-2">From</label>
          <div className="flex items-center gap-3">
            <select
              value={fromCurrency}
              onChange={(e) => setFromCurrency(e.target.value)}
              className="bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-white text-sm outline-none"
            >
              {MOCK_CURRENCIES.map(c => (
                <option key={c.symbol} value={c.symbol}>{c.icon} {c.symbol}</option>
              ))}
            </select>
            <input
              type="text"
              inputMode="decimal"
              placeholder="0.00"
              value={amount}
              onChange={(e) => setAmount(e.target.value.replace(/[^0-9.]/g, ''))}
              className="flex-1 text-right text-2xl font-bold bg-transparent outline-none text-white tabular-nums placeholder:text-slate-700"
            />
          </div>
          <p className="text-xs text-slate-500 mt-2 text-right">
            Balance: {(fromCurr.balance / 1_000_000).toLocaleString('en-US', { minimumFractionDigits: 2 })} {fromCurrency}
          </p>
        </div>

        {/* Swap button */}
        <div className="flex justify-center -my-3 relative z-10">
          <button
            onClick={handleSwap}
            className="w-10 h-10 rounded-full bg-slate-800 border border-slate-700 flex items-center justify-center hover:bg-slate-700 transition-colors"
          >
            <svg className="w-4 h-4 text-slate-300" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M7.5 21L3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5"/>
            </svg>
          </button>
        </div>

        {/* To */}
        <div className="rounded-xl bg-slate-900 border border-slate-800 p-4 mb-6">
          <label className="block text-xs text-slate-500 uppercase tracking-wider mb-2">To</label>
          <div className="flex items-center gap-3">
            <select
              value={toCurrency}
              onChange={(e) => setToCurrency(e.target.value)}
              className="bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-white text-sm outline-none"
            >
              {MOCK_CURRENCIES.map(c => (
                <option key={c.symbol} value={c.symbol}>{c.icon} {c.symbol}</option>
              ))}
            </select>
            <div className="flex-1 text-right text-2xl font-bold text-white tabular-nums">
              {amountNum > 0 ? receiveAmount.toFixed(2) : '0.00'}
            </div>
          </div>
        </div>

        {/* Rate info */}
        <div className="rounded-xl bg-slate-900 border border-slate-800 p-4 mb-6 space-y-2">
          <div className="flex justify-between text-sm">
            <span className="text-slate-400">Rate</span>
            <span className="text-white tabular-nums">1 {fromCurrency} = {rate.toFixed(4)} {toCurrency}</span>
          </div>
          <div className="flex justify-between text-sm">
            <span className="text-slate-400">Fee</span>
            <span className="font-semibold text-emerald-400">$0.00</span>
          </div>
          <div className="flex justify-between text-sm">
            <span className="text-slate-400">You receive</span>
            <span className="font-semibold text-white tabular-nums">
              {amountNum > 0 ? receiveAmount.toFixed(2) : '0.00'} {toCurrency}
            </span>
          </div>
        </div>

        {/* Convert button */}
        <button
          onClick={handleConvert}
          disabled={amountNum <= 0 || fromCurrency === toCurrency}
          className="w-full py-3 rounded-xl bg-emerald-600 hover:bg-emerald-500 disabled:bg-slate-800 disabled:text-slate-600 text-white font-semibold transition-colors"
        >
          Convert
        </button>
      </main>
    </div>
  );
}
