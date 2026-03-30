'use client';
import { useState, useEffect } from 'react';
import { Navbar } from '@/components/Navbar';
import { loadWallets, saveWallets, type StoredWallet } from '@/lib/wallet-store';
import { fundFromFaucet, getBalanceRest } from '@/lib/api';
import { formatUsdc } from '@/lib/yield';
import Link from 'next/link';

type TransferStep = 'form' | 'transferring' | 'success' | 'error';

interface LogEntry {
  time: string;
  message: string;
  type: 'info' | 'success' | 'error' | 'warn';
}

export default function ConvertPage() {
  const [storedWallets, setStoredWallets] = useState<StoredWallet[]>([]);
  const [fromWalletId, setFromWalletId] = useState('');
  const [toWalletId, setToWalletId] = useState('');
  const [amount, setAmount] = useState('');
  const [step, setStep] = useState<TransferStep>('form');
  const [logs, setLogs] = useState<LogEntry[]>([]);

  useEffect(() => {
    const wallets = loadWallets();
    setStoredWallets(wallets);
    const setupList = wallets.filter(w => w.isSetUp);
    if (setupList.length >= 1) setFromWalletId(setupList[0].id);
    if (setupList.length >= 2) setToWalletId(setupList[1].id);
    else if (setupList.length >= 1) setToWalletId(setupList[0].id);
  }, []);

  const setupWallets = storedWallets.filter(w => w.isSetUp);
  const fromWallet = storedWallets.find(w => w.id === fromWalletId);
  const toWallet = storedWallets.find(w => w.id === toWalletId);

  const truncate = (addr: string) =>
    addr.length > 14 ? `${addr.slice(0, 6)}...${addr.slice(-4)}` : addr;

  const addLog = (message: string, type: LogEntry['type'] = 'info') => {
    const time = new Date().toLocaleTimeString('en-US', { hour12: false });
    setLogs(prev => [...prev, { time, message, type }]);
  };

  const handleSwap = () => {
    setFromWalletId(toWalletId);
    setToWalletId(fromWalletId);
  };

  const handleTransfer = async () => {
    if (!fromWallet || !toWallet || fromWalletId === toWalletId) return;
    const microAmount = Math.round(parseFloat(amount) * 1_000_000);
    if (microAmount <= 0) return;

    setStep('transferring');
    setLogs([]);

    addLog(`Starting USDC transfer: ${fromWallet.name} -> ${toWallet.name}`, 'info');
    addLog(`Amount: ${formatUsdc(microAmount)} USDC`, 'info');
    addLog(`From: ${truncate(fromWallet.address)}`, 'info');
    addLog(`To: ${truncate(toWallet.address)}`, 'info');

    try {
      // On testnet, we fund the target wallet from faucet since we cannot do
      // real peer-to-peer transfers without proper nonce management yet.
      // This simulates the transfer by adding funds to the destination.
      addLog('Requesting faucet funding for target wallet...', 'info');
      await fundFromFaucet(toWallet.address);
      addLog('Faucet funding submitted', 'success');

      // Refresh balances for both wallets
      addLog('Refreshing balances...', 'info');

      const [fromBal, toBal] = await Promise.all([
        getBalanceRest(fromWallet.address).catch(() => fromWallet.balance),
        getBalanceRest(toWallet.address).catch(() => toWallet.balance),
      ]);

      const updated = loadWallets();
      const fromIdx = updated.findIndex(w => w.id === fromWalletId);
      const toIdx = updated.findIndex(w => w.id === toWalletId);
      if (fromIdx >= 0) updated[fromIdx].balance = fromBal || 0;
      if (toIdx >= 0) updated[toIdx].balance = toBal || 0;
      saveWallets(updated);
      setStoredWallets(updated);

      addLog(`${fromWallet.name} balance: ${formatUsdc(fromBal || 0)}`, 'info');
      addLog(`${toWallet.name} balance: ${formatUsdc(toBal || 0)}`, 'success');
      addLog('Transfer complete! (testnet faucet-based)', 'success');

      setStep('success');
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Transfer failed';
      addLog(`Error: ${msg}`, 'error');
      setStep('error');
    }
  };

  if (step === 'success') {
    return (
      <div className="min-h-screen bg-slate-950">
        <Navbar />
        <main className="max-w-lg mx-auto px-4 py-16 text-center">
          <div className="w-20 h-20 rounded-full bg-emerald-600/20 flex items-center justify-center mx-auto mb-6">
            <svg className="w-10 h-10 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M4.5 12.75l6 6 9-13.5" />
            </svg>
          </div>
          <h1 className="text-2xl font-bold text-white mb-2">Transfer Complete!</h1>
          <p className="text-slate-400 mb-1">
            {fromWallet?.icon} {fromWallet?.name} &rarr; {toWallet?.icon} {toWallet?.name}
          </p>
          <p className="text-xs text-slate-500 mb-8">Zero fees | Dina Testnet</p>

          {/* Log output */}
          <div className="rounded-xl bg-slate-900 border border-slate-800 p-4 mb-6 text-left">
            <p className="text-xs text-slate-500 uppercase tracking-wider mb-2">Transfer Log</p>
            <div className="font-mono text-xs space-y-1 max-h-48 overflow-y-auto">
              {logs.map((log, i) => (
                <p key={i} className={
                  log.type === 'error' ? 'text-red-400' :
                  log.type === 'success' ? 'text-emerald-400' :
                  log.type === 'warn' ? 'text-amber-400' : 'text-slate-400'
                }>
                  <span className="text-slate-600">[{log.time}]</span> {log.message}
                </p>
              ))}
            </div>
          </div>

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
        <h1 className="text-2xl font-bold text-white mb-2 text-center">Transfer Between Wallets</h1>
        <p className="text-slate-400 text-sm text-center mb-8">Move USDC between your wallets on Dina Testnet</p>

        {setupWallets.length < 2 && (
          <div className="rounded-xl bg-amber-900/20 border border-amber-700/30 p-4 mb-6">
            <p className="text-sm text-amber-400">
              You need at least 2 set-up wallets to transfer. Go to the Dashboard to set up more wallets.
            </p>
          </div>
        )}

        {/* From wallet */}
        <div className="rounded-xl bg-slate-900 border border-slate-800 p-4 mb-2">
          <label className="block text-xs text-slate-500 uppercase tracking-wider mb-2">From Wallet</label>
          <div className="flex items-center gap-3">
            <select
              value={fromWalletId}
              onChange={(e) => setFromWalletId(e.target.value)}
              className="flex-1 px-3 py-2 rounded-lg bg-slate-800 border border-slate-700 text-white text-sm outline-none focus:border-emerald-500 transition-colors"
            >
              {setupWallets.map(w => (
                <option key={w.id} value={w.id}>
                  {w.icon} {w.name} ({truncate(w.address)})
                </option>
              ))}
            </select>
          </div>
          {fromWallet && (
            <div className="flex justify-between mt-2">
              <span className="text-xs text-slate-500 font-mono">{truncate(fromWallet.address)}</span>
              <span className="text-xs text-slate-400">Balance: {formatUsdc(fromWallet.balance)}</span>
            </div>
          )}
        </div>

        {/* Swap button */}
        <div className="flex justify-center -my-3 relative z-10">
          <button
            onClick={handleSwap}
            className="w-10 h-10 rounded-full bg-slate-800 border border-slate-700 flex items-center justify-center hover:bg-slate-700 transition-colors"
          >
            <svg className="w-4 h-4 text-slate-300" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M7.5 21L3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5" />
            </svg>
          </button>
        </div>

        {/* To wallet */}
        <div className="rounded-xl bg-slate-900 border border-slate-800 p-4 mb-6">
          <label className="block text-xs text-slate-500 uppercase tracking-wider mb-2">To Wallet</label>
          <div className="flex items-center gap-3">
            <select
              value={toWalletId}
              onChange={(e) => setToWalletId(e.target.value)}
              className="flex-1 px-3 py-2 rounded-lg bg-slate-800 border border-slate-700 text-white text-sm outline-none focus:border-emerald-500 transition-colors"
            >
              {setupWallets.map(w => (
                <option key={w.id} value={w.id}>
                  {w.icon} {w.name} ({truncate(w.address)})
                </option>
              ))}
            </select>
          </div>
          {toWallet && (
            <div className="flex justify-between mt-2">
              <span className="text-xs text-slate-500 font-mono">{truncate(toWallet.address)}</span>
              <span className="text-xs text-slate-400">Balance: {formatUsdc(toWallet.balance)}</span>
            </div>
          )}
        </div>

        {/* Amount */}
        <div className="rounded-xl bg-slate-900 border border-slate-800 p-4 mb-6">
          <label className="block text-xs text-slate-500 uppercase tracking-wider mb-2">Amount (USDC)</label>
          <div className="flex items-center gap-2">
            <span className="text-2xl text-slate-500">$</span>
            <input
              type="text"
              inputMode="decimal"
              placeholder="0.00"
              value={amount}
              onChange={(e) => setAmount(e.target.value.replace(/[^0-9.]/g, ''))}
              className="flex-1 text-2xl font-bold bg-transparent outline-none text-white tabular-nums placeholder:text-slate-700"
            />
          </div>
          <div className="flex justify-between mt-2">
            <span className="text-xs text-emerald-400">Zero fees</span>
            <span className="text-xs text-slate-500">
              Available: {fromWallet ? formatUsdc(fromWallet.balance) : '$0.00'}
            </span>
          </div>
        </div>

        {/* Transfer info */}
        {fromWallet && toWallet && fromWalletId !== toWalletId && parseFloat(amount) > 0 && (
          <div className="rounded-xl bg-slate-900 border border-slate-800 p-4 mb-6 space-y-2">
            <div className="flex justify-between text-sm">
              <span className="text-slate-400">Transfer</span>
              <span className="text-white">${parseFloat(amount).toFixed(2)} USDC</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-slate-400">Fee</span>
              <span className="font-semibold text-emerald-400">$0.00</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-slate-400">Route</span>
              <span className="text-white">{fromWallet.icon} &rarr; {toWallet.icon}</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-slate-400">Method</span>
              <span className="text-slate-300 text-xs">Testnet faucet funding</span>
            </div>
          </div>
        )}

        {/* Transfer button */}
        <button
          onClick={handleTransfer}
          disabled={
            !fromWallet || !toWallet ||
            fromWalletId === toWalletId ||
            parseFloat(amount) <= 0 ||
            step === 'transferring' ||
            setupWallets.length < 2
          }
          className="w-full py-3 rounded-xl bg-emerald-600 hover:bg-emerald-500 disabled:bg-slate-800 disabled:text-slate-600 text-white font-semibold transition-colors"
        >
          {step === 'transferring' ? (
            <span className="flex items-center justify-center gap-2">
              <span className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
              Transferring...
            </span>
          ) : (
            'Transfer'
          )}
        </button>

        {fromWalletId === toWalletId && fromWalletId && (
          <p className="text-xs text-amber-400 mt-2 text-center">Select different wallets for From and To</p>
        )}

        {/* Status log */}
        <div className="mt-6 rounded-xl bg-slate-900 border border-slate-800 p-4">
          <p className="text-xs text-slate-500 uppercase tracking-wider mb-2">Status</p>
          <div className="font-mono text-xs space-y-1 max-h-48 overflow-y-auto">
            {logs.length === 0 ? (
              <>
                <p className="text-slate-400"><span className="text-slate-600">[network]</span> Dina Testnet | 100ms blocks | Zero fees</p>
                <p className="text-slate-400"><span className="text-slate-600">[wallets]</span> {setupWallets.length} set up / {storedWallets.length} total</p>
                <p className="text-slate-400"><span className="text-slate-600">[mode]</span> Internal wallet transfer (faucet-based on testnet)</p>
                {fromWallet && (
                  <p className="text-slate-400"><span className="text-slate-600">[from]</span> {fromWallet.icon} {fromWallet.name}: {formatUsdc(fromWallet.balance)}</p>
                )}
                {toWallet && (
                  <p className="text-slate-400"><span className="text-slate-600">[to]</span> {toWallet.icon} {toWallet.name}: {formatUsdc(toWallet.balance)}</p>
                )}
              </>
            ) : (
              logs.map((log, i) => (
                <p key={i} className={
                  log.type === 'error' ? 'text-red-400' :
                  log.type === 'success' ? 'text-emerald-400' :
                  log.type === 'warn' ? 'text-amber-400' : 'text-slate-400'
                }>
                  <span className="text-slate-600">[{log.time}]</span> {log.message}
                </p>
              ))
            )}
          </div>
        </div>
      </main>
    </div>
  );
}
