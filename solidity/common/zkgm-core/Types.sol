// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.27;

import "union/apps/ucs/03-zkgm/IZkgm.sol";

uint8 constant BASE_TOKEN_DECIMALS = 18;

bytes32 constant STAKE_HASH = keccak256(abi.encodePacked("stake"));
bytes32 constant UNSTAKE_HASH = keccak256(abi.encodePacked("stake"));

string constant HUB_BATCH_ACK = "hub_batch_ack";
string constant HUB_BATCH_UNBONDING_ACK = "hub_batch_unbonding_ack";
string constant HUB_BATCH_UNBONDING_RECEIVED = "hub_batch_unbonding_received";
string constant HUB_BATCH_UNBONDING_RELEASED = "hub_batch_unbonding_released";

bytes32 constant HUB_BATCH_ACK_HASH = keccak256(abi.encodePacked("hub_batch_ack"));
bytes32 constant HUB_BATCH_UNBONDING_ACK_HASH = keccak256(abi.encodePacked("hub_batch_unbonding_ack"));
bytes32 constant HUB_BATCH_UNBONDING_RECEIVED_HASH = keccak256(abi.encodePacked("hub_batch_unbonding_received"));
bytes32 constant HUB_BATCH_UNBONDING_RELEASED_HASH = keccak256(abi.encodePacked("hub_batch_unbonding_released"));

struct InitializePayload {
    uint256 feeRate;
    uint256 minStake;
    uint256 minUnstake;
    uint64 hubBatchPeriod;
    uint64 unbondingBatchPeriod;
    address owner;
    address baseToken; // native token (U)
    address lsToken; // liquid staking token (eU)
    address zkgm;
    string unionLstContractAddress;
    string unionSolverAddress;
    string baseTokenSymbol;
    string baseTokenName;
    bytes quoteToken;
    address feeReceiver;
    uint32 unionChannelId;
}

struct Config {
    uint256 feeRate;
    uint256 minStake;
    uint256 minUnstake;
    uint64 hubBatchPeriod;
    uint64 unbondingBatchPeriod;
    address zkgm;
    address lsToken;
    address baseToken;
    address feeReceiver;
    string baseTokenSymbol;
    string baseTokenName;
    string unionSolverAddress;
    string unionLstContractAddress;
    uint32 unionChannelId;
}

enum RecordType {
    Stake,
    Unstake
}

struct HubRecord {
    RecordType recordType;
    uint32 batchId;
    uint64 id;
    uint32 recipientChannelId;
    uint256 stakeAmount;
    uint256 mintAmount;
    uint256 unstakeAmount;
    uint256 releasedAmount;
    uint256 exchangeRate;
    uint64 timestamp;
    bytes sender;
    bytes staker;
    bytes recipient;
}

enum BatchStatus {
    Pending,
    Executed,
    ExecutedAndAcknowledged,
    Released
}

struct HubBatch {
    uint256 stakeAmount;
    uint256 mintAmount;
    uint256 unstakeAmount;
    uint256 releasedAmount;
    uint32 id;
    BatchStatus status;
}

enum UnbondingBatchStatus {
    Pending,
    Executed,
    ExecutedAndAcknowledged,
    UnionReceived,
    UnionReleased,
    Released
}

struct UnbondingBatch {
    uint256 unstakeAmount;
    uint256 exchangeRate;
    uint256 receivedAmount;
    UnbondingBatchStatus status;
    uint32 id;
}
