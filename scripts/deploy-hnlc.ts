#!/usr/bin/env node
/**
 * Deploy HNLc (Lempira Digital) as a DRC-1 token on Dina testnet.
 *
 * This script:
 *   1. Reads the treasury keypair from environment (TREASURY_PRIVATE_KEY)
 *      or generates + prints one for first-time setup.
 *   2. Funds the treasury address from the testnet faucet.
 *   3. Deploys the DRC-1 WASM contract with HNLc parameters.
 *   4. Initialises the contract (name, symbol, decimals, owner=treasury).
 *   5. Deploys the bridge-base contract (for the Base <-> Dina bridge).
 *   6. Prints all addresses for DEPLOYED_CONTRACTS.md and dinaContracts.ts.
 *
 * Usage:
 *   TREASURY_PRIVATE_KEY=<hex> ts-node scripts/deploy-hnlc.ts
 *
 * First run (generate a fresh key):
 *   ts-node scripts/deploy-hnlc.ts
 *   # Copy the printed private key to GCP Secret Manager as:
 *   #   projects/banco-el-tesoro/secrets/dina-treasury-private-key
 *   # Then re-run with TREASURY_PRIVATE_KEY set.
 */

import * as path from "path";
import * as fs from "fs";

// --- SDK -------------------------------------------------------------------
// The dina-js dist/ is pre-built; import from there.
const sdkPath = path.resolve(__dirname, "../sdk/dina-js/dist");
// eslint-disable-next-line @typescript-eslint/no-var-requires
const { DinaWallet, DinaClient } = require(sdkPath);

const REST_URL = process.env.DINA_RPC_URL ?? "http://35.184.213.248:8080";

// WASM bytes for the DRC-1 contract (compiled from contracts/drc1-token/).
// In production these are read from the compiled artifact.
const DRC1_WASM_PATH = path.resolve(
  __dirname,
  "../contracts/drc1-token/target/wasm32-unknown-unknown/release/drc1_token.wasm"
);
const BRIDGE_BASE_WASM_PATH = path.resolve(
  __dirname,
  "../contracts/bridge-base/target/wasm32-unknown-unknown/release/bridge_base.wasm"
);

async function requestFaucet(address: string): Promise<void> {
  console.log(`  Requesting faucet funds for ${address} ...`);
  const res = await fetch(`${REST_URL}/faucet`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ address }),
  });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`Faucet request failed (${res.status}): ${body}`);
  }
  console.log("  Faucet funded successfully.");
}

