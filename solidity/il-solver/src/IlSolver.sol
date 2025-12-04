// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Ownable2Step, Ownable} from "@openzeppelin/contracts/access/Ownable2Step.sol";

contract IlSolver is Ownable2Step {
    constructor(address _owner) Ownable(_owner) {}
}
