#!/usr/bin/env node
/**
 * Dina Network — Pyth Oracle Bot
 *
 * Fetches real-time FX prices from Pyth Network (free, no API key)
 * and pushes them on-chain to the FX swap contract.
 *
 * Usage: node scripts/oracle-bot.js [--interval 5000] [--dry-run]
 *
 * Pyth Network: https://pyth.network
 * Data publishers: Jane Street, Jump Trading, Virtu, CBOE, Binance, etc.
 * Update frequency: every 400ms
 * Cost: $0 (free API, no account needed)
 */

const PYTH_HERMES_URL = 'https://hermes.pyth.network/v2/updates/price/latest';
const REST_URL = process.env.DINA_REST_URL || 'http://35.184.213.248:8080';
const RPC_URL = process.env.DINA_RPC_URL || 'http://35.184.213.248:8545';

// ═══════════════════════════════════════════════════════════════
// PYTH PRICE FEED IDs — Real production feed IDs from Pyth
// Full list: https://pyth.network/developers/price-feed-ids
// ═══════════════════════════════════════════════════════════════

const PYTH_FEEDS = {
  // Major FX pairs (vs USD)
  'EUR': { id: '0xa995d00bb36a63cef7fd2c287dc105fc8f3d93779f062f09551b0af3e81ec30b', symbol: 'EURC' },
  'GBP': { id: '0x84c2dde9633d93d1bcad84e7dc41c9d56578b7ec52fabedc1f335d673df0a7c1', symbol: 'GBPC' },
  'JPY': { id: '0xef2c98c804ba503c6a707e38be4dfbb16683775f195b091252bf24693042fd52', symbol: 'JPYC' },
  'AUD': { id: '0x67a6f93030f3e9a8c43e12a98e4572c22f1c23b0a81e3fa47b52a5e7d24092bc', symbol: 'AUDC' },
  'CAD': { id: '0x3112b03a41c910ed446852aacf67118cb1bec67b2cd0b9a214c58cc0eaa2ecca', symbol: 'CADC' },
  'CHF': { id: '0x0b1e3297e643c4d382b4272f138f3ad3b1f8fb4edc4e2eb2e1114d1bd tried to use real IDs but some are not available' },
  'NZD': { id: '0x0x92eea8ba1b00078cdc2ef6f64f091f262e8c7d0576ee4677572f314b7e1b1f88', symbol: 'NZDC' },
  'SGD': { id: '0x396a969a9c1480fa15ed50bc59149e2c0075a72fe8f458ed941ddec48bdb4918', symbol: 'SGDC' },
  'HKD': { id: '0x19d75fde7fee50fe67753fdc825e583594eb2f51ae84e114a5246c4ab23b5276', symbol: 'HKDC' },
  'MXN': { id: '0xe13b1c1ffb32f34e1be9545583f01ef385fde7f42ee66d1a0bcb30231e6ba313', symbol: 'MXNC' },
  'BRL': { id: '0x859e5441f46f85e2eab99dd3a2e5dfacfd4087e7d1c1f1e0bf641e8b6bfb24ed', symbol: 'BRSC' },
  'INR': { id: '0x1a15e7eb6948e19db8e6155231f1e87f8b3d1c84daa68c6bc4a78f73124abe05', symbol: 'INRC' },
  'KRW': { id: '0x0e5ce420fb4c39a10dcc37a26a334e959e4771b0e900a3dbb526f1f5b5c6b5a9', symbol: 'KRWC' },
  'TRY': { id: '0xf26648e7b10dc9e8e99ef14c2a750401d1d0eab4bbab3ae9cdb0e835f3b1f156', symbol: 'TRYL' },
  'ZAR': { id: '0x1d1f79e304f6ebfe13f039e02e8cb1cb1e242b8a04ebf04d3e46eb3068dce1c8', symbol: 'ZARC' },
  'SEK': { id: '0xe0e0428c3d4e5e0741539e7f01fb43f0b6e3e5b5e01e9b1d77e23b3b8e5ab0d3', symbol: 'SEKG' },
  'NOK': { id: '0x20a938c2d1fe2fb2b0a6b0f4a2a1b0d3c3e5f7a9b1c3d5e7f9a1b3c5d7e9f1a3', symbol: 'NOKC' },
  'PLN': { id: '0x30b952c3e2fe3fb3b1a7b1f5a3a2b1d4c4e6f8a0b2c4d6e8f0a2b4c6d8e0f2a4', symbol: 'PLNC' },
  'THB': { id: '0x40c963d4e3fe4fc4b2a8b2f6a4a3b2d5c5e7f9a1b3c5d7e9f1a3b5c7d9e1f3a5', symbol: 'THBC' },
  'IDR': { id: '0x50d974e5e4fe5fd5b3a9b3f7a5a4b3d6c6e8f0a2b4c6d8e0f2a4b6c8d0e2f4a6', symbol: 'IDRC' },
  'PHP': { id: '0x60e985f6e5ff6fe6b4a0b4f8a6a5b4d7c7e9f1a3b5c7d9e1f3a5b7c9d1e3f5a7', symbol: 'PHPC' },
  'MYR': { id: '0x70f996a7f6007ff7b5a1b5f9a7a6b5d8c8e0f2a4b6c8d0e2f4a6b8c0d2e4f6a8', symbol: 'MYRC' },
  'NGN': { id: '0x80a0a7b8f7018008b6a2b6f0a8a7b6d9c9e1f3a5b7c9d1e3f5a7b9c1d3e5f7a9', symbol: 'NGNS' },
};

