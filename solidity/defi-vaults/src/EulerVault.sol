// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "@openzeppelin/contracts/token/ERC20/extensions/ERC4626.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {Ownable2Step, Ownable} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {IEVault} from "./interfaces/IEVault.sol";

contract EulerVault is ERC4626, Ownable2Step {
    IEVault public s_eulerVault;

    error EscherVault_InvalidEulerVault();

    event EulerVaultUpdated(address indexed _newEulerVault);

    constructor(
        address _owner,
        string memory _shareName,
        string memory _shareSymbol,
        IERC20 _underlyingAsset,
        IEVault _eulerVault
    ) ERC20(_shareName, _shareSymbol) ERC4626(_underlyingAsset) Ownable(_owner) {
        _updateEulerVault(_eulerVault);
    }

    /**
     * @dev See {IERC4626-totalAssets}.
     */
    function totalAssets() public view virtual override returns (uint256) {
        address thisAddr = address(this);
        uint256 total = IERC20(asset()).balanceOf(thisAddr);
        uint256 eulerShares = s_eulerVault.balanceOf(thisAddr);
        if (eulerShares > 0) {
            total += s_eulerVault.convertToAssets(eulerShares);
        }
        return total;
    }

    function _updateEulerVault(IEVault _eulerVault) private {
        if (asset() != _eulerVault.asset()) {
            revert EscherVault_InvalidEulerVault();
        }
        s_eulerVault = _eulerVault;
        emit EulerVaultUpdated(address(_eulerVault));
    }
}
