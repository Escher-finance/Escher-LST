// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

struct Liquidity {
    uint256 totalDelegated;
    uint256 totalLst;
}

struct Config {
    uint256 minBondAmount;
    uint256 minUnbondAmount;
}
