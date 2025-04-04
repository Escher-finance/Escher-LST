use crate::msg::ValidatorDelegation;
use crate::state::ValidatorsRegistry;
use crate::utils::calc;
use crate::utils::delegation;
use crate::utils::token;
use crate::ContractError;
use crate::{
    msg::{BondData, DelegationDiff, MintTokensPayload, UnbondData},
    state::{
        increment_tokens, unbond_record, Parameters, UnbondRecord, Validator, LOG, PARAMETERS,
        STATE, VALIDATORS_REGISTRY,
    },
};
use cosmwasm_std::{
    to_json_binary, Addr, Coin, CosmosMsg, Decimal, DelegationTotalRewardsResponse, DepsMut, Env,
    QuerierWrapper, StakingMsg, StdResult, Storage, SubMsg, Uint128, Uint256,
};
use std::collections::HashMap;
use std::str::FromStr;

pub const DEFAULT_TIMEOUT_TIMESTAMP_OFFSET: u64 = 600;

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
    reward_contract: String,
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

    let reward_balance = querier.query_balance(reward_contract, denom)?;
    total_rewards += reward_balance.amount;
    // add query reward contract balance
    Ok(total_rewards)
}

pub fn get_unclaimed_reward(
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

    // add query reward contract balance
    Ok(total_rewards)
}

/// Convert Uint256 to Uint128
pub fn to_uint128(v: Uint256) -> StdResult<Uint128> {
    Uint128::from_str(&v.to_string())
}

// for testing only
pub fn get_mock_total_reward(total_bond_amount: Uint128) -> Uint128 {
    let ratio = Decimal::from_ratio(Uint128::new(1000), Uint128::new(1005));
    calc::calculate_staking_token_from_rate(total_bond_amount, ratio)
}

/// return how much to undelegate native token from ratio of total delegated amount divide with total bond with reward value amount
/// NOTE: Not used
pub fn calculate_undelegate_amount(
    native_token_amount: Uint128,
    delegated_amount: Uint128,
    total_bonded_amount: Uint128,
) -> Uint128 {
    let native_token_undelegate_decimal = Decimal::from_ratio(native_token_amount, Uint128::one());
    let ratio = Decimal::from_ratio(delegated_amount, total_bonded_amount);

    println!(
        "native_token_undelegate_decimal: {:?}",
        native_token_undelegate_decimal
    );
    println!("ratio: {:?}", ratio);

    let undelegate_native_decimal = native_token_undelegate_decimal * ratio;
    undelegate_native_decimal.to_uint_floor()
}

/// NOTE: This is the same calculation as `utils:calculate_native_token_from_staking_token`
pub fn calculate_delegated_amount(amount: Uint128, ratio: Decimal) -> Uint128 {
    (ratio * Decimal::from_ratio(amount, Uint128::one())).to_uint_floor()
}

pub fn get_undelegate_from_validator_msgs(
    undelegate_amount: Uint128,
    coin_denom: String,
    validators: Vec<Validator>,
) -> Vec<CosmosMsg> {
    let mut msgs: Vec<CosmosMsg> = vec![];

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

        let undelegate_amount_dec = Decimal::from_ratio(undelegate_amount, Uint128::one());
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
        let undelegate_staking_msg: CosmosMsg = CosmosMsg::Staking(StakingMsg::Undelegate {
            validator: validator.address.to_string(),
            amount,
        });
        msgs.push(undelegate_staking_msg);
    }

    msgs
}

pub fn get_validator_delegation_map_with_total_bond(
    deps: DepsMut,
    delegator: String,
    validators: Vec<Validator>,
) -> Result<(HashMap<String, Uint128>, Uint128), ContractError> {
    let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

    let mut total_delegated_amount = Uint128::zero();

    for validator in validators {
        let validator_bond = deps
            .querier
            .query_delegation(delegator.clone(), validator.address.clone())?;

        let delegation_amount = match validator_bond {
            Some(delegation) => delegation.amount.amount,
            None => Uint128::zero(),
        };
        validator_delegation_map.insert(validator.address.to_string(), delegation_amount);
        total_delegated_amount += delegation_amount;
    }

    Ok((validator_delegation_map, total_delegated_amount))
}

