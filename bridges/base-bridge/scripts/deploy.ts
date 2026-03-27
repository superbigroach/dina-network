import { ethers } from "hardhat";
import * as dotenv from "dotenv";

dotenv.config();

async function main() {
  const usdcAddress = process.env.USDC_ADDRESS;
  const relayerAddress = process.env.RELAYER_ADDRESS;

  if (!usdcAddress || usdcAddress === "0x0000000000000000000000000000000000000000") {
    throw new Error("Set USDC_ADDRESS in .env");
  }
  if (!relayerAddress || relayerAddress === "0x0000000000000000000000000000000000000000") {
    throw new Error("Set RELAYER_ADDRESS in .env");
  }

  const [deployer] = await ethers.getSigners();
  console.log("Deploying DinaBridge with account:", deployer.address);
  console.log("Account balance:", ethers.formatEther(await ethers.provider.getBalance(deployer.address)), "ETH");

  const DinaBridge = await ethers.getContractFactory("DinaBridge");
  const bridge = await DinaBridge.deploy(usdcAddress, relayerAddress);
  await bridge.waitForDeployment();

  const address = await bridge.getAddress();
  console.log("DinaBridge deployed to:", address);
  console.log("USDC token:", usdcAddress);
  console.log("Relayer:", relayerAddress);

  console.log("\nNext steps:");
  console.log(`1. Verify: npx hardhat verify --network baseSepolia ${address} ${usdcAddress} ${relayerAddress}`);
  console.log("2. Update BRIDGE_ADDRESS_BASE in the relayer .env");
  console.log("3. Users must approve USDC spending: usdc.approve(bridgeAddress, amount)");
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
