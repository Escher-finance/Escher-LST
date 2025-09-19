// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {
    BatchStatus,
    HubBatch,
    UnbondingBatch,
    Config,
    HubRecord,
    UnbondingBatchStatus,
    RecordType,
    InitializePayload,
    HUB_BATCH_ACK_HASH,
    HUB_BATCH_UNBONDING_ACK_HASH,
    HUB_BATCH_UNBONDING_RECEIVED_HASH,
    HUB_BATCH_UNBONDING_RELEASED_HASH,
    STAKE_HASH,
    UNSTAKE_HASH
} from "./core/Types.sol";
import "./U.sol";
import "./eU.sol";
import "./core/Event.sol";
import "./core/Zkgm.sol";

import "@openzeppelin-upgradeable/contracts/proxy/utils/Initializable.sol";
import "@openzeppelin-upgradeable/contracts/access/Ownable2StepUpgradeable.sol";
import "@openzeppelin-upgradeable/contracts/proxy/utils/UUPSUpgradeable.sol";
import "@openzeppelin-upgradeable/contracts/utils/ReentrancyGuardUpgradeable.sol";
import "@openzeppelin-upgradeable/contracts/utils/PausableUpgradeable.sol";
import "union/apps/ucs/03-zkgm/IZkgmable.sol";

