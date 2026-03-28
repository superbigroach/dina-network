#!/usr/bin/env node
/**
 * Deploy all Dina Network stablecoins + FX swap + currency registry to testnet.
 *
 * Usage: node scripts/deploy-stablecoins.js [--rpc URL] [--rest URL] [--key HEX]
 *
 * Environment variables:
 *   DINA_RPC_URL   - JSON-RPC endpoint (default: http://35.184.213.248:8545)
 *   DINA_REST_URL  - REST API endpoint (default: http://35.184.213.248:8080)
 *   DINA_DEPLOY_KEY - Hex-encoded Ed25519 private key for the deployer wallet
 */

const { DinaClient, DinaWallet } = require('dina-js');
const fs = require('fs');
const path = require('path');

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const args = process.argv.slice(2);
function getArg(flag, envVar, fallback) {
  const idx = args.indexOf(flag);
  if (idx !== -1 && args[idx + 1]) return args[idx + 1];
  if (process.env[envVar]) return process.env[envVar];
  return fallback;
}

const RPC_URL = getArg('--rpc', 'DINA_RPC_URL', 'http://35.184.213.248:8545');
const REST_URL = getArg('--rest', 'DINA_REST_URL', 'http://35.184.213.248:8080');
const DEPLOY_KEY = getArg('--key', 'DINA_DEPLOY_KEY', '');

const MINT_AMOUNT = 1_000_000_000_000n; // 1,000,000 units (6 decimals)
const DEPLOY_TIMEOUT = 60_000; // 60s per contract
const CONCURRENCY = 3; // Deploy up to 3 contracts in parallel

// ---------------------------------------------------------------------------
// Currency definitions
// ---------------------------------------------------------------------------
// rate: micro-units of that currency per 1 USDC (6 decimals)
// yield: basis points (450 = 4.50% APY)

