"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import Link from "next/link";
import {
  Search,
  Blocks,
  ArrowUpRight,
  Copy,
  RefreshCw,
  Activity,
  Server,
  Hash,
  Clock,
  Check,
  Wifi,
  WifiOff,
} from "lucide-react";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ValidatorHealth {
  ip: string;
  label: string;
  blockHeight: number;
  peerCount: number;
  status: "online" | "offline";
  chainId: string;
  error?: string;
}

interface BlockSummary {
  number: number;
  hash: string;
  timestamp: string;
  txCount: number;
  proposer: string;
  size: number;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const VALIDATORS = [
  { ip: "35.184.213.248", label: "Validator 1 (US-Central)" },
  { ip: "35.193.28.189", label: "Validator 2 (US-East)" },
  { ip: "136.115.115.11", label: "Validator 3 (EU-West)" },
];

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
    return d.toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
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
      {copied ? <Check className="h-3.5 w-3.5 text-green-400" /> : <Copy className="h-3.5 w-3.5" />}
    </button>
  );
}

function Skeleton({ className = "" }: { className?: string }) {
  return <div className={`animate-pulse rounded bg-slate-800 ${className}`} />;
}

function StatCard({
  label,
  value,
  icon: Icon,
  loading,
}: {
  label: string;
  value: string | number;
  icon: React.ElementType;
  loading: boolean;
}) {
  return (
    <div className="rounded-xl border border-slate-800/60 bg-slate-900/50 p-5">
      <div className="flex items-center gap-2 text-sm text-slate-400 mb-2">
        <Icon className="h-4 w-4" />
        {label}
      </div>
      {loading ? (
        <Skeleton className="h-8 w-24" />
      ) : (
        <p className="text-2xl font-bold tracking-tight">{value}</p>
      )}
    </div>
  );
}

