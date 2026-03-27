/**
 * Base <-> Dina Bridge Relayer
 *
 * This service bridges USDC between Base and Dina Network:
 *
 *   Base -> Dina:
 *     1. Watches DinaBridge on Base for `Deposited` events
 *     2. For each deposit, computes the SHA-256 proof and calls `claim` on
 *        the Dina-side bridge-base contract to mint bridged USDC
 *
 *   Dina -> Base:
 *     1. Polls the Dina bridge-base contract for pending (unprocessed) withdrawals
 *     2. For each pending withdrawal, signs an EIP-191 withdrawal proof and
 *        submits `withdraw` on the Base DinaBridge contract to unlock USDC
 *     3. After Base tx confirms, calls `mark_withdrawal_processed` on Dina
 */

import { ethers } from "ethers";
import * as dotenv from "dotenv";
import * as crypto from "crypto";

dotenv.config();

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface JsonRpcResponse {
  jsonrpc: string;
  id: number;
  result?: any;
  error?: { code: number; message: string };
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const BASE_RPC_URL = process.env.BASE_RPC_URL || "https://sepolia.base.org";
const DINA_RPC_URL = process.env.DINA_RPC_URL || "http://35.184.213.248:8545";
const BRIDGE_ADDRESS_BASE = process.env.BRIDGE_ADDRESS_BASE || "";
const BRIDGE_CONTRACT_DINA = process.env.BRIDGE_CONTRACT_DINA || "bridge-base";
const RELAYER_PRIVATE_KEY = process.env.RELAYER_PRIVATE_KEY || "";
const DINA_RELAYER_ADDRESS = process.env.DINA_RELAYER_ADDRESS || "";
const DINA_POLL_INTERVAL_MS = parseInt(process.env.DINA_POLL_INTERVAL_MS || "5000", 10);
const BASE_CONFIRMATIONS = parseInt(process.env.BASE_CONFIRMATIONS || "2", 10);

// ---------------------------------------------------------------------------
// ABIs (minimal — only the functions/events we use)
// ---------------------------------------------------------------------------

const BASE_BRIDGE_ABI = [
  "event Deposited(address indexed sender, string dinaRecipient, uint256 amount, uint256 nonce)",
  "event Withdrawn(address indexed recipient, uint256 amount, bytes32 withdrawalId)",
  "function withdraw(address recipient, uint256 amount, bytes32 withdrawalId, bytes calldata relayerSignature) external",
  "function processedWithdrawals(bytes32) external view returns (bool)",
  "function depositNonce() external view returns (uint256)",
  "function totalLocked() external view returns (uint256)",
];

// ---------------------------------------------------------------------------
// Providers & wallets
// ---------------------------------------------------------------------------

let baseProvider: ethers.JsonRpcProvider;
let baseWallet: ethers.Wallet;
let baseBridge: ethers.Contract;

function initBase(): void {
  if (!BRIDGE_ADDRESS_BASE || BRIDGE_ADDRESS_BASE === "0x" + "00".repeat(20)) {
    throw new Error("Set BRIDGE_ADDRESS_BASE in .env");
  }
  if (!RELAYER_PRIVATE_KEY || RELAYER_PRIVATE_KEY === "0x" + "00".repeat(32)) {
    throw new Error("Set RELAYER_PRIVATE_KEY in .env");
  }

  baseProvider = new ethers.JsonRpcProvider(BASE_RPC_URL);
  baseWallet = new ethers.Wallet(RELAYER_PRIVATE_KEY, baseProvider);
  baseBridge = new ethers.Contract(BRIDGE_ADDRESS_BASE, BASE_BRIDGE_ABI, baseWallet);

  console.log(`[base] Provider: ${BASE_RPC_URL}`);
  console.log(`[base] Bridge:   ${BRIDGE_ADDRESS_BASE}`);
  console.log(`[base] Relayer:  ${baseWallet.address}`);
}

// ---------------------------------------------------------------------------
// SHA-256 proof for Dina claim
// ---------------------------------------------------------------------------

/**
 * Compute the proof that the Dina bridge-base contract expects:
 *   SHA-256(base_tx_hash || amount_le_bytes || recipient_32 || relayer_32)
 */
function computeDinaClaimProof(
  baseTxHash: Uint8Array,
  amount: bigint,
  recipient: Uint8Array,
  relayerAddr: Uint8Array
): Uint8Array {
  const amountBuf = Buffer.alloc(8);
  amountBuf.writeBigUInt64LE(amount);

  const input = Buffer.concat([
    Buffer.from(baseTxHash),
    amountBuf,
    Buffer.from(recipient),
    Buffer.from(relayerAddr),
  ]);

  return crypto.createHash("sha256").update(input).digest();
}

/**
 * Convert a hex string (0x-prefixed) into a 32-byte Uint8Array.
 * If the hex is shorter than 32 bytes, it is left-padded with zeros.
 */
function hexTo32Bytes(hex: string): Uint8Array {
  const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
  const padded = clean.padStart(64, "0");
  return Uint8Array.from(Buffer.from(padded, "hex"));
}

/**
 * Convert a 20-byte Ethereum address to a 32-byte array (left-padded with zeros).
 */
function addressTo32Bytes(addr: string): Uint8Array {
  return hexTo32Bytes(addr);
}

// ---------------------------------------------------------------------------
// Base -> Dina:  Watch deposits, mint on Dina
// ---------------------------------------------------------------------------

async function watchBaseDeposits(): Promise<void> {
  console.log("[base->dina] Watching for Deposited events...");

  baseBridge.on("Deposited", async (sender: string, dinaRecipient: string, amount: bigint, nonce: bigint) => {
    console.log(`[base->dina] Deposit #${nonce}: ${sender} -> ${dinaRecipient} for ${ethers.formatUnits(amount, 6)} USDC`);

    try {
      // Wait for confirmations
      // The event fires on the latest block; we need to wait for finality.
      const currentBlock = await baseProvider.getBlockNumber();
      console.log(`[base->dina] Current block: ${currentBlock}, waiting for ${BASE_CONFIRMATIONS} confirmations...`);

      // Get the transaction hash from the event log
      // In ethers v6, the event object from .on() doesn't include the log directly,
      // so we query recent logs to find the matching nonce.
      const filter = baseBridge.filters.Deposited(sender);
      const logs = await baseBridge.queryFilter(filter, currentBlock - 5, currentBlock);
      const matchingLog = logs.find((log: any) => {
        const parsed = baseBridge.interface.parseLog({ topics: log.topics as string[], data: log.data });
        return parsed && parsed.args[3] === nonce;
      });

      if (!matchingLog) {
        console.error(`[base->dina] Could not find log for nonce ${nonce}`);
        return;
      }

      const txHash = matchingLog.transactionHash;
      console.log(`[base->dina] Base tx: ${txHash}`);

      // Wait for confirmations
      const txReceipt = await baseProvider.waitForTransaction(txHash, BASE_CONFIRMATIONS);
      if (!txReceipt || txReceipt.status !== 1) {
        console.error(`[base->dina] Transaction ${txHash} failed or was reverted`);
        return;
      }

      // Mint on Dina
      await mintOnDina(txHash, dinaRecipient, amount);
    } catch (err) {
      console.error(`[base->dina] Error processing deposit #${nonce}:`, err);
    }
  });
}

/**
 * Call the Dina bridge-base contract's `claim` method to mint bridged USDC.
 *
 * The Dina RPC accepts JSON-RPC calls with a `contract_call` method:
 *   {
 *     "method": "contract_call",
 *     "params": {
 *       "contract": "bridge-base",
 *       "method": "claim",
 *       "args": { ... },
 *       "caller": "0x..."
 *     }
 *   }
 */
async function mintOnDina(baseTxHash: string, dinaRecipient: string, amount: bigint): Promise<void> {
  console.log(`[base->dina] Minting ${ethers.formatUnits(amount, 6)} bridged-USDC on Dina for ${dinaRecipient}`);

  const txHashBytes = hexTo32Bytes(baseTxHash);
  const recipientBytes = hexTo32Bytes(dinaRecipient);
  const relayerBytes = hexTo32Bytes(DINA_RELAYER_ADDRESS);

  const proof = computeDinaClaimProof(txHashBytes, amount, recipientBytes, relayerBytes);

  // Convert to arrays for JSON serialization
  const claimArgs = {
    base_tx_hash: Array.from(txHashBytes),
    amount: Number(amount),
    recipient: Array.from(recipientBytes),
    proof: Array.from(proof),
  };

  try {
    const response = await fetch(DINA_RPC_URL, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: Date.now(),
        method: "contract_call",
        params: {
          contract: BRIDGE_CONTRACT_DINA,
          method: "claim",
          args: claimArgs,
          caller: Array.from(relayerBytes),
        },
      }),
    });

    const result = await response.json() as JsonRpcResponse;

    if (result.error) {
      console.error(`[base->dina] Dina claim failed:`, result.error);
      return;
    }

    console.log(`[base->dina] Minted on Dina. Result:`, result.result);
  } catch (err) {
    console.error(`[base->dina] Failed to call Dina RPC:`, err);
  }
}