const CURRENCIES = [
  // Major currencies
  { symbol: 'EURC', name: 'Dina Euro', rate: 930_000, yield: 350 },
  { symbol: 'GBPC', name: 'Dina British Pound', rate: 790_000, yield: 400 },
  { symbol: 'JPYC', name: 'Dina Japanese Yen', rate: 154_000_000, yield: 50 },
  { symbol: 'CADC', name: 'Dina Canadian Dollar', rate: 1_380_000, yield: 380 },
  { symbol: 'AUDC', name: 'Dina Australian Dollar', rate: 1_540_000, yield: 400 },
  { symbol: 'CHFC', name: 'Dina Swiss Franc', rate: 880_000, yield: 150 },
  { symbol: 'CNHC', name: 'Dina Chinese Yuan', rate: 7_250_000, yield: 250 },
  { symbol: 'HKDC', name: 'Dina Hong Kong Dollar', rate: 7_810_000, yield: 430 },
  { symbol: 'SGDC', name: 'Dina Singapore Dollar', rate: 1_340_000, yield: 320 },
  { symbol: 'KRWC', name: 'Dina South Korean Won', rate: 1_380_000_000, yield: 320 },
  // European
  { symbol: 'SEKG', name: 'Dina Swedish Krona', rate: 10_500_000, yield: 300 },
  { symbol: 'NOKC', name: 'Dina Norwegian Krone', rate: 10_800_000, yield: 350 },
  { symbol: 'DKKC', name: 'Dina Danish Krone', rate: 6_950_000, yield: 320 },
  { symbol: 'PLNC', name: 'Dina Polish Zloty', rate: 4_050_000, yield: 550 },
  { symbol: 'CZKC', name: 'Dina Czech Koruna', rate: 23_500_000, yield: 400 },
  { symbol: 'HUFC', name: 'Dina Hungarian Forint', rate: 375_000_000, yield: 650 },
  { symbol: 'RONC', name: 'Dina Romanian Leu', rate: 4_650_000, yield: 550 },
  // Americas
  { symbol: 'MXNC', name: 'Dina Mexican Peso', rate: 17_500_000, yield: 1050 },
  { symbol: 'BRSC', name: 'Dina Brazilian Real', rate: 5_100_000, yield: 1250 },
  { symbol: 'ARSC', name: 'Dina Argentine Peso', rate: 900_000_000, yield: 4000 },
  { symbol: 'CLPC', name: 'Dina Chilean Peso', rate: 950_000_000, yield: 500 },
  { symbol: 'COPC', name: 'Dina Colombian Peso', rate: 4_200_000_000, yield: 900 },
  { symbol: 'PENC', name: 'Dina Peruvian Sol', rate: 3_750_000, yield: 600 },
  // Asia-Pacific
  { symbol: 'INRC', name: 'Dina Indian Rupee', rate: 83_500_000, yield: 650 },
  { symbol: 'IDRC', name: 'Dina Indonesian Rupiah', rate: 15_800_000_000, yield: 600 },
  { symbol: 'THBC', name: 'Dina Thai Baht', rate: 35_500_000, yield: 200 },
  { symbol: 'MYRC', name: 'Dina Malaysian Ringgit', rate: 4_700_000, yield: 300 },
  { symbol: 'PHPC', name: 'Dina Philippine Peso', rate: 56_500_000, yield: 550 },
  { symbol: 'VNDC', name: 'Dina Vietnamese Dong', rate: 25_000_000_000, yield: 400 },
  { symbol: 'TWDC', name: 'Dina Taiwan Dollar', rate: 32_500_000, yield: 150 },
  { symbol: 'NZDC', name: 'Dina New Zealand Dollar', rate: 1_670_000, yield: 450 },
  // Middle East & Africa
  { symbol: 'AEDC', name: 'Dina UAE Dirham', rate: 3_670_000, yield: 450 },
  { symbol: 'SARC', name: 'Dina Saudi Riyal', rate: 3_750_000, yield: 500 },
  { symbol: 'ZARC', name: 'Dina South African Rand', rate: 18_500_000, yield: 800 },
  { symbol: 'NGNS', name: 'Dina Nigerian Naira', rate: 1_550_000_000, yield: 1800 },
  { symbol: 'KESG', name: 'Dina Kenyan Shilling', rate: 152_000_000, yield: 1000 },
  { symbol: 'EGPC', name: 'Dina Egyptian Pound', rate: 48_500_000, yield: 2200 },
  { symbol: 'GHSC', name: 'Dina Ghanaian Cedi', rate: 15_200_000, yield: 2500 },
  { symbol: 'TRYL', name: 'Dina Turkish Lira', rate: 32_500_000, yield: 4500 },
  { symbol: 'ILSC', name: 'Dina Israeli Shekel', rate: 3_650_000, yield: 450 },
  // Other
  { symbol: 'BAMC', name: 'Dina Bermudian Dollar', rate: 1_000_000, yield: 450 },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatUSDC(micro) {
  return (Number(micro) / 1_000_000).toFixed(2);
}

function formatYield(bps) {
  return (bps / 100).toFixed(2) + '%';
}

function formatRate(rate) {
  return (rate / 1_000_000).toFixed(6);
}

function padRight(str, len) {
  return String(str).padEnd(len);
}

function padLeft(str, len) {
  return String(str).padStart(len);
}

async function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Load WASM bytes for a contract if available. Checks both the standard
 * build output path and a flat wasm/ directory.
 */
function loadWasmBytes(contractName) {
  const candidates = [
    path.join(__dirname, '..', 'target', 'wasm32-unknown-unknown', 'release', `${contractName}.wasm`),
    path.join(__dirname, '..', 'target', 'wasm32-unknown-unknown', 'release', `dina_${contractName}.wasm`),
    path.join(__dirname, '..', 'wasm', `${contractName}.wasm`),
  ];
  for (const p of candidates) {
    if (fs.existsSync(p)) {
      return new Uint8Array(fs.readFileSync(p));
    }
  }
  return null;
}

/**
 * POST to the REST API's faucet endpoint to fund the deployer.
 */
async function fundFromFaucet(address) {
  const res = await fetch(`${REST_URL}/faucet/${address}`, { method: 'POST' });
  return res.json();
}

// ---------------------------------------------------------------------------
// Deployment via SDK (uses DinaClient.deployContract)
// ---------------------------------------------------------------------------

async function deployStablecoinViaSdk(client, wallet, currency, wasmBytes) {
  const initArgs = {
    name: currency.name,
    symbol: currency.symbol,
    decimals: 6,
    initial_supply: MINT_AMOUNT.toString(),
    admin: wallet.address,
    fx_rate: currency.rate,
    yield_bps: currency.yield,
  };

  const txHash = await client.deployContract(wallet, {
    wasmBytes,
    initArgs,
  });

  const receipt = await client.waitForTransaction(txHash, DEPLOY_TIMEOUT);
  if (!receipt.success) {
    throw new Error(`Deploy tx failed: ${receipt.error || 'unknown error'}`);
  }

  // The contract address is typically derived from the deployer + nonce or
  // returned in the receipt. We'll look for it in common fields.
  const contractAddress = receipt.contractAddress || receipt.contract || txHash;
  return { txHash, contractAddress, receipt };
}

// ---------------------------------------------------------------------------
// Deployment via REST API (fallback when WASM is not available locally)
// ---------------------------------------------------------------------------

async function deployStablecoinViaRest(currency, deployerAddress) {
  const body = {
    type: 'deploy_stablecoin',
    deployer: deployerAddress,
    params: {
      name: currency.name,
      symbol: currency.symbol,
      decimals: 6,
      initial_supply: MINT_AMOUNT.toString(),
      fx_rate: currency.rate,
      yield_bps: currency.yield,
    },
  };

  const res = await fetch(`${REST_URL}/v1/contracts/deploy`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });

  if (!res.ok) {
    const text = await res.text();
    throw new Error(`REST deploy failed (${res.status}): ${text}`);
  }

  return res.json();
}