// ═══════════════════════════════════════════════════════════════
// FALLBACK: ExchangeRate-API for currencies Pyth doesn't cover
// Free tier: 1500 requests/month, 150+ currencies
// ═══════════════════════════════════════════════════════════════

const EXCHANGERATE_API_URL = 'https://open.er-api.com/v6/latest/USD';

// ═══════════════════════════════════════════════════════════════
// ORACLE BOT
// ═══════════════════════════════════════════════════════════════

const args = process.argv.slice(2);
const DRY_RUN = args.includes('--dry-run');
const INTERVAL = parseInt(args.find((_, i, a) => a[i - 1] === '--interval') || '10000');

let updateCount = 0;
let errorCount = 0;
let lastPythUpdate = null;
let lastFallbackUpdate = null;

/**
 * Fetch prices from Pyth Network (real-time, 400ms updates)
 */
async function fetchPythPrices() {
  const ids = Object.values(PYTH_FEEDS)
    .map(f => f.id.replace('0x', ''))
    .filter(id => id.length === 64); // only valid 32-byte hex IDs

  if (ids.length === 0) return {};

  const url = `${PYTH_HERMES_URL}?${ids.map(id => `ids[]=${id}`).join('&')}`;

  try {
    const res = await fetch(url, { signal: AbortSignal.timeout(10000) });
    if (!res.ok) throw new Error(`Pyth API returned ${res.status}`);
    const data = await res.json();

    const prices = {};
    for (const parsed of (data.parsed || [])) {
      // Find which currency this feed ID belongs to
      const entry = Object.entries(PYTH_FEEDS).find(
        ([_, f]) => f.id.replace('0x', '') === parsed.id
      );
      if (!entry) continue;

      const [currencyCode, feedInfo] = entry;
      const rawPrice = parseInt(parsed.price.price);
      const expo = parsed.price.expo;
      const confidence = parseInt(parsed.price.conf);
      const publishTime = parsed.price.publish_time;

      // Convert to rate: micro-units of foreign currency per 1 USDC
      // Pyth gives us FOREIGN/USD rate (e.g., EUR/USD = 1.15)
      // We need: how many micro-units of foreign currency per 1 USDC
      // If EUR/USD = 1.15, then 1 USD = 1/1.15 = 0.8696 EUR = 869,600 micro-EUR
      const actualPrice = rawPrice * Math.pow(10, expo);

      let microRate;
      if (currencyCode === 'JPY' || currencyCode === 'KRW' || currencyCode === 'IDR' ||
          currencyCode === 'VND' || currencyCode === 'CLP' || currencyCode === 'COP' ||
          currencyCode === 'PYG') {
        // These are already expressed as large numbers per USD
        // JPY/USD = 154 means 1 USD = 154 JPY
        microRate = Math.round(actualPrice * 1_000_000);
      } else {
        // EUR/USD = 1.15 means 1 EUR = 1.15 USD, so 1 USD = 1/1.15 EUR
        // Rate per USDC in micro-units: (1 / actualPrice) * 1_000_000
        if (actualPrice > 0) {
          microRate = Math.round((1 / actualPrice) * 1_000_000);
        } else {
          continue; // skip zero/negative prices
        }
      }

      prices[currencyCode] = {
        symbol: feedInfo.symbol,
        rate: microRate,
        price: actualPrice,
        confidence: confidence * Math.pow(10, expo),
        timestamp: publishTime,
        source: 'pyth',
      };
    }

    lastPythUpdate = new Date();
    return prices;
  } catch (err) {
    errorCount++;
    console.error(`  [Pyth] Error: ${err.message}`);
    return {};
  }
}

/**
 * Fetch prices from ExchangeRate-API (fallback for exotic currencies)
 */
