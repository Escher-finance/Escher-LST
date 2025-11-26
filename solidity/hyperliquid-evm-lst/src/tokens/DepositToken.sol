// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract DepositToken is ERC20 {
    constructor(string memory name, string memory symbol) ERC20(name, symbol) {}

    function decimals() public view virtual override returns (uint8) {
        return 6; // USDC has 6 decimals
    }

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}
