"use client";

import { useState, useCallback, useRef, useEffect } from "react";
import {
  Wallet as WalletIcon,
  Bot,
  Network,
  Copy,
  Check,
  Eye,
  EyeOff,
  Download,
  Droplets,
  AlertTriangle,
  Loader2,
  Zap,
} from "lucide-react";
import {
  generateWallet,
  generateAgentWallet,
  generateSwarm,
  type Wallet,
  type AgentWalletConfig,
  type SwarmResult,
} from "@/lib/wallet";
import { TESTNET_CONFIG } from "@/lib/constants";

// --- Tab types ---
type Tab = "standard" | "agent" | "swarm";

const TABS: { id: Tab; label: string; icon: typeof WalletIcon }[] = [
  { id: "standard", label: "Standard Wallet", icon: WalletIcon },
  { id: "agent", label: "Agent Wallet", icon: Bot },
  { id: "swarm", label: "Swarm Wallet", icon: Network },
];

// --- Copy button helper ---
function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  async function handleCopy() {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <button
      onClick={handleCopy}
      className="p-1.5 rounded-lg text-slate-500 hover:text-slate-300 hover:bg-slate-700/60 transition-colors"
      title="Copy"
    >
      {copied ? (
        <Check className="w-3.5 h-3.5 text-emerald-400" />
      ) : (
        <Copy className="w-3.5 h-3.5" />
      )}
    </button>
  );
}

// --- Field display ---
function KeyField({
  label,
  value,
  secret,
  mono,
}: {
  label: string;
  value: string;
  secret?: boolean;
  mono?: boolean;
}) {
  const [visible, setVisible] = useState(!secret);

  return (
    <div className="rounded-xl bg-slate-800/80 border border-slate-700/60 p-3.5">
      <div className="flex items-center justify-between mb-1.5">
        <span className="text-xs font-medium text-slate-400">{label}</span>
        <div className="flex items-center gap-0.5">
          {secret && (
            <button
              onClick={() => setVisible(!visible)}
              className="p-1.5 rounded-lg text-slate-500 hover:text-slate-300 hover:bg-slate-700/60 transition-colors"
              title={visible ? "Hide" : "Show"}
            >
              {visible ? (
                <EyeOff className="w-3.5 h-3.5" />
              ) : (
                <Eye className="w-3.5 h-3.5" />
              )}
            </button>
          )}
          <CopyButton text={value} />
        </div>
      </div>
      <p
        className={`text-sm break-all ${mono ? "font-mono" : ""} ${
          visible ? "text-slate-200" : "text-slate-600"
        }`}
      >
        {visible ? value : "\u2022".repeat(Math.min(value.length, 48))}
      </p>
    </div>
  );
}

