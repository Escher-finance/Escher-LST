// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

interface IStableStaking {
    // Structs
    struct Description {
        string moniker;
        string identity;
        string website;
        string securityContact;
        string details;
    }

    struct CommissionRates {
        uint256 rate;
        uint256 maxRate;
        uint256 maxChangeRate;
    }

    struct Coin {
        string denom;
        uint256 amount;
    }

    // Methods
    function createValidator(
        Description calldata description,
        CommissionRates calldata commissionRates,
        uint256 minSelfDelegation,
        address validatorAddress,
        string calldata pubkey,
        uint256 value
    ) external returns (bool success);

    function editValidator(
        Description calldata description,
        address validatorAddress,
        int256 commissionRate,
        int256 minSelfDelegation
    ) external returns (bool success);

    function delegate(
        address delegatorAddress,
        address validatorAddress,
        uint256 amount
    ) external returns (bool success);

    function undelegate(
        address delegatorAddress,
        address validatorAddress,
        uint256 amount
    ) external returns (bool success);

    function redelegate(
        address delegatorAddress,
        string calldata validatorSrc,
        string calldata validatorDst,
        uint256 amount
    ) external returns (bool success);

    // Events
    event CreateValidator(address indexed valiAddr, uint256 value);
    event EditValidator(
        address indexed valiAddr,
        int256 commissionRate,
        int256 minSelfDelegation
    );
    event Delegate(
        address indexed delegatorAddr,
        string indexed validatorAddr,
        uint256 amount,
        uint256 newShares
    );
    event Unbond(
        address indexed delegatorAddr,
        string indexed validatorAddr,
        uint256 amount,
        uint256 completionTime
    );
    event Redelegate(
        address indexed delegatorAddr,
        address indexed validatorSrcAddress,
        address indexed validatorDstAddress,
        uint256 amount,
        uint256 completionTime
    );
}
