// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "@openzeppelin/contracts/token/ERC20/extensions/ERC4626.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/IERC20Metadata.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {Ownable2Step, Ownable} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {IEVault} from "./interfaces/IEVault.sol";
import {IEthereumVaultConnector} from "./interfaces/IEthereumVaultConnector.sol";

contract EscherEulerVault is ERC4626, Ownable2Step {
    using SafeERC20 for IERC20;

    IEVault public s_eulerVault;
    IEthereumVaultConnector public s_eulerEVC;

    error EscherVault_InvalidEulerVault();
    error EscherVault_OldEulerVaultStillActive();
    error EscherVault_MissingEulerEVC();

    event EulerVaultUpdated(address indexed _newEulerVault);
    event EulerEVCUpdated(address indexed _newEulerEVC);

    modifier onlyWithEulerEVC() {
        if (address(s_eulerEVC) == address(0)) {
            revert EscherVault_MissingEulerEVC();
        }
        _;
    }

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
        _beforeWithdraw(assets);
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
        _beforeWithdraw(assets);
        _withdraw(_msgSender(), receiver, owner, assets, shares);

        return assets;
    }

    function borrow(uint256 assets) public onlyOwner {
        _borrow(assets);
    }

    function updateEulerVault(IEVault _eulerVault) public onlyOwner {
        if (s_eulerVault.balanceOf(address(this)) != 0) {
            revert EscherVault_OldEulerVaultStillActive();
        }
        _updateEulerVault(_eulerVault);
    }

    function updateEulerEVC(IEthereumVaultConnector _eulerEVC) public onlyOwner {
        _updateEulerEVC(_eulerEVC);
    }

    function _borrow(uint256 assets) internal onlyWithEulerEVC {
        address thisAddr = address(this);
        s_eulerEVC.enableCollateral(thisAddr, thisAddr);
        s_eulerEVC.enableController(thisAddr, address(s_eulerVault));
        s_eulerVault.borrow(assets, thisAddr);
    }

    function _afterDeposit(uint256 assets) internal {
        IERC20(asset()).safeIncreaseAllowance(address(s_eulerVault), assets);
        s_eulerVault.deposit(assets, address(this));
    }

    /// @dev This makes sure to only withdraw from Euler if there aren't enough assets in the vault
    function _beforeWithdraw(uint256 assets) internal {
        address eulerVaultAddr = address(s_eulerVault);
        address thisAddr = address(this);
        IERC20 eulerVaultToken = IERC20(eulerVaultAddr);

        uint256 assetsBalance = IERC20(s_eulerVault.asset()).balanceOf(thisAddr);
        uint256 assetsNeeded = 0;
        if (assets > assetsBalance) {
            assetsNeeded = assets - assetsBalance;
        }
        if (assetsNeeded != 0) {
            uint256 eulerShares = s_eulerVault.convertToShares(assetsNeeded);
            eulerVaultToken.safeIncreaseAllowance(eulerVaultAddr, eulerShares);
            s_eulerVault.withdraw(assetsNeeded, thisAddr, thisAddr);
        }
    }

    function _updateEulerVault(IEVault _eulerVault) private {
        if (asset() != _eulerVault.asset()) {
            revert EscherVault_InvalidEulerVault();
        }
        s_eulerVault = _eulerVault;
        emit EulerVaultUpdated(address(_eulerVault));
    }

    function _updateEulerEVC(IEthereumVaultConnector _eulerEVC) private {
        s_eulerEVC = _eulerEVC;
        emit EulerEVCUpdated(address(_eulerEVC));
    }
}
