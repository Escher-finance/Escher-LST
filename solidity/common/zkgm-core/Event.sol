// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

event ZkgmMessageReceived(
    uint256 path, uint32 sourceChannelId, uint32 destinationChannelId, string sender, bytes payload
);

event HubStake(
    uint64 indexed id,
    uint32 indexed batchId,
    bytes32 indexed stakerHash,
    uint32 recipientChannelId,
    uint256 stakeAmount,
    uint256 mintAmount,
    uint256 exchangeRate,
    uint64 timestamp,
    string sender,
    string staker,
    string recipient
);

event HubUnstake(
    uint64 indexed id,
    uint32 indexed batchId,
    bytes32 indexed stakerHash,
    uint32 recipientChannelId,
    uint256 unstakeAmount,
    uint256 exchangeRate,
    uint64 timestamp,
    string sender,
    string staker,
    string recipient
);

event SubmitHubBatch(
    uint32 indexed id,
    uint256 executedHeight,
    uint256 stakeAmount,
    uint256 mintAmount,
    uint256 unstakeAmount,
    uint256 releasedAmount,
    uint8 indexed status
);

event FastUnbond(
    uint64 indexed hubRecordId,
    uint32 indexed batchId,
    uint256 unstakeAmount,
    uint256 releasedAmount,
    uint256 exchangeRate,
    uint256 fee
);

event ExchangeRateUpdated(uint32 indexed id, bytes32 indexed actionHash, string action, uint256 rate);