// ---------------------------------------------------------------------------
// Dina -> Base:  Poll withdrawals, unlock on Base
// ---------------------------------------------------------------------------

/**
 * Poll the Dina bridge-base contract for pending withdrawals and process them.
 */
async function pollDinaWithdrawals(): Promise<void> {
  console.log(`[dina->base] Polling for pending withdrawals every ${DINA_POLL_INTERVAL_MS}ms...`);

  const poll = async () => {
    try {
      // Query pending withdrawal count
      const countResponse = await fetch(DINA_RPC_URL, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          jsonrpc: "2.0",
          id: Date.now(),
          method: "contract_call",
          params: {
            contract: BRIDGE_CONTRACT_DINA,
            method: "pending_withdrawal_count",
            args: {},
            caller: Array.from(hexTo32Bytes(DINA_RELAYER_ADDRESS)),
          },
        }),
      });

      const countResult = await countResponse.json() as JsonRpcResponse;
      const pendingCount = countResult.result;

      if (!pendingCount || pendingCount === 0) {
        return; // Nothing to process
      }

      console.log(`[dina->base] Found ${pendingCount} pending withdrawal(s)`);

      // Fetch each pending withdrawal by iterating IDs.
      // The Dina contract uses auto-incrementing IDs starting at 1.
      // We'll try recent IDs and check if they're unprocessed.
      // In production, track the last processed ID in persistent storage.
      for (let id = 1; id <= 10000; id++) {
        const wResponse = await fetch(DINA_RPC_URL, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            jsonrpc: "2.0",
            id: Date.now(),
            method: "contract_call",
            params: {
              contract: BRIDGE_CONTRACT_DINA,
              method: "get_withdrawal",
              args: { id },
              caller: Array.from(hexTo32Bytes(DINA_RELAYER_ADDRESS)),
            },
          }),
        });

        const wResult = await wResponse.json() as JsonRpcResponse;
        const withdrawal = wResult.result;

        if (!withdrawal) {
          break; // No more withdrawals
        }

        if (withdrawal.processed) {
          continue; // Already processed
        }

        console.log(`[dina->base] Processing withdrawal #${withdrawal.id}: ${withdrawal.amount} to Base recipient`);
        await processWithdrawalOnBase(withdrawal);
      }
    } catch (err) {
      console.error("[dina->base] Poll error:", err);
    }
  };

  // Initial poll, then schedule recurring
  await poll();
  setInterval(poll, DINA_POLL_INTERVAL_MS);
}

