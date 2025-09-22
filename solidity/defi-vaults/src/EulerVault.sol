// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "@openzeppelin/contracts/token/ERC20/extensions/ERC4626.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

contract UniVault is ERC4626, Ownable {
    constructor(address _owner, string memory _shareName, string memory _shareSymbol, IERC20 _underlyingAsset)
        ERC20(_shareName, _shareSymbol)
        ERC4626(_underlyingAsset)
        Ownable(_owner)
    {}
}
