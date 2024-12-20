use crate::msg::{TransferMsg, Ucs01RelayExecuteMsg, ValidatorDelegation};
use crate::token_factory_api::TokenFactoryMsg;
use crate::ContractError;
use crate::{msg::DelegationDiff, msg::UndelegationRecord, state::Validator};
use cosmwasm_std::{
    to_json_binary, Coin, CosmosMsg, Decimal, DelegationTotalRewardsResponse, DepsMut,
    QuerierWrapper, StakingMsg, StdResult, Uint128, Uint256, WasmMsg,
};
use std::collections::HashMap;
use std::str::FromStr;

const DECIMAL_FRACTIONAL: u128 = 1_000_000_000_000_000_000u128;

/// return how much staking token from underlying native coin denom
pub fn calculate_staking_token_from_rate(stake_amount: Uint128, exchange_rate: Decimal) -> Uint128 {
    let decimal_fract = Decimal::new(Uint128::from(DECIMAL_FRACTIONAL * DECIMAL_FRACTIONAL));
    let fract = (exchange_rate * decimal_fract).to_uint_ceil();
    Decimal::from_ratio(Uint128::from(DECIMAL_FRACTIONAL) * stake_amount, fract).to_uint_floor()
}

/// return how much underlying native coin denom from staking token base on exchange rate
pub fn calculate_native_token_from_staking_token(
    staking_token: Uint128,
    exchange_rate: Decimal,
) -> Uint128 {
    let decimal_fract = Decimal::new(Uint128::from(DECIMAL_FRACTIONAL * DECIMAL_FRACTIONAL));
    let fract = (exchange_rate * decimal_fract).to_uint_ceil();
    Decimal::from_ratio(fract * staking_token, Uint128::from(DECIMAL_FRACTIONAL)).to_uint_floor()
}

/// get total delegated token value from validators in native token
pub fn get_actual_total_delegated(
    querier: QuerierWrapper,
    delegator: String,
    denom: String,
    validators: Vec<String>,
) -> Uint128 {
    let delegations_resp = querier.query_all_delegations(delegator);
    let mut total: Uint128 = Uint128::new(0);

    if delegations_resp.is_ok() {
        total = delegations_resp
            .unwrap()
            .into_iter()
            .filter(|d| {
                d.amount.denom == denom
                    && !d.amount.amount.is_zero()
                    && validators.contains(&d.validator)
            })
            .map(|d| d.amount.amount)
            .sum();
    }

    total
}

/// get total delegated token value from validators in native token
pub fn get_actual_total_reward(
    querier: QuerierWrapper,
    delegator: String,
    denom: String,
    validators: Vec<String>,
) -> StdResult<Uint128> {
    let mut total_rewards = Uint128::new(0);
    let result: StdResult<DelegationTotalRewardsResponse> =
        querier.query_delegation_total_rewards(delegator);

    if result.is_ok() {
        for delegator_reward in result.unwrap().rewards {
            if validators.contains(&delegator_reward.validator_address) {
                for reward in delegator_reward.reward {
                    if reward.denom == denom {
                        let reward_val = to_uint128(reward.amount.to_uint_floor())?;
                        total_rewards += reward_val;
                    }
                }
            }
        }
    }

    Ok(total_rewards)
}

pub fn to_uint128(v: Uint256) -> StdResult<Uint128> {
    Uint128::from_str(&v.to_string())
}

pub fn get_mock_total_reward(total_bond_amount: Uint128) -> Uint128 {
    let ratio = Decimal::from_ratio(Uint128::new(1000), Uint128::new(1005));
    calculate_staking_token_from_rate(total_bond_amount, ratio)
}

/// return how much to undelegate native token from ratio of total delegated amount divide with total bond with reward value amount
pub fn calculate_undelegate_amount(
    native_token_amount: Uint128,
    delegated_amount: Uint128,
    total_bonded_amount: Uint128,
) -> Uint128 {
    let native_token_undelegate_decimal =
        Decimal::new(native_token_amount * Uint128::from(DECIMAL_FRACTIONAL));
    let ratio = Decimal::from_ratio(delegated_amount, total_bonded_amount);

    println!(
        "native_token_undelegate_decimal: {:?}",
        native_token_undelegate_decimal
    );
    println!("ratio: {:?}", ratio);

    let undelegate_native_decimal = native_token_undelegate_decimal * ratio;
    undelegate_native_decimal.to_uint_floor()
}
pub fn calculate_delegated_amount(amount: Uint128, ratio: Decimal) -> Uint128 {
    let decimal_fract = Decimal::new(Uint128::from(DECIMAL_FRACTIONAL * DECIMAL_FRACTIONAL));
    let fract = (ratio * decimal_fract).to_uint_ceil();
    let delegate_amount =
        Decimal::from_ratio(fract * amount, Uint128::from(DECIMAL_FRACTIONAL)).to_uint_floor();
    delegate_amount
}