// --- Standard Wallet Tab ---
function StandardTab() {
  const [wallet, setWallet] = useState<Wallet | null>(null);
  const [fundingStatus, setFundingStatus] = useState<{
    loading: boolean;
    message?: string;
    success?: boolean;
  }>({ loading: false });

  function handleGenerate() {
    setWallet(generateWallet());
    setFundingStatus({ loading: false });
  }

  async function handleFund() {
    if (!wallet) return;
    setFundingStatus({ loading: true });

    try {
      const v = TESTNET_CONFIG.validators[0];
      const res = await fetch(
        `http://${v.ip}:${v.restPort}/faucet/${wallet.address}`,
        { method: "POST" }
      );

      if (!res.ok) throw new Error("Faucet request failed");
      const data = await res.json();
      setFundingStatus({
        loading: false,
        success: true,
        message: data.message ?? "1,000 USDC sent successfully!",
      });
    } catch {
      setFundingStatus({
        loading: false,
        success: false,
        message: "Faucet request failed. Try again later.",
      });
    }
  }

  return (
    <div className="space-y-6">
      <p className="text-slate-400 text-sm leading-relaxed">
        Generate a new Ed25519 keypair for the Dina Network. Your private key is
        created entirely in your browser and never sent to any server.
      </p>

      <button
        onClick={handleGenerate}
        className="w-full rounded-xl bg-gradient-to-r from-blue-600 to-purple-600 px-6 py-3.5 text-sm font-semibold text-white shadow-lg shadow-blue-600/20 transition-all hover:shadow-blue-600/35 hover:from-blue-500 hover:to-purple-500 flex items-center justify-center gap-2"
      >
        <WalletIcon className="w-4 h-4" />
        Generate New Wallet
      </button>

      {wallet && (
        <div className="space-y-3 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <KeyField label="Address" value={wallet.address} mono />
          <KeyField label="Public Key" value={wallet.publicKey} mono />
          <KeyField
            label="Private Key"
            value={wallet.privateKey}
            secret
            mono
          />

          {/* Warning */}
          <div className="flex items-start gap-2.5 rounded-xl border border-amber-800/60 bg-amber-950/30 px-4 py-3">
            <AlertTriangle className="w-4 h-4 mt-0.5 shrink-0 text-amber-400" />
            <p className="text-xs text-amber-300/90 leading-relaxed">
              Save your private key! It cannot be recovered. Anyone with access
              to your private key controls your funds.
            </p>
          </div>

          {/* Fund button */}
          <button
            onClick={handleFund}
            disabled={fundingStatus.loading}
            className="w-full rounded-xl border border-slate-700 bg-slate-800/80 px-5 py-3 text-sm font-medium text-slate-300 transition-all hover:bg-slate-700/80 hover:text-white disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
          >
            {fundingStatus.loading ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                Requesting...
              </>
            ) : (
              <>
                <Droplets className="w-4 h-4" />
                Fund from Faucet
              </>
            )}
          </button>

          {fundingStatus.message && (
            <div
              className={`rounded-xl border px-4 py-3 text-sm ${
                fundingStatus.success
                  ? "border-emerald-800 bg-emerald-950/40 text-emerald-300"
                  : "border-red-800 bg-red-950/40 text-red-300"
              }`}
            >
              {fundingStatus.message}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// --- Agent Wallet Tab ---
function AgentTab() {
  const [ownerAddress, setOwnerAddress] = useState("");
  const [dailyLimit, setDailyLimit] = useState("1000");
  const [perTxLimit, setPerTxLimit] = useState("100");
  const [agentConfig, setAgentConfig] = useState<AgentWalletConfig | null>(
    null
  );

  function handleCreate() {
    const config = generateAgentWallet(
      ownerAddress,
      parseFloat(dailyLimit) || 1000,
      parseFloat(perTxLimit) || 100
    );
    setAgentConfig(config);
  }

  return (
    <div className="space-y-6">
      <div className="rounded-xl border border-slate-800 bg-slate-800/40 p-4">
        <h3 className="text-sm font-semibold text-slate-200 mb-2">
          What are Agent Wallets?
        </h3>
        <p className="text-xs text-slate-400 leading-relaxed">
          Agent wallets (DRC-101) are purpose-constrained wallets designed for
          autonomous AI agents. They operate under spending limits set by a human
          owner, enabling agents to transact independently while maintaining
          financial guardrails. The owner can revoke access or adjust limits at
          any time.
        </p>
      </div>

      {/* Form */}
      <div className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-slate-300 mb-1.5">
            Owner Address
          </label>
          <input
            type="text"
            placeholder="0x... (your main wallet address)"
            value={ownerAddress}
            onChange={(e) => setOwnerAddress(e.target.value.trim())}
            className="w-full rounded-xl border border-slate-700 bg-slate-800/80 px-4 py-3 text-white placeholder-slate-500 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-purple-500/60 focus:border-purple-500 transition-all"
          />
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium text-slate-300 mb-1.5">
              Daily Limit (USDC)
            </label>
            <input
              type="number"
              value={dailyLimit}
              onChange={(e) => setDailyLimit(e.target.value)}
              min={1}
              className="w-full rounded-xl border border-slate-700 bg-slate-800/80 px-4 py-3 text-white text-sm focus:outline-none focus:ring-2 focus:ring-purple-500/60 focus:border-purple-500 transition-all"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-slate-300 mb-1.5">
              Per-TX Limit (USDC)
            </label>
            <input
              type="number"
              value={perTxLimit}
              onChange={(e) => setPerTxLimit(e.target.value)}
              min={1}
              className="w-full rounded-xl border border-slate-700 bg-slate-800/80 px-4 py-3 text-white text-sm focus:outline-none focus:ring-2 focus:ring-purple-500/60 focus:border-purple-500 transition-all"
            />
          </div>
        </div>

        <button
          onClick={handleCreate}
          disabled={!ownerAddress}
          className="w-full rounded-xl bg-gradient-to-r from-purple-600 to-pink-600 px-6 py-3.5 text-sm font-semibold text-white shadow-lg shadow-purple-600/20 transition-all hover:shadow-purple-600/35 hover:from-purple-500 hover:to-pink-500 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
        >
          <Bot className="w-4 h-4" />
          Create Agent Wallet
        </button>
      </div>

      {agentConfig && (
        <div className="space-y-3 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <div className="rounded-xl border border-purple-800/40 bg-purple-950/20 p-4">
            <h4 className="text-xs font-semibold text-purple-300 uppercase tracking-wider mb-3">
              Agent Wallet Configuration
            </h4>
            <div className="grid grid-cols-2 gap-3 text-sm">
              <div>
                <span className="text-xs text-slate-500">Daily Limit</span>
                <p className="text-slate-200 font-mono">
                  {agentConfig.dailyLimitUsdc.toLocaleString()} USDC
                </p>
              </div>
              <div>
                <span className="text-xs text-slate-500">Per-TX Limit</span>
                <p className="text-slate-200 font-mono">
                  {agentConfig.perTxLimitUsdc.toLocaleString()} USDC
                </p>
              </div>
              <div className="col-span-2">
                <span className="text-xs text-slate-500">Owner</span>
                <p className="text-slate-200 font-mono text-xs break-all">
                  {agentConfig.ownerAddress}
                </p>
              </div>
            </div>
          </div>

          <KeyField
            label="Agent Address"
            value={agentConfig.wallet.address}
            mono
          />
          <KeyField
            label="Agent Public Key"
            value={agentConfig.wallet.publicKey}
            mono
          />
          <KeyField
            label="Agent Private Key"
            value={agentConfig.wallet.privateKey}
            secret
            mono
          />

          <div className="flex items-start gap-2.5 rounded-xl border border-amber-800/60 bg-amber-950/30 px-4 py-3">
            <AlertTriangle className="w-4 h-4 mt-0.5 shrink-0 text-amber-400" />
            <p className="text-xs text-amber-300/90 leading-relaxed">
              Save your agent private key! The owner can revoke this wallet but
              cannot recover the key.
            </p>
          </div>
        </div>
      )}
    </div>
  );
}

// --- Swarm Wallet Tab ---
function SwarmTab() {
  const [count, setCount] = useState(10);
  const [swarm, setSwarm] = useState<SwarmResult | null>(null);
  const [generating, setGenerating] = useState(false);
  const [progress, setProgress] = useState(0);
  const progressRef = useRef(0);

  const handleGenerate = useCallback(() => {
    setGenerating(true);
    setProgress(0);
    setSwarm(null);
    progressRef.current = 0;

    // Use requestAnimationFrame-based generation for large swarms
    // to keep the UI responsive and show the animation
    const batchSize = Math.max(1, Math.min(50, Math.ceil(count / 20)));
    const authority = generateWallet();
    const agents: Wallet[] = [];

    function generateBatch() {
      const end = Math.min(agents.length + batchSize, count);
      for (let i = agents.length; i < end; i++) {
        agents.push(generateWallet());
      }
      progressRef.current = agents.length;
      setProgress(agents.length);

      if (agents.length < count) {
        requestAnimationFrame(generateBatch);
      } else {
        setSwarm({ authority, agents });
        setGenerating(false);
      }
    }

    requestAnimationFrame(generateBatch);
  }, [count]);

  function handleExport() {
    if (!swarm) return;

    const exportData = {
      generated: new Date().toISOString(),
      network: "dina-testnet-1",
      authority: {
        address: swarm.authority.address,
        publicKey: swarm.authority.publicKey,
        privateKey: swarm.authority.privateKey,
      },
      agents: swarm.agents.map((a, i) => ({
        index: i,
        address: a.address,
        publicKey: a.publicKey,
        privateKey: a.privateKey,
      })),
    };

    const blob = new Blob([JSON.stringify(exportData, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `dina-swarm-${swarm.agents.length}-wallets.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

  const tps = count * 10_000;

  return (
    <div className="space-y-6">
      <div className="rounded-xl border border-slate-800 bg-gradient-to-br from-slate-800/60 to-purple-900/20 p-4">
        <h3 className="text-sm font-semibold text-slate-200 mb-2 flex items-center gap-2">
          <Zap className="w-4 h-4 text-purple-400" />
          Swarm Wallets -- The Killer Feature
        </h3>
        <p className="text-xs text-slate-400 leading-relaxed">
          Swarm wallets (DRC-63) let you generate hundreds or thousands of
          parallel agent wallets under a single authority. Because Dina uses
          lane-based parallel execution, each wallet can submit transactions
          independently -- multiplying your effective throughput linearly. No
          other blockchain supports this architecture.
        </p>
      </div>

      {/* Count input */}
      <div>
        <label className="flex items-center justify-between text-sm font-medium text-slate-300 mb-3">
          <span>Number of Agent Wallets</span>
          <span className="font-mono text-blue-400">{count}</span>
        </label>
        <input
          type="range"
          min={1}
          max={1000}
          value={count}
          onChange={(e) => setCount(parseInt(e.target.value))}
          className="w-full h-2 rounded-full appearance-none bg-slate-700 cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-5 [&::-webkit-slider-thumb]:h-5 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-gradient-to-r [&::-webkit-slider-thumb]:from-blue-500 [&::-webkit-slider-thumb]:to-purple-500 [&::-webkit-slider-thumb]:shadow-lg [&::-webkit-slider-thumb]:shadow-blue-500/30 [&::-webkit-slider-thumb]:cursor-pointer"
        />
        <div className="flex justify-between text-xs text-slate-600 mt-1">
          <span>1</span>
          <span>250</span>
          <span>500</span>
          <span>750</span>
          <span>1,000</span>
        </div>
      </div>

      {/* Stats preview */}
      <div className="grid grid-cols-2 gap-3">
        <div className="rounded-xl bg-slate-800/60 border border-slate-700/40 p-3.5 text-center">
          <p className="text-xs text-slate-500 mb-1">Parallel TPS Capacity</p>
          <p className="text-lg font-bold font-mono text-blue-400">
            {tps.toLocaleString()}
          </p>
          <p className="text-[10px] text-slate-600 mt-0.5">
            {count} x 10,000 TPS
          </p>
        </div>
        <div className="rounded-xl bg-slate-800/60 border border-slate-700/40 p-3.5 text-center">
          <p className="text-xs text-slate-500 mb-1">Total Wallets</p>
          <p className="text-lg font-bold font-mono text-purple-400">
            {(count + 1).toLocaleString()}
          </p>
          <p className="text-[10px] text-slate-600 mt-0.5">
            1 authority + {count} agents
          </p>
        </div>
      </div>

      {/* Generate button */}
      <button
        onClick={handleGenerate}
        disabled={generating}
        className="w-full rounded-xl bg-gradient-to-r from-purple-600 via-blue-600 to-cyan-500 px-6 py-3.5 text-sm font-semibold text-white shadow-lg shadow-purple-600/20 transition-all hover:shadow-purple-600/35 disabled:opacity-60 disabled:cursor-not-allowed flex items-center justify-center gap-2"
      >
        {generating ? (
          <>
            <Loader2 className="w-4 h-4 animate-spin" />
            Generating... {progress}/{count}
          </>
        ) : (
          <>
            <Network className="w-4 h-4" />
            Generate Swarm
          </>
        )}
      </button>

      {/* Progress bar during generation */}
      {generating && (
        <div className="w-full rounded-full bg-slate-800 h-2 overflow-hidden">
          <div
            className="h-full rounded-full bg-gradient-to-r from-purple-500 to-blue-500 transition-all duration-100"
            style={{ width: `${(progress / count) * 100}%` }}
          />
        </div>
      )}

      {/* Swarm result */}
      {swarm && (
        <div className="space-y-4 animate-in fade-in slide-in-from-bottom-2 duration-300">
          {/* Authority */}
          <div className="rounded-xl border border-blue-800/40 bg-blue-950/20 p-4">
            <h4 className="text-xs font-semibold text-blue-300 uppercase tracking-wider mb-2">
              Master Authority
            </h4>
            <KeyField
              label="Authority Address"
              value={swarm.authority.address}
              mono
            />
          </div>

          {/* Animated wallet grid */}
          <div>
            <div className="flex items-center justify-between mb-3">
              <h4 className="text-xs font-semibold text-slate-400 uppercase tracking-wider">
                Agent Wallets ({swarm.agents.length})
              </h4>
              <button
                onClick={handleExport}
                className="inline-flex items-center gap-1.5 rounded-lg bg-slate-800 border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-300 hover:text-white hover:bg-slate-700 transition-colors"
              >
                <Download className="w-3.5 h-3.5" />
                Export All Keys
              </button>
            </div>

            <SwarmGrid wallets={swarm.agents} />
          </div>

          <div className="flex items-start gap-2.5 rounded-xl border border-amber-800/60 bg-amber-950/30 px-4 py-3">
            <AlertTriangle className="w-4 h-4 mt-0.5 shrink-0 text-amber-400" />
            <p className="text-xs text-amber-300/90 leading-relaxed">
              The exported JSON contains all private keys. Store it securely and
              never share it.
            </p>
          </div>
        </div>
      )}
    </div>
  );
}

// --- Animated swarm grid ---
function SwarmGrid({ wallets }: { wallets: Wallet[] }) {
  const [visibleCount, setVisibleCount] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Animate wallets appearing in the grid
    const total = Math.min(wallets.length, 200); // cap visible tiles at 200
    let frame = 0;
    const perFrame = Math.max(1, Math.ceil(total / 30)); // fill in ~30 frames

    function step() {
      frame += perFrame;
      const next = Math.min(frame, total);
      setVisibleCount(next);
      if (next < total) {
        requestAnimationFrame(step);
      }
    }

    requestAnimationFrame(step);
    return () => {
      setVisibleCount(0);
    };
  }, [wallets]);

  const displayWallets = wallets.slice(0, 200);
  const hasMore = wallets.length > 200;

  return (
    <div>
      <div
        ref={containerRef}
        className="max-h-64 overflow-y-auto rounded-xl border border-slate-800 bg-slate-900/60 p-2"
      >
        <div className="grid grid-cols-5 sm:grid-cols-8 md:grid-cols-10 gap-1">
          {displayWallets.map((w, i) => (
            <div
              key={i}
              title={w.address}
              className={`aspect-square rounded-md flex items-center justify-center text-[9px] font-mono cursor-default transition-all duration-150 ${
                i < visibleCount
                  ? "bg-gradient-to-br from-blue-600/40 to-purple-600/40 border border-blue-500/20 text-blue-300 scale-100 opacity-100"
                  : "bg-slate-800/30 border border-transparent text-transparent scale-75 opacity-0"
              }`}
            >
              {i < visibleCount ? i + 1 : ""}
            </div>
          ))}
        </div>
        {hasMore && (
          <p className="text-center text-xs text-slate-600 mt-2 py-1">
            +{wallets.length - 200} more wallets (all included in export)
          </p>
        )}
      </div>

      {/* Scrollable address list */}
      <details className="mt-3 group">
        <summary className="text-xs text-slate-500 cursor-pointer hover:text-slate-300 transition-colors">
          View all addresses
        </summary>
        <div className="mt-2 max-h-48 overflow-y-auto rounded-xl border border-slate-800 bg-slate-900/60 p-3 space-y-1">
          {wallets.map((w, i) => (
            <div
              key={i}
              className="flex items-center justify-between text-xs py-1 border-b border-slate-800/40 last:border-0"
            >
              <span className="text-slate-600 w-8 shrink-0">#{i + 1}</span>
              <span className="font-mono text-slate-400 truncate flex-1 mx-2">
                {w.address}
              </span>
              <CopyButton text={w.address} />
            </div>
          ))}
        </div>
      </details>
    </div>
  );
}

// --- Main Page ---
export default function WalletsPage() {
  const [activeTab, setActiveTab] = useState<Tab>("standard");

  return (
    <div className="min-h-[calc(100vh-73px)] flex items-start justify-center pt-20 px-6 pb-20">
      <div className="w-full max-w-xl">
        {/* Header */}
        <div className="text-center mb-10">
          <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-gradient-to-br from-purple-500 to-blue-600 mb-6 shadow-lg shadow-purple-500/25">
            <WalletIcon className="w-8 h-8 text-white" />
          </div>
          <h1 className="text-3xl font-bold tracking-tight mb-3">
            Wallet Creator
          </h1>
          <p className="text-slate-400 text-lg leading-relaxed">
            Generate Ed25519 wallets for the Dina Network -- entirely in your
            browser
          </p>
        </div>

        {/* Tab bar */}
        <div className="flex rounded-xl bg-slate-900/80 border border-slate-800 p-1 mb-8">
          {TABS.map((tab) => {
            const Icon = tab.icon;
            const active = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`flex-1 flex items-center justify-center gap-2 rounded-lg px-3 py-2.5 text-sm font-medium transition-all ${
                  active
                    ? "bg-slate-800 text-white shadow-sm"
                    : "text-slate-500 hover:text-slate-300"
                }`}
              >
                <Icon className="w-4 h-4" />
                <span className="hidden sm:inline">{tab.label}</span>
              </button>
            );
          })}
        </div>

        {/* Tab content */}
        <div className="rounded-2xl border border-slate-800 bg-slate-900/80 backdrop-blur-sm p-8 shadow-xl">
          {activeTab === "standard" && <StandardTab />}
          {activeTab === "agent" && <AgentTab />}
          {activeTab === "swarm" && <SwarmTab />}
        </div>
      </div>
    </div>
  );
}
