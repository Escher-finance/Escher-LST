use crate::error::ContractError;
use crate::event::{BondEvent, UnbondEvent};
use crate::msg::{BondRewardsPayload, MintTokensPayload};
use crate::relay::send_to_evm;
use crate::reply::{BOND_WITHDRAW_REWARD_REPLY_ID, MINT_TOKENS_REPLY_ID};
use crate::state::{
    increment_tokens, unbond_history, UnbondHistory, PARAMETERS, STATE, VALIDATORS_REGISTRY,
};
use crate::token_factory_api::TokenFactoryMsg;
use crate::utils::{
    calculate_native_token_from_staking_token, calculate_staking_token_from_rate,
    calculate_undelegate_amount, get_actual_total_bonded, get_actual_total_reward,
    get_mock_total_reward, to_uint128,
};
use cosmwasm_std::{
    attr, to_json_binary, Addr, Coin, CosmosMsg, DecCoin, Decimal, DepsMut, DistributionMsg, Env,
    MessageInfo, Response, StakingMsg, StdResult, SubMsg, Timestamp, Uint128,
};

pub fn bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staker: Option<String>,
    amount: Option<Coin>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom;
    let sender = info.sender;
    let the_staker: String = staker.unwrap_or_else(|| "".to_string());

    let payment: Coin;
    // if amount is none it should use senders funds to delegate
    if amount.is_none() {
        // coin must have be sent along with transaction and it should be in underlying coin denom
        if info.funds.len() > 1usize {
            return Err(ContractError::InvalidAsset {});
        }

        // coin must have be sent along with transaction and it should be in underlying coin denom
        payment = Coin {
            amount: info
                .funds
                .iter()
                .find(|x| x.denom == coin_denom && x.amount > Uint128::zero())
                .ok_or_else(|| ContractError::NoAsset {})?
                .amount
                .clone(),
            denom: coin_denom.clone(),
        };
    } else {
        let the_amount = amount.unwrap().clone().amount;
        // if amount exists it should use this contract fund to delegate
        let lst_balance = deps
            .querier
            .query_balance(env.contract.address.to_string(), coin_denom.clone())?;

        if lst_balance.amount < the_amount.clone() {
            return Err(ContractError::NotEnoughFund {});
        }

        payment = Coin {
            amount: the_amount,
            denom: coin_denom.clone(),
        };
    }

    let total_validators = Uint128::from(validators_reg.validators.len() as u32);

    let delegate_amount = payment.amount / total_validators;
    let remaining_amount = payment.amount % total_validators;

    let mut msgs: Vec<CosmosMsg<TokenFactoryMsg>> = vec![];
    for (pos, validator) in validators_reg.validators.iter().enumerate() {
        let amount = Coin {
            amount: delegate_amount.clone(),
            denom: coin_denom.to_string(),
        };
        let mut staking_msg: CosmosMsg<TokenFactoryMsg> =
            CosmosMsg::Staking(StakingMsg::Delegate {
                validator: validator.address.to_string(),
                amount,
            });

        if pos == 0 {
            let amount = Coin {
                amount: delegate_amount + remaining_amount,
                denom: coin_denom.to_string(),
            };
            staking_msg = CosmosMsg::Staking(StakingMsg::Delegate {
                validator: validator.address.to_string(),
                amount,
            });
        }
        msgs.push(staking_msg.into());
    }

    let delegator = env.contract.address;
    let validators_list: Vec<String> = validators_reg
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();

    // logic to mint token and update the supply and total_bond_amount
    let mut state = STATE.load(deps.storage)?;
    let total_bond_amount: Uint128;
    let delegated_amount = get_actual_total_bonded(
        deps.querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list.clone(),
    );

    if !cfg!(test) {
        state.total_delegated_amount = delegated_amount;
        let reward = get_actual_total_reward(
            deps.querier,
            delegator.to_string(),
            coin_denom.clone(),
            validators_list,
        )?;

        total_bond_amount = delegated_amount + reward;
    } else {
        total_bond_amount = get_mock_total_reward(state.total_bond_amount);
    }

    let mut current_exchange_rate = state.exchange_rate;

    if !total_bond_amount.is_zero() && !state.total_lst_supply.is_zero() {
        current_exchange_rate = Decimal::from_ratio(total_bond_amount, state.total_lst_supply);
    }

    let mint_amount = calculate_staking_token_from_rate(payment.amount, current_exchange_rate);

    let total_lst_supply = state.total_lst_supply;

    // create bond event here
    let bond_event = BondEvent(
        sender.to_string(),
        the_staker.clone(),
        payment.amount.clone(),
        delegated_amount.clone(),
        total_bond_amount.clone(),
        total_lst_supply,
        current_exchange_rate,
    );

    // after update exchange rate we update the state
    state.bond_counter = state.bond_counter + 1;
    state.total_bond_amount = total_bond_amount + payment.amount;
    state.total_lst_supply = total_lst_supply + mint_amount;
    state.total_delegated_amount += payment.amount;
    state.last_bond_time = env.block.time.nanos();
    state.update_exchange_rate();

    STATE.save(deps.storage, &state)?;

    // Start to mint according to staked token
    let mint_msg = TokenFactoryMsg::MintTokens {
        denom: params.liquidstaking_denom.clone(),
        amount: mint_amount,
        mint_to_address: delegator.to_string(),
    };

    let mut sub_msgs: Vec<SubMsg<TokenFactoryMsg>> = vec![];
    if !cfg!(test) {
        let payload = MintTokensPayload {
            sender: sender.to_string(),
            staker: the_staker.clone(),
            amount: mint_amount,
        };
        let payload_bin = to_json_binary(&payload)?;

        let sub_msg: SubMsg<TokenFactoryMsg> = SubMsg::reply_always(mint_msg, MINT_TOKENS_REPLY_ID)
            .with_payload(payload_bin)
            .into();
        sub_msgs.push(sub_msg);
    }

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_messages(msgs)
        .add_submessages(sub_msgs)
        .add_event(bond_event)
        .add_attributes(vec![
            attr("action", "mint"),
            attr("from", sender),
            attr("payment_amount", payment.amount.to_string()),
            attr("delegate_amount", delegate_amount.to_string()),
            attr("remaining_amount", remaining_amount.to_string()),
            attr("denom", coin_denom.to_string()),
            attr("minted", mint_amount),
            attr("exchange_rate", state.exchange_rate.to_string()),
        ]);

    Ok(res)
}