/**
 * Process a single Dina withdrawal on Base:
 *   1. Derive the withdrawal ID as bytes32
 *   2. Sign the withdrawal proof
 *   3. Submit to DinaBridge.withdraw() on Base
 *   4. Mark as processed on Dina
 */
async function processWithdrawalOnBase(withdrawal: {
  id: number;
  sender: number[];
  base_recipient: number[];
  amount: number;
  timestamp: number;
  processed: boolean;
}): Promise<void> {
  // Convert base_recipient from 32-byte array to 20-byte Ethereum address.
  // The Dina contract stores it as zero-padded 32 bytes (left-padded).
  const recipientHex = "0x" + Buffer.from(withdrawal.base_recipient).toString("hex").slice(24); // last 20 bytes
  const amount = BigInt(withdrawal.amount);

  // Create a unique withdrawal ID as bytes32 from the Dina withdrawal ID
  const withdrawalId = ethers.solidityPackedKeccak256(
    ["string", "uint256"],
    ["dina-withdrawal-", withdrawal.id]
  );

  // Check if already processed on Base
  const alreadyProcessed = await baseBridge.processedWithdrawals(withdrawalId);
  if (alreadyProcessed) {
    console.log(`[dina->base] Withdrawal #${withdrawal.id} already processed on Base, marking on Dina...`);
    await markProcessedOnDina(withdrawal.id);
    return;
  }

  // Sign the withdrawal proof (EIP-191 personal sign)
  const chainId = (await baseProvider.getNetwork()).chainId;
  const messageHash = ethers.solidityPackedKeccak256(
    ["address", "uint256", "bytes32", "uint256"],
    [recipientHex, amount, withdrawalId, chainId]
  );
  const signature = await baseWallet.signMessage(ethers.getBytes(messageHash));

  console.log(`[dina->base] Submitting withdraw to Base: ${recipientHex} for ${ethers.formatUnits(amount, 6)} USDC`);

  try {
    const tx = await baseBridge.withdraw(recipientHex, amount, withdrawalId, signature);
    console.log(`[dina->base] Base tx submitted: ${tx.hash}`);

    const receipt = await tx.wait(BASE_CONFIRMATIONS);
    if (!receipt || receipt.status !== 1) {
      console.error(`[dina->base] Base tx ${tx.hash} failed`);
      return;
    }

    console.log(`[dina->base] Base tx confirmed: ${tx.hash}`);

    // Mark as processed on Dina
    await markProcessedOnDina(withdrawal.id);
  } catch (err) {
    console.error(`[dina->base] Failed to submit withdrawal on Base:`, err);
  }
}

