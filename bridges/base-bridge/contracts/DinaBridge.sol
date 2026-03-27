// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

/**
 * @title DinaBridge
 * @notice Lock USDC on Base, mint bridged-USDC on Dina Network.
 *
 * Flow:
 *   Base -> Dina: User locks USDC here -> relayer calls `claim` on Dina's
 *                 bridge-base contract to mint bridged USDC.
 *   Dina -> Base: User calls `withdraw` on Dina (burns bridged USDC) ->
 *                 relayer signs a withdrawal proof -> anyone can submit
 *                 `withdraw` here to unlock USDC to the recipient.
 *
 * The bridge uses a single trusted relayer. In production this should be
 * upgraded to a decentralised relayer set with threshold signatures or a
 * light-client verification scheme.
 */
contract DinaBridge is Ownable, ReentrancyGuard {
    using SafeERC20 for IERC20;

    // -----------------------------------------------------------------------
    // State
    // -----------------------------------------------------------------------

    /// @notice The USDC token contract on Base.
    IERC20 public immutable usdc;

    /// @notice Address of the trusted relayer that signs withdrawal proofs.
    address public relayer;

    /// @notice Auto-incrementing nonce included in every deposit event so the
    ///         relayer can detect missed events.
    uint256 public depositNonce;

    /// @notice Tracks which Dina withdrawal IDs have already been processed
    ///         on Base (prevents replay).
    mapping(bytes32 => bool) public processedWithdrawals;

    // -- Limits -------------------------------------------------------------

    /// @notice Minimum deposit amount (1 USDC = 1e6).
    uint256 public minDeposit = 1e6;

    /// @notice Maximum deposit amount per transaction (1 M USDC).
    uint256 public maxDeposit = 1_000_000e6;

    /// @notice Rolling 24-hour volume cap.
    uint256 public dailyLimit = 10_000_000e6;

    /// @notice Volume used in the current 24-hour window.
    uint256 public dailyVolume;

    /// @notice The day number (block.timestamp / 1 days) of the last reset.
    uint256 public lastResetDay;

    /// @notice Emergency pause flag.
    bool public paused;

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    /// @notice Emitted when a user locks USDC for bridging to Dina.
    event Deposited(
        address indexed sender,
        string dinaRecipient,
        uint256 amount,
        uint256 nonce
    );

    /// @notice Emitted when USDC is unlocked on Base after a Dina withdrawal.
    event Withdrawn(
        address indexed recipient,
        uint256 amount,
        bytes32 withdrawalId
    );

    /// @notice Emitted when the relayer address is changed.
    event RelayerUpdated(address indexed oldRelayer, address indexed newRelayer);

    /// @notice Emitted when limits are changed.
    event LimitsUpdated(uint256 minDeposit, uint256 maxDeposit, uint256 dailyLimit);

    /// @notice Emitted when the bridge is paused or unpaused.
    event PauseToggled(bool paused);

    // -----------------------------------------------------------------------
    // Constructor
    // -----------------------------------------------------------------------

    /**
     * @param _usdc   Address of the USDC token on Base.
     * @param _relayer Address of the trusted relayer.
     */
    constructor(address _usdc, address _relayer) Ownable(msg.sender) {
        require(_usdc != address(0), "DinaBridge: zero USDC address");
        require(_relayer != address(0), "DinaBridge: zero relayer address");
        usdc = IERC20(_usdc);
        relayer = _relayer;
    }

    // -----------------------------------------------------------------------
    // Modifiers
    // -----------------------------------------------------------------------

    modifier whenNotPaused() {
        require(!paused, "DinaBridge: paused");
        _;
    }

    // -----------------------------------------------------------------------
    // Base -> Dina  (lock USDC)
    // -----------------------------------------------------------------------

    /**
     * @notice Lock USDC on Base. The relayer watches for the `Deposited`
     *         event and mints bridged USDC on Dina.
     * @param amount        Amount of USDC (6 decimals) to bridge.
     * @param dinaRecipient Dina Network address as a hex string (0x + 64 hex chars).
     */
    function deposit(uint256 amount, string calldata dinaRecipient)
        external
        nonReentrant
        whenNotPaused
    {
        require(amount >= minDeposit, "DinaBridge: below minimum");
        require(amount <= maxDeposit, "DinaBridge: above maximum");
        require(bytes(dinaRecipient).length == 66, "DinaBridge: invalid Dina address");

        _checkDailyLimit(amount);

        usdc.safeTransferFrom(msg.sender, address(this), amount);

        unchecked {
            ++depositNonce;
        }

        emit Deposited(msg.sender, dinaRecipient, amount, depositNonce);
    }

    // -----------------------------------------------------------------------
    // Dina -> Base  (unlock USDC)
    // -----------------------------------------------------------------------

    /**
     * @notice Unlock USDC on Base after a user burned bridged USDC on Dina.
     *         The relayer signs (recipient, amount, withdrawalId, chainId)
     *         to authorise the release.
     * @param recipient        The Base address to receive USDC.
     * @param amount           Amount of USDC to unlock.
     * @param withdrawalId     Unique ID from the Dina-side withdrawal.
     * @param relayerSignature EIP-191 signature from the relayer.
     */
    function withdraw(
        address recipient,
        uint256 amount,
        bytes32 withdrawalId,
        bytes calldata relayerSignature
    ) external nonReentrant whenNotPaused {
        require(recipient != address(0), "DinaBridge: zero recipient");
        require(amount > 0, "DinaBridge: zero amount");
        require(!processedWithdrawals[withdrawalId], "DinaBridge: already processed");

        // Reconstruct the message the relayer should have signed.
        bytes32 messageHash = keccak256(
            abi.encodePacked(recipient, amount, withdrawalId, block.chainid)
        );
        bytes32 ethSignedHash = keccak256(
            abi.encodePacked("\x19Ethereum Signed Message:\n32", messageHash)
        );

        address signer = _recoverSigner(ethSignedHash, relayerSignature);
        require(signer == relayer, "DinaBridge: invalid relayer signature");

        processedWithdrawals[withdrawalId] = true;

        usdc.safeTransfer(recipient, amount);

        emit Withdrawn(recipient, amount, withdrawalId);
    }

    // -----------------------------------------------------------------------
    // Admin
    // -----------------------------------------------------------------------

    /**
     * @notice Replace the relayer address. Only callable by the owner.
     */
    function setRelayer(address _relayer) external onlyOwner {
        require(_relayer != address(0), "DinaBridge: zero relayer");
        emit RelayerUpdated(relayer, _relayer);
        relayer = _relayer;
    }

    /**
     * @notice Toggle the emergency pause. Only callable by the owner.
     */
    function setPaused(bool _paused) external onlyOwner {
        paused = _paused;
        emit PauseToggled(_paused);
    }

    /**
     * @notice Update deposit limits. Only callable by the owner.
     */
    function setLimits(uint256 _min, uint256 _max, uint256 _daily)
        external
        onlyOwner
    {
        require(_min < _max, "DinaBridge: min >= max");
        require(_daily >= _max, "DinaBridge: daily < max");
        minDeposit = _min;
        maxDeposit = _max;
        dailyLimit = _daily;
        emit LimitsUpdated(_min, _max, _daily);
    }

    /**
     * @notice Emergency withdrawal of stuck funds. Only callable by the owner.
     *         This should only be used when the bridge is paused and funds
     *         need to be rescued.
     */
    function emergencyWithdraw(uint256 amount) external onlyOwner {
        require(paused, "DinaBridge: must be paused");
        usdc.safeTransfer(owner(), amount);
    }

    // -----------------------------------------------------------------------
    // Views
    // -----------------------------------------------------------------------

    /**
     * @notice Total USDC currently locked in the bridge.
     */
    function totalLocked() external view returns (uint256) {
        return usdc.balanceOf(address(this));
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /**
     * @dev Enforce the rolling daily limit. Resets at each UTC day boundary.
     */
    function _checkDailyLimit(uint256 amount) internal {
        uint256 today = block.timestamp / 1 days;
        if (today != lastResetDay) {
            dailyVolume = 0;
            lastResetDay = today;
        }
        dailyVolume += amount;
        require(dailyVolume <= dailyLimit, "DinaBridge: daily limit exceeded");
    }

    /**
     * @dev Recover the signer of an EIP-191 signed message.
     */
    function _recoverSigner(bytes32 hash, bytes memory sig)
        internal
        pure
        returns (address)
    {
        require(sig.length == 65, "DinaBridge: bad signature length");

        bytes32 r;
        bytes32 s;
        uint8 v;

        assembly {
            r := mload(add(sig, 32))
            s := mload(add(sig, 64))
            v := byte(0, mload(add(sig, 96)))
        }

        // EIP-2: restrict s to lower half to prevent malleability.
        require(
            uint256(s) <= 0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF5D576E7357A4501DDFE92F46681B20A0,
            "DinaBridge: invalid s value"
        );

        if (v < 27) {
            v += 27;
        }
        require(v == 27 || v == 28, "DinaBridge: invalid v value");

        address recovered = ecrecover(hash, v, r, s);
        require(recovered != address(0), "DinaBridge: ecrecover failed");
        return recovered;
    }
}
