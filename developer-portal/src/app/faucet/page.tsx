"use client";

import { useState, useEffect } from "react";
import { Droplets, CheckCircle, XCircle, Loader2, Radio } from "lucide-react";
import { TESTNET_CONFIG } from "@/lib/constants";

interface ValidatorStatus {
  name: string;
  healthy: boolean;
  blockHeight?: number;
}

export default function FaucetPage() {
  const [address, setAddress] = useState("");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<{
    success: boolean;
    message: string;
    balance?: string;
  } | null>(null);
  const [validators, setValidators] = useState<ValidatorStatus[]>([]);
  const [checkingHealth, setCheckingHealth] = useState(true);

  // Check validator health on mount
  useEffect(() => {
    async function checkHealth() {
      setCheckingHealth(true);
      const statuses: ValidatorStatus[] = [];

      for (const v of TESTNET_CONFIG.validators) {
        try {
          const res = await fetch(`http://${v.ip}:${v.restPort}/health`, {
            signal: AbortSignal.timeout(5000),
          });
          if (res.ok) {
            const data = await res.json();
            statuses.push({
              name: v.name,
              healthy: true,
              blockHeight: data.block_height ?? data.blockHeight,
            });
          } else {
            statuses.push({ name: v.name, healthy: false });
          }
        } catch {
          statuses.push({ name: v.name, healthy: false });
        }
      }

      setValidators(statuses);
      setCheckingHealth(false);
    }

    checkHealth();
  }, []);

  async function handleRequest() {
    if (!address.startsWith("0x") || address.length < 10) {
      setResult({
        success: false,
        message: "Please enter a valid Dina address (0x... format)",
      });
      return;
    }

    setLoading(true);
    setResult(null);

    try {
      const res = await fetch(
        `http://${TESTNET_CONFIG.validators[0].ip}:${TESTNET_CONFIG.validators[0].restPort}/faucet/${address}`,
        { method: "POST" }
      );

      if (!res.ok) {
        const text = await res.text();
        throw new Error(text || `Request failed (${res.status})`);
      }

      const data = await res.json();

      // Try to fetch the updated balance
      let balanceStr: string | undefined;
      try {
        const balRes = await fetch(
          `http://${TESTNET_CONFIG.validators[0].ip}:${TESTNET_CONFIG.validators[0].restPort}/accounts/${address}`
        );
        if (balRes.ok) {
          const balData = await balRes.json();
          const micro = balData.balance ?? balData.Balance ?? 0;
          const whole = Math.floor(micro / 1_000_000);
          const frac = micro % 1_000_000;
          balanceStr = `${whole.toLocaleString()}.${String(frac).padStart(6, "0")} USDC`;
        }
      } catch {
        // balance fetch is best-effort
      }

      setResult({
        success: true,
        message:
          data.message ?? `Successfully sent 1,000 USDC to ${address.slice(0, 10)}...`,
        balance: balanceStr,
      });
    } catch (err) {
      setResult({
        success: false,
        message:
          err instanceof Error
            ? err.message
            : "Faucet request failed. Please try again.",
      });
    } finally {
      setLoading(false);
    }
  }

  const healthyCount = validators.filter((v) => v.healthy).length;

  return (
    <div className="min-h-[calc(100vh-73px)] flex items-start justify-center pt-20 px-6 pb-20">
      <div className="w-full max-w-lg">
        {/* Header */}
        <div className="text-center mb-10">
          <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-gradient-to-br from-blue-500 to-cyan-400 mb-6 shadow-lg shadow-blue-500/25">
            <Droplets className="w-8 h-8 text-white" />
          </div>
          <h1 className="text-3xl font-bold tracking-tight mb-3">
            Testnet Faucet
          </h1>
          <p className="text-slate-400 text-lg leading-relaxed">
            Get free testnet USDC to start building on Dina Network
          </p>
        </div>

        {/* Main card */}
        <div className="rounded-2xl border border-slate-800 bg-slate-900/80 backdrop-blur-sm p-8 shadow-xl">
          {/* Address input */}
          <label
            htmlFor="address"
            className="block text-sm font-medium text-slate-300 mb-2"
          >
            Dina Address
          </label>
          <input
            id="address"
            type="text"
            placeholder="0x1a2b3c4d5e6f..."
            value={address}
            onChange={(e) => setAddress(e.target.value.trim())}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !loading) handleRequest();
            }}
            className="w-full rounded-xl border border-slate-700 bg-slate-800/80 px-4 py-3 text-white placeholder-slate-500 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/60 focus:border-blue-500 transition-all"
          />

          {/* Submit button */}
          <button
            onClick={handleRequest}
            disabled={loading || !address}
            className="mt-5 w-full rounded-xl bg-gradient-to-r from-blue-600 to-blue-500 px-6 py-3.5 text-sm font-semibold text-white shadow-lg shadow-blue-600/25 transition-all hover:shadow-blue-600/40 hover:from-blue-500 hover:to-blue-400 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:shadow-blue-600/25 flex items-center justify-center gap-2"
          >
            {loading ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                Requesting...
              </>
            ) : (
              <>
                <Droplets className="w-4 h-4" />
                Request 1,000 USDC
              </>
            )}
          </button>

          {/* Result message */}
          {result && (
            <div
              className={`mt-5 rounded-xl border px-4 py-4 text-sm ${
                result.success
                  ? "border-emerald-800 bg-emerald-950/50 text-emerald-300"
                  : "border-red-800 bg-red-950/50 text-red-300"
              }`}
            >
              <div className="flex items-start gap-2.5">
                {result.success ? (
                  <CheckCircle className="w-5 h-5 mt-0.5 shrink-0 text-emerald-400" />
                ) : (
                  <XCircle className="w-5 h-5 mt-0.5 shrink-0 text-red-400" />
                )}
                <div>
                  <p>{result.message}</p>
                  {result.balance && (
                    <p className="mt-1.5 font-mono text-emerald-200">
                      Current balance: {result.balance}
                    </p>
                  )}
                </div>
              </div>
            </div>
          )}

          {/* Rate limit notice */}
          <p className="mt-5 text-center text-xs text-slate-500">
            Limited to 1 request per address per hour
          </p>
        </div>

        {/* Network status */}
        <div className="mt-8 rounded-2xl border border-slate-800 bg-slate-900/80 backdrop-blur-sm p-6">
          <div className="flex items-center gap-2 mb-4">
            <Radio className="w-4 h-4 text-slate-400" />
            <h2 className="text-sm font-semibold text-slate-300">
              Network Status
            </h2>
            {!checkingHealth && (
              <span
                className={`ml-auto inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium ${
                  healthyCount === validators.length
                    ? "bg-emerald-950/60 text-emerald-400"
                    : healthyCount > 0
                      ? "bg-yellow-950/60 text-yellow-400"
                      : "bg-red-950/60 text-red-400"
                }`}
              >
                <span
                  className={`w-1.5 h-1.5 rounded-full ${
                    healthyCount === validators.length
                      ? "bg-emerald-400"
                      : healthyCount > 0
                        ? "bg-yellow-400"
                        : "bg-red-400"
                  }`}
                />
                {healthyCount}/{validators.length} Validators
              </span>
            )}
          </div>

          {checkingHealth ? (
            <div className="flex items-center gap-2 text-sm text-slate-500">
              <Loader2 className="w-4 h-4 animate-spin" />
              Checking validator health...
            </div>
          ) : (
            <div className="space-y-2.5">
              {validators.map((v) => (
                <div
                  key={v.name}
                  className="flex items-center justify-between rounded-lg bg-slate-800/60 px-4 py-2.5"
                >
                  <div className="flex items-center gap-2.5">
                    <span
                      className={`w-2 h-2 rounded-full ${
                        v.healthy ? "bg-emerald-400" : "bg-red-400"
                      }`}
                    />
                    <span className="text-sm text-slate-300">{v.name}</span>
                  </div>
                  <span className="text-xs font-mono text-slate-500">
                    {v.healthy
                      ? `Block #${(v.blockHeight ?? 0).toLocaleString()}`
                      : "Offline"}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