pub fn unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staker: Option<String>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom;
    let liquidstaking_denom = params.liquidstaking_denom;
    let sender = info.sender.to_string();
    let the_staker: String = staker.unwrap_or_else(|| "".to_string());

    // coin must have be sent along with transaction and it should be in liquid staking coin denom
    if info.funds.len() > 1usize {
        return Err(ContractError::InvalidAsset {});
    }

    // coin must have be sent along with transaction and it should be in liquid staking coin denom
    let payment = info
        .funds
        .iter()
        .find(|x| x.denom == liquidstaking_denom && x.amount > Uint128::zero())
        .ok_or_else(|| ContractError::NoAsset {})?;

    let total_validators = Uint128::from(validators_reg.validators.len() as u32);
    let validators_list: Vec<String> = validators_reg
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();

    let mut state = STATE.load(deps.storage)?;

    let delegator = env.contract.address;
    let delegated_amount = get_actual_total_bonded(
        deps.querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list.clone(),
    );
    state.total_delegated_amount = delegated_amount;
    let reward = get_actual_total_reward(
        deps.querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list,
    )?;

    let total_bond_amount = delegated_amount + reward;

    if total_bond_amount.is_zero() || state.total_lst_supply.is_zero() {
        return Err(ContractError::ZeroSupplyOrDelegatedAmount {});
    }
    let current_exchange_rate = Decimal::from_ratio(total_bond_amount, state.total_lst_supply);

    // calculate native token undelegated amount from liquid staking payment amount
    let native_token_unbond_amount =
        calculate_native_token_from_staking_token(payment.amount.clone(), current_exchange_rate);

    let mut undelegate_amount = calculate_undelegate_amount(
        native_token_unbond_amount,
        delegated_amount,
        total_bond_amount,
    );
    undelegate_amount = undelegate_amount / total_validators;

    let remaining_amount = undelegate_amount % total_validators;

    let mut msgs: Vec<CosmosMsg<TokenFactoryMsg>> = vec![];
    for (pos, validator) in validators_reg.validators.iter().enumerate() {
        let amount = Coin {
            amount: undelegate_amount.clone(),
            denom: coin_denom.to_string(),
        };
        let mut undelegate_staking_msg: CosmosMsg<TokenFactoryMsg> =
            CosmosMsg::Staking(StakingMsg::Undelegate {
                validator: validator.address.to_string(),
                amount,
            });

        if pos == 0 {
            let amount = Coin {
                amount: undelegate_amount + remaining_amount,
                denom: coin_denom.to_string(),
            };
            undelegate_staking_msg = CosmosMsg::Staking(StakingMsg::Undelegate {
                validator: validator.address.to_string(),
                amount,
            });
        }
        msgs.push(undelegate_staking_msg.into());
    }

    let unbond_event = UnbondEvent(
        sender.clone(),
        the_staker.clone(),
        payment.amount.clone(),
        delegated_amount.clone(),
        total_bond_amount.clone(),
        state.total_lst_supply.clone(),
        current_exchange_rate,
    );

    let burn_msg = TokenFactoryMsg::BurnTokens {
        denom: liquidstaking_denom.clone(),
        amount: payment.amount,
        burn_from_address: delegator.to_string(),
    };

    msgs.push(burn_msg.into());

    let id = increment_tokens(deps.storage).unwrap();
    let unbond_amount = Coin {
        amount: payment.amount.clone(),
        denom: liquidstaking_denom.clone(),
    };

    let history = UnbondHistory {
        id,
        sender: sender.clone(),
        staker: the_staker.clone(),
        amount: unbond_amount,
        exchange_rate: current_exchange_rate,
        unbond_time: env.block.time,
        released: false,
        released_time: Timestamp::from_nanos(000_000_000),
    };
    unbond_history().save(deps.storage, id, &history)?;

    // // update total bond, supply and exchange rate here
    state.total_bond_amount = total_bond_amount - native_token_unbond_amount;
    state.total_lst_supply = state.total_lst_supply - payment.amount;
    state.total_delegated_amount = delegated_amount - undelegate_amount;
    state.update_exchange_rate();
    STATE.save(deps.storage, &state)?;

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_messages(msgs)
        .add_event(unbond_event)
        .add_attributes(vec![
            attr("action", "undelegate"),
            attr("sender", sender),
            attr("staker", the_staker),
            attr("current_exchange_rate", current_exchange_rate.to_string()),
            attr(
                "native_token_unbond_amount",
                native_token_unbond_amount.to_string(),
            ),
            attr("undelegate_amount", undelegate_amount.to_string()),
            attr("delegated_amount", state.total_delegated_amount.to_string()),
            attr("total_bond_amount", state.total_bond_amount.to_string()),
            attr("total_lst_supply", state.total_lst_supply.to_string()),
            attr("remaining_amount", remaining_amount.to_string()),
            attr("denom", coin_denom.to_string()),
        ]);

    Ok(res)
}

