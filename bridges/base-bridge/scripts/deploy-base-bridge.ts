/**
 * Deploy BaseBridge.sol to Base Sepolia.
 *
 * Usage:
 *   cp .env.example .env
 *   # Set DEPLOYER_PRIVATE_KEY, RELAYER_ADDRESS in .env
 *   npx hardhat run scripts/deploy-base-bridge.ts --network baseSepolia
 *
 * USDC on Base Sepolia: 0x036CbD53842c5426634e7929541eC2318f3dCF7e
 */

import { ethers } from "hardhat";
import * as dotenv from "dotenv";

dotenv.config();

const USDC_BASE_SEPOLIA = "0x036CbD53842c5426634e7929541eC2318f3dCF7e";

async function main() {
  const relayerAddress = process.env.RELAYER_ADDRESS;
  if (!relayerAddress || relayerAddress === "0x0000000000000000000000000000000000000000") {
    throw new Error(
      "Set RELAYER_ADDRESS in .env — use the El Tesoro relayer service address"
    );
  }

  const [deployer] = await ethers.getSigners();
  const balance = await ethers.provider.getBalance(deployer.address);

  console.log("Deploying BaseBridge to Base Sepolia");
  console.log("  Deployer:", deployer.address);
  console.log("  Balance: ", ethers.formatEther(balance), "ETH");
  console.log("  USDC:    ", USDC_BASE_SEPOLIA);
  console.log("  Relayer: ", relayerAddress);

  if (balance < ethers.parseEther("0.001")) {
    throw new Error(
      "Insufficient ETH for deployment. Get testnet ETH from https://base-faucet.alchemy.com/"
    );
  }

  const BaseBridge = await ethers.getContractFactory("BaseBridge");
  const bridge = await BaseBridge.deploy(USDC_BASE_SEPOLIA, relayerAddress);
  await bridge.waitForDeployment();

  const address = await bridge.getAddress();

  console.log("\nBaseBridge deployed successfully.");
  console.log("  Contract address:", address);
  console.log("  Network:          Base Sepolia (chainId 84532)");

  console.log("\nNext steps:");
  console.log(
    `  1. Verify:  npx hardhat verify --network baseSepolia ${address} ${USDC_BASE_SEPOLIA} ${relayerAddress}`
  );
  console.log("  2. Update dinaContracts.ts in El Tesoro backend with this address.");
  console.log("  3. Update DEPLOYED_CONTRACTS.md in dina-network repo.");
  console.log("  4. Start the relayer service (bridges/base-bridge/relayer/).");
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
