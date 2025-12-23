// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.28;

struct InitializePayload {
    address initialOwner;
    address lstAddress;
}

struct Validator {
    address validator;
    uint64 weight;
}

struct DelegatorSummary {
    uint64 delegated;
    uint64 undelegated;
    uint64 totalPendingWithdrawal;
    uint64 nPendingWithdrawals;
    uint64 rewards;
}

struct Rate {
    uint256 bondRate;
    uint256 unbondRate;
}
