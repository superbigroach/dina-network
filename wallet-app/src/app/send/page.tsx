'use client';
import { useState } from 'react';
import { Navbar } from '@/components/Navbar';
import { MOCK_WALLETS } from '@/lib/constants';
import { formatUsdc } from '@/lib/yield';
import Link from 'next/link';

type Step = 'form' | 'confirm' | 'success';

export default function SendPage() {
  const [step, setStep] = useState<Step>('form');
  const [amount, setAmount] = useState('');
  const [recipient, setRecipient] = useState('');
  const [selectedWallet, setSelectedWallet] = useState('main-1');

  const activeWallets = MOCK_WALLETS.filter(w => w.isSetUp);
  const wallet = activeWallets.find(w => w.id === selectedWallet) || activeWallets[0];

  const handleConfirm = () => {
    setStep('confirm');
  };

  const handleSend = () => {
    setTimeout(() => setStep('success'), 100);
  };

  if (step === 'success') {
    return (
      <div className="min-h-screen bg-slate-950">
        <Navbar />
        <main className="max-w-lg mx-auto px-4 py-16 text-center">
          <div className="w-20 h-20 rounded-full bg-emerald-600/20 flex items-center justify-center mx-auto mb-6">
            <svg className="w-10 h-10 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M4.5 12.75l6 6 9-13.5"/>
            </svg>
          </div>
          <h1 className="text-2xl font-bold text-white mb-2">Sent!</h1>
          <p className="text-slate-400 mb-1">
            ${amount} to {recipient}
          </p>
          <p className="text-xs text-slate-500 mb-8">Confirmed in &lt;100ms</p>
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

  if (step === 'confirm') {
    return (
      <div className="min-h-screen bg-slate-950">
        <Navbar />
        <main className="max-w-lg mx-auto px-4 py-12">
          <h1 className="text-2xl font-bold text-white mb-8 text-center">Confirm Send</h1>
          <div className="rounded-xl bg-slate-900 border border-slate-800 p-6 space-y-4 mb-6">
            <div className="flex justify-between">
              <span className="text-slate-400">Amount</span>
              <span className="font-semibold text-white">${amount}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-slate-400">To</span>
              <span className="font-semibold text-white">{recipient}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-slate-400">From</span>
              <span className="font-semibold text-white">{wallet.name}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-slate-400">Fee</span>
              <span className="font-semibold text-emerald-400">$0.00</span>
            </div>
            <div className="border-t border-slate-800 pt-3 flex justify-between">
              <span className="text-slate-400">Total</span>
              <span className="font-bold text-white">${amount}</span>
            </div>
          </div>
          <div className="flex gap-3">
            <button
              onClick={() => setStep('form')}
              className="flex-1 py-3 rounded-xl bg-slate-800 hover:bg-slate-700 text-white font-semibold transition-colors border border-slate-700"
            >
              Back
            </button>
            <button
              onClick={handleSend}
              className="flex-1 py-3 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white font-semibold transition-colors"
            >
              Confirm & Send
            </button>
          </div>
        </main>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-slate-950">
      <Navbar />
      <main className="max-w-lg mx-auto px-4 py-12">
        <h1 className="text-2xl font-bold text-white mb-8 text-center">Send Money</h1>

        {/* Amount */}
        <div className="text-center mb-8">
          <div className="inline-flex items-baseline gap-1">
            <span className="text-4xl text-slate-500">$</span>
            <input
              type="text"
              inputMode="decimal"
              placeholder="0.00"
              value={amount}
              onChange={(e) => {
                const v = e.target.value.replace(/[^0-9.]/g, '');
                setAmount(v);
              }}
              className="text-5xl font-bold bg-transparent border-none outline-none text-white text-center w-64 tabular-nums placeholder:text-slate-700"
            />
          </div>
          {wallet && (
            <p className="text-sm text-slate-500 mt-2">
              Available: {formatUsdc(wallet.balance)}
            </p>
          )}
        </div>

        {/* Recipient */}
        <div className="mb-6">
          <label className="block text-sm text-slate-400 mb-2">Recipient</label>
          <input
            type="text"
            placeholder="Phone, email, or address"
            value={recipient}
            onChange={(e) => setRecipient(e.target.value)}
            className="w-full px-4 py-3 rounded-xl bg-slate-900 border border-slate-800 text-white placeholder:text-slate-600 outline-none focus:border-emerald-600 transition-colors"
          />
        </div>

        {/* Wallet selector */}
        <div className="mb-8">
          <label className="block text-sm text-slate-400 mb-2">From wallet</label>
          <select
            value={selectedWallet}
            onChange={(e) => setSelectedWallet(e.target.value)}
            className="w-full px-4 py-3 rounded-xl bg-slate-900 border border-slate-800 text-white outline-none focus:border-emerald-600 transition-colors"
          >
            {activeWallets.map(w => (
              <option key={w.id} value={w.id}>
                {w.icon} {w.name} — {formatUsdc(w.balance)}
              </option>
            ))}
          </select>
        </div>

        {/* Send button */}
        <button
          onClick={handleConfirm}
          disabled={!amount || !recipient || parseFloat(amount) <= 0}
          className="w-full py-3 rounded-xl bg-emerald-600 hover:bg-emerald-500 disabled:bg-slate-800 disabled:text-slate-600 text-white font-semibold transition-colors"
        >
          Continue
        </button>
      </main>
    </div>
  );
}