pub fn get_burn_msg(denom: String, burn_amount: Uint128, delegator: String) -> TokenFactoryMsg {
    let burn_msg = TokenFactoryMsg::BurnTokens {
        denom: denom.clone(),
        amount: burn_amount,
        burn_from_address: delegator,
    };

    burn_msg
}

pub fn get_undelegate_from_validator_msgs(
    undelegate_amount: Uint128,
    coin_denom: String,
    validators: Vec<Validator>,
) -> (Vec<CosmosMsg<TokenFactoryMsg>>, Vec<UndelegationRecord>) {
    let mut msgs: Vec<CosmosMsg<TokenFactoryMsg>> = vec![];
    let mut undelegations: Vec<UndelegationRecord> = vec![];

    let total_weight = Uint128::from(
        validators
            .iter()
            .map(|v| v.weight)
            .reduce(|a, b| (a + b))
            .unwrap_or(0),
    );

    let total_validators = validators.len();
    let mut total_undelegated: Uint128 = Uint128::from(0u32);

    for (pos, validator) in validators.into_iter().enumerate() {
        let ratio = Decimal::from_ratio(Uint128::from(validator.weight), total_weight);

        let undelegate_amount_dec =
            Decimal::new(undelegate_amount * Uint128::from(DECIMAL_FRACTIONAL));
        let mut undelegate_amount_for_validator = (undelegate_amount_dec * ratio).to_uint_floor();
        total_undelegated += undelegate_amount_for_validator;

        if pos == (total_validators - 1) {
            let remaining = undelegate_amount - total_undelegated;
            undelegate_amount_for_validator += remaining;
        }

        let amount = Coin {
            amount: undelegate_amount_for_validator.clone(),
            denom: coin_denom.to_string(),
        };
        let undelegate_staking_msg: CosmosMsg<TokenFactoryMsg> =
            CosmosMsg::Staking(StakingMsg::Undelegate {
                validator: validator.address.to_string(),
                amount,
            });
        msgs.push(undelegate_staking_msg);

        undelegations.push(UndelegationRecord {
            amount: undelegate_amount_for_validator,
            validator,
        })
    }

    (msgs, undelegations)
}

pub fn send_to_evm(
    contract_addr: String,
    channel: String,
    receiver: String,
    funds: Vec<Coin>,
) -> Result<WasmMsg, ContractError> {
    let relay_transfer_msg: Ucs01RelayExecuteMsg = Ucs01RelayExecuteMsg::Transfer(TransferMsg {
        channel,
        receiver,
        memo: "Send back to EVM".to_string(),
        timeout: None,
    });
    let transfer_relay_msg = to_json_binary(&relay_transfer_msg)?;

    return Ok(WasmMsg::Execute {
        contract_addr,
        msg: transfer_relay_msg,
        funds,
    });
}

pub fn get_validator_delegation_map_with_total_bond(
    deps: DepsMut,
    delegator: String,
    validators: Vec<Validator>,
) -> Result<(HashMap<String, Uint128>, Uint128), ContractError> {
    let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

    let mut total_delegated_amount = Uint128::from(0u32);

    for validator in validators {
        let validator_bond = deps
            .querier
            .query_delegation(delegator.clone(), validator.address.clone())?;

        let delegation_amount = match validator_bond {
            Some(delegation) => delegation.amount.amount,
            None => Uint128::from(0u32),
        };
        validator_delegation_map.insert(validator.address.to_string(), delegation_amount);
        total_delegated_amount += delegation_amount;
    }

    Ok((validator_delegation_map, total_delegated_amount))
}

pub fn get_validator_delegation_map_base_on_weight(
    validators: Vec<Validator>,
    total_delegated_amount: Uint128,
) -> Result<HashMap<String, Uint128>, ContractError> {
    let total_weight = validators
        .iter()
        .map(|v| v.weight)
        .reduce(|a, b| (a + b))
        .unwrap_or(0);

    let mut correct_validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

    for validator in validators {
        let ratio = Decimal::from_ratio(validator.weight, total_weight);
        let delegation_amount = calculate_delegated_amount(total_delegated_amount, ratio);
        correct_validator_delegation_map.insert(validator.address.to_string(), delegation_amount);
    }

    Ok(correct_validator_delegation_map)
}

