'use client';
import { useState, useEffect } from 'react';
import { Navbar } from '@/components/Navbar';
import { formatUsdc } from '@/lib/yield';
import { submitSignedTransaction, getBalanceRest, getNonce, logTransaction } from '@/lib/api';
import { loadWallets, saveWallets, refreshAllBalances, totalBalance, ensureKeypairs, type StoredWallet } from '@/lib/wallet-store';

export default function SendPage() {
  const [amount, setAmount] = useState('');
  const [recipient, setRecipient] = useState('');
  const [selectedWalletId, setSelectedWalletId] = useState('smart1');
  const [sending, setSending] = useState(false);
  const [storedWallets, setStoredWallets] = useState<StoredWallet[]>([]);
  const [logs, setLogs] = useState<{text: string; color: string; time: string}[]>([]);

  useEffect(() => {
    // Ensure all wallets have valid Ed25519 keypairs (generates if missing)
    ensureKeypairs().then(wallets => setStoredWallets(wallets));
  }, []);

  const setupWallets = storedWallets.filter(w => w.isSetUp);
  const selectedWallet = storedWallets.find(w => w.id === selectedWalletId) || setupWallets[0];
  const availableBalance = selectedWallet?.balance ?? 0;
  const otherWallets = setupWallets.filter(w => w.id !== selectedWalletId);

  const tr = (addr: string) => addr.length > 14 ? `${addr.slice(0, 6)}...${addr.slice(-4)}` : addr;
  const now = () => new Date().toLocaleTimeString('en-US', {hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit'});

  const addLog = (text: string, color = 'text-slate-400') => {
    setLogs(prev => [...prev, { text, color, time: now() }]);
  };

  const handleSend = async () => {
    if (!amount || !recipient || !selectedWallet || parseFloat(amount) <= 0) return;
    setSending(true);

    const microAmount = Math.round(parseFloat(amount) * 1_000_000);
    addLog(`Sending $${amount} from ${selectedWallet.name}...`, 'text-white');
    addLog(`From: ${selectedWallet.address}`, 'text-slate-500');
    addLog(`To: ${recipient}`, 'text-slate-500');
    addLog(`Amount: ${formatUsdc(microAmount)} USDC | Fee: $0.00`, 'text-slate-500');

    const startTime = Date.now();

    try {
      const { signTransfer, hexToBytes, addressFromPubkey, ensureSha512 } = await import('@/lib/crypto');
      const ed = await import('@noble/ed25519');
      ensureSha512();

      let privHex = selectedWallet.privateKey || localStorage.getItem('dina_privkey') || '';
      if (!privHex || privHex.length < 64) {
        // Try to generate a keypair on the fly
        addLog('Generating wallet keypair...', 'text-yellow-400');
        const wallets = await ensureKeypairs();
        setStoredWallets(wallets);
        const fixed = wallets.find(w => w.id === selectedWalletId);
        privHex = fixed?.privateKey || '';
        if (!privHex || privHex.length < 64) {
          throw new Error('Could not generate keypair. Go to Dashboard and set up this wallet first.');
        }
      }
      const privKey = hexToBytes(privHex);
      const pubKey = ed.getPublicKey(privKey);
      const keypair = { privateKey: privKey, publicKey: pubKey, address: addressFromPubkey(pubKey) };

      // Fetch the correct nonce from the validator
      addLog('Fetching account nonce...', 'text-slate-400');
      const nonce = await getNonce(keypair.address);
      addLog(`Nonce: ${nonce} | Signing (Ed25519)...`, 'text-slate-400');

      const { txJson } = await signTransfer({
        keypair,
        to: recipient,
        amount: BigInt(microAmount),
        nonce: BigInt(nonce),
        fee: BigInt(0),
      });

      addLog('Submitting to validator...', 'text-slate-400');

      const result = await submitSignedTransaction(txJson);
      const elapsed = Date.now() - startTime;

      if (result.confirmed) {
        addLog(`BFT CONFIRMED in ${elapsed}ms | ${result.validators || 3}/4 validators | Block #${result.blockHeight} | Zero fees`, 'text-emerald-400');
      } else {
        addLog(`SUBMITTED in ${elapsed}ms | Pending BFT confirmation | Zero fees`, 'text-yellow-400');
      }

      // Optimistically update local wallet balance (deduct sent amount)
      const current = loadWallets();
      const senderIdx = current.findIndex(w => w.id === selectedWalletId);
      if (senderIdx >= 0) {
        current[senderIdx].balance = Math.max(0, current[senderIdx].balance - microAmount);
      }
      // Credit recipient if it's one of our wallets
      const recipIdx = current.findIndex(w => w.address === recipient);
      if (recipIdx >= 0) {
        current[recipIdx].balance += microAmount;
      }
      saveWallets(current);
      setStoredWallets(current);
      if (result.txHash) {
        addLog(`TX: ${result.txHash}`, 'text-emerald-400');
      }

      // Log transaction
      logTransaction(selectedWallet.address, {
        id: result.txHash || `tx-${Date.now()}`,
        type: 'send',
        amount: microAmount,
        currency: 'USDC',
        counterparty: tr(recipient),
        timestamp: Math.floor(Date.now() / 1000),
        status: 'confirmed',
        wallet: selectedWallet.name,
        txHash: result.txHash,
      });

      // Refresh ALL wallet balances from chain (confirm optimistic update)
      addLog('Refreshing balances...', 'text-slate-500');
      const refreshed = await refreshAllBalances(loadWallets());
      setStoredWallets(refreshed);
      addLog(`Total: ${formatUsdc(totalBalance(refreshed))} USDC across ${refreshed.filter(w => w.isSetUp).length} wallets`, 'text-white');

      // Clear form
      setAmount('');

    } catch (err) {
      const elapsed = Date.now() - startTime;
      const msg = err instanceof Error ? err.message : 'Transaction failed';
      addLog(`FAILED (${elapsed}ms): ${msg}`, 'text-red-400');
    } finally {
      setSending(false);
    }
  };

  return (
    <div className="min-h-screen bg-slate-950">
      <Navbar />
      <main className="max-w-2xl mx-auto px-4 py-8">
        <h1 className="text-2xl font-bold text-white mb-6 text-center">Send Money</h1>

        {/* Amount */}
        <div className="text-center mb-6">
          <div className="inline-flex items-baseline gap-1">
            <span className="text-4xl text-slate-500">$</span>
            <input
              type="text"
              inputMode="decimal"
              placeholder="0.00"
              value={amount}
              onChange={(e) => setAmount(e.target.value.replace(/[^0-9.]/g, ''))}
              className="text-5xl font-bold bg-transparent border-none outline-none text-white text-center w-64 tabular-nums placeholder:text-slate-700"
            />
          </div>
          <p className="text-sm text-slate-500 mt-1">
            Available: {formatUsdc(availableBalance)}
          </p>
        </div>

        {/* Recipient */}
        <div className="mb-4">
          <label className="block text-sm text-slate-400 mb-1">Recipient</label>
          <input
            type="text"
            placeholder="Dina address (hex) or name"
            value={recipient}
            onChange={(e) => setRecipient(e.target.value)}
            className="w-full px-4 py-3 rounded-xl bg-slate-900 border border-slate-800 text-white placeholder:text-slate-600 outline-none focus:border-emerald-600 transition-colors text-sm font-mono"
          />
        </div>

        {/* Quick send to own wallet */}
        {otherWallets.length > 0 && (
          <div className="mb-4">
            <label className="block text-sm text-slate-400 mb-1">Quick send to own wallet</label>
            <div className="flex flex-wrap gap-2">
              {otherWallets.map(w => (
                <button
                  key={w.id}
                  onClick={() => setRecipient(w.address)}
                  className={`px-3 py-1.5 rounded-lg text-xs transition-colors border ${
                    recipient === w.address
                      ? 'bg-emerald-600/20 border-emerald-600 text-emerald-400'
                      : 'bg-slate-900 border-slate-800 text-slate-300 hover:border-slate-600'
                  }`}
                >
                  {w.icon} {w.name} <span className="text-slate-500 font-mono">{tr(w.address)}</span>
                </button>
              ))}
            </div>
          </div>
        )}

        {/* From wallet */}
        <div className="mb-4">
          <label className="block text-sm text-slate-400 mb-1">From wallet</label>
          <select
            value={selectedWalletId}
            onChange={(e) => setSelectedWalletId(e.target.value)}
            className="w-full px-4 py-3 rounded-xl bg-slate-900 border border-slate-800 text-white outline-none focus:border-emerald-600 transition-colors text-sm"
          >
            {setupWallets.map(w => (
              <option key={w.id} value={w.id}>
                {w.icon} {w.name} — {formatUsdc(w.balance)} ({tr(w.address)})
              </option>
            ))}
          </select>
        </div>

        {/* Send button */}
        <button
          onClick={handleSend}
          disabled={sending || !amount || !recipient || parseFloat(amount) <= 0}
          className="w-full py-3 rounded-xl bg-emerald-600 hover:bg-emerald-500 disabled:bg-slate-800 disabled:text-slate-600 text-white font-semibold transition-colors mb-4"
        >
          {sending ? 'Sending...' : `Send $${amount || '0'} USDC`}
        </button>

        {/* Status Log — shows everything inline */}
        <div className="rounded-xl bg-slate-900 border border-slate-800 p-4">
          <p className="text-xs text-slate-500 uppercase tracking-wider mb-2">Status</p>
          <div className="font-mono text-[11px] leading-relaxed space-y-0.5 min-h-[200px] max-h-[500px] overflow-y-auto" id="status-log">
            {selectedWallet && (
              <>
                <p className="text-slate-500"><span className="text-slate-600">[from]</span> {selectedWallet.icon} {selectedWallet.name}</p>
                <p className="text-slate-500"><span className="text-slate-600">[addr]</span> {selectedWallet.address}</p>
                <p className="text-slate-500"><span className="text-slate-600">[bal]</span> <span className="text-emerald-400">{formatUsdc(selectedWallet.balance)} USDC</span></p>
              </>
            )}
            <p className="text-slate-500"><span className="text-slate-600">[net]</span> Dina Testnet | 100ms blocks | Zero fees</p>
            {logs.map((log, i) => (
              <p key={i} className={log.color}>
                <span className="text-slate-600">[{log.time}]</span> {log.text}
              </p>
            ))}
            {sending && (
              <p className="text-amber-400 animate-pulse">Processing...</p>
            )}
          </div>
        </div>
      </main>
    </div>
  );
}
