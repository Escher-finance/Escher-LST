// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./BasedeU.sol";

contract eU is BasedeU {
    function mint(address to, uint256 amount) public onlyOwner {
        _mint(to, amount);
    }

    function burn(address account, uint256 value) public onlyOwner {
        _burn(account, value);
    }
}
