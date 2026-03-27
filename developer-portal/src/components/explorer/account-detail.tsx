"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  ArrowUpRight,
  Copy,
  Check,
  ArrowLeft,
  Hash,
  Wallet,
  FileCode2,
  Layers,
  RefreshCw,
} from "lucide-react";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface AccountInfo {
  address: string;
  balance: string;
  nonce: number;
  codeHash: string | null;
}

interface AccountTx {
  hash: string;
  from: string;
  to: string;
  amount: string;
  fee: string;
  status: string;
  blockNumber: number;
  timestamp: string;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const API_BASE = "http://35.184.213.248:8080";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function truncateHash(hash: string, chars = 8): string {
  if (!hash) return "";
  if (hash.length <= chars * 2 + 2) return hash;
  return `${hash.slice(0, chars + 2)}...${hash.slice(-chars)}`;
}

function formatBalance(raw: string): string {
  try {
    const num = parseFloat(raw);
    if (isNaN(num)) return raw;
    // Assume 6 decimal places for USDC
    const formatted = (num / 1_000_000).toFixed(6);
    return `${formatted} USDC`;
  } catch {
    return raw;
  }
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // clipboard not available
    }
  };

  return (
    <button
      onClick={handleCopy}
      className="ml-1.5 inline-flex items-center text-slate-500 hover:text-slate-300 transition-colors"
      title="Copy to clipboard"
    >
      {copied ? (
        <Check className="h-3.5 w-3.5 text-green-400" />
      ) : (
        <Copy className="h-3.5 w-3.5" />
      )}
    </button>
  );
}

function Skeleton({ className = "" }: { className?: string }) {
  return <div className={`animate-pulse rounded bg-slate-800 ${className}`} />;
}

// ---------------------------------------------------------------------------
// Account Detail Page
// ---------------------------------------------------------------------------