pub fn transfer(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    amount: Coin,
    receiver: Addr,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;

    let funds = vec![amount.clone()];
    let msg: CosmosMsg<TokenFactoryMsg> = send_to_evm(
        params.ucs01_relay_contract,
        params.ucs01_channel,
        receiver.to_string(),
        funds,
    )?
    .into();

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_message(msg)
        .add_attribute("action", "transfer")
        .add_attribute("receiver", receiver.to_string())
        .add_attribute("amount", amount.amount.to_string())
        .add_attribute("denom", amount.denom);
    Ok(res)
}

pub fn set_token_admin(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
    new_admin: Addr,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let msg = TokenFactoryMsg::ChangeAdmin {
        denom: denom.clone(),
        new_admin_address: new_admin.to_string(),
    };

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_message(msg)
        .add_attribute("action", "set_token_admin")
        .add_attribute("denom", denom)
        .add_attribute("admin", new_admin.to_string());
    Ok(res)
}

pub fn bond_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom;
    let sender = info.sender;
    let delegator = env.contract.address;
    let mut sub_msgs: Vec<SubMsg<TokenFactoryMsg>> = vec![];

    let mut attrs = vec![attr("action", "bond_rewards"), attr("from", sender)];

    for validator in validators_reg.validators {
        let result: StdResult<Vec<DecCoin>> = deps
            .querier
            .query_delegation_rewards(delegator.clone(), validator.address.to_string());

        let mut payload = BondRewardsPayload {
            validator: validator.address.clone(),
            amount: Uint128::new(0),
        };

        if result.is_ok() {
            for reward in result.unwrap() {
                if reward.denom == coin_denom {
                    payload.amount = to_uint128(reward.amount.to_uint_floor())?;
                }
            }
        }

        let withdraw_reward_msg: CosmosMsg<TokenFactoryMsg> =
            CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
                validator: validator.address.to_string(),
            });

        let payload_bin = to_json_binary(&payload)?;

        let sub_msg: SubMsg<TokenFactoryMsg> =
            SubMsg::reply_always(withdraw_reward_msg, BOND_WITHDRAW_REWARD_REPLY_ID)
                .with_payload(payload_bin)
                .into();
        sub_msgs.push(sub_msg);

        attrs.push(attr("amount", payload.amount.to_string()));
    }

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_submessages(sub_msgs)
        .add_attributes(attrs);

    Ok(res)
}