// ---------------------------------------------------------------------------
// Registry and FX setup via contract calls
// ---------------------------------------------------------------------------

async function registerCurrency(client, wallet, registryAddr, currency, contractAddr) {
  return client.callContract(wallet, {
    contract: registryAddr,
    method: 'register_currency',
    args: {
      symbol: currency.symbol,
      name: currency.name,
      contract_address: contractAddr,
      fx_rate: currency.rate,
      yield_bps: currency.yield,
      decimals: 6,
    },
  });
}

async function setupFxPool(client, wallet, fxSwapAddr, currency, contractAddr) {
  return client.callContract(wallet, {
    contract: fxSwapAddr,
    method: 'create_pool',
    args: {
      base: 'USDC',
      quote: currency.symbol,
      quote_contract: contractAddr,
      oracle_rate: currency.rate,
      fee_bps: 30, // 0.30% swap fee
    },
  });
}

async function mintTestTokens(client, wallet, contractAddr, amount) {
  return client.callContract(wallet, {
    contract: contractAddr,
    method: 'mint',
    args: {
      to: wallet.address,
      amount: amount.toString(),
    },
  });
}

// ---------------------------------------------------------------------------
// Deploy infrastructure contracts (registry + FX swap)
// ---------------------------------------------------------------------------

async function deployInfrastructure(client, wallet) {
  console.log('');
  console.log('DEPLOYING INFRASTRUCTURE CONTRACTS');
  console.log('='.repeat(50));

  const results = { registry: null, fxSwap: null };

  // Try to deploy the currency registry via WASM first, then REST
  const registryWasm = loadWasmBytes('dina_currency_registry');
  if (registryWasm) {
    console.log('  Currency Registry: deploying via SDK (WASM found)...');
    const r = await deployStablecoinViaSdk(client, wallet, {
      name: 'Dina Currency Registry',
      symbol: 'REGISTRY',
      rate: 0,
      yield: 0,
    }, registryWasm);
    results.registry = r.contractAddress;
    console.log(`  Currency Registry: ${results.registry}`);
  } else {
    console.log('  Currency Registry: deploying via REST API...');
    try {
      const res = await fetch(`${REST_URL}/v1/contracts/deploy`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          type: 'deploy_registry',
          deployer: wallet.address,
          params: { name: 'Dina Currency Registry', admin: wallet.address },
        }),
      });
      if (res.ok) {
        const data = await res.json();
        results.registry = data.contract_address || data.address || data.txHash;
        console.log(`  Currency Registry: ${results.registry}`);
      } else {
        console.log(`  Currency Registry: REST deploy returned ${res.status} — using simulated address`);
        results.registry = `dina1registry${wallet.address.slice(10, 26)}`;
      }
    } catch (e) {
      console.log(`  Currency Registry: ${e.message} — using simulated address`);
      results.registry = `dina1registry${wallet.address.slice(10, 26)}`;
    }
  }

  // FX Swap contract
  const fxSwapWasm = loadWasmBytes('dina_fx_swap');
  if (fxSwapWasm) {
    console.log('  FX Swap:           deploying via SDK (WASM found)...');
    const r = await deployStablecoinViaSdk(client, wallet, {
      name: 'Dina FX Swap',
      symbol: 'FXSWAP',
      rate: 0,
      yield: 0,
    }, fxSwapWasm);
    results.fxSwap = r.contractAddress;
    console.log(`  FX Swap:           ${results.fxSwap}`);
  } else {
    console.log('  FX Swap:           deploying via REST API...');
    try {
      const res = await fetch(`${REST_URL}/v1/contracts/deploy`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          type: 'deploy_fx_swap',
          deployer: wallet.address,
          params: { name: 'Dina FX Swap', admin: wallet.address, fee_bps: 30 },
        }),
      });
      if (res.ok) {
        const data = await res.json();
        results.fxSwap = data.contract_address || data.address || data.txHash;
        console.log(`  FX Swap:           ${results.fxSwap}`);
      } else {
        console.log(`  FX Swap:           REST deploy returned ${res.status} — using simulated address`);
        results.fxSwap = `dina1fxswap${wallet.address.slice(10, 26)}`;
      }
    } catch (e) {
      console.log(`  FX Swap:           ${e.message} — using simulated address`);
      results.fxSwap = `dina1fxswap${wallet.address.slice(10, 26)}`;
    }
  }

  console.log('');
  return results;
}

