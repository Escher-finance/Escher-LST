// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "@openzeppelin-upgradeable/contracts/token/ERC20/ERC20Upgradeable.sol";
import "@openzeppelin-upgradeable/contracts/access/Ownable2StepUpgradeable.sol";
import "@openzeppelin-upgradeable/contracts/token/ERC20/extensions/ERC20PermitUpgradeable.sol";
import "@openzeppelin-upgradeable/contracts/proxy/utils/Initializable.sol";
import "@openzeppelin-upgradeable/contracts/proxy/utils/UUPSUpgradeable.sol";
import "union/apps/ucs/03-zkgm/ISolver.sol";

contract BasedeU is
    Initializable,
    ERC20Upgradeable,
    Ownable2StepUpgradeable,
    ERC20PermitUpgradeable,
    UUPSUpgradeable,
    ISolver
{
    address s_zkgm;
    /// @dev path => destinationChannelId => baseToken => counterparty beneficiary
    mapping(uint256 => mapping(uint32 => mapping(bytes => bytes))) s_fungibleCounterparties;

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    function initialize(address initialOwner, address _zkgm) public initializer {
        require(_zkgm != address(0));
        s_zkgm = _zkgm;
        __ERC20_init("eUToken", "eUT");
        __Ownable_init(initialOwner);
        __ERC20Permit_init("eUToken");
        __UUPSUpgradeable_init();
    }

    function _authorizeUpgrade(address newImplementation) internal override onlyOwner {}

    function allowMarketMakers() external pure returns (bool) {
        return false;
    }

    function solve(
        IBCPacket calldata packet,
        TokenOrderV2 calldata order,
        uint256 path,
        address caller,
        address relayer,
        bytes calldata relayerMsg,
        bool intent
    ) external override returns (bytes memory) {
        require(msg.sender == s_zkgm, "only zkgm");
        require(!intent, "only finalized txs are supported");

        bytes memory counterpartyBeneficiary =
            s_fungibleCounterparties[path][packet.destinationChannelId][order.baseToken];
        require(counterpartyBeneficiary.length != 0, "counterparty is not fungible");

        uint256 fee = order.baseAmount - order.quoteAmount;
        if (fee > 0) {
            _mint(relayer, fee);
        }
        if (order.quoteAmount > 0) {
            _mint(address(bytes20(order.receiver)), order.quoteAmount);
        }
        return counterpartyBeneficiary;
    }

    function setFungibleCounterparty(
        uint256 path,
        uint32 channelId,
        bytes calldata token,
        bytes calldata counterpartyBeneficiary
    ) external onlyOwner {
        s_fungibleCounterparties[path][channelId][token] = counterpartyBeneficiary;
    }
}
