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
    // time in seconds to wait the undelegation can be withdrawed
    uint64 undelegatePeriodSeconds;
}
