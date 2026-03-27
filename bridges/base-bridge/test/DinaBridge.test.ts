import { expect } from "chai";
import { ethers } from "hardhat";
import { DinaBridge } from "../typechain-types";
import { SignerWithAddress } from "@nomicfoundation/hardhat-ethers/signers";

describe("DinaBridge", function () {
  let bridge: DinaBridge;
  let usdc: any; // MockERC20
  let owner: SignerWithAddress;
  let relayer: SignerWithAddress;
  let user: SignerWithAddress;
  let recipient: SignerWithAddress;

  const INITIAL_USDC = ethers.parseUnits("1000000", 6); // 1M USDC
  const DEPOSIT_AMOUNT = ethers.parseUnits("100", 6); // 100 USDC
  const DINA_ADDRESS = "0x" + "ab".repeat(32); // 66 chars: 0x + 64 hex

  beforeEach(async function () {
    [owner, relayer, user, recipient] = await ethers.getSigners();

    // Deploy a mock ERC20 for USDC
    const MockERC20 = await ethers.getContractFactory("MockUSDC");
    usdc = await MockERC20.deploy();
    await usdc.waitForDeployment();

    // Deploy bridge
    const DinaBridgeFactory = await ethers.getContractFactory("DinaBridge");
    bridge = await DinaBridgeFactory.deploy(
      await usdc.getAddress(),
      relayer.address
    ) as DinaBridge;
    await bridge.waitForDeployment();

    // Give user some USDC and approve bridge
    await usdc.mint(user.address, INITIAL_USDC);
    await usdc.connect(user).approve(await bridge.getAddress(), INITIAL_USDC);
  });

  // ---------------------------------------------------------------------------
  // Deployment
  // ---------------------------------------------------------------------------

  describe("Deployment", function () {
    it("should set correct USDC address", async function () {
      expect(await bridge.usdc()).to.equal(await usdc.getAddress());
    });

    it("should set correct relayer", async function () {
      expect(await bridge.relayer()).to.equal(relayer.address);
    });

    it("should set correct owner", async function () {
      expect(await bridge.owner()).to.equal(owner.address);
    });

    it("should revert on zero USDC address", async function () {
      const Factory = await ethers.getContractFactory("DinaBridge");
      await expect(
        Factory.deploy(ethers.ZeroAddress, relayer.address)
      ).to.be.revertedWith("DinaBridge: zero USDC address");
    });

    it("should revert on zero relayer address", async function () {
      const Factory = await ethers.getContractFactory("DinaBridge");
      await expect(
        Factory.deploy(await usdc.getAddress(), ethers.ZeroAddress)
      ).to.be.revertedWith("DinaBridge: zero relayer address");
    });
  });

  // ---------------------------------------------------------------------------
  // Deposits  (Base -> Dina)
  // ---------------------------------------------------------------------------

  describe("Deposits", function () {
    it("should lock USDC and emit Deposited event", async function () {
      await expect(bridge.connect(user).deposit(DEPOSIT_AMOUNT, DINA_ADDRESS))
        .to.emit(bridge, "Deposited")
        .withArgs(user.address, DINA_ADDRESS, DEPOSIT_AMOUNT, 1);

      expect(await bridge.totalLocked()).to.equal(DEPOSIT_AMOUNT);
      expect(await bridge.depositNonce()).to.equal(1);
    });

    it("should increment nonce on each deposit", async function () {
      await bridge.connect(user).deposit(DEPOSIT_AMOUNT, DINA_ADDRESS);
      await bridge.connect(user).deposit(DEPOSIT_AMOUNT, DINA_ADDRESS);
      expect(await bridge.depositNonce()).to.equal(2);
    });

    it("should revert below minimum", async function () {
      await expect(
        bridge.connect(user).deposit(100, DINA_ADDRESS) // 0.0001 USDC
      ).to.be.revertedWith("DinaBridge: below minimum");
    });

    it("should revert above maximum", async function () {
      const tooMuch = ethers.parseUnits("2000000", 6); // 2M
      await usdc.mint(user.address, tooMuch);
      await usdc.connect(user).approve(await bridge.getAddress(), tooMuch);
      await expect(
        bridge.connect(user).deposit(tooMuch, DINA_ADDRESS)
      ).to.be.revertedWith("DinaBridge: above maximum");
    });

    it("should revert with invalid Dina address length", async function () {
      await expect(
        bridge.connect(user).deposit(DEPOSIT_AMOUNT, "0xdeadbeef")
      ).to.be.revertedWith("DinaBridge: invalid Dina address");
    });

    it("should revert when paused", async function () {
      await bridge.setPaused(true);
      await expect(
        bridge.connect(user).deposit(DEPOSIT_AMOUNT, DINA_ADDRESS)
      ).to.be.revertedWith("DinaBridge: paused");
    });
  });

  // ---------------------------------------------------------------------------
  // Withdrawals  (Dina -> Base)
  // ---------------------------------------------------------------------------

  describe("Withdrawals", function () {
    const withdrawalId = ethers.keccak256(ethers.toUtf8Bytes("withdrawal-1"));

    async function signWithdrawal(
      signer: SignerWithAddress,
      recipientAddr: string,
      amount: bigint,
      wId: string
    ): Promise<string> {
      const chainId = (await ethers.provider.getNetwork()).chainId;
      const messageHash = ethers.solidityPackedKeccak256(
        ["address", "uint256", "bytes32", "uint256"],
        [recipientAddr, amount, wId, chainId]
      );
      return signer.signMessage(ethers.getBytes(messageHash));
    }

    beforeEach(async function () {
      // Seed the bridge with USDC so it can pay out withdrawals
      await usdc.mint(await bridge.getAddress(), DEPOSIT_AMOUNT);
    });

    it("should unlock USDC with valid relayer signature", async function () {
      const sig = await signWithdrawal(
        relayer,
        recipient.address,
        DEPOSIT_AMOUNT,
        withdrawalId
      );

      await expect(
        bridge.withdraw(recipient.address, DEPOSIT_AMOUNT, withdrawalId, sig)
      )
        .to.emit(bridge, "Withdrawn")
        .withArgs(recipient.address, DEPOSIT_AMOUNT, withdrawalId);

      expect(await usdc.balanceOf(recipient.address)).to.equal(DEPOSIT_AMOUNT);
    });

    it("should revert on double withdrawal", async function () {
      const sig = await signWithdrawal(
        relayer,
        recipient.address,
        DEPOSIT_AMOUNT,
        withdrawalId
      );

      await bridge.withdraw(recipient.address, DEPOSIT_AMOUNT, withdrawalId, sig);
      await expect(
        bridge.withdraw(recipient.address, DEPOSIT_AMOUNT, withdrawalId, sig)
      ).to.be.revertedWith("DinaBridge: already processed");
    });

    it("should revert on invalid signer", async function () {
      // user signs instead of relayer
      const sig = await signWithdrawal(
        user,
        recipient.address,
        DEPOSIT_AMOUNT,
        withdrawalId
      );

      await expect(
        bridge.withdraw(recipient.address, DEPOSIT_AMOUNT, withdrawalId, sig)
      ).to.be.revertedWith("DinaBridge: invalid relayer signature");
    });

    it("should revert on zero recipient", async function () {
      const sig = await signWithdrawal(
        relayer,
        ethers.ZeroAddress,
        DEPOSIT_AMOUNT,
        withdrawalId
      );

      await expect(
        bridge.withdraw(ethers.ZeroAddress, DEPOSIT_AMOUNT, withdrawalId, sig)
      ).to.be.revertedWith("DinaBridge: zero recipient");
    });
  });

  // ---------------------------------------------------------------------------
  // Admin
  // ---------------------------------------------------------------------------

  describe("Admin", function () {
    it("should update relayer", async function () {
      await expect(bridge.setRelayer(user.address))
        .to.emit(bridge, "RelayerUpdated")
        .withArgs(relayer.address, user.address);
      expect(await bridge.relayer()).to.equal(user.address);
    });

    it("should reject setRelayer from non-owner", async function () {
      await expect(
        bridge.connect(user).setRelayer(user.address)
      ).to.be.revertedWithCustomError(bridge, "OwnableUnauthorizedAccount");
    });

    it("should update limits", async function () {
      const min = ethers.parseUnits("10", 6);
      const max = ethers.parseUnits("500000", 6);
      const daily = ethers.parseUnits("5000000", 6);
      await bridge.setLimits(min, max, daily);
      expect(await bridge.minDeposit()).to.equal(min);
      expect(await bridge.maxDeposit()).to.equal(max);
      expect(await bridge.dailyLimit()).to.equal(daily);
    });

    it("should reject limits where min >= max", async function () {
      await expect(
        bridge.setLimits(100, 100, 200)
      ).to.be.revertedWith("DinaBridge: min >= max");
    });

    it("should allow emergency withdraw only when paused", async function () {
      await usdc.mint(await bridge.getAddress(), DEPOSIT_AMOUNT);
      await expect(
        bridge.emergencyWithdraw(DEPOSIT_AMOUNT)
      ).to.be.revertedWith("DinaBridge: must be paused");

      await bridge.setPaused(true);
      await bridge.emergencyWithdraw(DEPOSIT_AMOUNT);
      expect(await usdc.balanceOf(owner.address)).to.equal(DEPOSIT_AMOUNT);
    });
  });
});