contract Lst is
    IZkgmable,
    Initializable,
    UUPSUpgradeable,
    Ownable2StepUpgradeable,
    ReentrancyGuardUpgradeable,
    PausableUpgradeable
{
    Config s_config;

    // map of timestamp to exchange rate
    mapping(uint64 => uint256) exchangeRate;

    // map of hub record id to hub record
    mapping(uint64 => HubRecord) public hubRecords;

    // map of batch id to batch queue
    mapping(uint32 => HubBatch) public batches;

    // map of unbonding batch id to unbonding batch
    mapping(uint32 => UnbondingBatch) public unbondingBatches;

    // map of hub batch id to hub record id
    mapping(uint32 => uint64[]) public batchHubRecordIds;

    // map of unbonding batch id with hub batch ids to get ilst of hub batch ids
    mapping(uint32 => uint32[]) public unbondingHubBatchIds;

    // next hub record id
    uint64 hubRecordId;

    // what was union last block recorded in the batch
    uint64 public lastUnionBlockRecorded;

    // current hub batch id
    uint32 public currentHubBatchId;

    // current unbonding batch id
    uint32 public currentUnbondingBatchId;

    // unbonding batch id that need to be released to users/stakers
    uint32 public pendingReleasedUnbondingBatchId;

    // last update timestamp for exchange rate
    uint64 public lastUpdateTimestamp;

    // last timestamp of submit hub batch is executed
    uint64 public lastHubBatchTimestamp;

    // last timestamp of zkgm is updated
    uint64 public lastZkgmTimestamp;

    // last timestamp of unbonding batch is executed
    uint64 public lastUnbondingBatchTimestamp;

    uint256 public constant SCALING_FACTOR = 10 ** 18;

    constructor() {
        _disableInitializers();
    }

    // Required by UUPSUpgradeable - only owner can upgrade
    function _authorizeUpgrade(address newImplementation) internal override onlyOwner {}

    function initialize(InitializePayload calldata payload) public initializer {
        require(payload.owner != address(0));
        require(payload.zkgm != address(0));
        require(payload.baseToken != address(0));
        require(payload.lsToken != address(0));
        require(payload.feeReceiver != address(0));

        __Ownable_init(payload.owner);
        __UUPSUpgradeable_init();

        s_config = Config({
            zkgm: payload.zkgm,
            lsToken: payload.lsToken,
            unionChannelId: payload.unionChannelId,
            unionLstContractAddress: payload.unionLstContractAddress,
            unionSolverAddress: payload.unionSolverAddress,
            baseToken: payload.baseToken,
            baseTokenSymbol: payload.baseTokenSymbol,
            baseTokenName: payload.baseTokenName,
            feeReceiver: payload.feeReceiver,
            feeRate: payload.feeRate,
            hubBatchPeriod: payload.hubBatchPeriod,
            unbondingBatchPeriod: payload.unbondingBatchPeriod,
            minStake: payload.minStake,
            minUnstake: payload.minUnstake
        });

        lastHubBatchTimestamp = uint64(block.timestamp);
        lastUpdateTimestamp = uint64(block.timestamp);
        lastUnbondingBatchTimestamp = uint64(block.timestamp);
        exchangeRate[lastUpdateTimestamp] = SCALING_FACTOR;
        hubRecordId = 1;
        currentHubBatchId = 1;
        currentUnbondingBatchId = 1;
        HubBatch memory initialBatch = HubBatch({
            id: currentHubBatchId,
            stakeAmount: 0,
            mintAmount: 0,
            unstakeAmount: 0,
            releasedAmount: 0,
            status: BatchStatus.Pending
        });

        batches[currentHubBatchId] = initialBatch;
    }

    function acceptOwnershipTransfer() external onlyOwner {
        eU lsToken = eU(s_config.lsToken);
        lsToken.acceptOwnership(); // Calls Ownable2Step's acceptOwnership
    }

    function getVersion() public pure returns (uint16) {
        return 1;
    }

    function isTestMode() public view returns (bool) {
        // Forge often uses chainId 31337 for local testing
        return block.chainid == 31337;
    }

    function getTransferAndCallInstruction(address _sender, uint256 _amount, bytes memory contractCalldata)
        internal
        returns (
            //bytes calldata contractCalldata
            Instruction memory
        )
    {
        TokenOrderV2 memory tokenOrder = TokenOrderV2({
            sender: abi.encodePacked(_sender),
            receiver: abi.encodePacked(s_config.unionLstContractAddress),
            baseToken: abi.encodePacked(s_config.baseToken),
            baseAmount: _amount,
            quoteToken: abi.encodePacked(s_config.baseTokenName),
            quoteAmount: _amount,
            kind: ZkgmLib.TOKEN_ORDER_KIND_SOLVE,
            metadata: ZkgmLib.encodeSolverMetadata(
                SolverMetadata({solverAddress: abi.encodePacked(s_config.unionSolverAddress), metadata: hex""})
            )
        });

        Instruction[] memory instructions = new Instruction[](2);
        instructions[0] = Instruction({
            version: ZkgmLib.INSTR_VERSION_2,
            opcode: ZkgmLib.OP_TOKEN_ORDER,
            operand: ZkgmLib.encodeTokenOrderV2(tokenOrder)
        });
        instructions[1] = ZkgmLstLib.makeCall(_sender, false, bytes(s_config.unionLstContractAddress), contractCalldata);
        Instruction memory batchInstruction = ZkgmLib.makeBatch(instructions);
        return batchInstruction;
    }

    function getTransferInstruction(address _sender, uint256 _amount, bytes calldata _receiver)
        internal
        view
        returns (
            //bytes calldata contractCalldata
            Instruction memory
        )
    {
        TokenOrderV2 memory tokenOrder = TokenOrderV2({
            sender: abi.encodePacked(_sender),
            receiver: _receiver,
            baseToken: abi.encodePacked(s_config.baseToken),
            baseAmount: _amount,
            quoteToken: abi.encodePacked(s_config.baseTokenName),
            quoteAmount: _amount,
            kind: ZkgmLib.TOKEN_ORDER_KIND_SOLVE,
            metadata: ZkgmLib.encodeSolverMetadata(
                SolverMetadata({solverAddress: abi.encodePacked(s_config.unionSolverAddress), metadata: hex""})
            )
        });

        Instruction memory instruction = Instruction({
            version: ZkgmLib.INSTR_VERSION_2,
            opcode: ZkgmLib.OP_TOKEN_ORDER,
            operand: ZkgmLib.encodeTokenOrderV2(tokenOrder)
        });

        return instruction;
    }

    function checkAndTransferRequiredToken(address token, uint256 _amount) internal {
        // Check allowance first
        require(IERC20(token).allowance(msg.sender, address(this)) >= _amount, "Insufficient allowance");

        // Transfer tokens to contract
        require(IERC20(token).transferFrom(msg.sender, address(this), _amount), "Transfer failed");
    }

    function validateRequest(bytes calldata recipient, uint256 amount, string memory requestType) internal view {
        require(recipient.length != 0, "recipient must not be 0 bytes");
        address to = address(bytes20(recipient));
        require(to != address(0), "recipient cannot be zero address");

        bytes32 request_type = keccak256(abi.encodePacked(requestType));
        if (request_type == STAKE_HASH) {
            require(amount >= s_config.minStake, "stake amount is too small");
        }
        if (request_type == UNSTAKE_HASH) {
            require(amount >= s_config.minUnstake, "unstake amount is too small");
        }
    }

    function stake(uint256 _amount, bytes calldata recipient, uint32 recipientChannelId)
        external
        nonReentrant
        whenNotPaused
        returns (uint64)
    {
        validateRequest(recipient, _amount, "stake");

        string memory sender = Strings.toHexString(uint160(msg.sender), 20);
        uint256 mintAmount = calculateLstAmount(_amount);

        HubRecord memory hubRecord = HubRecord({
            id: hubRecordId,
            recordType: RecordType.Stake,
            batchId: currentHubBatchId,
            sender: abi.encodePacked(msg.sender),
            staker: abi.encodePacked(msg.sender),
            stakeAmount: _amount,
            mintAmount: mintAmount,
            unstakeAmount: 0,
            releasedAmount: 0,
            exchangeRate: exchangeRate[lastUpdateTimestamp],
            recipient: recipient,
            recipientChannelId: recipientChannelId,
            timestamp: uint64(block.timestamp)
        });
        hubRecords[hubRecordId] = hubRecord;

        HubBatch storage batch = batches[currentHubBatchId];
        batch.stakeAmount += _amount;
        batch.mintAmount += mintAmount;
        batchHubRecordIds[currentHubBatchId].push(hubRecord.id);

        string memory recipientString = Strings.toHexString(uint160(bytes20(recipient)), 20);
        emit HubStake(
            hubRecordId,
            currentHubBatchId,
            keccak256(abi.encodePacked(sender)),
            recipientChannelId,
            _amount,
            mintAmount,
            exchangeRate[lastUpdateTimestamp],
            uint64(block.timestamp),
            sender,
            sender,
            recipientString
        );

        checkAndTransferRequiredToken(s_config.baseToken, _amount);

        if (recipientChannelId == 0) {
            address recipientAddr = address(bytes20(recipient));
            eU lsToken = eU(s_config.lsToken);
            lsToken.mint(recipientAddr, mintAmount);
        } else {
            bytes memory rawSalt = abi.encodePacked(block.timestamp, msg.sender);
            bytes32 salt = keccak256(rawSalt);
            zkgm_transfer(mintAmount, salt, recipient, recipientChannelId);
        }

        uint64 stakeRecordId = hubRecordId; //save current record id to return

        hubRecordId++;
        return stakeRecordId;
    }

    function unstake(uint256 _amount, bytes calldata recipient, uint32 recipientChannelId)
        external
        nonReentrant
        whenNotPaused
        returns (uint64)
    {
        validateRequest(recipient, _amount, "unstake");

        string memory sender = Strings.toHexString(uint160(msg.sender), 20);
        HubRecord memory hubRecord = HubRecord({
            id: hubRecordId,
            recordType: RecordType.Unstake,
            batchId: currentHubBatchId,
            sender: abi.encodePacked(msg.sender),
            staker: abi.encodePacked(msg.sender),
            stakeAmount: 0,
            mintAmount: 0,
            unstakeAmount: _amount,
            releasedAmount: 0,
            exchangeRate: 0,
            recipient: recipient,
            recipientChannelId: recipientChannelId,
            timestamp: uint64(block.timestamp)
        });
        hubRecords[hubRecordId] = hubRecord;

        HubBatch storage batch = batches[currentHubBatchId];
        batch.unstakeAmount += _amount;

        batchHubRecordIds[currentHubBatchId].push(hubRecord.id);

        string memory recipientString = Strings.toHexString(uint160(bytes20(recipient)), 20);

        emit HubUnstake(
            hubRecordId,
            currentHubBatchId,
            keccak256(abi.encodePacked(sender)),
            recipientChannelId,
            _amount,
            exchangeRate[lastUpdateTimestamp],
            uint64(block.timestamp),
            sender,
            sender,
            recipientString
        );

        checkAndTransferRequiredToken(s_config.lsToken, _amount);
        uint64 unstakeRecordId = hubRecordId;

        hubRecordId++;
        return unstakeRecordId;
    }

    function calculateLstAmount(uint256 _amount) internal view returns (uint256) {
        uint256 lstAmount = (_amount * SCALING_FACTOR) / exchangeRate[lastUpdateTimestamp];
        return lstAmount;
    }

    function getHubRecord(uint64 id) public view returns (HubRecord memory) {
        return hubRecords[id];
    }

    function getHubBatch(uint32 id) public view returns (HubBatch memory) {
        return batches[id];
    }

    function getPayloadString(
        uint32 id,
        string memory delegate_amount,
        string memory unstake_amount,
        string memory mint_amount,
        bytes32 salt
    ) private pure returns (string memory) {
        string memory payload = string(
            abi.encodePacked(
                '{"hub_batch": {"id":',
                Strings.toString(id),
                ',"delegate_amount":"',
                delegate_amount,
                '","unstake_amount":"',
                unstake_amount,
                '","mint_amount":"',
                mint_amount,
                '","salt":"',
                Strings.toHexString(uint256(salt), 32),
                '"}}'
            )
        );
        return payload;
    }

    function submitBatch(bytes32 _salt) public nonReentrant whenNotPaused {
        require(
            uint64(block.timestamp) > (lastHubBatchTimestamp + s_config.hubBatchPeriod),
            string(
                abi.encodePacked(
                    "next batch should be after ", Strings.toString(lastHubBatchTimestamp + s_config.hubBatchPeriod)
                )
            )
        );

        HubBatch storage batch = batches[currentHubBatchId];
        require(batch.status == BatchStatus.Pending, "batch is executed already");

        batch.status = BatchStatus.Executed;
        unbondingBatches[currentUnbondingBatchId].unstakeAmount += batch.unstakeAmount;

        emit SubmitHubBatch(
            currentHubBatchId,
            block.number,
            batch.stakeAmount,
            batch.mintAmount,
            batch.unstakeAmount,
            batch.releasedAmount,
            uint8(batch.status)
        );

        string memory payload = getPayloadString(
            currentHubBatchId,
            Strings.toString(batch.stakeAmount),
            Strings.toString(batch.unstakeAmount),
            Strings.toString(batch.mintAmount),
            _salt
        );

        unbondingHubBatchIds[currentUnbondingBatchId].push(currentHubBatchId); // put current batch id to be part of unbonding hub batch ids
        currentHubBatchId++;

        // create new pending hub batch
        HubBatch memory newBatch = HubBatch({
            id: currentHubBatchId,
            stakeAmount: 0,
            mintAmount: 0,
            unstakeAmount: 0,
            releasedAmount: 0,
            status: BatchStatus.Pending
        });
        batches[currentHubBatchId] = newBatch;

        if (!isTestMode()) {
            BaseToken(s_config.baseToken).approve(s_config.zkgm, batch.stakeAmount);
            uint64 timeoutTimestamp = uint64(block.timestamp + 604800) * 1_000_000_000; // timeout to 7 days
            // // call union LST contract to process total amount of stake and unstake records in the batch via zkgm
            IZkgm(s_config.zkgm).send(
                s_config.unionChannelId,
                0,
                timeoutTimestamp,
                _salt,
                getTransferAndCallInstruction(address(this), batch.stakeAmount, bytes(payload))
            );
        }
    }

    function submitUnbondingBatch(bytes32 _salt) public nonReentrant {
        uint64 nextBatchTimestamp = lastUnbondingBatchTimestamp + s_config.unbondingBatchPeriod;
        require(
            uint64(block.timestamp) > nextBatchTimestamp,
            string(
                abi.encodePacked(
                    "not yet unbonding batch time, next batch timestamp:",
                    nextBatchTimestamp,
                    " current: ",
                    uint64(block.timestamp)
                )
            )
        );
        string memory payload =
            string(abi.encodePacked('{"submit_batch": {"salt":"', Strings.toHexString(uint256(_salt), 32), '"}}'));

        bytes memory contractCalldata = bytes(payload);

        Instruction memory callInstruction =
            ZkgmLstLib.makeCall(address(this), false, bytes(s_config.unionLstContractAddress), contractCalldata);

        uint64 timeoutTimestamp = uint64(block.timestamp + 604800) * 1_000_000_000;

        // update status
        unbondingBatches[currentUnbondingBatchId].status = UnbondingBatchStatus.Executed;

        // burn the unstake amount (liquid staking token)
        eU lsToken = eU(s_config.lsToken);
        lsToken.burn(address(this), unbondingBatches[currentUnbondingBatchId].unstakeAmount);

        lastUnbondingBatchTimestamp = uint64(block.timestamp);

        // // call union LST contract to process submit pending unbonding batch via zkgm
        IZkgm(s_config.zkgm).send(s_config.unionChannelId, 0, timeoutTimestamp, _salt, callInstruction);

        // prepare next unbonding Batch
        currentUnbondingBatchId++;

        unbondingBatches[currentUnbondingBatchId] = UnbondingBatch({
            id: currentUnbondingBatchId,
            exchangeRate: 0,
            status: UnbondingBatchStatus.Pending,
            unstakeAmount: 0,
            receivedAmount: 0
        });

        // reset unbondingHubBatchIds of new unbonding batch id to new array
        unbondingHubBatchIds[currentUnbondingBatchId] = new uint32[](0);
    }

    function calculateReleaseAmount(uint256 totalReceived, uint256 userStake, uint256 totalStake)
        public
        pure
        returns (uint256)
    {
        uint256 releaseAmount = totalReceived * userStake / totalStake;
        return releaseAmount;
    }

    function batchWithdrawal() public nonReentrant {
        // get current released batch
        require(pendingReleasedUnbondingBatchId != 0, "no pending unbonding batch");
        // check unbondingBatchStatus
        require(
            unbondingBatches[pendingReleasedUnbondingBatchId].status == UnbondingBatchStatus.UnionReleased,
            "batch not yet released on union"
        );
        // get hub records part of the unbonding batch
        UnbondingBatch storage pendingReleasedBatch = unbondingBatches[pendingReleasedUnbondingBatchId];
        uint32[] memory hubBatchIds = unbondingHubBatchIds[pendingReleasedUnbondingBatchId];

        uint256 totalHubBatches = hubBatchIds.length;
        for (uint32 i = 0; i < totalHubBatches; i++) {
            HubBatch storage batch = batches[hubBatchIds[i]];

            uint64[] memory hubRecordIds = batchHubRecordIds[batch.id];
            uint256 totalRecords = hubRecordIds.length;

            for (uint256 j; j < totalRecords; j++) {
                HubRecord storage record = hubRecords[hubRecordIds[j]];

                if (record.releasedAmount == 0 && record.unstakeAmount > 0) {
                    // calculate how much the recipient should receive base on unstake amount compare to total unstake on batch
                    // with the total received on the unbonding batch
                    uint256 releaseAmount = calculateReleaseAmount(
                        pendingReleasedBatch.receivedAmount, record.unstakeAmount, pendingReleasedBatch.unstakeAmount
                    );

                    // send U to recipient
                    address recipient = address(bytes20(record.recipient));
                    BaseToken(s_config.baseToken).transfer(recipient, releaseAmount);

                    // update record
                    record.releasedAmount = releaseAmount;
                    batch.releasedAmount += releaseAmount;
                }
            }
        }

        pendingReleasedBatch.status = UnbondingBatchStatus.Released;
        pendingReleasedUnbondingBatchId = 0;
    }

    function onZkgm(
        address _caller,
        uint256 path,
        uint32 sourceChannelId,
        uint32 destinationChannelId,
        bytes calldata sender,
        bytes calldata message,
        address _relayer,
        bytes calldata relayerMsg
    ) external nonReentrant whenNotPaused {
        // Verify caller is zkgm contract
        require(msg.sender == address(s_config.zkgm), "Only zkgm");

        lastZkgmTimestamp = uint64(block.timestamp);
        ZkgmMsg memory payload = ZkgmLstLib.decode(message);

        emit ZkgmMessageReceived(path, sourceChannelId, destinationChannelId, string(sender), message);

        // Update the lastUnionBlockRecorded when we receive a message
        if (payload.union_block > 0) {
            exchangeRate[uint64(block.timestamp)] = payload.rate;
            lastUpdateTimestamp = uint64(block.timestamp);
            // Use the dedicated unionBlock field from the zkgm message
            lastUnionBlockRecorded = payload.union_block;

            emit ExchangeRateUpdated(
                payload.id, keccak256(abi.encodePacked(payload.action)), payload.action, payload.rate
            );
        }

        bytes32 actionHash = keccak256(abi.encodePacked(payload.action));
        if (actionHash == HUB_BATCH_ACK_HASH) {
            // if ack then update hub batch status
            // Update the current batch with the new union block
            if (batches[payload.id].status == BatchStatus.Executed) {
                batches[payload.id].status = BatchStatus.ExecutedAndAcknowledged;
            }
        } else if (actionHash == HUB_BATCH_UNBONDING_ACK_HASH) {
            // if ack then update unbonding batch status
            if (unbondingBatches[payload.id].status == UnbondingBatchStatus.Executed) {
                unbondingBatches[payload.id].status = UnbondingBatchStatus.ExecutedAndAcknowledged;
                unbondingBatches[payload.id].exchangeRate = payload.rate;
            }
        } else if (actionHash == HUB_BATCH_UNBONDING_RECEIVED_HASH) {
            // if ack then update unbonding batch received status
            if (unbondingBatches[payload.id].status == UnbondingBatchStatus.ExecutedAndAcknowledged) {
                unbondingBatches[payload.id].status = UnbondingBatchStatus.UnionReceived;
                unbondingBatches[payload.id].receivedAmount = payload.amount;
            }
        } else if (actionHash == HUB_BATCH_UNBONDING_RELEASED_HASH) {
            // if ack then update unbonding batch released status
            if (unbondingBatches[payload.id].status == UnbondingBatchStatus.UnionReceived) {
                unbondingBatches[payload.id].status = UnbondingBatchStatus.UnionReleased;
                pendingReleasedUnbondingBatchId = payload.id;
            }
        }
    }

    function onIntentZkgm(
        address caller,
        uint256 path,
        uint32 sourceChannelId,
        uint32 destinationChannelId,
        bytes calldata sender,
        bytes calldata message,
        address relayer,
        bytes calldata relayerMsg
    ) public nonReentrant whenNotPaused {
        if (msg.sender != address(s_config.zkgm)) {
            revert ZkgmLib.ErrUnauthorized();
        }
    }

    function zkgm_transfer(uint256 _amount, bytes32 _salt, bytes calldata _receiver, uint32 channelId) private {
        eU lsToken = eU(s_config.lsToken);
        lsToken.mint(address(this), _amount);
        lsToken.approve(s_config.zkgm, _amount);
        Instruction memory instruction = getTransferInstruction(address(this), _amount, _receiver);
        uint256 timeoutTimestamp = (block.timestamp + 604800) * 1_000_000_000;

        IZkgm(s_config.zkgm).send(channelId, 0, uint64(timeoutTimestamp), _salt, instruction);
    }

    function pauseToggle() external onlyOwner {
        if (paused()) {
            _unpause();
        } else {
            _pause();
        }
    }

    function config() public view returns (Config memory) {
        return s_config;
    }

    function currentRate() public view returns (uint256) {
        return exchangeRate[lastUpdateTimestamp];
    }

    function unbondingBatch() public view returns (UnbondingBatch memory) {
        return unbondingBatches[currentUnbondingBatchId];
    }
}
