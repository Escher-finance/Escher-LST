// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

struct Liquidity {
    // total value of delegated token
    uint256 totalDelegated;
    // total minted liquid staking token
    uint256 totalLst;
}

struct Config {
    // minimum bond amount
    uint256 minBondAmount;
    // minimum unbond amount
    uint256 minUnbondAmount;
    // time in seconds before batch can be submitted
    uint64 batchPeriodSeconds;
    // time in seconds to wait the undelegation can be withdrawed
    uint64 undelegatePeriodSeconds;
}

// Status of an unbond batch
enum BatchStatus {
    Pending, // Batch is accepting new unbond requests
    Submitted, // Batch has been submitted for undelegation
    Received // Batch has received the undelegated/unbonded tokens
}

// Individual unbond request from a user
struct UnbondRequest {
    // Address of the user who made the request
    address user;
    // Address that will receive the unbonded assets
    address recipient;
    // Amount of LST shares to unbond
    uint256 shares;
    // The batch ID this request belongs to
    uint256 batchId;
}

// Batch of unbond requests
struct UnbondBatch {
    // Unique batch ID
    uint256 batchId;
    // Current status of the batch
    BatchStatus status;
    // Total LST shares in this batch
    uint256 totalShares;
    // Total assets to be received (calculated when batch is submitted)
    uint256 totalAssets;
    // Timestamp when the next action can be performed on this batch
    uint256 nextActionTime;
    // Array of request IDs in this batch
    uint256[] requestIds;
}