/**
 * Call `mark_withdrawal_processed` on the Dina bridge-base contract.
 */
async function markProcessedOnDina(withdrawalId: number): Promise<void> {
  try {
    const response = await fetch(DINA_RPC_URL, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: Date.now(),
        method: "contract_call",
        params: {
          contract: BRIDGE_CONTRACT_DINA,
          method: "mark_withdrawal_processed",
          args: { withdrawal_id: withdrawalId },
          caller: Array.from(hexTo32Bytes(DINA_RELAYER_ADDRESS)),
        },
      }),
    });

    const result = await response.json() as JsonRpcResponse;
    if (result.error) {
      console.error(`[dina->base] Failed to mark withdrawal #${withdrawalId} processed on Dina:`, result.error);
    } else {
      console.log(`[dina->base] Withdrawal #${withdrawalId} marked processed on Dina`);
    }
  } catch (err) {
    console.error(`[dina->base] Error marking processed on Dina:`, err);
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
  console.log("=== Dina Bridge Relayer ===");
  console.log(`Base RPC:  ${BASE_RPC_URL}`);
  console.log(`Dina RPC:  ${DINA_RPC_URL}`);

  initBase();

  const nonce = await baseBridge.depositNonce();
  const locked = await baseBridge.totalLocked();
  console.log(`[base] Current deposit nonce: ${nonce}`);
  console.log(`[base] Total USDC locked: ${ethers.formatUnits(locked, 6)}`);

  // Start both watchers
  await Promise.all([
    watchBaseDeposits(),
    pollDinaWithdrawals(),
  ]);

  console.log("[relayer] Running. Press Ctrl+C to stop.");
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});