pub fn get_surplus_deficit_validators(
    validator_delegation_map: HashMap<String, Uint128>,
    correct_validator_delegation_map: HashMap<String, Uint128>,
) -> (Vec<ValidatorDelegation>, Vec<ValidatorDelegation>) {
    let mut surplus_validators: Vec<ValidatorDelegation> = vec![];
    let mut deficient_validators: Vec<ValidatorDelegation> = vec![];
    for (key, previous_amount) in validator_delegation_map.clone().iter_mut() {
        // check if old validator key exists on new validators map
        if correct_validator_delegation_map.get(key).is_none() {
            // because old validator not exists on new one that means the previous validator
            // need to be restaked fully so it is surplus
            surplus_validators.push(ValidatorDelegation {
                address: key.to_string(),
                delegation_diff: DelegationDiff::Surplus,
                diff_amount: *previous_amount,
            })
        };
    }

    for (new_validator_key, correct_amount) in correct_validator_delegation_map.clone().iter_mut() {
        // check if previous validator exists
        match validator_delegation_map.get(new_validator_key) {
            Some(previous_amount) => {
                if *previous_amount > *correct_amount {
                    surplus_validators.push(ValidatorDelegation {
                        address: new_validator_key.to_string(),
                        delegation_diff: DelegationDiff::Surplus,
                        diff_amount: *previous_amount - *correct_amount,
                    })
                } else {
                    deficient_validators.push(ValidatorDelegation {
                        address: new_validator_key.to_string(),
                        delegation_diff: DelegationDiff::Deficit,
                        diff_amount: *correct_amount - *previous_amount,
                    })
                }
            }
            None => deficient_validators.push(ValidatorDelegation {
                address: new_validator_key.to_string(),
                delegation_diff: DelegationDiff::Deficit,
                diff_amount: correct_amount.clone(),
            }),
        }
    }

    (surplus_validators, deficient_validators)
}

pub fn get_restaking_msgs(
    mut surplus_validators: Vec<ValidatorDelegation>,
    mut deficient_validators: Vec<ValidatorDelegation>,
    denom: String,
) -> Vec<CosmosMsg<TokenFactoryMsg>> {
    let mut msgs: Vec<CosmosMsg<TokenFactoryMsg>> = vec![];

    surplus_validators.sort_by(|a, b| b.diff_amount.cmp(&a.diff_amount));
    deficient_validators.sort_by_key(|a| a.diff_amount);

    for surplus_validator in surplus_validators.iter_mut() {
        for deficient_validator in deficient_validators.iter_mut() {
            if surplus_validator.diff_amount < deficient_validator.diff_amount {
                if surplus_validator.diff_amount == Uint128::from(0u32) {
                    break;
                }

                // the deficit amount higher than surplus amount so we can restake all surplus amount
                let undelegate_msg = CosmosMsg::Staking(StakingMsg::Redelegate {
                    src_validator: surplus_validator.address.clone(),
                    dst_validator: deficient_validator.address.clone(),
                    amount: Coin {
                        amount: surplus_validator.diff_amount,
                        denom: denom.clone(),
                    },
                });
                surplus_validator.diff_amount = Uint128::from(0u32);
                deficient_validator.diff_amount =
                    deficient_validator.diff_amount - surplus_validator.diff_amount;

                msgs.push(undelegate_msg);
                //
            } else {
                println!("{:?} <> {:?}", surplus_validator, deficient_validator);

                let undelegate_msg: CosmosMsg<TokenFactoryMsg> =
                    CosmosMsg::Staking(StakingMsg::Redelegate {
                        src_validator: surplus_validator.address.clone(),
                        dst_validator: deficient_validator.address.clone(),
                        amount: Coin {
                            amount: deficient_validator.diff_amount,
                            denom: denom.clone(),
                        },
                    });

                surplus_validator.diff_amount =
                    surplus_validator.diff_amount - deficient_validator.diff_amount;
                deficient_validator.diff_amount = Uint128::from(0u32);
                msgs.push(undelegate_msg);
            }
        }
    }

    msgs
}

pub fn get_delegate_to_validator_msgs(
    delegate_amount: Uint128,
    coin_denom: String,
    validators: Vec<Validator>,
) -> Vec<CosmosMsg<TokenFactoryMsg>> {
    let total_weight = validators
        .iter()
        .map(|v| v.weight)
        .reduce(|a, b| (a + b))
        .unwrap_or(1);

    let mut total_delegated: Uint128 = Uint128::from(0u32);

    let mut msgs: Vec<CosmosMsg<TokenFactoryMsg>> = vec![];
    let mut first_validator: String = "".to_string();

    for (pos, validator) in validators.into_iter().enumerate() {
        let ratio =
            Decimal::from_ratio(Uint128::from(validator.weight), Uint128::from(total_weight));

        let delegate_amount = calculate_delegated_amount(delegate_amount, ratio);
        total_delegated += delegate_amount;
        let amount = Coin {
            amount: delegate_amount.clone(),
            denom: coin_denom.to_string(),
        };
        let staking_msg: CosmosMsg<TokenFactoryMsg> = CosmosMsg::Staking(StakingMsg::Delegate {
            validator: validator.address.to_string(),
            amount,
        });

        msgs.push(staking_msg.into());

        if pos == 0 {
            first_validator = validator.address.to_string();
        }
    }

    // calculate remaining

    let remaining_amount = delegate_amount - total_delegated;
    if !remaining_amount.is_zero() {
        let remaining_staking_msg: CosmosMsg<TokenFactoryMsg> =
            CosmosMsg::Staking(StakingMsg::Delegate {
                validator: first_validator,
                amount: Coin {
                    denom: coin_denom.clone(),
                    amount: remaining_amount,
                },
            });

        msgs.push(remaining_staking_msg.into());
    }
    msgs
}
