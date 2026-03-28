#!/usr/bin/env node
/**
 * Dina Network — Testnet Status Checker
 *
 * Shows testnet health, deployed contracts, and stablecoin balances.
 *
 * Usage: node scripts/testnet-status.js [--rest URL] [--address ADDR] [--verbose]
 */

const fs = require('fs');
const path = require('path');

const args = process.argv.slice(2);
function getArg(flag, envVar, fallback) {
  const idx = args.indexOf(flag);
  if (idx !== -1 && args[idx + 1]) return args[idx + 1];
  if (process.env[envVar]) return process.env[envVar];
  return fallback;
}

const REST_URL = getArg('--rest', 'DINA_REST_URL', 'http://35.184.213.248:8080');
const RPC_URL = getArg('--rpc', 'DINA_RPC_URL', 'http://35.184.213.248:8545');
const CHECK_ADDRESS = getArg('--address', 'DINA_CHECK_ADDRESS', '');
const VERBOSE = args.includes('--verbose') || args.includes('-v');

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function padRight(str, len) {
  return String(str).padEnd(len);
}

function padLeft(str, len) {
  return String(str).padStart(len);
}

function formatUSDC(micro) {
  return (Number(micro) / 1_000_000).toFixed(2);
}

async function fetchJson(url, timeoutMs = 10_000) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const res = await fetch(url, { signal: controller.signal });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return await res.json();
  } finally {
    clearTimeout(timer);
  }
}

async function rpcCall(method, params = []) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 10_000);
  try {
    const res = await fetch(RPC_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }),
      signal: controller.signal,
    });
    const json = await res.json();
    if (json.error) throw new Error(json.error.message);
    return json.result;
  } finally {
    clearTimeout(timer);
  }
}

// ---------------------------------------------------------------------------
// Status sections
// ---------------------------------------------------------------------------

async function showHealth() {
  console.log('NETWORK HEALTH');
  console.log('-'.repeat(50));

  try {
    const health = await fetchJson(`${REST_URL}/health`);
    const chainId = health.chain_id || health.chainId || 'dina-testnet-1';
    const height = health.height || health.blockHeight || '?';
    const status = health.status || 'unknown';
    const peers = health.peers || health.peerCount || '?';
    const version = health.version || '?';

    console.log(`  Chain:      ${chainId}`);
    console.log(`  Status:     ${status}`);
    console.log(`  Height:     Block #${height}`);
    console.log(`  Peers:      ${peers}`);
    console.log(`  Version:    ${version}`);

    if (health.epoch) console.log(`  Epoch:      ${health.epoch}`);
    if (health.uptime) console.log(`  Uptime:     ${health.uptime}`);

    if (VERBOSE) {
      console.log('');
      console.log('  Raw health response:');
      console.log('  ' + JSON.stringify(health, null, 2).replace(/\n/g, '\n  '));
    }

    return { healthy: true, height };
  } catch (e) {
    console.log(`  Status:     UNREACHABLE`);
    console.log(`  Error:      ${e.message}`);
    console.log(`  REST URL:   ${REST_URL}`);
    return { healthy: false, height: 0 };
  }
}

async function showRpcStatus() {
  console.log('');
  console.log('RPC STATUS');
  console.log('-'.repeat(50));

  const checks = [
    { label: 'Chain ID', method: 'dina_chainId', params: [] },
    { label: 'Gas Price', method: 'dina_gasPrice', params: [] },
    { label: 'Network Info', method: 'dina_networkInfo', params: [] },
  ];

  for (const check of checks) {
    try {
      const result = await rpcCall(check.method, check.params);
      if (typeof result === 'object') {
        console.log(`  ${padRight(check.label + ':', 16)} ${JSON.stringify(result)}`);
      } else {
        console.log(`  ${padRight(check.label + ':', 16)} ${result}`);
      }
    } catch (e) {
      console.log(`  ${padRight(check.label + ':', 16)} ERROR — ${e.message}`);
    }
  }
}