async function main(): Promise<void> {
  console.log("=".repeat(60));
  console.log("  HNLc (Lempira Digital) — Dina Testnet Deployment");
  console.log(`  RPC: ${REST_URL}`);
  console.log("=".repeat(60));

  // -------------------------------------------------------------------------
  // 1. Treasury keypair
  // -------------------------------------------------------------------------
  let wallet: typeof DinaWallet;
  const privKeyHex = process.env.TREASURY_PRIVATE_KEY;

  if (!privKeyHex) {
    wallet = DinaWallet.generate();
    // Access private key bytes via internal field for display (server-only use)
    const walletJson = wallet.toJSON();
    console.log("\n[!] No TREASURY_PRIVATE_KEY set — generated a fresh keypair.");
    console.log("    Store the private key in GCP Secret Manager immediately:");
    console.log("      Secret name: dina-treasury-private-key");
    console.log("      Project:     banco-el-tesoro");
    console.log(`      Public key:  ${walletJson.publicKey}`);
    console.log(`      Address:     ${walletJson.address}`);
    console.log("\n    Re-run with TREASURY_PRIVATE_KEY=<hex> after storing it.\n");
    process.exit(0);
  } else {
    wallet = DinaWallet.fromPrivateKey(privKeyHex);
  }

  const { address: treasuryAddress, publicKey: treasuryPublicKey } = wallet.toJSON();
  console.log(`\nTreasury address:    ${treasuryAddress}`);
  console.log(`Treasury public key: ${treasuryPublicKey}`);

  const client = new DinaClient(REST_URL);

  // -------------------------------------------------------------------------
  // 2. Fund treasury from faucet
  // -------------------------------------------------------------------------
  console.log("\n[1/4] Funding treasury from faucet ...");
  await requestFaucet(treasuryAddress);

  // -------------------------------------------------------------------------
  // 3. Deploy HNLc DRC-1 token
  // -------------------------------------------------------------------------
  console.log("\n[2/4] Deploying HNLc DRC-1 token ...");

  if (!fs.existsSync(DRC1_WASM_PATH)) {
    throw new Error(
      `DRC-1 WASM not found at ${DRC1_WASM_PATH}.\n` +
        "  Build it with:\n" +
        "    cd contracts/drc1-token\n" +
        "    cargo build --target wasm32-unknown-unknown --release"
    );
  }

  const drc1Wasm = fs.readFileSync(DRC1_WASM_PATH);

  const deployTxHash: string = await client.deployContract(wallet, {
    wasmBytes: drc1Wasm,
    initArgs: {
      name: "Lempira Digital",
      symbol: "HNLc",
      decimals: 2,
    },
  });

  console.log(`  Deploy tx hash: ${deployTxHash}`);
  const receipt = await client.waitForTransaction(deployTxHash, 30_000);

  if (!receipt.success) {
    throw new Error(`HNLc deploy failed: ${receipt.error}`);
  }

  // Contract address is returned in the receipt (Dina convention: address = tx hash of deploy)
  const hnlcAddress: string = deployTxHash;
  console.log(`  HNLc token address: ${hnlcAddress}`);

  // -------------------------------------------------------------------------
  // 4. Deploy bridge-base contract
  // -------------------------------------------------------------------------
  console.log("\n[3/4] Deploying bridge-base (Dina side) ...");

  if (!fs.existsSync(BRIDGE_BASE_WASM_PATH)) {
    throw new Error(
      `bridge-base WASM not found at ${BRIDGE_BASE_WASM_PATH}.\n` +
        "  Build it with:\n" +
        "    cd contracts/bridge-base\n" +
        "    cargo build --target wasm32-unknown-unknown --release"
    );
  }

  const bridgeWasm = fs.readFileSync(BRIDGE_BASE_WASM_PATH);

  const bridgeTxHash: string = await client.deployContract(wallet, {
    wasmBytes: bridgeWasm,
    initArgs: {
      // Relayer = treasury for testnet; replace with dedicated relayer key in production
      relayer: treasuryAddress,
      usdc_token: hnlcAddress,
    },
  });

  console.log(`  Bridge deploy tx hash: ${bridgeTxHash}`);
  const bridgeReceipt = await client.waitForTransaction(bridgeTxHash, 30_000);

  if (!bridgeReceipt.success) {
    throw new Error(`bridge-base deploy failed: ${bridgeReceipt.error}`);
  }

  const baseBridgeDinaAddress: string = bridgeTxHash;
  console.log(`  Bridge-base address: ${baseBridgeDinaAddress}`);

  // -------------------------------------------------------------------------
  // 5. Print summary
  // -------------------------------------------------------------------------
  console.log("\n[4/4] Deployment complete.");
  console.log("=".repeat(60));
  console.log("  SUMMARY");
  console.log("=".repeat(60));
  console.log(`  HNLc token (Dina):        ${hnlcAddress}`);
  console.log(`  Base Bridge (Dina side):  ${baseBridgeDinaAddress}`);
  console.log(`  Treasury address:         ${treasuryAddress}`);
  console.log(`  Treasury public key:      ${treasuryPublicKey}`);
  console.log("=".repeat(60));
  console.log("\nNext steps:");
  console.log("  1. Deploy BaseBridge.sol to Base Sepolia (see bridges/base-bridge/).");
  console.log("  2. Update docs/DEPLOYED_CONTRACTS.md with the addresses above.");
  console.log("  3. Update backend/src/data/dinaContracts.ts.");
  console.log("  4. Store TREASURY_PRIVATE_KEY in GCP Secret Manager.");
}

main().catch((err) => {
  console.error("\nDeploy failed:", err.message ?? err);
  process.exit(1);
});
