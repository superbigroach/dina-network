'use client';
import { useEffect, useState, useCallback, useRef } from 'react';
import { Navbar } from '@/components/Navbar';
import { loadWallets, saveWallets, type StoredWallet } from '@/lib/wallet-store';

// ---------------------------------------------------------------------------
// Crypto helpers (Web Crypto API — no external deps)
// ---------------------------------------------------------------------------

async function hashPin(pin: string): Promise<string> {
  const enc = new TextEncoder().encode(pin);
  const hash = await crypto.subtle.digest('SHA-256', enc);
  return Array.from(new Uint8Array(hash))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

async function deriveKey(password: string, salt: Uint8Array): Promise<CryptoKey> {
  const enc = new TextEncoder().encode(password);
  const base = await crypto.subtle.importKey('raw', enc as BufferSource, 'PBKDF2', false, [
    'deriveKey',
  ]);
  return crypto.subtle.deriveKey(
    { name: 'PBKDF2', salt: salt as BufferSource, iterations: 100_000, hash: 'SHA-256' },
    base,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt'],
  );
}

async function encryptData(
  plaintext: string,
  password: string,
): Promise<string> {
  const salt = crypto.getRandomValues(new Uint8Array(16));
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const key = await deriveKey(password, salt);
  const ct = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    key,
    new TextEncoder().encode(plaintext),
  );
  // Pack: salt(16) + iv(12) + ciphertext
  const packed = new Uint8Array(salt.length + iv.length + new Uint8Array(ct).length);
  packed.set(salt, 0);
  packed.set(iv, salt.length);
  packed.set(new Uint8Array(ct), salt.length + iv.length);
  return btoa(String.fromCharCode(...packed));
}

async function decryptData(
  b64: string,
  password: string,
): Promise<string> {
  const packed = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
  const salt = packed.slice(0, 16);
  const iv = packed.slice(16, 28);
  const ct = packed.slice(28);
  const key = await deriveKey(password, salt);
  const pt = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, key, ct);
  return new TextDecoder().decode(pt);
}

// ---------------------------------------------------------------------------
// localStorage keys
// ---------------------------------------------------------------------------
const PIN_HASH_KEY = 'dina_pin_hash';
const SETTINGS_KEY = 'dina_security_settings';

interface SecuritySettings {
  autoLockMinutes: number; // 0 = never
  lastBackup: string | null; // ISO timestamp
  lastActivity: string; // ISO timestamp
}

function loadSettings(): SecuritySettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (raw) return JSON.parse(raw);
  } catch { /* ignore */ }
  return { autoLockMinutes: 5, lastBackup: null, lastActivity: new Date().toISOString() };
}

function saveSettings(s: SecuritySettings) {
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(s));
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

type Modal =
  | null
  | { kind: 'setup-pin' }
  | { kind: 'change-pin' }
  | { kind: 'confirm-pin'; action: 'view-key'; walletId: string }
  | { kind: 'confirm-pin'; action: 'export' }
  | { kind: 'confirm-pin'; action: 'clear-all' }
  | { kind: 'confirm-pin'; action: 'reset-wallets' }
  | { kind: 'import' }
  | { kind: 'lock' };