// ---------------------------------------------------------------------------
// Deploy a single stablecoin
// ---------------------------------------------------------------------------

async function deploySingleCurrency(client, wallet, currency, infra, stablecoinWasm) {
  const result = {
    symbol: currency.symbol,
    name: currency.name,
    rate: currency.rate,
    yield: currency.yield,
    contractAddress: null,
    txHash: null,
    registered: false,
    fxPool: false,
    minted: false,
    error: null,
  };

  try {
    // Step 1: Deploy the stablecoin contract
    if (stablecoinWasm) {
      const r = await deployStablecoinViaSdk(client, wallet, currency, stablecoinWasm);
      result.contractAddress = r.contractAddress;
      result.txHash = r.txHash;
    } else {
      const data = await deployStablecoinViaRest(currency, wallet.address);
      result.contractAddress = data.contract_address || data.address || data.txHash;
      result.txHash = data.txHash || data.tx_hash;
    }

    // Step 2: Register in currency registry
    if (infra.registry && result.contractAddress) {
      try {
        const regTx = await registerCurrency(
          client, wallet, infra.registry, currency, result.contractAddress
        );
        await client.waitForTransaction(regTx, DEPLOY_TIMEOUT);
        result.registered = true;
      } catch (e) {
        // Non-fatal — registry may not be deployed yet
        result.registered = false;
      }
    }

    // Step 3: Set up FX swap pool
    if (infra.fxSwap && result.contractAddress) {
      try {
        const fxTx = await setupFxPool(
          client, wallet, infra.fxSwap, currency, result.contractAddress
        );
        await client.waitForTransaction(fxTx, DEPLOY_TIMEOUT);
        result.fxPool = true;
      } catch (e) {
        result.fxPool = false;
      }
    }

    // Step 4: Mint test tokens
    if (result.contractAddress) {
      try {
        const mintTx = await mintTestTokens(
          client, wallet, result.contractAddress, MINT_AMOUNT
        );
        await client.waitForTransaction(mintTx, DEPLOY_TIMEOUT);
        result.minted = true;
      } catch (e) {
        result.minted = false;
      }
    }
  } catch (e) {
    result.error = e.message;
  }

  return result;
}

// ---------------------------------------------------------------------------
// Batch deployment with concurrency control
// ---------------------------------------------------------------------------

