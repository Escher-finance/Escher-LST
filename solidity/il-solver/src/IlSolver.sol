// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Ownable2Step, Ownable} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {IPositionManager, PoolKey} from "univ4-periphery/interfaces/IPositionManager.sol";

contract IlSolver is Ownable2Step {
    IPositionManager public s_posm;
    PoolKey s_poolKey;

    constructor(address _owner, IPositionManager _posm, PoolKey memory _poolKey) Ownable(_owner) {
        s_posm = _posm;
        s_poolKey = _poolKey;
    }
}
