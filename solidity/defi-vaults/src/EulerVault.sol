// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "@openzeppelin/contracts/token/ERC20/extensions/ERC4626.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {Ownable2Step, Ownable} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {IEVault} from "./interfaces/IEVault.sol";

contract EulerVault is ERC4626, Ownable2Step {
    using SafeERC20 for IERC20;

    IEVault public s_eulerVault;

    error EscherVault_InvalidEulerVault();
    error EscherVault_OldEulerVaultStillActive();

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

    /// @inheritdoc IERC4626
    function totalAssets() public view virtual override returns (uint256) {
        address thisAddr = address(this);
        uint256 total = IERC20(asset()).balanceOf(thisAddr);
        uint256 eulerShares = s_eulerVault.balanceOf(thisAddr);
        if (eulerShares != 0) {
            total += s_eulerVault.convertToAssets(eulerShares);
        }
        return total;
    }

    /// @inheritdoc IERC4626
    function deposit(uint256 assets, address receiver) public virtual override returns (uint256) {
        uint256 maxAssets = maxDeposit(receiver);
        if (assets > maxAssets) {
            revert ERC4626ExceededMaxDeposit(receiver, assets, maxAssets);
        }

        uint256 shares = previewDeposit(assets);
        _deposit(_msgSender(), receiver, assets, shares);
        _afterDeposit(assets);

        return shares;
    }

    /// @inheritdoc IERC4626
    function mint(uint256 shares, address receiver) public virtual override returns (uint256) {
        uint256 maxShares = maxMint(receiver);
        if (shares > maxShares) {
            revert ERC4626ExceededMaxMint(receiver, shares, maxShares);
        }

        uint256 assets = previewMint(shares);
        _deposit(_msgSender(), receiver, assets, shares);
        _afterDeposit(assets);

        return assets;
    }

    /// @inheritdoc IERC4626
    function withdraw(uint256 assets, address receiver, address owner) public virtual override returns (uint256) {
        uint256 maxAssets = maxWithdraw(owner);
        if (assets > maxAssets) {
            revert ERC4626ExceededMaxWithdraw(owner, assets, maxAssets);
        }

        uint256 shares = previewWithdraw(assets);
        _beforeWithdraw(assets, shares);
        _withdraw(_msgSender(), receiver, owner, assets, shares);

        return shares;
    }

    /// @inheritdoc IERC4626
    function redeem(uint256 shares, address receiver, address owner) public virtual override returns (uint256) {
        uint256 maxShares = maxRedeem(owner);
        if (shares > maxShares) {
            revert ERC4626ExceededMaxRedeem(owner, shares, maxShares);
        }

        uint256 assets = previewRedeem(shares);
        _beforeWithdraw(assets, shares);
        _withdraw(_msgSender(), receiver, owner, assets, shares);

        return assets;
    }

    function updateEulerVault(IEVault _eulerVault) public onlyOwner {
        if (s_eulerVault.balanceOf(address(this)) != 0) {
            revert EscherVault_OldEulerVaultStillActive();
        }
        _updateEulerVault(_eulerVault);
    }

    function _afterDeposit(uint256 assets) internal {
        IERC20(asset()).safeIncreaseAllowance(address(s_eulerVault), assets);
        s_eulerVault.deposit(assets, address(this));
    }

    function _beforeWithdraw(uint256 assets, uint256 shares) internal {
        address eulerVaultAddr = address(s_eulerVault);
        IERC20(eulerVaultAddr).safeIncreaseAllowance(eulerVaultAddr, shares);
        s_eulerVault.withdraw(assets, address(this), address(this));
    }

    function _updateEulerVault(IEVault _eulerVault) private {
        if (asset() != _eulerVault.asset()) {
            revert EscherVault_InvalidEulerVault();
        }
        s_eulerVault = _eulerVault;
        emit EulerVaultUpdated(address(_eulerVault));
    }
}
