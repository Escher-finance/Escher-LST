// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

/// @title IStaking - Stable Staking Precompile Interface
/// @notice Contract address: 0x0000000000000000000000000000000000000800

interface IStableStaking {
    // ============ Structs ============

    struct Coin {
        string denom;
        uint256 amount;
    }

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

    struct Validator {
        address operatorAddress;
        string consensusPubkey;
        bool jailed;
        int32 status;
        uint256 tokens;
        uint256 delegatorShares;
        string description;
        int64 unbondingHeight;
        int64 unbondingTime;
        uint256 commission;
        uint256 minSelfDelegation;
    }

    struct PageReq {
        bytes key;
        int64 offset;
        int64 limit;
        bool countTotal;
        bool reverse;
    }

    struct PageResp {
        bytes nextKey;
        uint64 total;
    }

    struct UnbondingDelegationEntry {
        uint64 creationHeight;
        uint64 completionTime;
        Coin initialBalance;
        Coin balance;
    }

    struct UnbondingDelegationOutput {
        address validatorAddress;
        address delegatorAddress;
        UnbondingDelegationEntry[] entries;
    }

    struct RedelegationEntry {
        uint64 creationHeight;
        uint64 completionTime;
        Coin initialBalance;
        Coin balance;
    }

    struct RedelegationOutput {
        address delegatorAddress;
        address validatorSrcAddress;
        address validatorDstAddress;
        RedelegationEntry[] entries;
    }

    // ============ Write Functions ============

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

    function delegate(address delegatorAddress, address validatorAddress, uint256 amount)
        external
        returns (bool success);

    function undelegate(address delegatorAddress, address validatorAddress, uint256 amount)
        external
        returns (bool success);

    function redelegate(
        address delegatorAddress,
        string calldata validatorSrc,
        string calldata validatorDst,
        uint256 amount
    ) external returns (bool success);

    // ============ Query Functions ============

    function delegation(address delegatorAddress, address validatorAddress)
        external
        view
        returns (uint256 shares, Coin memory balance);

    function unbondingDelegation(address delegatorAddress, address validatorAddress)
        external
        view
        returns (UnbondingDelegationOutput memory);

    function validator(address validatorAddress) external view returns (Validator memory);

    function validators(string calldata status, PageReq calldata pageRequest)
        external
        view
        returns (Validator[] memory, PageResp memory);

    function redelegation(address delegatorAddress, address srcValidatorAddress, address dstValidatorAddress)
        external
        view
        returns (RedelegationOutput memory);

    function redelegations(
        address delegatorAddress,
        address srcValidatorAddress,
        address dstValidatorAddress,
        PageReq calldata pageRequest
    ) external view returns (RedelegationOutput[] memory, PageResp memory);

    // ============ Events ============

    event CreateValidator(address indexed valiAddr, uint256 value);
    event EditValidator(address indexed valiAddr, int256 commissionRate, int256 minSelfDelegation);
    event Delegate(address indexed delegatorAddr, string indexed validatorAddr, uint256 amount, uint256 newShares);
    event Unbond(address indexed delegatorAddr, string indexed validatorAddr, uint256 amount, uint256 completionTime);
    event Redelegate(
        address indexed delegatorAddr,
        address indexed validatorSrcAddress,
        address indexed validatorDstAddress,
        uint256 amount,
        uint256 completionTime
    );
}
