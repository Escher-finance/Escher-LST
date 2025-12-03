// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {BaseHook} from "univ4-periphery/utils/BaseHook.sol";
import {IPoolManager} from "univ4-core/interfaces/IPoolManager.sol";
import {Hooks} from "univ4-core/libraries/Hooks.sol";

contract TrackerHook is BaseHook {
    constructor(IPoolManager _poolManager) BaseHook(_poolManager) {}

    function getHookPermissions() public pure override returns (Hooks.Permissions memory) {
        return Hooks.Permissions({
            beforeInitialize: false,
            afterInitialize: false,
            beforeAddLiquidity: false,
            afterAddLiquidity: false,
            beforeRemoveLiquidity: false,
            afterRemoveLiquidity: false,
            beforeSwap: false,
            afterSwap: false,
            beforeDonate: false,
            afterDonate: false,
            beforeSwapReturnDelta: false,
            afterSwapReturnDelta: false,
            afterAddLiquidityReturnDelta: false,
            afterRemoveLiquidityReturnDelta: false
        });
    }
}