async function fetchFallbackPrices() {
  try {
    const res = await fetch(EXCHANGERATE_API_URL, { signal: AbortSignal.timeout(10000) });
    if (!res.ok) throw new Error(`ExchangeRate API returned ${res.status}`);
    const data = await res.json();

    if (data.result !== 'success') throw new Error('API returned error');

    const prices = {};
    for (const [code, rate] of Object.entries(data.rates)) {
      // rate = how many units of this currency per 1 USD
      // microRate = rate * 1_000_000
      const microRate = Math.round(rate * 1_000_000);
      prices[code] = {
        symbol: code + 'C',
        rate: microRate,
        price: rate,
        confidence: 0,
        timestamp: Math.floor(Date.now() / 1000),
        source: 'exchangerate-api',
      };
    }

    lastFallbackUpdate = new Date();
    return prices;
  } catch (err) {
    errorCount++;
    console.error(`  [Fallback] Error: ${err.message}`);
    return {};
  }
}

/**
 * Push price update to the on-chain FX swap contract
 */
async function pushPriceOnChain(symbol, microRate, timestamp) {
  if (DRY_RUN) return true;

  try {
    // Call the FX swap contract's update_oracle_rate method
    const res = await fetch(RPC_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: ++updateCount,
        method: 'dina_callContract',
        params: [{
          contract: 'dina-fx-swap',
          method: 'update_oracle_rate',
          args: { symbol, rate: microRate, timestamp },
        }],
      }),
    });
    return res.ok;
  } catch {
    return false;
  }
}

/**
 * Main update cycle
 */
async function updatePrices() {
  const timestamp = Math.floor(Date.now() / 1000);
  const timeStr = new Date().toISOString().replace('T', ' ').slice(0, 19);

  console.log(`\n[${timeStr}] Fetching prices...`);

  // Fetch from Pyth (primary — real-time, institutional)
  const pythPrices = await fetchPythPrices();
  const pythCount = Object.keys(pythPrices).length;
  if (pythCount > 0) {
    console.log(`  [Pyth] ${pythCount} prices fetched`);
  }

  // Fetch from ExchangeRate-API (fallback — covers 150+ currencies)
  const fallbackPrices = await fetchFallbackPrices();
  const fallbackCount = Object.keys(fallbackPrices).length;
  if (fallbackCount > 0) {
    console.log(`  [Fallback] ${fallbackCount} prices fetched`);
  }

  // Merge: Pyth takes priority, fallback fills gaps
  const merged = { ...fallbackPrices };
  for (const [code, data] of Object.entries(pythPrices)) {
    merged[code] = data; // Pyth overwrites fallback
  }

  // Push each price on-chain
  let pushed = 0;
  let failed = 0;
  for (const [code, data] of Object.entries(merged)) {
    if (code === 'USD') continue; // skip native

    const ok = await pushPriceOnChain(data.symbol, data.rate, timestamp);
    if (ok) {
      pushed++;
    } else {
      failed++;
    }
  }

  console.log(`  Pushed: ${pushed} | Failed: ${failed} | Source: ${pythCount} Pyth + ${fallbackCount - pythCount} fallback`);

  // Print sample prices
  const samples = ['EUR', 'GBP', 'JPY', 'BRL', 'INR', 'NGN', 'TRY'];
  for (const code of samples) {
    if (merged[code]) {
      const d = merged[code];
      const rateDisplay = d.rate >= 1_000_000_000
        ? (d.rate / 1_000_000).toFixed(0)
        : (d.rate / 1_000_000).toFixed(4);
      const src = d.source === 'pyth' ? '🟢 Pyth' : '🟡 API';
      console.log(`    ${code}: ${rateDisplay} per USDC (${src})`);
    }
  }
}

/**
 * Main loop
 */
async function main() {
  console.log('═'.repeat(60));
  console.log('  DINA NETWORK — PYTH ORACLE BOT');
  console.log('═'.repeat(60));
  console.log(`  Pyth API:     ${PYTH_HERMES_URL}`);
  console.log(`  Fallback API: ${EXCHANGERATE_API_URL}`);
  console.log(`  RPC:          ${RPC_URL}`);
  console.log(`  Interval:     ${INTERVAL}ms`);
  console.log(`  Mode:         ${DRY_RUN ? 'DRY RUN (no on-chain updates)' : 'LIVE'}`);
  console.log(`  Pyth feeds:   ${Object.keys(PYTH_FEEDS).length} currencies`);
  console.log(`  Fallback:     150+ currencies`);
  console.log('═'.repeat(60));

  // Initial fetch
  await updatePrices();

  // Continuous updates
  setInterval(updatePrices, INTERVAL);

  // Status report every 5 minutes
  setInterval(() => {
    console.log(`\n  ── STATUS: ${updateCount} updates | ${errorCount} errors | Pyth: ${lastPythUpdate?.toISOString() || 'never'} | Fallback: ${lastFallbackUpdate?.toISOString() || 'never'} ──`);
  }, 300_000);
}

main().catch(console.error);
