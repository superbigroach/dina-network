// Dina Network client library for the developer portal
import { TESTNET_CONFIG } from "./constants";

export async function fetchHealth(validatorIndex = 0) {
  const v = TESTNET_CONFIG.validators[validatorIndex];
  const res = await fetch(`http://${v.ip}:${v.restPort}/health`, {
    cache: "no-store",
  });
  return res.json();
}

export async function fetchAccount(address: string) {
  const v = TESTNET_CONFIG.validators[0];
  const res = await fetch(`http://${v.ip}:${v.restPort}/accounts/${address}`, {
    cache: "no-store",
  });
  if (!res.ok) return null;
  return res.json();
}

export async function fetchBlock(numberOrLatest: string | number = "latest") {
  const v = TESTNET_CONFIG.validators[0];
  const res = await fetch(
    `http://${v.ip}:${v.restPort}/blocks/${numberOrLatest}`,
    { cache: "no-store" }
  );
  if (!res.ok) return null;
  return res.json();
}

export async function rpcCall(method: string, params: unknown[] = []) {
  const v = TESTNET_CONFIG.validators[0];
  const res = await fetch(`http://${v.ip}:${v.rpcPort}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      method,
      params,
      id: 1,
    }),
  });
  const data = await res.json();
  if (data.error) throw new Error(data.error.message);
  return data.result;
}

export async function requestFaucet(address: string) {
  const v = TESTNET_CONFIG.validators[0];
  const res = await fetch(`http://${v.ip}:${v.restPort}/faucet/${address}`, {
    method: "POST",
  });
  if (!res.ok) throw new Error("Faucet request failed");
  return res.json();
}

export function formatUSDC(microUsdc: number): string {
  const whole = Math.floor(microUsdc / 1_000_000);
  const frac = microUsdc % 1_000_000;
  return `${whole.toLocaleString()}.${String(frac).padStart(6, "0")} USDC`;
}
