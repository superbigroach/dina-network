"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  Blocks,
  ArrowUpRight,
  Copy,
  Check,
  ArrowLeft,
  Hash,
  Clock,
  User,
  FileCode2,
  Layers,
} from "lucide-react";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface BlockDetail {
  number: number;
  hash: string;
  parentHash: string;
  timestamp: string;
  proposer: string;
  stateRoot: string;
  txCount: number;
  size: number;
  transactions: TransactionSummary[];
}

interface TransactionSummary {
  hash: string;
  from: string;
  to: string;
  amount: string;
  fee: string;
  status: string;
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
// Block Detail Page
// ---------------------------------------------------------------------------

export default function BlockDetailPage() {
  const pathname = usePathname();
  const blockNumber = pathname.split("/explorer/block/")[1] ?? "0";

  const [block, setBlock] = useState<BlockDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchBlock = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/blocks/${blockNumber}`, {
        signal: AbortSignal.timeout(5000),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const d = await res.json();

      const txs: TransactionSummary[] = Array.isArray(d.transactions)
        ? d.transactions.map((tx: Record<string, unknown>) => ({
            hash: (tx.hash ?? tx.tx_hash ?? "") as string,
            from: (tx.from ?? tx.sender ?? "") as string,
            to: (tx.to ?? tx.recipient ?? "") as string,
            amount: (tx.amount ?? tx.value ?? "0") as string,
            fee: (tx.fee ?? tx.gas_used ?? "0") as string,
            status: (tx.status ?? "success") as string,
          }))
        : [];

      setBlock({
        number: (d.number ?? d.block_number ?? d.height ?? 0) as number,
        hash: (d.hash ?? d.block_hash ?? "") as string,
        parentHash: (d.parent_hash ?? d.parentHash ?? d.previous_hash ?? "") as string,
        timestamp: (d.timestamp ?? d.time ?? "") as string,
        proposer: (d.proposer ?? d.proposer_address ?? d.validator ?? "") as string,
        stateRoot: (d.state_root ?? d.stateRoot ?? d.app_hash ?? "") as string,
        txCount: (d.tx_count ?? d.num_txs ?? txs.length) as number,
        size: (d.size ?? d.block_size ?? 0) as number,
        transactions: txs,
      });
      setError(null);
    } catch {
      setError("Unable to connect to the network or block not found.");
    } finally {
      setLoading(false);
    }
  }, [blockNumber]);

  useEffect(() => {
    fetchBlock();
  }, [fetchBlock]);

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
        <div className="h-10 w-10 rounded-xl bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center shadow-lg shadow-blue-500/20">
          <Blocks className="h-5 w-5" />
        </div>
        <div>
          <h1 className="text-2xl font-bold tracking-tight">
            Block #{Number(blockNumber).toLocaleString()}
          </h1>
          <p className="text-sm text-slate-400">Block details and transactions</p>
        </div>
      </div>

      {error ? (
        <div className="rounded-xl border border-red-500/20 bg-red-500/5 p-8 text-center">
          <p className="text-red-400">{error}</p>
          <button
            onClick={() => {
              setLoading(true);
              setError(null);
              fetchBlock();
            }}
            className="mt-4 rounded-lg bg-slate-800 px-4 py-2 text-sm hover:bg-slate-700 transition-colors"
          >
            Retry
          </button>
        </div>
      ) : (
        <>
          {/* Block details */}
          <div className="rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 mb-8">
            <h2 className="text-lg font-semibold mb-4">Block Header</h2>
            <div>
              <DetailRow label="Block Number" icon={Blocks} loading={loading}>
                <span className="font-mono font-medium">
                  {block?.number.toLocaleString()}
                </span>
              </DetailRow>
              <DetailRow label="Block Hash" icon={Hash} loading={loading}>
                <span className="inline-flex items-center font-mono break-all">
                  {block?.hash}
                  {block?.hash && <CopyButton text={block.hash} />}
                </span>
              </DetailRow>
              <DetailRow label="Parent Hash" icon={Layers} loading={loading}>
                {block?.parentHash ? (
                  <span className="inline-flex items-center font-mono break-all">
                    <Link
                      href={`/explorer/block/${(block.number ?? 1) - 1}`}
                      className="text-blue-400 hover:text-blue-300"
                    >
                      {block.parentHash}
                    </Link>
                    <CopyButton text={block.parentHash} />
                  </span>
                ) : (
                  <span className="text-slate-500">Genesis</span>
                )}
              </DetailRow>
              <DetailRow label="Timestamp" icon={Clock} loading={loading}>
                {block?.timestamp ? formatTimestamp(block.timestamp) : "--"}
              </DetailRow>
              <DetailRow label="Proposer" icon={User} loading={loading}>
                {block?.proposer ? (
                  <span className="inline-flex items-center">
                    <Link
                      href={`/explorer/account/${block.proposer}`}
                      className="font-mono text-blue-400 hover:text-blue-300"
                    >
                      {truncateHash(block.proposer, 10)}
                    </Link>
                    <CopyButton text={block.proposer} />
                  </span>
                ) : (
                  <span className="text-slate-500">--</span>
                )}
              </DetailRow>
              <DetailRow label="State Root" icon={FileCode2} loading={loading}>
                {block?.stateRoot ? (
                  <span className="inline-flex items-center font-mono break-all">
                    {block.stateRoot}
                    <CopyButton text={block.stateRoot} />
                  </span>
                ) : (
                  <span className="text-slate-500">--</span>
                )}
              </DetailRow>
              <DetailRow label="Transactions" icon={Layers} loading={loading}>
                {block?.txCount ?? 0}
              </DetailRow>
              <DetailRow label="Size" icon={Hash} loading={loading}>
                {block?.size ? `${block.size} bytes` : "--"}
              </DetailRow>
            </div>
          </div>

          {/* Transactions in block */}
          <div className="rounded-xl border border-slate-800/60 bg-slate-900/50 overflow-hidden">
            <div className="px-5 py-4 border-b border-slate-800/60">
              <h2 className="text-lg font-semibold">
                Transactions ({block?.transactions.length ?? 0})
              </h2>
            </div>

            {loading ? (
              <div className="p-5 space-y-3">
                {Array.from({ length: 3 }).map((_, i) => (
                  <Skeleton key={i} className="h-10 w-full" />
                ))}
              </div>
            ) : !block?.transactions.length ? (
              <div className="p-10 text-center text-slate-500">
                No transactions in this block.
              </div>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-slate-800/60 text-left text-xs text-slate-400 uppercase tracking-wider">
                      <th className="px-5 py-3">Tx Hash</th>
                      <th className="px-5 py-3">From</th>
                      <th className="px-5 py-3">To</th>
                      <th className="px-5 py-3">Amount</th>
                      <th className="px-5 py-3">Status</th>
                    </tr>
                  </thead>
                  <tbody>
                    {block.transactions.map((tx, i) => (
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
                            href={`/explorer/account/${tx.from}`}
                            className="font-mono text-slate-300 hover:text-blue-400 transition-colors"
                          >
                            {truncateHash(tx.from, 6)}
                          </Link>
                        </td>
                        <td className="px-5 py-3">
                          <Link
                            href={`/explorer/account/${tx.to}`}
                            className="font-mono text-slate-300 hover:text-blue-400 transition-colors"
                          >
                            {truncateHash(tx.to, 6)}
                          </Link>
                        </td>
                        <td className="px-5 py-3 text-slate-300">{tx.amount}</td>
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
                    ))}
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