export default function SecurityPage() {
  const [wallets, setWallets] = useState<StoredWallet[]>([]);
  const [settings, setSettings] = useState<SecuritySettings>(loadSettings);
  const [hasPinHash, setHasPinHash] = useState(false);
  const [modal, setModal] = useState<Modal>(null);
  const [locked, setLocked] = useState(false);

  // PIN input state
  const [pin1, setPin1] = useState('');
  const [pin2, setPin2] = useState('');
  const [pinError, setPinError] = useState('');

  // Visible private key
  const [visibleKey, setVisibleKey] = useState<{ walletId: string; key: string } | null>(null);
  const visibleTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Export password
  const [exportPassword, setExportPassword] = useState('');
  const [exportError, setExportError] = useState('');

  // Import
  const [importFile, setImportFile] = useState<string | null>(null);
  const [importPassword, setImportPassword] = useState('');
  const [importError, setImportError] = useState('');

  // Double-confirm for danger zone
  const [dangerConfirm, setDangerConfirm] = useState(false);

  // Copy feedback
  const [copied, setCopied] = useState<string | null>(null);

  // ---------------------------------------------------------------------------
  // Init
  // ---------------------------------------------------------------------------
  useEffect(() => {
    setWallets(loadWallets());
    setHasPinHash(!!localStorage.getItem(PIN_HASH_KEY));
    const s = loadSettings();
    setSettings(s);
    // Touch last activity
    s.lastActivity = new Date().toISOString();
    saveSettings(s);
  }, []);

  // Auto-lock timer
  useEffect(() => {
    if (settings.autoLockMinutes <= 0 || !hasPinHash) return;
    const ms = settings.autoLockMinutes * 60 * 1000;
    const timer = setTimeout(() => setLocked(true), ms);
    const reset = () => {
      clearTimeout(timer);
    };
    window.addEventListener('mousemove', reset, { once: true });
    window.addEventListener('keydown', reset, { once: true });
    return () => {
      clearTimeout(timer);
      window.removeEventListener('mousemove', reset);
      window.removeEventListener('keydown', reset);
    };
  }, [settings.autoLockMinutes, hasPinHash, locked]);

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  const copyToClipboard = useCallback((text: string, label: string) => {
    navigator.clipboard.writeText(text);
    setCopied(label);
    setTimeout(() => setCopied(null), 2000);
  }, []);

  const clearModal = useCallback(() => {
    setModal(null);
    setPin1('');
    setPin2('');
    setPinError('');
    setExportPassword('');
    setExportError('');
    setImportFile(null);
    setImportPassword('');
    setImportError('');
    setDangerConfirm(false);
  }, []);

  // ---------------------------------------------------------------------------
  // PIN actions
  // ---------------------------------------------------------------------------

  const handleSetPin = async () => {
    if (pin1.length !== 6 || !/^\d{6}$/.test(pin1)) {
      setPinError('PIN must be exactly 6 digits');
      return;
    }
    if (pin1 !== pin2) {
      setPinError('PINs do not match');
      return;
    }
    const h = await hashPin(pin1);
    localStorage.setItem(PIN_HASH_KEY, h);
    setHasPinHash(true);
    clearModal();
  };

  const handleChangePin = async () => {
    // pin1 = current, pin2 = new (we repurpose fields)
    // Actually we need 3 fields. Let's use pin1=current, pin2=new, and re-validate.
    // For simplicity, the "change pin" modal collects old + new + confirm via a mini flow.
    // We'll just reuse pin1=old, pin2=new for this implementation.
    const storedHash = localStorage.getItem(PIN_HASH_KEY) || '';
    const oldHash = await hashPin(pin1);
    if (oldHash !== storedHash) {
      setPinError('Current PIN is incorrect');
      return;
    }
    if (pin2.length !== 6 || !/^\d{6}$/.test(pin2)) {
      setPinError('New PIN must be exactly 6 digits');
      return;
    }
    const h = await hashPin(pin2);
    localStorage.setItem(PIN_HASH_KEY, h);
    clearModal();
  };

  const verifyPin = async (): Promise<boolean> => {
    const storedHash = localStorage.getItem(PIN_HASH_KEY) || '';
    const h = await hashPin(pin1);
    if (h !== storedHash) {
      setPinError('Incorrect PIN');
      return false;
    }
    return true;
  };

  // ---------------------------------------------------------------------------
  // Confirm-pin gated actions
  // ---------------------------------------------------------------------------

  const handleConfirmAction = async () => {
    if (!modal || modal.kind !== 'confirm-pin') return;
    const ok = await verifyPin();
    if (!ok) return;

    switch (modal.action) {
      case 'view-key': {
        const w = wallets.find((w) => w.id === modal.walletId);
        if (w) {
          setVisibleKey({ walletId: w.id, key: w.privateKey });
          if (visibleTimer.current) clearTimeout(visibleTimer.current);
          visibleTimer.current = setTimeout(() => setVisibleKey(null), 10_000);
        }
        clearModal();
        break;
      }
      case 'export': {
        if (!exportPassword || exportPassword.length < 4) {
          setExportError('Password must be at least 4 characters');
          return;
        }
        try {
          const payload = wallets.map((w) => ({
            id: w.id,
            name: w.name,
            type: w.type,
            address: w.address,
            privateKey: w.privateKey,
            isSetUp: w.isSetUp,
          }));
          const encrypted = await encryptData(JSON.stringify(payload), exportPassword);
          const blob = new Blob(
            [JSON.stringify({ version: 1, encrypted, exportedAt: new Date().toISOString() }, null, 2)],
            { type: 'application/json' },
          );
          const url = URL.createObjectURL(blob);
          const a = document.createElement('a');
          a.href = url;
          a.download = `dina-wallet-backup-${Date.now()}.json`;
          a.click();
          URL.revokeObjectURL(url);
          const s = { ...settings, lastBackup: new Date().toISOString() };
          setSettings(s);
          saveSettings(s);
        } catch (e) {
          setExportError('Export failed');
        }
        clearModal();
        break;
      }
      case 'clear-all': {
        if (!dangerConfirm) {
          setDangerConfirm(true);
          setPinError('');
          return;
        }
        localStorage.clear();
        window.location.reload();
        break;
      }
      case 'reset-wallets': {
        if (!dangerConfirm) {
          setDangerConfirm(true);
          setPinError('');
          return;
        }
        localStorage.removeItem('dina_wallets');
        localStorage.removeItem('dina_address');
        localStorage.removeItem('dina_privkey');
        localStorage.removeItem('dina_balance_ts');
        setWallets(loadWallets());
        clearModal();
        break;
      }
    }
  };

  // ---------------------------------------------------------------------------
  // Import
  // ---------------------------------------------------------------------------

  const handleImport = async () => {
    if (!importFile || !importPassword) {
      setImportError('Please select a file and enter the password');
      return;
    }
    try {
      const parsed = JSON.parse(importFile);
      if (!parsed.encrypted) {
        setImportError('Invalid backup file');
        return;
      }
      const decrypted = await decryptData(parsed.encrypted, importPassword);
      const imported = JSON.parse(decrypted) as Array<{
        id: string;
        name: string;
        type: string;
        address: string;
        privateKey: string;
        isSetUp: boolean;
      }>;
      // Merge into existing wallets
      const current = loadWallets();
      for (const imp of imported) {
        const idx = current.findIndex((w) => w.id === imp.id);
        if (idx >= 0 && imp.isSetUp && imp.address && imp.privateKey) {
          current[idx].address = imp.address;
          current[idx].privateKey = imp.privateKey;
          current[idx].isSetUp = true;
        }
      }
      saveWallets(current);
      setWallets(current);
      clearModal();
    } catch {
      setImportError('Decryption failed — wrong password or corrupt file');
    }
  };

  // ---------------------------------------------------------------------------
  // Lock screen
  // ---------------------------------------------------------------------------

  const handleUnlock = async () => {
    const ok = await verifyPin();
    if (!ok) return;
    setLocked(false);
    clearModal();
    const s = { ...settings, lastActivity: new Date().toISOString() };
    setSettings(s);
    saveSettings(s);
  };

  if (locked) {
    return (
      <div className="min-h-screen bg-slate-950 flex items-center justify-center">
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-8 w-full max-w-sm text-center">
          <div className="text-4xl mb-4">🔒</div>
          <h2 className="text-xl font-bold text-white mb-2">Wallet Locked</h2>
          <p className="text-slate-400 text-sm mb-6">Enter your 6-digit PIN to unlock</p>
          <input
            type="password"
            inputMode="numeric"
            maxLength={6}
            value={pin1}
            onChange={(e) => {
              setPin1(e.target.value.replace(/\D/g, '').slice(0, 6));
              setPinError('');
            }}
            placeholder="------"
            className="w-full bg-slate-800 border border-slate-700 rounded-lg px-4 py-3 text-center text-white text-2xl tracking-[0.5em] placeholder:tracking-[0.3em] placeholder:text-slate-600 focus:outline-none focus:border-emerald-500 mb-3"
          />
          {pinError && <p className="text-red-400 text-xs mb-3">{pinError}</p>}
          <button
            onClick={handleUnlock}
            disabled={pin1.length !== 6}
            className="w-full py-3 rounded-lg bg-emerald-600 hover:bg-emerald-500 disabled:bg-slate-700 disabled:text-slate-500 text-white font-medium transition-colors"
          >
            Unlock
          </button>
        </div>
      </div>
    );
  }

  // ---------------------------------------------------------------------------
  // Security score
  // ---------------------------------------------------------------------------

  const setupCount = wallets.filter((w) => w.isSetUp).length;
  const checks = [
    { label: 'PIN set', done: hasPinHash },
    { label: 'Backup created', done: !!settings.lastBackup },
    { label: 'Auto-lock enabled', done: settings.autoLockMinutes > 0 },
  ];
  const score = checks.filter((c) => c.done).length;
  const totalChecks = checks.length;

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  return (
    <div className="min-h-screen bg-slate-950 text-white">
      <Navbar />

      <main className="max-w-4xl mx-auto px-4 py-8 space-y-6">
        <h1 className="text-2xl font-bold">Security</h1>

        {/* ---- PIN Setup Banner ---- */}
        {!hasPinHash && (
          <div className="bg-yellow-900/30 border border-yellow-700 rounded-xl p-5">
            <div className="flex items-start gap-3">
              <span className="text-2xl">⚠️</span>
              <div className="flex-1">
                <h2 className="text-yellow-300 font-semibold mb-1">Set up a PIN</h2>
                <p className="text-yellow-200/70 text-sm mb-3">
                  A 6-digit PIN is required to view private keys, export backups, and use security features.
                </p>
                <button
                  onClick={() => setModal({ kind: 'setup-pin' })}
                  className="px-4 py-2 rounded-lg bg-yellow-600 hover:bg-yellow-500 text-white text-sm font-medium transition-colors"
                >
                  Set Up PIN
                </button>
              </div>
            </div>
          </div>
        )}

        {/* ---- Security Status Dashboard ---- */}
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-5">
          <h2 className="text-lg font-semibold mb-4">Security Status</h2>
          <div className="grid grid-cols-2 sm:grid-cols-3 gap-4 mb-4">
            {checks.map((c) => (
              <div key={c.label} className="flex items-center gap-2 text-sm">
                <span className={c.done ? 'text-emerald-400' : 'text-slate-500'}>
                  {c.done ? '✅' : '❌'}
                </span>
                <span className={c.done ? 'text-white' : 'text-slate-400'}>{c.label}</span>
              </div>
            ))}
          </div>
          <div className="flex items-center justify-between text-xs text-slate-400 border-t border-slate-800 pt-3">
            <span>
              Wallets set up: <span className="text-white font-medium">{setupCount}/9</span>
            </span>
            <span>
              Score: <span className="text-emerald-400 font-medium">{score}/{totalChecks}</span>
            </span>
            <span>
              Last activity:{' '}
              <span className="text-white">
                {settings.lastActivity
                  ? new Date(settings.lastActivity).toLocaleString()
                  : '—'}
              </span>
            </span>
          </div>
        </div>

        {/* ---- Wallet Keys Management ---- */}
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-5">
          <h2 className="text-lg font-semibold mb-1">Wallet Keys</h2>
          <p className="text-xs text-red-400 mb-4">
            Never share your private keys. Anyone with your private key has full control of that wallet.
          </p>
          <div className="space-y-3">
            {wallets.map((w) => (
              <div
                key={w.id}
                className="bg-slate-800/50 border border-slate-700/50 rounded-lg p-3"
              >
                <div className="flex items-center justify-between mb-1">
                  <div className="flex items-center gap-2">
                    <span className="text-lg">{w.icon}</span>
                    <div>
                      <span className="text-sm font-medium text-white">{w.name}</span>
                      <span className="ml-2 text-xs text-slate-500 capitalize">{w.type}</span>
                    </div>
                  </div>
                  <span
                    className={`text-xs px-2 py-0.5 rounded-full ${
                      w.isSetUp
                        ? 'bg-emerald-900/40 text-emerald-400'
                        : 'bg-slate-700 text-slate-400'
                    }`}
                  >
                    {w.isSetUp ? 'Active' : 'Not set up'}
                  </span>
                </div>

                {w.isSetUp && w.address && (
                  <>
                    <div className="flex items-center gap-2 mt-2">
                      <code className="text-xs text-slate-300 truncate flex-1 bg-slate-900 rounded px-2 py-1">
                        {w.address}
                      </code>
                      <button
                        onClick={() => copyToClipboard(w.address, `addr-${w.id}`)}
                        className="text-xs px-2 py-1 rounded bg-slate-700 hover:bg-slate-600 text-slate-300 transition-colors whitespace-nowrap"
                      >
                        {copied === `addr-${w.id}` ? 'Copied!' : 'Copy Address'}
                      </button>
                    </div>

                    {visibleKey?.walletId === w.id ? (
                      <div className="mt-2">
                        <div className="flex items-center gap-2">
                          <code className="text-xs text-red-300 truncate flex-1 bg-red-950/30 border border-red-900/50 rounded px-2 py-1">
                            {visibleKey.key}
                          </code>
                          <button
                            onClick={() => copyToClipboard(visibleKey.key, `key-${w.id}`)}
                            className="text-xs px-2 py-1 rounded bg-red-900/40 hover:bg-red-800/50 text-red-300 transition-colors whitespace-nowrap"
                          >
                            {copied === `key-${w.id}` ? 'Copied!' : 'Copy Key'}
                          </button>
                          <button
                            onClick={() => {
                              setVisibleKey(null);
                              if (visibleTimer.current) clearTimeout(visibleTimer.current);
                            }}
                            className="text-xs px-2 py-1 rounded bg-slate-700 hover:bg-slate-600 text-slate-300 transition-colors"
                          >
                            Hide
                          </button>
                        </div>
                        <p className="text-xs text-red-400/60 mt-1">
                          Auto-hides in 10 seconds
                        </p>
                      </div>
                    ) : (
                      <button
                        onClick={() => {
                          if (!hasPinHash) {
                            setModal({ kind: 'setup-pin' });
                            return;
                          }
                          setModal({ kind: 'confirm-pin', action: 'view-key', walletId: w.id });
                        }}
                        className="mt-2 text-xs px-3 py-1.5 rounded bg-slate-700 hover:bg-slate-600 text-slate-300 transition-colors"
                      >
                        View Private Key
                      </button>
                    )}
                  </>
                )}
              </div>
            ))}
          </div>
        </div>

        {/* ---- Backup & Recovery ---- */}
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-5">
          <h2 className="text-lg font-semibold mb-1">Backup & Recovery</h2>
          <p className="text-xs text-slate-400 mb-4">
            Export an encrypted backup of all wallet keys. You will need the password to restore.
          </p>

          <div className="flex flex-wrap gap-3 mb-4">
            <button
              onClick={() => {
                if (!hasPinHash) {
                  setModal({ kind: 'setup-pin' });
                  return;
                }
                setModal({ kind: 'confirm-pin', action: 'export' });
              }}
              className="px-4 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-sm font-medium transition-colors"
            >
              Export All Keys
            </button>
            <button
              onClick={() => setModal({ kind: 'import' })}
              className="px-4 py-2 rounded-lg bg-slate-700 hover:bg-slate-600 text-white text-sm font-medium transition-colors"
            >
              Import Keys
            </button>
          </div>

          <div className="text-xs text-slate-400 space-y-1">
            <p>
              Last backed up:{' '}
              <span className="text-white">
                {settings.lastBackup
                  ? new Date(settings.lastBackup).toLocaleString()
                  : 'Never'}
              </span>
            </p>
            <p className="text-slate-500 italic">
              Recovery phrase will be available on mainnet.
            </p>
          </div>
        </div>

        {/* ---- Session Security ---- */}
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-5">
          <h2 className="text-lg font-semibold mb-4">Session Security</h2>

          <div className="flex flex-col sm:flex-row sm:items-center gap-4 mb-4">
            <div className="flex-1">
              <label className="text-xs text-slate-400 block mb-1">Auto-lock timeout</label>
              <select
                value={settings.autoLockMinutes}
                onChange={(e) => {
                  const s = { ...settings, autoLockMinutes: Number(e.target.value) };
                  setSettings(s);
                  saveSettings(s);
                }}
                className="w-full sm:w-48 bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-emerald-500"
              >
                <option value={1}>1 minute</option>
                <option value={5}>5 minutes</option>
                <option value={15}>15 minutes</option>
                <option value={30}>30 minutes</option>
                <option value={0}>Never</option>
              </select>
            </div>

            <button
              onClick={() => {
                if (!hasPinHash) {
                  setModal({ kind: 'setup-pin' });
                  return;
                }
                setLocked(true);
                setPin1('');
                setPinError('');
              }}
              className="px-4 py-2 rounded-lg bg-slate-700 hover:bg-slate-600 text-white text-sm font-medium transition-colors self-start"
            >
              Lock Now
            </button>
          </div>
        </div>

        {/* ---- PIN Management ---- */}
        {hasPinHash && (
          <div className="bg-slate-900 border border-slate-800 rounded-xl p-5">
            <h2 className="text-lg font-semibold mb-4">PIN Management</h2>
            <button
              onClick={() => setModal({ kind: 'change-pin' })}
              className="px-4 py-2 rounded-lg bg-slate-700 hover:bg-slate-600 text-white text-sm font-medium transition-colors"
            >
              Change PIN
            </button>
          </div>
        )}

        {/* ---- Danger Zone ---- */}
        <div className="bg-slate-900 border border-red-900/50 rounded-xl p-5">
          <h2 className="text-lg font-semibold text-red-400 mb-1">Danger Zone</h2>
          <p className="text-xs text-slate-400 mb-4">
            These actions are irreversible. Make sure you have a backup before proceeding.
          </p>

          <div className="flex flex-wrap gap-3">
            <button
              onClick={() => {
                if (!hasPinHash) {
                  setModal({ kind: 'setup-pin' });
                  return;
                }
                setModal({ kind: 'confirm-pin', action: 'reset-wallets' });
              }}
              className="px-4 py-2 rounded-lg bg-red-900/30 border border-red-800 hover:bg-red-900/50 text-red-300 text-sm font-medium transition-colors"
            >
              Reset Wallets
            </button>
            <button
              onClick={() => {
                if (!hasPinHash) {
                  setModal({ kind: 'setup-pin' });
                  return;
                }
                setModal({ kind: 'confirm-pin', action: 'clear-all' });
              }}
              className="px-4 py-2 rounded-lg bg-red-900/30 border border-red-800 hover:bg-red-900/50 text-red-300 text-sm font-medium transition-colors"
            >
              Clear All Data
            </button>
          </div>
        </div>
      </main>

      {/* ====================================================================
          MODALS
         ==================================================================== */}
      {modal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm px-4">
          <div className="bg-slate-900 border border-slate-700 rounded-xl p-6 w-full max-w-md">
            {/* ---- Setup PIN ---- */}
            {modal.kind === 'setup-pin' && (
              <>
                <h3 className="text-lg font-semibold text-white mb-1">Set Up PIN</h3>
                <p className="text-xs text-slate-400 mb-4">
                  Choose a 6-digit PIN. This is required for all sensitive operations.
                </p>
                <label className="text-xs text-slate-400 block mb-1">PIN</label>
                <input
                  type="password"
                  inputMode="numeric"
                  maxLength={6}
                  value={pin1}
                  onChange={(e) => {
                    setPin1(e.target.value.replace(/\D/g, '').slice(0, 6));
                    setPinError('');
                  }}
                  placeholder="6 digits"
                  className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-white text-sm mb-3 focus:outline-none focus:border-emerald-500"
                />
                <label className="text-xs text-slate-400 block mb-1">Confirm PIN</label>
                <input
                  type="password"
                  inputMode="numeric"
                  maxLength={6}
                  value={pin2}
                  onChange={(e) => {
                    setPin2(e.target.value.replace(/\D/g, '').slice(0, 6));
                    setPinError('');
                  }}
                  placeholder="Re-enter PIN"
                  className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-white text-sm mb-3 focus:outline-none focus:border-emerald-500"
                />
                {pinError && <p className="text-red-400 text-xs mb-3">{pinError}</p>}
                <div className="flex gap-3">
                  <button
                    onClick={clearModal}
                    className="flex-1 py-2 rounded-lg bg-slate-700 hover:bg-slate-600 text-white text-sm font-medium transition-colors"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={handleSetPin}
                    className="flex-1 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-sm font-medium transition-colors"
                  >
                    Set PIN
                  </button>
                </div>
              </>
            )}

            {/* ---- Change PIN ---- */}
            {modal.kind === 'change-pin' && (
              <>
                <h3 className="text-lg font-semibold text-white mb-4">Change PIN</h3>
                <label className="text-xs text-slate-400 block mb-1">Current PIN</label>
                <input
                  type="password"
                  inputMode="numeric"
                  maxLength={6}
                  value={pin1}
                  onChange={(e) => {
                    setPin1(e.target.value.replace(/\D/g, '').slice(0, 6));
                    setPinError('');
                  }}
                  placeholder="Current 6-digit PIN"
                  className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-white text-sm mb-3 focus:outline-none focus:border-emerald-500"
                />
                <label className="text-xs text-slate-400 block mb-1">New PIN</label>
                <input
                  type="password"
                  inputMode="numeric"
                  maxLength={6}
                  value={pin2}
                  onChange={(e) => {
                    setPin2(e.target.value.replace(/\D/g, '').slice(0, 6));
                    setPinError('');
                  }}
                  placeholder="New 6-digit PIN"
                  className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-white text-sm mb-3 focus:outline-none focus:border-emerald-500"
                />
                {pinError && <p className="text-red-400 text-xs mb-3">{pinError}</p>}
                <div className="flex gap-3">
                  <button
                    onClick={clearModal}
                    className="flex-1 py-2 rounded-lg bg-slate-700 hover:bg-slate-600 text-white text-sm font-medium transition-colors"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={handleChangePin}
                    className="flex-1 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-sm font-medium transition-colors"
                  >
                    Change PIN
                  </button>
                </div>
              </>
            )}

            {/* ---- Confirm PIN (gated actions) ---- */}
            {modal.kind === 'confirm-pin' && (
              <>
                <h3 className="text-lg font-semibold text-white mb-1">
                  {modal.action === 'view-key' && 'View Private Key'}
                  {modal.action === 'export' && 'Export Wallet Backup'}
                  {modal.action === 'clear-all' && 'Clear All Data'}
                  {modal.action === 'reset-wallets' && 'Reset Wallets'}
                </h3>
                <p className="text-xs text-slate-400 mb-4">Enter your PIN to continue.</p>

                {(modal.action === 'clear-all' || modal.action === 'reset-wallets') && (
                  <div className="bg-red-950/30 border border-red-900/50 rounded-lg p-3 mb-4">
                    <p className="text-xs text-red-300">
                      {modal.action === 'clear-all'
                        ? 'This will permanently erase ALL data including wallets, PIN, and settings. This cannot be undone.'
                        : 'This will delete all wallet data (addresses and keys) but keep your PIN and settings.'}
                    </p>
                  </div>
                )}

                <label className="text-xs text-slate-400 block mb-1">PIN</label>
                <input
                  type="password"
                  inputMode="numeric"
                  maxLength={6}
                  value={pin1}
                  onChange={(e) => {
                    setPin1(e.target.value.replace(/\D/g, '').slice(0, 6));
                    setPinError('');
                  }}
                  placeholder="6-digit PIN"
                  className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-white text-sm mb-3 focus:outline-none focus:border-emerald-500"
                />

                {modal.action === 'export' && (
                  <>
                    <label className="text-xs text-slate-400 block mb-1">
                      Encryption password for backup file
                    </label>
                    <input
                      type="password"
                      value={exportPassword}
                      onChange={(e) => {
                        setExportPassword(e.target.value);
                        setExportError('');
                      }}
                      placeholder="Choose a strong password"
                      className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-white text-sm mb-3 focus:outline-none focus:border-emerald-500"
                    />
                    {exportError && <p className="text-red-400 text-xs mb-3">{exportError}</p>}
                  </>
                )}

                {pinError && <p className="text-red-400 text-xs mb-3">{pinError}</p>}

                {dangerConfirm && (modal.action === 'clear-all' || modal.action === 'reset-wallets') && (
                  <p className="text-yellow-400 text-xs mb-3 font-medium">
                    Press the button again to confirm.
                  </p>
                )}

                <div className="flex gap-3">
                  <button
                    onClick={clearModal}
                    className="flex-1 py-2 rounded-lg bg-slate-700 hover:bg-slate-600 text-white text-sm font-medium transition-colors"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={handleConfirmAction}
                    className={`flex-1 py-2 rounded-lg text-white text-sm font-medium transition-colors ${
                      modal.action === 'clear-all' || modal.action === 'reset-wallets'
                        ? 'bg-red-600 hover:bg-red-500'
                        : 'bg-emerald-600 hover:bg-emerald-500'
                    }`}
                  >
                    {dangerConfirm ? 'Confirm — I understand' : 'Continue'}
                  </button>
                </div>
              </>
            )}

            {/* ---- Import ---- */}
            {modal.kind === 'import' && (
              <>
                <h3 className="text-lg font-semibold text-white mb-1">Import Keys</h3>
                <p className="text-xs text-slate-400 mb-4">
                  Upload a previously exported backup file and enter the password used to encrypt it.
                </p>
                <label className="text-xs text-slate-400 block mb-1">Backup file</label>
                <input
                  type="file"
                  accept=".json"
                  onChange={(e) => {
                    const file = e.target.files?.[0];
                    if (!file) return;
                    const reader = new FileReader();
                    reader.onload = () => setImportFile(reader.result as string);
                    reader.readAsText(file);
                  }}
                  className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-white text-sm mb-3 file:mr-3 file:px-3 file:py-1 file:rounded file:border-0 file:bg-slate-600 file:text-white file:text-xs file:cursor-pointer"
                />
                <label className="text-xs text-slate-400 block mb-1">Decryption password</label>
                <input
                  type="password"
                  value={importPassword}
                  onChange={(e) => {
                    setImportPassword(e.target.value);
                    setImportError('');
                  }}
                  placeholder="Enter backup password"
                  className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-white text-sm mb-3 focus:outline-none focus:border-emerald-500"
                />
                {importError && <p className="text-red-400 text-xs mb-3">{importError}</p>}
                <div className="flex gap-3">
                  <button
                    onClick={clearModal}
                    className="flex-1 py-2 rounded-lg bg-slate-700 hover:bg-slate-600 text-white text-sm font-medium transition-colors"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={handleImport}
                    className="flex-1 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-sm font-medium transition-colors"
                  >
                    Import
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
