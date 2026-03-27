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
  ArrowRight,
  Clock,
  Blocks,
  Wallet,
  Zap,
  FileCode2,
  RefreshCw,
} from "lucide-react";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface TransactionDetail {
  hash: string;
  from: string;
  to: string;
  amount: string;
  fee: string;
  status: string;
  blockNumber: number;
  timestamp: string;
  nonce: number;
  gasUsed: string;
  events: TxEvent[];
}

interface TxEvent {
  type: string;
  attributes: Record<string, string>;
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

function formatTimestamp(ts: string): string {
  try {
    const d = new Date(ts);
    if (isNaN(d.getTime())) return ts;
    return d.toLocaleString();
  } catch {
    return ts;
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

function DetailRow({
  label,
  icon: Icon,
  children,
  loading,
}: {
  label: string;
  icon: React.ElementType;
  children: React.ReactNode;
  loading: boolean;
}) {
  return (
    <div className="flex flex-col sm:flex-row sm:items-center gap-1 sm:gap-4 py-3 border-b border-slate-800/40">
      <div className="flex items-center gap-2 text-sm text-slate-400 sm:w-44 shrink-0">
        <Icon className="h-4 w-4" />
        {label}
      </div>
      <div className="text-sm text-slate-200 min-w-0">
        {loading ? <Skeleton className="h-5 w-48" /> : children}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Transaction Detail Page
// ---------------------------------------------------------------------------

export default function TransactionDetailPage() {
  const pathname = usePathname();
  const txHash = pathname.split("/explorer/tx/")[1] ?? "";

  const [tx, setTx] = useState<TransactionDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchTx = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/transactions/${txHash}`, {
        signal: AbortSignal.timeout(5000),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const d = await res.json();

      const events: TxEvent[] = Array.isArray(d.events)
        ? d.events.map((e: Record<string, unknown>) => ({
            type: (e.type ?? e.event_type ?? "") as string,
            attributes:
              typeof e.attributes === "object" && e.attributes !== null
                ? (e.attributes as Record<string, string>)
                : {},
          }))
        : [];

      setTx({
        hash: (d.hash ?? d.tx_hash ?? txHash) as string,
        from: (d.from ?? d.sender ?? "") as string,
        to: (d.to ?? d.recipient ?? "") as string,
        amount: String(d.amount ?? d.value ?? "0"),
        fee: String(d.fee ?? d.gas_fee ?? "0"),
        status: (d.status ?? "success") as string,
        blockNumber: (d.block_number ?? d.blockNumber ?? d.height ?? 0) as number,
        timestamp: (d.timestamp ?? d.time ?? "") as string,
        nonce: (d.nonce ?? d.sequence ?? 0) as number,
        gasUsed: String(d.gas_used ?? d.gasUsed ?? "0"),
        events,
      });
      setError(null);
    } catch {
      setError("Unable to connect to the network or transaction not found.");
    } finally {
      setLoading(false);
    }
  }, [txHash]);

  useEffect(() => {
    fetchTx();
  }, [fetchTx]);

  const isSuccess = tx?.status === "success" || tx?.status === "confirmed";

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
        <div className="h-10 w-10 rounded-xl bg-gradient-to-br from-orange-500 to-amber-600 flex items-center justify-center shadow-lg shadow-orange-500/20">
          <Zap className="h-5 w-5" />
        </div>
        <div className="min-w-0 flex-1">
          <h1 className="text-2xl font-bold tracking-tight">Transaction</h1>
          <div className="flex items-center gap-1">
            <p className="text-sm text-slate-400 font-mono truncate">{txHash}</p>
            <CopyButton text={txHash} />
          </div>
        </div>
        <button
          onClick={() => {
            setLoading(true);
            setError(null);
            fetchTx();
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
              fetchTx();
            }}
            className="mt-4 rounded-lg bg-slate-800 px-4 py-2 text-sm hover:bg-slate-700 transition-colors"
          >
            Retry
          </button>
        </div>
      ) : (
        <>
          {/* Status banner */}
          {!loading && tx && (
            <div
              className={`rounded-xl border p-4 mb-8 flex items-center gap-3 ${
                isSuccess
                  ? "border-green-500/20 bg-green-500/5"
                  : "border-red-500/20 bg-red-500/5"
              }`}
            >
              <span
                className={`inline-flex rounded-full px-3 py-1 text-sm font-medium ${
                  isSuccess
                    ? "bg-green-500/10 text-green-400"
                    : "bg-red-500/10 text-red-400"
                }`}
              >
                {isSuccess ? "Success" : tx.status}
              </span>
              <span className="text-sm text-slate-400">
                {tx.timestamp ? formatTimestamp(tx.timestamp) : ""}
              </span>
            </div>
          )}

          {/* Transaction details */}
          <div className="rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 mb-8">
            <h2 className="text-lg font-semibold mb-4">Transaction Details</h2>
            <div>
              <DetailRow label="Transaction Hash" icon={Hash} loading={loading}>
                <span className="inline-flex items-center font-mono break-all">
                  {tx?.hash}
                  {tx?.hash && <CopyButton text={tx.hash} />}
                </span>
              </DetailRow>
              <DetailRow label="Block" icon={Blocks} loading={loading}>
                {tx?.blockNumber ? (
                  <Link
                    href={`/explorer/block/${tx.blockNumber}`}
                    className="inline-flex items-center gap-1 text-blue-400 hover:text-blue-300 font-mono"
                  >
                    #{tx.blockNumber.toLocaleString()}
                    <ArrowUpRight className="h-3 w-3" />
                  </Link>
                ) : (
                  <span className="text-slate-500">Pending</span>
                )}
              </DetailRow>
              <DetailRow label="Timestamp" icon={Clock} loading={loading}>
                {tx?.timestamp ? formatTimestamp(tx.timestamp) : "--"}
              </DetailRow>

              {/* From -> To */}
              <DetailRow label="From" icon={Wallet} loading={loading}>
                {tx?.from ? (
                  <span className="inline-flex items-center">
                    <Link
                      href={`/explorer/account/${tx.from}`}
                      className="font-mono text-blue-400 hover:text-blue-300"
                    >
                      {tx.from}
                    </Link>
                    <CopyButton text={tx.from} />
                  </span>
                ) : (
                  <span className="text-slate-500">--</span>
                )}
              </DetailRow>
              <DetailRow label="To" icon={ArrowRight} loading={loading}>
                {tx?.to ? (
                  <span className="inline-flex items-center">
                    <Link
                      href={`/explorer/account/${tx.to}`}
                      className="font-mono text-blue-400 hover:text-blue-300"
                    >
                      {tx.to}
                    </Link>
                    <CopyButton text={tx.to} />
                  </span>
                ) : (
                  <span className="text-slate-500">--</span>
                )}
              </DetailRow>

              <DetailRow label="Amount" icon={Wallet} loading={loading}>
                <span className="font-mono font-medium">{tx?.amount ?? "0"}</span>
              </DetailRow>
              <DetailRow label="Fee" icon={Zap} loading={loading}>
                <span className="font-mono">{tx?.fee ?? "0"}</span>
              </DetailRow>
              <DetailRow label="Nonce" icon={Hash} loading={loading}>
                <span className="font-mono">{tx?.nonce ?? 0}</span>
              </DetailRow>
              <DetailRow label="Gas Used" icon={Zap} loading={loading}>
                <span className="font-mono">{tx?.gasUsed ?? "0"}</span>
              </DetailRow>
            </div>
          </div>

          {/* Events */}
          <div className="rounded-xl border border-slate-800/60 bg-slate-900/50 overflow-hidden">
            <div className="px-5 py-4 border-b border-slate-800/60">
              <h2 className="text-lg font-semibold flex items-center gap-2">
                <FileCode2 className="h-5 w-5 text-slate-400" />
                Events ({tx?.events.length ?? 0})
              </h2>
            </div>

            {loading ? (
              <div className="p-5 space-y-3">
                {Array.from({ length: 2 }).map((_, i) => (
                  <Skeleton key={i} className="h-16 w-full" />
                ))}
              </div>
            ) : !tx?.events.length ? (
              <div className="p-10 text-center text-slate-500">
                No events emitted by this transaction.
              </div>
            ) : (
              <div className="divide-y divide-slate-800/40">
                {tx.events.map((event, i) => (
                  <div key={i} className="p-5">
                    <div className="flex items-center gap-2 mb-3">
                      <span className="inline-flex rounded-full bg-purple-500/10 border border-purple-500/20 px-2.5 py-0.5 text-xs font-medium text-purple-400">
                        {event.type}
                      </span>
                      <span className="text-xs text-slate-500">Event #{i}</span>
                    </div>
                    {Object.keys(event.attributes).length > 0 ? (
                      <div className="rounded-lg bg-slate-800/50 p-3 font-mono text-xs space-y-1">
                        {Object.entries(event.attributes).map(([key, val]) => (
                          <div key={key} className="flex gap-2">
                            <span className="text-slate-400">{key}:</span>
                            <span className="text-slate-200 break-all">
                              {val}
                            </span>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <p className="text-xs text-slate-500">No attributes</p>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
