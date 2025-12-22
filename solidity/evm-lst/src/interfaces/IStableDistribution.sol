// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

interface IStableDistribution {
    // Structs
    struct Coin {
        string denom;
        uint256 amount;
    }

    struct DecCoin {
        string denom;
        uint256 amount;
        uint8 precision;
    }

    struct ValidatorDistributionInfo {
        address operatorAddress;
        DecCoin[] selfBondRewards;
        DecCoin[] commission;
    }

    struct Dec {
        uint64 value;
        uint8 precision;
    }

    struct ValidatorSlashEvent {
        uint64 validatorPeriod;
        Dec fraction;
    }

    struct PageReq {
        bytes key;
        uint64 offset;
        uint64 limit;
        bool countTotal;
        bool reverse;
    }

    struct PageResp {
        bytes nextKey;
        uint64 total;
    }

    struct DelegationDelegatorReward {
        address validatorAddress;
        DecCoin[] reward;
    }

    // Methods
    function setWithdrawAddress(address delegatorAddress, address withdrawerAddress) external returns (bool success);

    function withdrawDelegatorRewards(address delegatorAddress, address validatorAddress)
        external
        returns (Coin[] memory amount);

    function withdrawValidatorCommission(address validatorAddress) external returns (Coin[] memory amount);

    function validatorDistributionInfo(address validatorAddress)
        external
        view
        returns (ValidatorDistributionInfo memory distributionInfo);

    function validatorOutstandingRewards(address validatorAddress) external view returns (DecCoin[] memory rewards);

    function validatorCommission(address validatorAddress) external view returns (DecCoin[] memory commission);

    function validatorSlashes(
        address validatorAddress,
        uint64 startingHeight,
        uint64 endingHeight,
        PageReq calldata pageRequest
    ) external view returns (ValidatorSlashEvent[] memory slashes, PageResp memory pagination);

    function delegationRewards(address delegatorAddress, address validatorAddress)
        external
        view
        returns (DecCoin[] memory rewards);

    function delegationTotalRewards(address delegatorAddress)
        external
        view
        returns (DelegationDelegatorReward[] memory rewards, DecCoin[] memory total);

    function delegatorValidators(address delegatorAddress) external view returns (string[] memory validators);

    function delegatorWithdrawAddress(address delegatorAddress) external view returns (address withdrawAddress);

    // Events
    event SetWithdrawAddress(address indexed caller, address withdrawAddress);
    event WithdrawDelegatorRewards(address indexed delegatorAddress, address indexed validatorAddress, uint256 amount);
    event WithdrawValidatorCommission(address indexed validatorAddress, uint256 commission);
}