async function deployBatch(client, wallet, currencies, infra, stablecoinWasm) {
  const results = [];
  const total = currencies.length;

  for (let i = 0; i < total; i += CONCURRENCY) {
    const batch = currencies.slice(i, i + CONCURRENCY);
    const batchResults = await Promise.allSettled(
      batch.map((c) => deploySingleCurrency(client, wallet, c, infra, stablecoinWasm))
    );

    for (const br of batchResults) {
      if (br.status === 'fulfilled') {
        results.push(br.value);
        const r = br.value;
        const status = r.error
          ? `FAILED: ${r.error}`
          : `OK  addr=${(r.contractAddress || '').substring(0, 20)}...`;
        console.log(`  [${results.length}/${total}] ${padRight(r.symbol, 6)} ${status}`);
      } else {
        results.push({
          symbol: batch[results.length % batch.length]?.symbol || '???',
          error: br.reason?.message || 'unknown',
        });
        console.log(`  [${results.length}/${total}] FAILED: ${br.reason?.message}`);
      }
    }

    // Small delay between batches to avoid overwhelming the node
    if (i + CONCURRENCY < total) {
      await sleep(500);
    }
  }

  return results;
}

// ---------------------------------------------------------------------------
// Summary table
// ---------------------------------------------------------------------------

