'use client';
import { useState, useEffect } from 'react';
import { Navbar } from '@/components/Navbar';
import { loadWallets, type StoredWallet } from '@/lib/wallet-store';
import { formatUsdc } from '@/lib/yield';

function QrPlaceholder({ value }: { value: string }) {
  // Generate a deterministic SVG pattern from the address string
  const size = 21;
  const cells: boolean[][] = [];
  let hash = 0;
  for (let i = 0; i < value.length; i++) {
    hash = ((hash << 5) - hash + value.charCodeAt(i)) | 0;
  }
  for (let y = 0; y < size; y++) {
    cells[y] = [];
    for (let x = 0; x < size; x++) {
      // Fixed finder patterns (top-left, top-right, bottom-left)
      const inFinderTL = x < 7 && y < 7;
      const inFinderTR = x >= size - 7 && y < 7;
      const inFinderBL = x < 7 && y >= size - 7;
      if (inFinderTL || inFinderTR || inFinderBL) {
        const fx = inFinderTR ? x - (size - 7) : x;
        const fy = inFinderBL ? y - (size - 7) : y;
        const isBorder = fx === 0 || fx === 6 || fy === 0 || fy === 6;
        const isInner = fx >= 2 && fx <= 4 && fy >= 2 && fy <= 4;
        cells[y][x] = isBorder || isInner;
      } else {
        hash = ((hash << 5) - hash + (x * 31 + y * 17)) | 0;
        cells[y][x] = (Math.abs(hash) % 3) < 1;
      }
    }
  }

  const cellSize = 8;
  const svgSize = size * cellSize;

  return (
    <svg viewBox={`0 0 ${svgSize} ${svgSize}`} className="w-48 h-48">
      <rect width={svgSize} height={svgSize} fill="white" rx="8"/>
      {cells.map((row, y) =>
        row.map((filled, x) =>
          filled ? (
            <rect
              key={`${x}-${y}`}
              x={x * cellSize}
              y={y * cellSize}
              width={cellSize}
              height={cellSize}
              fill="#0f172a"
            />
          ) : null
        )
      )}
    </svg>
  );
}

export default function ReceivePage() {
  const [copied, setCopied] = useState(false);
  const [storedWallets, setStoredWallets] = useState<StoredWallet[]>([]);
  const [selectedWalletId, setSelectedWalletId] = useState('main');

  useEffect(() => {
    const wallets = loadWallets();
    setStoredWallets(wallets);
    // Default to first set-up wallet
    const firstSetup = wallets.find(w => w.isSetUp);
    if (firstSetup) setSelectedWalletId(firstSetup.id);
  }, []);

  const setupWallets = storedWallets.filter(w => w.isSetUp);
  const selectedWallet = storedWallets.find(w => w.id === selectedWalletId);
  const displayAddress = selectedWallet?.address || 'Loading...';

  const truncate = (addr: string) =>
    addr.length > 14 ? `${addr.slice(0, 6)}...${addr.slice(-4)}` : addr;

  const handleCopy = () => {
    navigator.clipboard.writeText(displayAddress).catch(() => {});
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleShare = () => {
    if (navigator.share) {
      navigator.share({
        title: 'Dina Wallet Address',
        text: `Send USDC to my Dina wallet: ${displayAddress}`,
      }).catch(() => {});
    }
  };

  return (
    <div className="min-h-screen bg-slate-950">
      <Navbar />
      <main className="max-w-lg mx-auto px-4 py-12 text-center">
        <h1 className="text-2xl font-bold text-white mb-2">Receive Money</h1>
        <p className="text-slate-400 mb-6">Share your QR code or address to receive funds</p>

        {/* Wallet selector */}
        {setupWallets.length > 1 && (
          <div className="mb-6 text-left">
            <label className="block text-sm text-slate-400 mb-2">Select wallet</label>
            <select
              value={selectedWalletId}
              onChange={(e) => setSelectedWalletId(e.target.value)}
              className="w-full px-4 py-3 rounded-xl bg-slate-900 border border-slate-800 text-white outline-none focus:border-emerald-600 transition-colors"
            >
              {setupWallets.map(w => (
                <option key={w.id} value={w.id}>
                  {w.icon} {w.name} — {formatUsdc(w.balance)} ({truncate(w.address)})
                </option>
              ))}
            </select>
          </div>
        )}

        {/* QR Code */}
        <div className="inline-block rounded-2xl bg-white p-4 mb-8 shadow-xl shadow-emerald-900/10">
          <QrPlaceholder value={displayAddress} />
        </div>

        {/* Address */}
        <div className="rounded-xl bg-slate-900 border border-slate-800 p-4 mb-6">
          <p className="text-xs text-slate-500 uppercase tracking-wider mb-2">Your address</p>
          <p className="text-sm font-mono text-slate-300 break-all mb-3">
            {displayAddress}
          </p>
          <button
            onClick={handleCopy}
            className="w-full py-2.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-sm font-medium text-white transition-colors"
          >
            {copied ? 'Copied!' : 'Copy Address'}
          </button>
        </div>

        {/* Share */}
        <button
          onClick={handleShare}
          className="w-full py-3 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white font-semibold transition-colors flex items-center justify-center gap-2"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M7.217 10.907a2.25 2.25 0 100 2.186m0-2.186c.18.324.283.696.283 1.093s-.103.77-.283 1.093m0-2.186l9.566-5.314m-9.566 7.5l9.566 5.314m0 0a2.25 2.25 0 103.935 2.186 2.25 2.25 0 00-3.935-2.186zm0-12.814a2.25 2.25 0 103.933-2.185 2.25 2.25 0 00-3.933 2.185z"/>
          </svg>
          Share Link
        </button>

        <p className="text-xs text-slate-600 mt-4">
          Supports USDC on Dina Network testnet
        </p>

        {/* Status box */}
        <div className="mt-6 rounded-xl bg-slate-900 border border-slate-800 p-4 text-left">
          <p className="text-xs text-slate-500 uppercase tracking-wider mb-2">Wallet Info</p>
          <div className="font-mono text-xs text-slate-400 space-y-1">
            {selectedWallet && (
              <>
                <p><span className="text-slate-600">[wallet]</span> {selectedWallet.icon} {selectedWallet.name}</p>
                <p><span className="text-slate-600">[address]</span> {selectedWallet.address}</p>
                <p><span className="text-slate-600">[balance]</span> <span className="text-emerald-400">{formatUsdc(selectedWallet.balance)} USDC</span></p>
                <p><span className="text-slate-600">[type]</span> {selectedWallet.type}</p>
              </>
            )}
            <p><span className="text-slate-600">[network]</span> Dina Testnet | 100ms blocks | Zero fees</p>
            <p><span className="text-slate-600">[wallets]</span> {setupWallets.length} set up / {storedWallets.length} total</p>
          </div>
        </div>
      </main>
    </div>
  );
}