async function showDeployedContracts() {
  console.log('');
  console.log('DEPLOYED STABLECOINS');
  console.log('-'.repeat(50));

  // Load from the deployment manifest if it exists
  const manifestPath = path.join(__dirname, '..', 'testnet-stablecoins.json');
  if (!fs.existsSync(manifestPath)) {
    console.log('  No deployment manifest found.');
    console.log(`  Expected: ${manifestPath}`);
    console.log('  Run: node scripts/deploy-stablecoins.js');
    return [];
  }

  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf-8'));
  const currencies = manifest.currencies || [];

  console.log(`  Deployed at: ${manifest.deployedAt || '?'}`);
  console.log(`  Currencies:  ${currencies.length}`);
  console.log('');

  // Infrastructure
  if (manifest.infrastructure) {
    console.log('  Infrastructure:');
    console.log(`    Registry:  ${manifest.infrastructure.registry || 'N/A'}`);
    console.log(`    FX Swap:   ${manifest.infrastructure.fxSwap || 'N/A'}`);
    console.log('');
  }

  // Table
  const hdr = [
    padRight('#', 4),
    padRight('Symbol', 7),
    padRight('Name', 28),
    padLeft('Rate/USDC', 14),
    padLeft('APY', 8),
    padRight('Status', 10),
  ].join(' ');

  console.log('  ' + hdr);
  console.log('  ' + '-'.repeat(hdr.length));

  let deployed = 0;
  let failed = 0;

  currencies.forEach((c, i) => {
    const rate = c.rate ? (c.rate / 1_000_000).toFixed(6) : '';
    const apy = c.yield ? (c.yield / 100).toFixed(2) + '%' : '';
    const status = c.error ? 'FAILED' : (c.contractAddress ? 'OK' : 'PENDING');

    if (c.contractAddress) deployed++;
    if (c.error) failed++;

    const row = [
      padRight(i + 1, 4),
      padRight(c.symbol, 7),
      padRight(c.name || '', 28),
      padLeft(rate, 14),
      padLeft(apy, 8),
      padRight(status, 10),
    ].join(' ');

    console.log('  ' + row);
  });

  console.log('  ' + '-'.repeat(hdr.length));
  console.log(`  Deployed: ${deployed}  |  Failed: ${failed}  |  Total: ${currencies.length}`);

  return currencies;
}

async function showBalances(currencies) {
  if (!CHECK_ADDRESS) return;

  console.log('');
  console.log(`BALANCES FOR ${CHECK_ADDRESS.substring(0, 20)}...`);
  console.log('-'.repeat(50));

  // USDC balance
  try {
    const bal = await fetchJson(`${REST_URL}/v1/balance/${CHECK_ADDRESS}`);
    console.log(`  USDC:  ${formatUSDC(bal.balance || 0)}`);
  } catch (e) {
    console.log(`  USDC:  ERROR — ${e.message}`);
  }

  // Stablecoin balances (if contracts are deployed)
  for (const c of (currencies || [])) {
    if (!c.contractAddress) continue;
    try {
      const result = await rpcCall('dina_callView', [
        c.contractAddress,
        'balance_of',
        { account: CHECK_ADDRESS },
      ]);
      const balance = result?.balance || result || '0';
      console.log(`  ${padRight(c.symbol + ':', 7)} ${formatUSDC(balance)}`);
    } catch (e) {
      if (VERBOSE) {
        console.log(`  ${padRight(c.symbol + ':', 7)} ERROR — ${e.message}`);
      }
    }
  }
}

async function showLatestBlocks() {
  console.log('');
  console.log('LATEST BLOCKS');
  console.log('-'.repeat(50));

  try {
    const latest = await rpcCall('dina_getLatestBlock');
    const height = latest.height || latest.blockHeight || 0;

    for (let h = height; h > Math.max(0, height - 5); h--) {
      try {
        const block = await rpcCall('dina_getBlock', [h]);
        const txCount = block.transactions?.length || block.txCount || 0;
        const time = block.timestamp
          ? new Date(block.timestamp * 1000).toISOString().replace('T', ' ').substring(0, 19)
          : '?';
        console.log(`  #${padLeft(h, 8)}  ${time}  ${txCount} txs`);
      } catch {
        console.log(`  #${padLeft(h, 8)}  (not available)`);
      }
    }
  } catch (e) {
    console.log(`  Could not fetch blocks: ${e.message}`);
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  console.log('');
  console.log('='.repeat(50));
  console.log('  DINA TESTNET STATUS');
  console.log('='.repeat(50));
  console.log(`  RPC:  ${RPC_URL}`);
  console.log(`  REST: ${REST_URL}`);
  console.log(`  Time: ${new Date().toISOString()}`);
  console.log('');

  const { healthy } = await showHealth();

  if (healthy) {
    await showRpcStatus();
    await showLatestBlocks();
  }

  const currencies = await showDeployedContracts();
  await showBalances(currencies);

  console.log('');
  console.log('='.repeat(50));
  console.log(`  ${healthy ? 'TESTNET ONLINE' : 'TESTNET OFFLINE'}`);
  console.log('='.repeat(50));
  console.log('');
}

main().catch((e) => {
  console.error('Status check failed:', e.message);
  process.exit(1);
});
