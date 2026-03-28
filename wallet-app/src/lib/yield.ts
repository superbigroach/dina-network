const SECONDS_PER_YEAR = 31_536_000;
const BPS_DENOMINATOR = 10_000;

export function calculateAccruedYield(
  balance: number,
  yieldRateBps: number,
  lastUpdate: number,
  now: number
): number {
  if (now <= lastUpdate || balance <= 0) return 0;
  const elapsed = now - lastUpdate;
  return Math.floor(
    (balance * yieldRateBps * elapsed) / (BPS_DENOMINATOR * SECONDS_PER_YEAR)
  );
}

export function effectiveBalance(
  balance: number,
  yieldRateBps: number,
  lastUpdate: number,
  now: number
): number {
  return balance + calculateAccruedYield(balance, yieldRateBps, lastUpdate, now);
}

export function formatUsdc(microUsdc: number): string {
  const dollars = microUsdc / 1_000_000;
  return dollars.toLocaleString('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: 6,
  });
}

export function formatUsdcStreaming(microUsdc: number): string {
  const dollars = microUsdc / 1_000_000;
  return '$' + dollars.toFixed(6);
}

export function yieldPerDay(balance: number, yieldRateBps: number): number {
  return Math.floor((balance * yieldRateBps) / (BPS_DENOMINATOR * 365));
}