pub fn reset(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut state = STATE.load(deps.storage)?;
    state.bond_counter = 0;
    state.total_bond_amount = Uint128::new(0);
    state.total_lst_supply = Uint128::new(0);
    state.total_delegated_amount = Uint128::new(0);
    state.last_bond_time = 0;
    state.exchange_rate = Decimal::one();
    STATE.save(deps.storage, &state)?;

    unbond_history().clear(deps.storage);

    let res: Response<TokenFactoryMsg> = Response::new().add_attribute("action", "reset");

    Ok(res)
}

/// Update the ownership of the contract.
#[allow(clippy::needless_pass_by_value)]
pub fn update_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action: cw_ownable::Action,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    if action == cw_ownable::Action::RenounceOwnership {
        return Err(ContractError::OwnershipCannotBeRenounced);
    };

    cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;

    let res: Response<TokenFactoryMsg> =
        Response::new().add_attribute("action", "update_ownership");

    Ok(res)
}

pub fn set_parameters(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    underlying_coin_denom: Option<String>,
    liquidstaking_denom: Option<String>,
    ucs01_channel: Option<String>,
    ucs01_relay_contract: Option<String>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut params = PARAMETERS.load(deps.storage)?;

    params.underlying_coin_denom = underlying_coin_denom
        .clone()
        .unwrap_or_else(|| params.underlying_coin_denom);
    params.liquidstaking_denom = liquidstaking_denom
        .clone()
        .unwrap_or_else(|| params.liquidstaking_denom);
    params.ucs01_channel = ucs01_channel
        .clone()
        .unwrap_or_else(|| params.ucs01_channel);
    params.ucs01_relay_contract = ucs01_relay_contract
        .clone()
        .unwrap_or_else(|| params.ucs01_relay_contract);

    PARAMETERS.save(deps.storage, &params)?;

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_attribute("action", "set_parameters")
        .add_attribute(
            "liquidstaking_denom",
            liquidstaking_denom.unwrap_or_else(|| "".to_string()),
        )
        .add_attribute(
            "underlying_coin_denom",
            underlying_coin_denom.unwrap_or_else(|| "".to_string()),
        )
        .add_attribute(
            "ucs01_channel",
            ucs01_channel.unwrap_or_else(|| "".to_string()),
        )
        .add_attribute(
            "ucs01_relay_contract",
            ucs01_relay_contract.unwrap_or_else(|| "".to_string()),
        );

    Ok(res)
}
