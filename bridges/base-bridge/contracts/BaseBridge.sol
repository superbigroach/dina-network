// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

/**
 * @title BaseBridge
 * @notice Lock/mint bridge for USDC between Base Sepolia and Dina Network.
 *
 * Flow:
 *   Base -> Dina: User calls lockAndBridge(amount, dinaRecipient) to lock USDC
 *                 here. The relayer observes the BridgeLocked event and mints
 *                 USDC.e on the Dina side (bridge-base contract).
 *
 *   Dina -> Base: User burns USDC.e on Dina. The relayer observes the burn
 *                 event and calls release(recipient, amount) here to unlock USDC.
 *
 * USDC on Base Sepolia: 0x036CbD53842c5426634e7929541eC2318f3dCF7e
 */
contract BaseBridge is Ownable, ReentrancyGuard {
    using SafeERC20 for IERC20;

    // -----------------------------------------------------------------------
    // State
    // -----------------------------------------------------------------------

    /// @notice The USDC token contract on Base Sepolia.
    IERC20 public immutable usdc;

    /// @notice Address of the trusted relayer that processes releases.
    address public relayer;

    /// @notice Auto-incrementing nonce so the relayer can detect missed events.
    uint256 public bridgeNonce;

    /// @notice Emergency pause flag.
    bool public paused;

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    /// @notice Emitted when a user locks USDC for bridging to Dina.
    event BridgeLocked(
        address indexed sender,
        bytes32 indexed dinaRecipient,
        uint256 amount,
        uint256 nonce
    );

    /// @notice Emitted when USDC is released to a recipient on Base.
    event BridgeReleased(
        address indexed recipient,
        uint256 amount
    );

    /// @notice Emitted when the relayer address is updated.
    event RelayerUpdated(address indexed oldRelayer, address indexed newRelayer);

    // -----------------------------------------------------------------------
    // Constructor
    // -----------------------------------------------------------------------

    /**
     * @param _usdc    USDC token address on Base Sepolia.
     *                 Mainnet value: 0x036CbD53842c5426634e7929541eC2318f3dCF7e
     * @param _relayer Address of the trusted relayer service.
     */
    constructor(address _usdc, address _relayer) Ownable(msg.sender) {
        require(_usdc != address(0), "BaseBridge: zero USDC address");
        require(_relayer != address(0), "BaseBridge: zero relayer address");
        usdc = IERC20(_usdc);
        relayer = _relayer;
    }

    // -----------------------------------------------------------------------
    // Modifiers
    // -----------------------------------------------------------------------

    modifier onlyRelayer() {
        require(msg.sender == relayer, "BaseBridge: caller is not the relayer");
        _;
    }

    modifier whenNotPaused() {
        require(!paused, "BaseBridge: paused");
        _;
    }

    // -----------------------------------------------------------------------
    // Base -> Dina  (lock USDC)
    // -----------------------------------------------------------------------

    /**
     * @notice Lock USDC on Base. The relayer watches for the BridgeLocked
     *         event and mints USDC.e on the Dina bridge-base contract.
     *
     * @param amount        Amount of USDC to bridge (6 decimals).
     * @param dinaRecipient 32-byte Dina Network address of the recipient.
     *                      Dina addresses are 32-byte SHA-256 hashes of an
     *                      Ed25519 public key, represented as bytes32.
     */
    function lockAndBridge(uint256 amount, bytes32 dinaRecipient)
        external
        nonReentrant
        whenNotPaused
    {
        require(amount > 0, "BaseBridge: amount must be positive");
        require(dinaRecipient != bytes32(0), "BaseBridge: zero Dina recipient");

        usdc.safeTransferFrom(msg.sender, address(this), amount);

        unchecked {
            ++bridgeNonce;
        }

        emit BridgeLocked(msg.sender, dinaRecipient, amount, bridgeNonce);
    }

    // -----------------------------------------------------------------------
    // Dina -> Base  (release USDC)
    // -----------------------------------------------------------------------

    /**
     * @notice Release previously locked USDC to a recipient.
     *         Called by the relayer after a user burns USDC.e on Dina.
     *
     * @param recipient The Base address to receive USDC.
     * @param amount    Amount of USDC to release (6 decimals).
     */
    function release(address recipient, uint256 amount)
        external
        onlyRelayer
        nonReentrant
        whenNotPaused
    {
        require(recipient != address(0), "BaseBridge: zero recipient");
        require(amount > 0, "BaseBridge: amount must be positive");
        require(
            usdc.balanceOf(address(this)) >= amount,
            "BaseBridge: insufficient locked USDC"
        );

        usdc.safeTransfer(recipient, amount);

        emit BridgeReleased(recipient, amount);
    }

    // -----------------------------------------------------------------------
    // Admin
    // -----------------------------------------------------------------------

    /**
     * @notice Update the relayer address. Only callable by the owner.
     */
    function setRelayer(address _relayer) external onlyOwner {
        require(_relayer != address(0), "BaseBridge: zero relayer address");
        emit RelayerUpdated(relayer, _relayer);
        relayer = _relayer;
    }

    /**
     * @notice Toggle the emergency pause. Only callable by the owner.
     */
    function setPaused(bool _paused) external onlyOwner {
        paused = _paused;
    }

    /**
     * @notice Emergency rescue of stuck funds. Only callable by the owner
     *         when the bridge is paused.
     */
    function emergencyWithdraw(uint256 amount) external onlyOwner {
        require(paused, "BaseBridge: must be paused first");
        usdc.safeTransfer(owner(), amount);
    }

    // -----------------------------------------------------------------------
    // Views
    // -----------------------------------------------------------------------

    /**
     * @notice Total USDC currently locked in this bridge.
     */
    function totalLocked() external view returns (uint256) {
        return usdc.balanceOf(address(this));
    }
}
