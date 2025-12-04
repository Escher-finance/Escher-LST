// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {
    BaseHook,
    Hooks,
    IPoolManager,
    PoolKey,
    BalanceDelta,
    ModifyLiquidityParams
} from "univ4-periphery/utils/BaseHook.sol";
import {Ownable2Step, Ownable} from "@openzeppelin/contracts/access/Ownable2Step.sol";

interface IMsgSender {
    function msgSender() external view returns (address);
}

contract TrackerHook is BaseHook, Ownable2Step {
    constructor(address _owner, IPoolManager _poolManager) BaseHook(_poolManager) Ownable(_owner) {}

    mapping(address => bool) public s_verifiedRouters;

    function _getRealSender(address sender) internal returns (address) {
        if (s_verifiedRouters[sender]) {
            try IMsgSender(sender).msgSender() returns (address s) {
                return s;
            } catch {
                revert("router missing msgSender()");
            }
        } else {
            return sender;
        }
    }

    function toggleVerifiedRouter(IMsgSender router) public onlyOwner {
        address routerAddr = address(router);
        s_verifiedRouters[routerAddr] = !s_verifiedRouters[routerAddr];
    }

    function getHookPermissions() public pure override returns (Hooks.Permissions memory) {
        return Hooks.Permissions({
            beforeInitialize: false,
            afterInitialize: false,
            beforeAddLiquidity: false,
            afterAddLiquidity: true,
            beforeRemoveLiquidity: false,
            afterRemoveLiquidity: true,
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

    function _afterAddLiquidity(
        address sender,
        PoolKey calldata key,
        ModifyLiquidityParams calldata params,
        BalanceDelta delta,
        BalanceDelta feesAccrued,
        bytes calldata hookData
    ) internal override returns (bytes4, BalanceDelta) {
        return (BaseHook.afterAddLiquidity.selector, delta);
    }

    function _afterRemoveLiquidity(
        address sender,
        PoolKey calldata key,
        ModifyLiquidityParams calldata params,
        BalanceDelta delta,
        BalanceDelta feesAccrued,
        bytes calldata hookData
    ) internal override returns (bytes4, BalanceDelta) {
        return (BaseHook.afterRemoveLiquidity.selector, delta);
    }
}
