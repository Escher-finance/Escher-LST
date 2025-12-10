// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {ILiquidStakingManager} from "./interfaces/ILiquidStakingManager.sol";
import {IDelegationManager} from "./interfaces/IDelegationManager.sol";
import {Lst} from "./tokens/Lst.sol";
import {Config, Liquidity, BatchStatus, UnbondRequest, UnbondBatch} from "./models/State.sol";
import {DelegatorSummary} from "./models/Type.sol";

import "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import "@openzeppelin/contracts-upgradeable/access/Ownable2StepUpgradeable.sol";
import "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

contract LiquidStakingManager is
    ILiquidStakingManager,
    Initializable,
    UUPSUpgradeable,
    Ownable2StepUpgradeable,
    PausableUpgradeable,
    ReentrancyGuard
{
    Lst share;
    IDelegationManager public delegationManager;
    Config private s_config;
    Liquidity private s_liquidity;
    uint256 public constant SCALING_FACTOR = 10 ** 18;

    // Batch management storage
    uint256 private s_pendingBatchId;
    uint256 private s_nextRequestId;

    // Mapping from batch ID to batch info
    mapping(uint256 => UnbondBatch) private s_batches;

    // Mapping from request ID to unbond request
    mapping(uint256 => UnbondRequest) private s_unbondRequests;

    // Mapping from user address to their request IDs
    mapping(address => uint256[]) private s_userRequestIds;

    // Required by UUPSUpgradeable - only owner can upgrade
    function _authorizeUpgrade(address newImplementation) internal override onlyOwner {}

    constructor() {
        _disableInitializers();
    }

    function initialize(address initialOwner, address lstAddress, address _delegationManager) public initializer {
        // Checks that the initialOwner address is not zero.
        require(initialOwner != address(0), "zero address");
        __Ownable_init(initialOwner);
        share = Lst(lstAddress);
        delegationManager = IDelegationManager(_delegationManager);

        s_config =
            Config({minBondAmount: 1000, minUnbondAmount: 1000, batchPeriodSeconds: 300, undelegatePeriodSeconds: 300});
        s_liquidity = Liquidity({totalDelegated: 0, totalLst: 0});

        // Initialize batch management
        s_pendingBatchId = 1;
        s_nextRequestId = 1;

        // Create the first pending batch
        s_batches[s_pendingBatchId] = UnbondBatch({
            batchId: s_pendingBatchId,
            status: BatchStatus.Pending,
            totalShares: 0,
            totalAssets: 0,
            nextActionTime: block.timestamp + s_config.batchPeriodSeconds,
            requestIds: new uint256[](0)
        });

        emit BatchCreated(s_pendingBatchId);
    }

    function acceptOwnershipTransfer() external onlyOwner {
        share.acceptOwnership(); // Calls Ownable2Step's acceptOwnership
    }

    function getConfig() external view returns (Config memory) {
        return s_config;
    }

    function getLiquidity() external view returns (Liquidity memory) {
        return s_liquidity;
    }

    function getLst() external view returns (Lst) {
        return share;
    }

    function getDelegationManager() external view returns (address) {
        return address(delegationManager);
    }

    /**
     * @notice function to stake assets and receive liquid staking tokens in exchange
     * @param _assets amount of the asset token
     */
    function bond(uint256 _assets, address _recipient) external payable nonReentrant {
        // checks that the deposited amount is greater than zero.
        require(_assets > s_config.minBondAmount, "bond amount should be more than min amount");

        // checks that the deposited amount is greater than zero.
        require(msg.value > s_config.minBondAmount, "asset should be more than min amount");

        // Checks that the _receiver address is not zero.
        require(_recipient != address(0), "recipient zero address");

        // Checks that the delegationManager address is not zero.
        require(address(delegationManager) != address(0), "delegationManager zero address");

        // call delegate and send the required native asset
        delegationManager.delegate{value: msg.value}();

        // calculate how much shares from the assets
        uint256 shares = _convertToShares(_assets);

        // mint the liquid staking token and send to recipient
        share.mint(_recipient, shares);
        // increase the total minted liquid staking token
        s_liquidity.totalLst += shares;

        emit Bond(msg.sender, _assets, _recipient);
    }

    /**
     * @notice function to calculate how much shares from amount of assets base on exchange rate
     * @param assets amount of the asset token
     */
    function _convertToShares(uint256 assets) internal view returns (uint256) {
        return (bondRate() * assets) / SCALING_FACTOR;
    }

    /**
     * @notice function to calculate how much assets from amount of shares base on exchange rate
     * @param shares amount of shares/LST
     */
    function _convertToAssets(uint256 shares) internal view returns (uint256) {
        return (unbondRate() * shares) / SCALING_FACTOR;
    }

    function totalAssets() public view returns (uint256) {
        DelegatorSummary memory summary = delegationManager.delegationSummary();
        return summary.delegated;
    }

    /**
     * @notice function to calculate the rate to get shares token/lst from assets token
     */
    function bondRate() public view returns (uint256) {
        if ((s_liquidity.totalLst == 0) || (s_liquidity.totalDelegated == 0)) {
            return 1 * SCALING_FACTOR;
        }
        return (totalAssets() * SCALING_FACTOR) / s_liquidity.totalLst;
    }

    /**
     * @notice function to calculate the rate to get assets token from shares token/lst
     */
    function unbondRate() public view returns (uint256) {
        if ((s_liquidity.totalLst == 0) || (s_liquidity.totalDelegated == 0)) {
            return 1 * SCALING_FACTOR;
        }
        return (s_liquidity.totalLst * SCALING_FACTOR) / totalAssets();
    }

    /**
     * @notice Function to allow msg.sender to request to unbond of their staked asset
     * @param _shares amount of shares the user wants to convert
     * @param _recipient address of the user who will receive the assets
     */
    function unbondRequest(uint256 _shares, address _recipient) external nonReentrant returns (uint256) {
        // checks that the deposited amount is greater than zero.
        require(_shares > s_config.minUnbondAmount, "unbond should be more than min unbond amount");
        // Checks that the _receiver address is not zero.
        require(_recipient != address(0), "recipient zero address");

        // Checks that the delegationManager address is not zero.
        require(address(delegationManager) != address(0), "delegation Manager zero address");

        // transfer asset to this contract
        share.transferFrom(msg.sender, address(this), _shares);

        // Get current request ID and increment
        uint256 requestId = s_nextRequestId;
        s_nextRequestId++;

        // Create the unbond request
        s_unbondRequests[requestId] =
            UnbondRequest({user: msg.sender, recipient: _recipient, shares: _shares, batchId: s_pendingBatchId});

        // Add request ID to user's list
        s_userRequestIds[msg.sender].push(requestId);

        // Add request to the current batch
        UnbondBatch storage currentBatch = s_batches[s_pendingBatchId];
        currentBatch.requestIds.push(requestId);
        currentBatch.totalShares += _shares;

        emit UnbondRequested(msg.sender, _shares, _recipient, requestId, s_pendingBatchId);

        return requestId;
    }

    /**
     * @notice Submit the current pending batch for undelegation
     */
    function submitBatch() external nonReentrant {
        // get pending batch
        UnbondBatch storage batch = s_batches[s_pendingBatchId];

        require(batch.batchId != 0, "batch does not exist");
        require(batch.status == BatchStatus.Pending, "batch is not in pending status");
        require(batch.totalShares > 0, "batch has no requests");

        uint256 submittedBatchId = s_pendingBatchId;

        // Calculate total assets to undelegate based on current exchange rate
        uint256 assetsToUndelegate = _convertToAssets(batch.totalShares);
        batch.totalAssets = assetsToUndelegate;

        // Update batch status to submitted
        batch.status = BatchStatus.Submitted;

        // Set next action time (when tokens can be received)
        batch.nextActionTime = block.timestamp + s_config.undelegatePeriodSeconds;

        // Call undelegate on delegation manager
        delegationManager.undelegate(uint64(assetsToUndelegate));

        // Burn the LST tokens held by this contract for this batch
        share.burn(address(this), batch.totalShares);

        // Decrease total LST
        s_liquidity.totalLst -= batch.totalShares;

        // Create a new pending batch
        s_pendingBatchId++;
        s_batches[s_pendingBatchId] = UnbondBatch({
            batchId: s_pendingBatchId,
            status: BatchStatus.Pending,
            totalShares: 0,
            totalAssets: 0,
            nextActionTime: block.timestamp + s_config.batchPeriodSeconds,
            requestIds: new uint256[](0)
        });

        emit BatchCreated(s_pendingBatchId);

        emit BatchSubmitted(submittedBatchId, batch.totalShares, batch.totalAssets, batch.nextActionTime);
    }

    /**
     * @notice Mark a submitted batch as received after undelegation period
     * @param batchId The ID of the batch to mark as received
     */
    function receiveBatch(uint256 batchId) external nonReentrant {
        UnbondBatch storage batch = s_batches[batchId];

        require(batch.batchId != 0, "batch does not exist");
        require(batch.status == BatchStatus.Submitted, "batch is not in submitted status");
        require(block.timestamp >= batch.nextActionTime, "undelegation period not yet passed");

        // Update batch status to received
        batch.status = BatchStatus.Received;

        // get the unbonded assets from delegation Manager
        delegationManager.receiveBatch(batch.totalAssets);

        emit BatchReceived(batchId, batch.totalAssets);
    }

    /**
     * @notice Internal function to remove a request ID from user's array
     * @param user The user address
     * @param requestId The request ID to remove
     */
    function _removeUserRequest(address user, uint256 requestId) internal {
        uint256[] storage userRequests = s_userRequestIds[user];
        uint256 length = userRequests.length;

        for (uint256 i = 0; i < length; i++) {
            if (userRequests[i] == requestId) {
                // Move the last element to this position and pop
                userRequests[i] = userRequests[length - 1];
                userRequests.pop();
                break;
            }
        }
    }

    /**
     * @notice Claim all unbonded assets for the caller
     */
    function claimUnbond() external nonReentrant {
        uint256[] storage userRequests = s_userRequestIds[msg.sender];
        require(userRequests.length > 0, "no unbond requests found");

        for (uint256 i = userRequests.length; i > 0; i--) {
            uint256 requestId = userRequests[i - 1];
            UnbondRequest storage request = s_unbondRequests[requestId];

            UnbondBatch storage batch = s_batches[request.batchId];

            // Skip if batch is not yet received
            if (batch.status != BatchStatus.Received) {
                continue;
            }

            // Calculate the user's share of assets
            uint256 userAssets = (request.shares * batch.totalAssets) / batch.totalShares;

            // Store recipient before deleting
            address recipient = request.recipient;

            // Remove from user's request array (swap and pop)
            userRequests[i - 1] = userRequests[userRequests.length - 1];
            userRequests.pop();

            // Delete the request from storage
            delete s_unbondRequests[requestId];

            // Transfer assets to recipient
            (bool success,) = payable(recipient).call{value: userAssets}("");
            require(success, "transfer failed");

            emit UnbondClaimed(msg.sender, requestId, userAssets, recipient);
        }
    }

    /**
     * @notice Claim unbonded assets for a specific request
     * @param requestId The ID of the unbond request to claim
     */
    function claimUnbondRequest(uint256 requestId) external nonReentrant {
        UnbondRequest storage request = s_unbondRequests[requestId];

        require(request.user != address(0), "request does not exist");
        require(request.user == msg.sender, "not request owner");

        UnbondBatch storage batch = s_batches[request.batchId];
        require(batch.status == BatchStatus.Received, "batch not yet received");

        // Calculate the user's share of assets
        uint256 userAssets = (request.shares * batch.totalAssets) / batch.totalShares;

        // Store recipient before deleting
        address recipient = request.recipient;

        // Remove request from user's array
        _removeUserRequest(msg.sender, requestId);

        // Delete the request from storage
        delete s_unbondRequests[requestId];

        // Transfer assets to recipient
        (bool success,) = payable(recipient).call{value: userAssets}("");
        require(success, "transfer failed");

        emit UnbondClaimed(msg.sender, requestId, userAssets, recipient);
    }

    /**
     * @notice Get the current pending batch ID
     * @return The current pending batch ID
     */
    function getPendingBatchId() external view returns (uint256) {
        return s_pendingBatchId;
    }

    /**
     * @notice Get the current pending batch ID (alias for interface compatibility)
     * @return The current pending batch ID
     */
    function getCurrentBatchId() external view returns (uint256) {
        return s_pendingBatchId;
    }

    /**
     * @notice Get batch information by ID
     * @param batchId The batch ID to query
     * @return The batch information
     */
    function getBatch(uint256 batchId) external view returns (UnbondBatch memory) {
        return s_batches[batchId];
    }

    /**
     * @notice Get unbond request information by ID
     * @param requestId The request ID to query
     * @return The unbond request information
     */
    function getUnbondRequest(uint256 requestId) external view returns (UnbondRequest memory) {
        return s_unbondRequests[requestId];
    }

    /**
     * @notice Get all request IDs for a batch
     * @param batchId The batch ID to query
     * @return Array of request IDs
     */
    function getBatchRequestIds(uint256 batchId) external view returns (uint256[] memory) {
        return s_batches[batchId].requestIds;
    }

    /**
     * @notice Get all request IDs for a user
     * @param user The user address to query
     * @return Array of request IDs
     */
    function getUserRequestIds(address user) external view returns (uint256[] memory) {
        return s_userRequestIds[user];
    }

    /**
     * @notice Get the next request ID that will be assigned
     * @return The next request ID
     */
    function getNextRequestId() external view returns (uint256) {
        return s_nextRequestId;
    }

    // Allow contract to receive native tokens for unbonding
    receive() external payable {}
}