function printSummary(results, infra) {
  console.log('');
  console.log('='.repeat(110));
  console.log('  DEPLOYMENT SUMMARY');
  console.log('='.repeat(110));
  console.log('');

  // Infrastructure
  console.log('  INFRASTRUCTURE');
  console.log('  ' + '-'.repeat(60));
  console.log(`  Currency Registry: ${infra.registry || 'NOT DEPLOYED'}`);
  console.log(`  FX Swap:           ${infra.fxSwap || 'NOT DEPLOYED'}`);
  console.log('');

  // Stablecoins table header
  const hdr = [
    padRight('#', 4),
    padRight('Symbol', 7),
    padRight('Name', 30),
    padLeft('Rate/USDC', 16),
    padLeft('APY', 8),
    padRight('Reg', 4),
    padRight('FX', 4),
    padRight('Mint', 5),
    padRight('Contract Address', 26),
  ].join(' ');

  console.log('  STABLECOINS');
  console.log('  ' + '-'.repeat(hdr.length));
  console.log('  ' + hdr);
  console.log('  ' + '-'.repeat(hdr.length));

  let deployed = 0;
  let failed = 0;

  results.forEach((r, i) => {
    if (r.error && !r.contractAddress) {
      failed++;
      const row = [
        padRight(i + 1, 4),
        padRight(r.symbol, 7),
        padRight(r.name || '', 30),
        padLeft('', 16),
        padLeft('', 8),
        padRight('', 4),
        padRight('', 4),
        padRight('', 5),
        'FAILED: ' + (r.error || '').substring(0, 40),
      ].join(' ');
      console.log('  ' + row);
    } else {
      deployed++;
      const addr = r.contractAddress || '';
      const row = [
        padRight(i + 1, 4),
        padRight(r.symbol, 7),
        padRight(r.name || '', 30),
        padLeft(formatRate(r.rate), 16),
        padLeft(formatYield(r.yield), 8),
        padRight(r.registered ? 'Y' : 'N', 4),
        padRight(r.fxPool ? 'Y' : 'N', 4),
        padRight(r.minted ? 'Y' : 'N', 5),
        addr.length > 24 ? addr.substring(0, 22) + '..' : addr,
      ].join(' ');
      console.log('  ' + row);
    }
  });

  console.log('  ' + '-'.repeat(hdr.length));
  console.log('');
  console.log(`  Total: ${results.length} currencies`);
  console.log(`  Deployed: ${deployed}  |  Failed: ${failed}`);
  console.log('');

  // Save results to JSON
  const outputPath = path.join(__dirname, '..', 'testnet-stablecoins.json');
  const output = {
    deployedAt: new Date().toISOString(),
    rpcUrl: RPC_URL,
    restUrl: REST_URL,
    infrastructure: infra,
    currencies: results.map((r) => ({
      symbol: r.symbol,
      name: r.name,
      rate: r.rate,
      yield: r.yield,
      contractAddress: r.contractAddress,
      txHash: r.txHash,
      registered: r.registered,
      fxPool: r.fxPool,
      minted: r.minted,
      error: r.error,
    })),
  };
  fs.writeFileSync(outputPath, JSON.stringify(output, null, 2));
  console.log(`  Results saved to: ${outputPath}`);
  console.log('');
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  console.log('============================================');
  console.log('  DINA NETWORK — STABLECOIN DEPLOYMENT');
  console.log('============================================');
  console.log('');
  console.log(`  RPC:   ${RPC_URL}`);
  console.log(`  REST:  ${REST_URL}`);
  console.log(`  Count: ${CURRENCIES.length} currencies`);
  console.log('');

  // ── 1. Initialize client and wallet ──
  const client = new DinaClient(RPC_URL, { timeout: DEPLOY_TIMEOUT });

  let wallet;
  if (DEPLOY_KEY) {
    wallet = DinaWallet.fromPrivateKey(DEPLOY_KEY);
    console.log(`  Wallet: ${wallet.address} (from key)`);
  } else {
    wallet = DinaWallet.generate();
    console.log(`  Wallet: ${wallet.address} (generated)`);
    console.log(`  Key:    ${wallet.exportPrivateKey()}`);
    console.log('');
    console.log('  WARNING: No deploy key provided. Generated a new wallet.');
    console.log('  Save the private key above to reuse this deployer.');
    console.log('  Set DINA_DEPLOY_KEY=<hex> or pass --key <hex>.');
  }
  console.log('');

  // ── 2. Check testnet connectivity ──
  console.log('CHECKING TESTNET');
  console.log('-'.repeat(40));
  try {
    const health = await fetch(`${REST_URL}/health`).then((r) => r.json());
    console.log(`  Chain:  ${health.chain_id || health.chainId || 'dina-testnet-1'}`);
    console.log(`  Height: Block #${health.height || health.blockHeight || '?'}`);
    console.log(`  Status: ${health.status || 'unknown'}`);
  } catch (e) {
    console.log(`  Status: UNREACHABLE (${e.message})`);
    console.log('  Continuing anyway — individual deploys will fail if node is down.');
  }
  console.log('');

  // ── 3. Fund deployer via faucet (testnet only) ──
  console.log('FUNDING DEPLOYER');
  console.log('-'.repeat(40));
  try {
    const faucetResult = await fundFromFaucet(wallet.address);
    if (faucetResult.success) {
      console.log('  Faucet: Funded 1,000 USDC');
    } else {
      console.log(`  Faucet: ${faucetResult.error || 'already funded or rate limited'}`);
    }
  } catch (e) {
    console.log(`  Faucet: ${e.message}`);
  }

  try {
    const balance = await client.getBalance(wallet.address);
    console.log(`  Balance: ${formatUSDC(balance)} USDC`);
  } catch (e) {
    console.log(`  Balance: could not query (${e.message})`);
  }
  console.log('');

  // ── 4. Check for pre-built WASM ──
  console.log('CHECKING WASM ARTIFACTS');
  console.log('-'.repeat(40));
  const stablecoinWasm = loadWasmBytes('dina_stablecoin_factory');
  if (stablecoinWasm) {
    console.log(`  Stablecoin WASM: found (${stablecoinWasm.length} bytes)`);
    console.log('  Will deploy via SDK (DinaClient.deployContract)');
  } else {
    console.log('  Stablecoin WASM: not found');
    console.log('  Will deploy via REST API (/v1/contracts/deploy)');
    console.log('');
    console.log('  To build WASM locally, run:');
    console.log('    cd contracts/dina-stablecoin-factory');
    console.log('    cargo build --target wasm32-unknown-unknown --release');
  }
  console.log('');

  // ── 5. Deploy infrastructure ──
  const infra = await deployInfrastructure(client, wallet);

  // ── 6. Deploy all stablecoins ──
  console.log('DEPLOYING STABLECOINS');
  console.log('='.repeat(50));
  console.log(`  Deploying ${CURRENCIES.length} currencies (concurrency: ${CONCURRENCY})`);
  console.log('');

  const results = await deployBatch(client, wallet, CURRENCIES, infra, stablecoinWasm);

  // ── 7. Print summary ──
  printSummary(results, infra);
}

main().catch((e) => {
  console.error('');
  console.error('DEPLOYMENT FAILED');
  console.error('-'.repeat(40));
  console.error(e.message);
  if (e.stack) {
    console.error(e.stack);
  }
  process.exit(1);
});