pub fn get_validator_delegation_map_base_on_weight(
    validators: Vec<Validator>,
    total_delegated_amount: Uint128,
) -> HashMap<String, Uint128> {
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

    correct_validator_delegation_map
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
) -> Vec<CosmosMsg> {
    let mut msgs: Vec<CosmosMsg> = vec![];

    surplus_validators.sort_by(|a, b| b.diff_amount.cmp(&a.diff_amount));
    deficient_validators.sort_by_key(|a| a.diff_amount);

    for surplus_validator in surplus_validators.iter_mut() {
        for deficient_validator in deficient_validators.iter_mut() {
            if surplus_validator.diff_amount < deficient_validator.diff_amount {
                if surplus_validator.diff_amount.is_zero() {
                    continue;
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
            } else {
                if deficient_validator.diff_amount.is_zero() {
                    continue;
                }

                println!("{:?} <> {:?}", surplus_validator, deficient_validator);

                let undelegate_msg: CosmosMsg = CosmosMsg::Staking(StakingMsg::Redelegate {
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
) -> Vec<CosmosMsg> {
    let total_weight = validators
        .iter()
        .map(|v| v.weight)
        .reduce(|a, b| (a + b))
        .unwrap_or(1);

    let mut total_delegated: Uint128 = Uint128::from(0u32);

    let mut msgs: Vec<CosmosMsg> = vec![];
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
        let staking_msg: CosmosMsg = CosmosMsg::Staking(StakingMsg::Delegate {
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
        let remaining_staking_msg: CosmosMsg = CosmosMsg::Staking(StakingMsg::Delegate {
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

pub fn get_unbond_all_messages(
    deps: DepsMut,
    delegator: Addr,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let delegations_resp = deps.querier.query_all_delegations(delegator);
    let params = PARAMETERS.load(deps.storage)?;
    let denom = params.underlying_coin_denom;

    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let mut msgs: Vec<CosmosMsg> = vec![];
    for (_pos, validator) in validators_reg.validators.iter().enumerate() {
        let undelegate_amount: Uint128 = delegations_resp
            .as_ref()
            .unwrap()
            .into_iter()
            .filter(|d| {
                d.amount.denom == denom
                    && !d.amount.amount.is_zero()
                    && d.validator == validator.address
            })
            .map(|d| d.amount.amount)
            .sum();

        let amount = Coin {
            amount: undelegate_amount.clone(),
            denom: denom.to_string(),
        };
        let undelegate_staking_msg: CosmosMsg = CosmosMsg::Staking(StakingMsg::Undelegate {
            validator: validator.address.to_string(),
            amount,
        });

        msgs.push(undelegate_staking_msg.into());
    }

    Ok(msgs)
}

pub fn adjust_validators_delegation(
    deps: DepsMut,
    delegator: Addr,
    prev_validators: Vec<Validator>,
    validators: Vec<Validator>,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let denom = params.underlying_coin_denom;

    let (validator_delegation_map, total_delegated_amount) =
        get_validator_delegation_map_with_total_bond(deps, delegator.to_string(), prev_validators)?;

    let correct_validator_delegation_map =
        get_validator_delegation_map_base_on_weight(validators, total_delegated_amount);

    let (surplus_validators, deficient_validators) =
        get_surplus_deficit_validators(validator_delegation_map, correct_validator_delegation_map);

    let msgs: Vec<CosmosMsg> = get_restaking_msgs(surplus_validators, deficient_validators, denom);

    Ok(msgs)
}

/// Process bond call to mint liquid staking token, delegate/stake base on the bond amount and exchange rate
pub fn process_bond(
    storage: &mut dyn Storage,
    querier: QuerierWrapper,
    sender: String,
    staker: String,
    delegator: Addr,
    amount: Uint128,
    bond_time: u64,
    params: Parameters,
    validators_reg: ValidatorsRegistry,
    salt: String,
    channel_id: Option<u32>,
) -> Result<(Vec<CosmosMsg>, Vec<SubMsg>, BondData), ContractError> {
    let coin_denom = params.underlying_coin_denom.to_string();
    let msgs = delegation::get_delegate_to_validator_msgs(
        amount,
        coin_denom.to_string(),
        validators_reg.validators.clone(),
    );

    let validators_list: Vec<String> = validators_reg
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();

    // logic to mint token and update the supply and total_bond_amount
    let mut state = STATE.load(storage)?;

    let delegated_amount = get_actual_total_delegated(
        querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list.clone(),
    );

    let total_bond_amount: Uint128;
    if !cfg!(test) {
        state.total_delegated_amount = delegated_amount;
        let reward = get_actual_total_reward(
            querier,
            delegator.to_string(),
            coin_denom.clone(),
            validators_list,
            params.reward_address.into(),
        )?;

        total_bond_amount = delegated_amount + reward;
    } else {
        total_bond_amount = get_mock_total_reward(state.total_bond_amount);
    }

    let mut exchange_rate = state.exchange_rate;

    if !total_bond_amount.is_zero() && !state.total_supply.is_zero() {
        exchange_rate = Decimal::from_ratio(total_bond_amount, state.total_supply);
    }

    let mint_amount = calc::calculate_staking_token_from_rate(amount, exchange_rate);

    // after update exchange rate we update the state
    state.bond_counter = state.bond_counter + 1;
    state.total_bond_amount = total_bond_amount + amount;
    state.total_supply += mint_amount;
    state.total_delegated_amount += amount;
    state.last_bond_time = bond_time;
    state.update_exchange_rate();

    STATE.save(storage, &state)?;

    let mut sub_msgs: Vec<SubMsg> = vec![];
    let payload = MintTokensPayload {
        sender: sender.to_string(),
        staker: staker.clone(),
        amount: mint_amount,
        salt,
        channel_id,
    };
    let payload_bin = to_json_binary(&payload)?;

    if !cfg!(test) {
        // Start to mint according to staked token only if it is not test
        let sub_msg: SubMsg = token::get_staked_token_submsg(
            delegator.to_string(),
            delegator.to_string(),
            mint_amount,
            params.liquidstaking_denom.clone(),
            payload_bin,
            params.cw20_address,
        );
        sub_msgs.push(sub_msg);
    }

    Ok((
        msgs,
        sub_msgs,
        BondData {
            mint_amount,
            delegated_amount: state.total_delegated_amount,
            total_bond_amount: state.total_bond_amount,
            exchange_rate,
            total_supply: state.total_supply,
        },
    ))
}

/// Process unbond that will burn liquid staking token, undelegate some amount from validator according to exchange rate and create UnbondRecord
pub fn process_unbond(
    env: Env,
    storage: &mut dyn Storage,
    querier: QuerierWrapper,
    sender: String,
    staker: String,
    delegator: Addr,
    unbond_amount: Uint128,
    params: Parameters,
    validators_reg: ValidatorsRegistry,
    channel_id: Option<u32>,
) -> Result<(Vec<CosmosMsg>, UnbondData), ContractError> {
    let coin_denom = params.underlying_coin_denom;

    let validators_list: Vec<String> = validators_reg
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();

    let mut state = STATE.load(storage)?;

    let delegated_amount = get_actual_total_delegated(
        querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list.clone(),
    );
    state.total_delegated_amount = delegated_amount;
    let reward = get_actual_total_reward(
        querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list,
        params.reward_address.into(),
    )?;

    let total_bond_amount = delegated_amount + reward;

    if total_bond_amount.is_zero() || state.total_supply.is_zero() {
        return Err(ContractError::ZeroSupplyOrDelegatedAmount {});
    }
    let current_exchange_rate = Decimal::from_ratio(total_bond_amount, state.total_supply);

    // calculate how much native token undelegated amount from staked token amount base on current exchange rate
    let undelegate_amount: Uint128 = calc::calculate_native_token_from_staking_token(
        unbond_amount.clone(),
        current_exchange_rate,
    );

    let mut msgs: Vec<CosmosMsg> = vec![];
    if delegated_amount < undelegate_amount {
        return Err(ContractError::NotEnoughAvailableFund {}); // this error only happen on development or sole staker and if process rewards not happen yet
    }
    let undelegate_msgs = get_undelegate_from_validator_msgs(
        undelegate_amount,
        coin_denom.clone(),
        validators_reg.validators,
    );
    msgs.extend(undelegate_msgs.clone());

    LOG.save(
        storage,
        &format!(
            "undelegate_amount: {}, {:?}",
            undelegate_amount, undelegate_msgs
        ),
    )?;
    let burn_msg = token::burn_token(unbond_amount, params.cw20_address.to_string());
    msgs.push(burn_msg.into());

    let id: u64 = increment_tokens(storage).unwrap();
    let history = UnbondRecord {
        id,
        height: env.block.height,
        channel_id,
        sender: sender.clone(),
        staker: staker.clone(),
        amount: unbond_amount,
        undelegate_amount,
        created: env.block.time,
        released_height: 0,
        released: false,
    };
    unbond_record().save(storage, id, &history)?;

    // // update total bond, supply and exchange rate here
    state.total_bond_amount = total_bond_amount - undelegate_amount;
    state.total_supply = state.total_supply - unbond_amount;
    state.total_delegated_amount = delegated_amount - undelegate_amount;
    state.update_exchange_rate();
    STATE.save(storage, &state)?;

    Ok((
        msgs,
        UnbondData {
            record_id: id,
            undelegate_amount,
            delegated_amount: state.total_delegated_amount,
            reward,
            exchange_rate: current_exchange_rate,
            total_supply: state.total_supply,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::assert_approx_eq;

    #[test]
    fn test_calculate_delegated_amount() {
        let amount = Uint128::new(112382);
        assert_eq!(
            calculate_delegated_amount(amount, Decimal::from_ratio(1_u128, 2_u128)),
            amount / Uint128::new(2)
        );
    }

    #[test]
    fn test_get_undelegate_from_validator_msgs() {
        let undelegate_amount = Uint128::from(500642u32);
        let coin_denom = "muno".to_string();

        let validators = vec![
            Validator {
                address: "abc".to_string(),
                weight: 10,
            },
            Validator {
                address: "bcd".to_string(),
                weight: 20,
            },
        ];
        let undelegate_msgs =
            get_undelegate_from_validator_msgs(undelegate_amount, coin_denom.clone(), validators);
        let undelegate_msgs_unwrapped = undelegate_msgs
            .iter()
            .filter_map(|msg| {
                if let CosmosMsg::Staking(StakingMsg::Undelegate { validator, amount }) = msg {
                    return Some((validator, amount.amount));
                }
                return None;
            })
            .collect::<Vec<_>>();
        assert_eq!(undelegate_msgs_unwrapped.len(), 2);
        assert_eq!(
            undelegate_msgs_unwrapped[0].1 + undelegate_msgs_unwrapped[1].1,
            undelegate_amount
        );
        assert_approx_eq!(
            undelegate_msgs_unwrapped[0].1 * Uint128::from(2_u128),
            undelegate_msgs_unwrapped[1].1,
            "0.001"
        );
    }

    #[test]
    fn test_get_validator_delegation_map_base_on_weight() {
        let validators = vec![
            Validator {
                address: "abc".to_string(),
                weight: 10,
            },
            Validator {
                address: "bcd".to_string(),
                weight: 20,
            },
        ];
        let total_delegated_amount = Uint128::new(250000);
        let map =
            get_validator_delegation_map_base_on_weight(validators.clone(), total_delegated_amount);
        assert_eq!(map.len(), 2);
        assert_approx_eq!(
            map.values().sum::<Uint128>(),
            total_delegated_amount,
            "0.0001"
        );
        assert_approx_eq!(
            map.get("abc").unwrap() * Uint128::new(2),
            *map.get("bcd").unwrap(),
            "0.0001"
        );

        assert_eq!(
            HashMap::new(),
            get_validator_delegation_map_base_on_weight(vec![], total_delegated_amount)
        );
        assert_eq!(
            HashMap::from([
                (validators[0].address.clone(), Uint128::zero()),
                (validators[1].address.clone(), Uint128::zero())
            ]),
            get_validator_delegation_map_base_on_weight(validators, Uint128::zero())
        );
    }

    #[test]
    fn test_get_surplus_deficit_validators() {
        let validator_a = "a".to_string();
        let validator_b = "b".to_string();
        let validator_c = "c".to_string();
        let validator_d = "d".to_string();
        let amount_a = Uint128::new(100);
        let amount_b = Uint128::new(1000);
        let amount_c = Uint128::new(250);
        let correct_amount_b = Uint128::new(800);
        let correct_amount_c = Uint128::new(350);
        let correct_amount_d = Uint128::new(500);
        let validator_delegation_map = HashMap::from([
            (validator_a.clone(), amount_a),
            (validator_b.clone(), amount_b),
            (validator_c.clone(), amount_c),
        ]);
        let correct_validator_delegation_map = HashMap::from([
            (validator_b.clone(), correct_amount_b),
            (validator_c.clone(), correct_amount_c),
            (validator_d.clone(), correct_amount_d),
        ]);
        let (mut surplus_validators, mut deficit_validators) = get_surplus_deficit_validators(
            validator_delegation_map,
            correct_validator_delegation_map,
        );
        surplus_validators.sort_by_key(|v| v.address.clone());
        deficit_validators.sort_by_key(|v| v.address.clone());
        assert_eq!(
            surplus_validators,
            vec![
                ValidatorDelegation {
                    address: validator_a.clone(),
                    delegation_diff: DelegationDiff::Surplus,
                    diff_amount: amount_a,
                },
                ValidatorDelegation {
                    address: validator_b.clone(),
                    delegation_diff: DelegationDiff::Surplus,
                    diff_amount: amount_b.abs_diff(correct_amount_b),
                },
            ]
        );
        assert_eq!(
            deficit_validators,
            vec![
                ValidatorDelegation {
                    address: validator_c.clone(),
                    delegation_diff: DelegationDiff::Deficit,
                    diff_amount: amount_c.abs_diff(correct_amount_c),
                },
                ValidatorDelegation {
                    address: validator_d.clone(),
                    delegation_diff: DelegationDiff::Deficit,
                    diff_amount: correct_amount_d,
                },
            ]
        );
    }

    #[test]
    fn test_get_restaking_msgs() {
        let surplus_validators = Vec::from([
            ValidatorDelegation {
                address: "a".to_string(),
                delegation_diff: DelegationDiff::Surplus,
                diff_amount: Uint128::new(200),
            },
            ValidatorDelegation {
                address: "b".to_string(),
                delegation_diff: DelegationDiff::Surplus,
                diff_amount: Uint128::new(900),
            },
        ]);
        let deficient_validators = vec![
            ValidatorDelegation {
                address: "c".to_string(),
                delegation_diff: DelegationDiff::Deficit,
                diff_amount: Uint128::new(1000),
            },
            ValidatorDelegation {
                address: "d".to_string(),
                delegation_diff: DelegationDiff::Deficit,
                diff_amount: Uint128::new(500),
            },
        ];
        let msgs = get_restaking_msgs(
            surplus_validators,
            deficient_validators,
            "denom".to_string(),
        );
        assert!(msgs.len() > 0);
        let zero_redelegate = msgs.iter().find(|msg| {
            if let CosmosMsg::Staking(StakingMsg::Redelegate {
                src_validator: _,
                dst_validator: _,
                amount,
            }) = msg
            {
                return amount.amount.is_zero();
            }
            false
        });
        // Amount cannot be zero
        assert!(zero_redelegate.is_none());

        let surplus_validators = Vec::from([
            ValidatorDelegation {
                address: "a".to_string(),
                delegation_diff: DelegationDiff::Surplus,
                diff_amount: Uint128::new(5000),
            },
            ValidatorDelegation {
                address: "b".to_string(),
                delegation_diff: DelegationDiff::Surplus,
                diff_amount: Uint128::new(5000),
            },
        ]);
        let deficient_validators = vec![
            ValidatorDelegation {
                address: "c".to_string(),
                delegation_diff: DelegationDiff::Deficit,
                diff_amount: Uint128::new(7500),
            },
            ValidatorDelegation {
                address: "d".to_string(),
                delegation_diff: DelegationDiff::Deficit,
                diff_amount: Uint128::new(2500),
            },
        ];
        let msgs = get_restaking_msgs(
            surplus_validators,
            deficient_validators,
            "denom".to_string(),
        );
        let mut net_amounts = msgs
            .into_iter()
            .filter_map(|msg| {
                if let CosmosMsg::Staking(StakingMsg::Redelegate {
                    src_validator: _,
                    dst_validator,
                    amount,
                }) = msg
                {
                    return Some((dst_validator, amount.amount.u128()));
                }
                None
            })
            .fold(HashMap::new(), |mut h, pair| {
                h.entry(pair.0)
                    .and_modify(|amount| *amount += pair.1)
                    .or_insert(pair.1);
                h
            })
            .into_iter()
            .collect::<Vec<_>>();
        net_amounts.sort_by_key(|a| a.1);
        // Should redelegate in totality
        assert_eq!(
            net_amounts,
            vec![("d".to_string(), 2500_u128), ("c".to_string(), 7500_u128)]
        );
    }

    #[test]
    fn test_get_delegate_to_validator_msgs() {
        let delegate_amount = Uint128::new(50000);
        let coin_denom = "denom".to_string();
        let validators = vec![
            Validator {
                address: "abc".to_string(),
                weight: 10,
            },
            Validator {
                address: "bcd".to_string(),
                weight: 20,
            },
        ];
        let msgs = get_delegate_to_validator_msgs(delegate_amount, coin_denom, validators);
        let amounts = msgs
            .into_iter()
            .filter_map(|msg| {
                if let CosmosMsg::Staking(StakingMsg::Delegate { validator, amount }) = msg {
                    return Some((validator, amount.amount));
                }
                None
            })
            .fold(HashMap::new(), |mut h, pair| {
                h.entry(pair.0)
                    .and_modify(|amount| *amount += pair.1)
                    .or_insert(pair.1);
                h
            });
        assert_approx_eq!(
            amounts.get("abc").unwrap() * Uint128::new(2),
            *amounts.get("bcd").unwrap(),
            "0.001"
        );
        assert_eq!(
            amounts.iter().map(|a| a.1).sum::<Uint128>(),
            delegate_amount
        );
    }
}