function ValidatorCard({ v, loading }: { v: ValidatorHealth; loading: boolean }) {
  const isOnline = v.status === "online";
  return (
    <div className="rounded-xl border border-slate-800/60 bg-slate-900/50 p-5 hover:border-slate-700 transition-colors">
      <div className="flex items-center justify-between mb-3">
        <h3 className="font-semibold text-sm">{v.label}</h3>
        <span
          className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium ${
            isOnline
              ? "bg-green-500/10 text-green-400 border border-green-500/20"
              : "bg-red-500/10 text-red-400 border border-red-500/20"
          }`}
        >
          {isOnline ? <Wifi className="h-3 w-3" /> : <WifiOff className="h-3 w-3" />}
          {isOnline ? "Online" : "Offline"}
        </span>
      </div>
      <div className="space-y-2 text-sm">
        <div className="flex justify-between">
          <span className="text-slate-400">IP Address</span>
          {loading ? <Skeleton className="h-4 w-28" /> : <span className="font-mono text-slate-200">{v.ip}:8080</span>}
        </div>
        <div className="flex justify-between">
          <span className="text-slate-400">Block Height</span>
          {loading ? (
            <Skeleton className="h-4 w-16" />
          ) : (
            <span className="font-mono text-slate-200">{v.blockHeight.toLocaleString()}</span>
          )}
        </div>
        <div className="flex justify-between">
          <span className="text-slate-400">Peers</span>
          {loading ? <Skeleton className="h-4 w-8" /> : <span className="font-mono text-slate-200">{v.peerCount}</span>}
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main Explorer Page
// ---------------------------------------------------------------------------

export default function ExplorerPage() {
  const [validators, setValidators] = useState<ValidatorHealth[]>(
    VALIDATORS.map((v) => ({
      ip: v.ip,
      label: v.label,
      blockHeight: 0,
      peerCount: 0,
      status: "offline" as const,
      chainId: "",
    }))
  );
  const [blocks, setBlocks] = useState<BlockSummary[]>([]);
  const [loadingValidators, setLoadingValidators] = useState(true);
  const [loadingBlocks, setLoadingBlocks] = useState(true);
  const [searchQuery, setSearchQuery] = useState("");
  const [tps, setTps] = useState(0);
  const prevHeightRef = useRef(0);
  const prevTimeRef = useRef(Date.now());

  // ---- Fetch validator health ----
  const fetchValidators = useCallback(async () => {
    const results = await Promise.allSettled(
      VALIDATORS.map(async (v) => {
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), 4000);
        try {
          const res = await fetch(`http://${v.ip}:8080/health`, {
            signal: controller.signal,
          });
          if (!res.ok) throw new Error(`HTTP ${res.status}`);
          const data = await res.json();
          return {
            ip: v.ip,
            label: v.label,
            blockHeight: data.block_height ?? data.blockHeight ?? 0,
            peerCount: data.peer_count ?? data.peerCount ?? data.peers ?? 0,
            status: "online" as const,
            chainId: data.chain_id ?? data.chainId ?? "dina-testnet-1",
          };
        } catch {
          return {
            ip: v.ip,
            label: v.label,
            blockHeight: 0,
            peerCount: 0,
            status: "offline" as const,
            chainId: "",
            error: "Unable to connect",
          };
        } finally {
          clearTimeout(timeout);
        }
      })
    );

    const mapped = results.map((r) =>
      r.status === "fulfilled"
        ? r.value
        : {
            ip: "",
            label: "",
            blockHeight: 0,
            peerCount: 0,
            status: "offline" as const,
            chainId: "",
          }
    );

    setValidators(mapped);
    setLoadingValidators(false);

    // Calculate TPS from block height change
    const maxHeight = Math.max(...mapped.map((v) => v.blockHeight));
    const now = Date.now();
    if (prevHeightRef.current > 0 && maxHeight > prevHeightRef.current) {
      const elapsed = (now - prevTimeRef.current) / 1000;
      const blocksDelta = maxHeight - prevHeightRef.current;
      // Rough estimate: assume ~100 txs per block if no better data
      setTps(Math.round((blocksDelta * 100) / elapsed));
    }
    prevHeightRef.current = maxHeight;
    prevTimeRef.current = now;
  }, []);

  // ---- Fetch recent blocks ----
  const fetchBlocks = useCallback(async () => {
    try {
      // Try fetching latest block first
      const latestRes = await fetch(`${API_BASE}/blocks/latest`, {
        signal: AbortSignal.timeout(4000),
      });
      if (!latestRes.ok) throw new Error(`HTTP ${latestRes.status}`);
      const latestData = await latestRes.json();

      const latestNumber =
        latestData.number ?? latestData.block_number ?? latestData.height ?? 0;

      // Fetch previous blocks
      const blockNumbers = Array.from(
        { length: Math.min(10, latestNumber) },
        (_, i) => latestNumber - i
      ).filter((n) => n > 0);

      const blockResults = await Promise.allSettled(
        blockNumbers.map(async (num) => {
          if (num === latestNumber) return latestData;
          const res = await fetch(`${API_BASE}/blocks/${num}`, {
            signal: AbortSignal.timeout(4000),
          });
          if (!res.ok) throw new Error(`HTTP ${res.status}`);
          return res.json();
        })
      );

      const parsed: BlockSummary[] = blockResults
        .filter((r) => r.status === "fulfilled")
        .map((r) => {
          const d = (r as PromiseFulfilledResult<Record<string, unknown>>).value;
          return {
            number: (d.number ?? d.block_number ?? d.height ?? 0) as number,
            hash: (d.hash ?? d.block_hash ?? "") as string,
            timestamp: (d.timestamp ?? d.time ?? new Date().toISOString()) as string,
            txCount: (d.tx_count ?? d.num_txs ?? (Array.isArray(d.transactions) ? d.transactions.length : 0)) as number,
            proposer: (d.proposer ?? d.proposer_address ?? d.validator ?? "") as string,
            size: (d.size ?? d.block_size ?? 0) as number,
          };
        })
        .sort((a, b) => b.number - a.number);

      setBlocks(parsed);
    } catch {
      // If endpoint doesn't exist, show empty
      setBlocks([]);
    } finally {
      setLoadingBlocks(false);
    }
  }, []);

  // ---- Auto-refresh ----
  useEffect(() => {
    fetchValidators();
    fetchBlocks();

    const interval = setInterval(() => {
      fetchValidators();
      fetchBlocks();
    }, 2000);

    return () => clearInterval(interval);
  }, [fetchValidators, fetchBlocks]);

  // ---- Search handler ----
  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    const q = searchQuery.trim();
    if (!q) return;

    // Detect query type
    if (/^\d+$/.test(q)) {
      window.location.href = `/explorer/block/${q}`;
    } else if (q.startsWith("0x") && q.length === 66) {
      window.location.href = `/explorer/tx/${q}`;
    } else if (q.startsWith("dina") || q.startsWith("0x")) {
      window.location.href = `/explorer/account/${q}`;
    } else {
      // Default: try as tx hash
      window.location.href = `/explorer/tx/${q}`;
    }
  };

  // ---- Derived stats ----
  const onlineCount = validators.filter((v) => v.status === "online").length;
  const maxHeight = Math.max(...validators.map((v) => v.blockHeight), 0);

  return (
    <div className="mx-auto max-w-7xl px-6 py-10">
      {/* Header */}
      <div className="mb-8">
        <div className="flex items-center gap-3 mb-2">
          <div className="h-10 w-10 rounded-xl bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center shadow-lg shadow-blue-500/20">
            <Blocks className="h-5 w-5" />
          </div>
          <h1 className="text-3xl font-bold tracking-tight">Block Explorer</h1>
          <span className="ml-auto flex items-center gap-2 text-xs text-slate-400">
            <RefreshCw className="h-3.5 w-3.5 animate-spin" style={{ animationDuration: "2s" }} />
            Auto-refreshing every 2s
          </span>
        </div>
        <p className="text-slate-400">
          Real-time view of the Dina Network testnet.
        </p>
      </div>

      {/* Network overview stats */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
        <StatCard
          label="Block Height"
          value={maxHeight.toLocaleString()}
          icon={Blocks}
          loading={loadingValidators}
        />
        <StatCard
          label="Chain ID"
          value="dina-testnet-1"
          icon={Hash}
          loading={false}
        />
        <StatCard
          label="Validators Online"
          value={`${onlineCount} / ${VALIDATORS.length}`}
          icon={Server}
          loading={loadingValidators}
        />
        <StatCard
          label="Est. TPS"
          value={tps > 0 ? tps.toLocaleString() : "--"}
          icon={Activity}
          loading={loadingValidators}
        />
      </div>

      {/* Validator cards */}
      <div className="mb-8">
        <h2 className="text-lg font-semibold mb-4">Validators</h2>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {validators.map((v) => (
            <ValidatorCard key={v.ip} v={v} loading={loadingValidators} />
          ))}
        </div>
      </div>

      {/* Search */}
      <form onSubmit={handleSearch} className="mb-8">
        <div className="relative">
          <Search className="absolute left-4 top-1/2 -translate-y-1/2 h-5 w-5 text-slate-400" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search by address, tx hash, or block number..."
            className="w-full rounded-xl border border-slate-800 bg-slate-900/50 py-3.5 pl-12 pr-4 text-sm text-white placeholder:text-slate-500 focus:border-blue-500/50 focus:outline-none focus:ring-1 focus:ring-blue-500/30 transition-colors"
          />
          <button
            type="submit"
            className="absolute right-2 top-1/2 -translate-y-1/2 rounded-lg bg-blue-600 px-4 py-1.5 text-sm font-medium hover:bg-blue-500 transition-colors"
          >
            Search
          </button>
        </div>
      </form>

      {/* Recent blocks table */}
      <div className="rounded-xl border border-slate-800/60 bg-slate-900/50 overflow-hidden">
        <div className="flex items-center justify-between px-5 py-4 border-b border-slate-800/60">
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <Clock className="h-5 w-5 text-slate-400" />
            Recent Blocks
          </h2>
        </div>

        {loadingBlocks ? (
          <div className="p-5 space-y-3">
            {Array.from({ length: 5 }).map((_, i) => (
              <Skeleton key={i} className="h-10 w-full" />
            ))}
          </div>
        ) : blocks.length === 0 ? (
          <div className="p-10 text-center text-slate-500">
            <WifiOff className="h-8 w-8 mx-auto mb-3 opacity-50" />
            <p>Unable to connect to the network or no blocks available yet.</p>
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-slate-800/60 text-left text-xs text-slate-400 uppercase tracking-wider">
                  <th className="px-5 py-3">Block</th>
                  <th className="px-5 py-3">Timestamp</th>
                  <th className="px-5 py-3">Txs</th>
                  <th className="px-5 py-3">Proposer</th>
                  <th className="px-5 py-3">Size</th>
                </tr>
              </thead>
              <tbody>
                {blocks.map((block) => (
                  <tr
                    key={block.number}
                    className="border-b border-slate-800/30 hover:bg-slate-800/30 transition-colors"
                  >
                    <td className="px-5 py-3">
                      <Link
                        href={`/explorer/block/${block.number}`}
                        className="inline-flex items-center gap-1 text-blue-400 hover:text-blue-300 font-mono font-medium"
                      >
                        #{block.number.toLocaleString()}
                        <ArrowUpRight className="h-3 w-3" />
                      </Link>
                    </td>
                    <td className="px-5 py-3 text-slate-300">
                      {formatTimestamp(block.timestamp)}
                    </td>
                    <td className="px-5 py-3 text-slate-300">{block.txCount}</td>
                    <td className="px-5 py-3">
                      {block.proposer ? (
                        <span className="inline-flex items-center">
                          <Link
                            href={`/explorer/account/${block.proposer}`}
                            className="font-mono text-slate-300 hover:text-blue-400 transition-colors"
                          >
                            {truncateHash(block.proposer, 6)}
                          </Link>
                          <CopyButton text={block.proposer} />
                        </span>
                      ) : (
                        <span className="text-slate-500">--</span>
                      )}
                    </td>
                    <td className="px-5 py-3 text-slate-400">
                      {block.size > 0 ? `${block.size} B` : "--"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