export default function AccountDetailPage() {
  const pathname = usePathname();
  const address = pathname.split("/explorer/account/")[1] ?? "";

  const [account, setAccount] = useState<AccountInfo | null>(null);
  const [transactions, setTransactions] = useState<AccountTx[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchAccount = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/accounts/${address}`, {
        signal: AbortSignal.timeout(5000),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const d = await res.json();

      setAccount({
        address: (d.address ?? address) as string,
        balance: String(d.balance ?? d.amount ?? "0"),
        nonce: (d.nonce ?? d.sequence ?? 0) as number,
        codeHash: (d.code_hash ?? d.codeHash ?? null) as string | null,
      });

      // Try fetching transaction history
      try {
        const txRes = await fetch(
          `${API_BASE}/accounts/${address}/transactions`,
          { signal: AbortSignal.timeout(5000) }
        );
        if (txRes.ok) {
          const txData = await txRes.json();
          const txList = Array.isArray(txData)
            ? txData
            : Array.isArray(txData.transactions)
              ? txData.transactions
              : [];
          setTransactions(
            txList.map((tx: Record<string, unknown>) => ({
              hash: (tx.hash ?? tx.tx_hash ?? "") as string,
              from: (tx.from ?? tx.sender ?? "") as string,
              to: (tx.to ?? tx.recipient ?? "") as string,
              amount: String(tx.amount ?? tx.value ?? "0"),
              fee: String(tx.fee ?? tx.gas_used ?? "0"),
              status: (tx.status ?? "success") as string,
              blockNumber: (tx.block_number ?? tx.blockNumber ?? tx.height ?? 0) as number,
              timestamp: (tx.timestamp ?? tx.time ?? "") as string,
            }))
          );
        }
      } catch {
        // Transaction history endpoint may not exist
      }

      setError(null);
    } catch {
      setError("Unable to connect to the network or account not found.");
    } finally {
      setLoading(false);
    }
  }, [address]);

  useEffect(() => {
    fetchAccount();
  }, [fetchAccount]);

  const isContract = account?.codeHash && account.codeHash !== "" && account.codeHash !== "0x";

  return (
    <div className="mx-auto max-w-7xl px-6 py-10">
      {/* Back link */}
      <Link
        href="/explorer"
        className="inline-flex items-center gap-1.5 text-sm text-slate-400 hover:text-slate-200 transition-colors mb-6"
      >
        <ArrowLeft className="h-4 w-4" />
        Back to Explorer
      </Link>

      {/* Header */}
      <div className="flex items-center gap-3 mb-8">
        <div className="h-10 w-10 rounded-xl bg-gradient-to-br from-green-500 to-emerald-600 flex items-center justify-center shadow-lg shadow-green-500/20">
          <Wallet className="h-5 w-5" />
        </div>
        <div className="min-w-0 flex-1">
          <h1 className="text-2xl font-bold tracking-tight">Account</h1>
          <div className="flex items-center gap-1">
            <p className="text-sm text-slate-400 font-mono truncate">{address}</p>
            <CopyButton text={address} />
          </div>
        </div>
        <button
          onClick={() => {
            setLoading(true);
            setError(null);
            fetchAccount();
          }}
          className="rounded-lg border border-slate-700 p-2 text-slate-400 hover:text-white hover:border-slate-600 transition-colors"
          title="Refresh"
        >
          <RefreshCw className="h-4 w-4" />
        </button>
      </div>

      {error ? (
        <div className="rounded-xl border border-red-500/20 bg-red-500/5 p-8 text-center">
          <p className="text-red-400">{error}</p>
          <button
            onClick={() => {
              setLoading(true);
              setError(null);
              fetchAccount();
            }}
            className="mt-4 rounded-lg bg-slate-800 px-4 py-2 text-sm hover:bg-slate-700 transition-colors"
          >
            Retry
          </button>
        </div>
      ) : (
        <>
          {/* Account info cards */}
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-8">
            <div className="rounded-xl border border-slate-800/60 bg-slate-900/50 p-5">
              <div className="flex items-center gap-2 text-sm text-slate-400 mb-2">
                <Wallet className="h-4 w-4" />
                Balance
              </div>
              {loading ? (
                <Skeleton className="h-8 w-32" />
              ) : (
                <p className="text-2xl font-bold tracking-tight">
                  {formatBalance(account?.balance ?? "0")}
                </p>
              )}
            </div>
            <div className="rounded-xl border border-slate-800/60 bg-slate-900/50 p-5">
              <div className="flex items-center gap-2 text-sm text-slate-400 mb-2">
                <Hash className="h-4 w-4" />
                Nonce
              </div>
              {loading ? (
                <Skeleton className="h-8 w-16" />
              ) : (
                <p className="text-2xl font-bold tracking-tight">
                  {account?.nonce ?? 0}
                </p>
              )}
            </div>
            <div className="rounded-xl border border-slate-800/60 bg-slate-900/50 p-5">
              <div className="flex items-center gap-2 text-sm text-slate-400 mb-2">
                <FileCode2 className="h-4 w-4" />
                Type
              </div>
              {loading ? (
                <Skeleton className="h-8 w-24" />
              ) : (
                <p className="text-2xl font-bold tracking-tight">
                  {isContract ? "Contract" : "Account"}
                </p>
              )}
            </div>
          </div>

          {/* Contract info */}
          {isContract && (
            <div className="rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 mb-8">
              <h2 className="text-lg font-semibold mb-3 flex items-center gap-2">
                <FileCode2 className="h-5 w-5 text-slate-400" />
                Contract Info
              </h2>
              <div className="flex flex-col sm:flex-row sm:items-center gap-1 sm:gap-4 py-2">
                <span className="text-sm text-slate-400 sm:w-32">Code Hash</span>
                <span className="text-sm font-mono text-slate-200 break-all inline-flex items-center">
                  {account?.codeHash}
                  {account?.codeHash && <CopyButton text={account.codeHash} />}
                </span>
              </div>
            </div>
          )}

          {/* Transaction history */}
          <div className="rounded-xl border border-slate-800/60 bg-slate-900/50 overflow-hidden">
            <div className="px-5 py-4 border-b border-slate-800/60">
              <h2 className="text-lg font-semibold flex items-center gap-2">
                <Layers className="h-5 w-5 text-slate-400" />
                Transaction History ({transactions.length})
              </h2>
            </div>

            {loading ? (
              <div className="p-5 space-y-3">
                {Array.from({ length: 3 }).map((_, i) => (
                  <Skeleton key={i} className="h-10 w-full" />
                ))}
              </div>
            ) : transactions.length === 0 ? (
              <div className="p-10 text-center text-slate-500">
                No transactions found for this account.
              </div>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-slate-800/60 text-left text-xs text-slate-400 uppercase tracking-wider">
                      <th className="px-5 py-3">Tx Hash</th>
                      <th className="px-5 py-3">Block</th>
                      <th className="px-5 py-3">From</th>
                      <th className="px-5 py-3">To</th>
                      <th className="px-5 py-3">Amount</th>
                      <th className="px-5 py-3">Status</th>
                    </tr>
                  </thead>
                  <tbody>
                    {transactions.map((tx, i) => {
                      const isIncoming =
                        tx.to.toLowerCase() === address.toLowerCase();
                      return (
                        <tr
                          key={tx.hash || i}
                          className="border-b border-slate-800/30 hover:bg-slate-800/30 transition-colors"
                        >
                          <td className="px-5 py-3">
                            <Link
                              href={`/explorer/tx/${tx.hash}`}
                              className="inline-flex items-center gap-1 font-mono text-blue-400 hover:text-blue-300"
                            >
                              {truncateHash(tx.hash)}
                              <ArrowUpRight className="h-3 w-3" />
                            </Link>
                          </td>
                          <td className="px-5 py-3">
                            <Link
                              href={`/explorer/block/${tx.blockNumber}`}
                              className="text-blue-400 hover:text-blue-300 font-mono"
                            >
                              #{tx.blockNumber}
                            </Link>
                          </td>
                          <td className="px-5 py-3">
                            <span className="font-mono text-slate-300">
                              {truncateHash(tx.from, 6)}
                            </span>
                          </td>
                          <td className="px-5 py-3">
                            <span className="font-mono text-slate-300">
                              {truncateHash(tx.to, 6)}
                            </span>
                          </td>
                          <td className="px-5 py-3">
                            <span
                              className={
                                isIncoming ? "text-green-400" : "text-slate-300"
                              }
                            >
                              {isIncoming ? "+" : "-"}
                              {tx.amount}
                            </span>
                          </td>
                          <td className="px-5 py-3">
                            <span
                              className={`inline-flex rounded-full px-2 py-0.5 text-xs font-medium ${
                                tx.status === "success"
                                  ? "bg-green-500/10 text-green-400"
                                  : "bg-red-500/10 text-red-400"
                              }`}
                            >
                              {tx.status}
                            </span>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
