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

    error TrackerHook_verifiedRouterMissingMsgSender(address router);

    /// @dev Original event from `IPoolManager` with changed name
    /// @notice Emitted when a liquidity position is modified
    /// @param id The abi encoded hash of the pool key struct for the pool that was modified
    /// @param sender The address that modified the pool
    /// @param tickLower The lower tick of the position
    /// @param tickUpper The upper tick of the position
    /// @param liquidityDelta The amount of liquidity that was added or removed
    /// @param salt The extra data to make positions unique
    event TrackedModifyLiquidity(
        bytes32 indexed id,
        address indexed sender,
        int24 tickLower,
        int24 tickUpper,
        int256 liquidityDelta,
        bytes32 salt
    );

    /// @dev Equivalent to PoolId.sol from v4-core
    function _getPoolId(PoolKey calldata key) internal returns (bytes32) {
        return keccak256(abi.encode(key));
    }

    function _getRealSender(address sender) internal returns (address) {
        if (s_verifiedRouters[sender]) {
            try IMsgSender(sender).msgSender() returns (address s) {
                return s;
            } catch {
                revert TrackerHook_verifiedRouterMissingMsgSender(sender);
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
        emit TrackedModifyLiquidity(
            _getPoolId(key),
            _getRealSender(sender),
            params.tickLower,
            params.tickUpper,
            params.liquidityDelta,
            params.salt
        );
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
        emit TrackedModifyLiquidity(
            _getPoolId(key),
            _getRealSender(sender),
            params.tickLower,
            params.tickUpper,
            params.liquidityDelta,
            params.salt
        );
        return (BaseHook.afterRemoveLiquidity.selector, delta);
    }
}
