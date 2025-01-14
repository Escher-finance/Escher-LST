use std::str::FromStr;

use crate::error::ContractError;
use crate::event::{
    BondEvent, ProcessRewardsEvent, ProcessUnbondingEvent, UnbondEvent, UpdateValidatorsEvent,
};
use crate::msg::{BondRewardsPayload, MintTokensPayload};
use crate::reply::{
    BOND_WITHDRAW_REWARD_REPLY_ID, MINT_CW20_TOKENS_REPLY_ID, MINT_TOKENS_REPLY_ID,
};
use crate::state::{
    increment_tokens, unbond_record, Parameters, UnbondRecord, Validator, PARAMETERS, STATE,
    VALIDATORS_REGISTRY,
};
use crate::token_factory_api::TokenFactoryMsg;
use crate::utils::{
    self, calculate_native_token_from_staking_token, calculate_staking_token_from_rate,
    get_actual_total_delegated, get_actual_total_reward, get_mock_total_reward, to_uint128,
};
use cosmwasm_std::{
    attr, to_json_binary, Addr, Attribute, BankMsg, Binary, Coin, CosmosMsg, DecCoin, Decimal,
    DepsMut, DistributionMsg, Env, MessageInfo, Response, StakingMsg, StdResult, SubMsg, Timestamp,
    Uint128, Uint256, WasmMsg,
};
use unionlabs_primitives::{Bytes, FixedBytes};

pub fn bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staker: Option<String>,
    amount: Option<Coin>,
    salt: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom.clone();
    let sender = info.sender;
    let the_staker: String = staker.unwrap_or_else(|| sender.to_string());
    let delegator = env.contract.address;

    let payment: Coin;
    // if amount is none it should use senders funds to delegate and this assume the
    // transaction happen on same chain, original staker/sender to contract is on same cosmos based chain
    // so it will use sender funds to do bond
    if amount.is_none() {
        // coin must have be sent along with transaction and it should be in underlying coin denom
        if info.funds.len() > 1usize {
            return Err(ContractError::InvalidAsset {});
        }

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
        // if amount exists it should use this contract fund to delegate
        // and this only can be called by "owner" or backend script using owner sign to do bond on behalf of original staker
        cw_ownable::assert_owner(deps.storage, &sender)?;

        let the_amount = amount.unwrap().clone().amount;

        let lst_balance = deps
            .querier
            .query_balance(delegator.to_string(), coin_denom.clone())?;

        if lst_balance.amount < the_amount.clone() {
            return Err(ContractError::NotEnoughFund {});
        }

        payment = Coin {
            amount: the_amount,
            denom: coin_denom.clone(),
        };
    }

    let msgs = utils::get_delegate_to_validator_msgs(
        payment.amount,
        params.underlying_coin_denom.to_string(),
        validators_reg.validators.clone(),
    );

    let validators_list: Vec<String> = validators_reg
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();

    // logic to mint token and update the supply and total_bond_amount
    let mut state = STATE.load(deps.storage)?;

    let delegated_amount = get_actual_total_delegated(
        deps.querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list.clone(),
    );

    let total_bond_amount: Uint128;
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

    let mut sub_msgs: Vec<SubMsg<TokenFactoryMsg>> = vec![];
    let payload = MintTokensPayload {
        sender: sender.to_string(),
        staker: the_staker.clone(),
        amount: mint_amount,
        salt,
    };
    let payload_bin = to_json_binary(&payload)?;

    if !cfg!(test) {
        // Start to mint according to staked token only if it is not test
        let sub_msg: SubMsg<TokenFactoryMsg> = get_staked_token_submsg(
            delegator.to_string(),
            the_staker.to_string(),
            mint_amount,
            params.liquidstaking_denom.clone(),
            payload_bin,
            params,
        );
        sub_msgs.push(sub_msg);
    }

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_messages(msgs)
        .add_submessages(sub_msgs)
        .add_event(bond_event)
        .add_attributes(vec![
            attr("action", "mint"),
            attr("from", sender),
            attr("staker", the_staker.to_string()),
            attr("payment_amount", payment.amount.to_string()),
            attr("denom", coin_denom.to_string()),
            attr("minted", mint_amount),
            attr("exchange_rate", state.exchange_rate.to_string()),
        ]);

    Ok(res)
}

#[cfg(not(nonunion))]
fn get_staked_token_submsg(
    delegator: String,
    _staker: String,
    mint_amount: Uint128,
    liquidstaking_denom: String,
    payload_bin: Binary,
    _params: Parameters,
) -> SubMsg<TokenFactoryMsg> {
    let mint_msg = TokenFactoryMsg::MintTokens {
        denom: liquidstaking_denom,
        amount: mint_amount,
        mint_to_address: delegator.to_string(),
    };

    let sub_msg: SubMsg<TokenFactoryMsg> = SubMsg::reply_always(mint_msg, MINT_TOKENS_REPLY_ID)
        .with_payload(payload_bin)
        .into();
    sub_msg
}

#[cfg(nonunion)]
fn get_staked_token_submsg(
    _delegator: String,
    staker: String,
    mint_amount: Uint128,
    _liquidstaking_denom: String,
    payload_bin: Binary,
    params: Parameters,
) -> SubMsg<TokenFactoryMsg> {
    let mint = cw20::Cw20ExecuteMsg::Mint {
        recipient: staker,
        amount: mint_amount,
    };
    let mint_bin = to_json_binary(&mint).unwrap();
    let mint_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: params.cw20_address.unwrap().to_string(),
        msg: mint_bin,
        funds: vec![],
    });
    let sub_msg: SubMsg<TokenFactoryMsg> =
        SubMsg::reply_always(mint_msg, MINT_CW20_TOKENS_REPLY_ID)
            .with_payload(payload_bin)
            .into();
    sub_msg
}

#[cfg(not(nonunion))]
fn burn_token(
    delegator: String,
    amount: Uint128,
    liquidstaking_denom: String,
    _cw20_address: Option<Addr>,
) -> CosmosMsg<TokenFactoryMsg> {
    let burn_msg = utils::get_burn_msg(liquidstaking_denom.clone(), amount, delegator.to_string());
    let msg: CosmosMsg<TokenFactoryMsg> = burn_msg.into();
    msg
}

#[cfg(nonunion)]
fn burn_token(
    _delegator: String,
    amount: Uint128,
    _liquidstaking_denom: String,
    cw20_address: Option<Addr>,
) -> CosmosMsg<TokenFactoryMsg> {
    let execute_burn = cw20::Cw20ExecuteMsg::Burn { amount };
    let burn_bin = to_json_binary(&execute_burn).unwrap();
    let burn_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cw20_address.unwrap().to_string(),
        msg: burn_bin,
        funds: vec![],
    });
    let msg: CosmosMsg<TokenFactoryMsg> = burn_msg.into();
    msg
}

pub fn unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staker: Option<String>,
    amount: Option<Uint128>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom;
    let liquidstaking_denom = params.liquidstaking_denom;
    let sender = info.sender.to_string();
    let the_staker: String = staker.unwrap_or_else(|| sender.to_string());
    let delegator = env.contract.address;

    let unbond_amount: Uint128;
    if cfg!(not(nonunion)) {
        // this will handle union chain
        // coin must be sent along with transaction and it should be in liquid staking coin denom
        if info.funds.len() > 1usize {
            return Err(ContractError::InvalidAsset {});
        }

        let payment = info
            .funds
            .iter()
            .find(|x| x.denom == liquidstaking_denom && x.amount > Uint128::zero())
            .ok_or_else(|| ContractError::NoAsset {})?;

        unbond_amount = payment.amount;
    } else {
        // this will handle non union chain
        // need to find staked token balance of sender
        unbond_amount = amount.unwrap();
        let msg = cw20::Cw20QueryMsg::Balance {
            address: delegator.to_string(),
        };

        let balance: cw20::BalanceResponse = deps
            .querier
            .query_wasm_smart(params.cw20_address.clone().unwrap(), &msg)?;

        if balance.balance < unbond_amount {
            return Err(ContractError::NotEnoughAvailableFund {});
        }
    }

    let validators_list: Vec<String> = validators_reg
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();

    let mut state = STATE.load(deps.storage)?;

    let delegated_amount = get_actual_total_delegated(
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

    // calculate how much native token undelegated amount from staked token amount base on current exchange rate
    let undelegate_amount: Uint128 =
        calculate_native_token_from_staking_token(unbond_amount.clone(), current_exchange_rate);

    let mut msgs: Vec<CosmosMsg<TokenFactoryMsg>> = vec![];

    if delegated_amount < undelegate_amount {
        // throw error
        return Err(ContractError::NotEnoughAvailableFund {}); // this error only happen on development or sole staker
    }
    let (undelegate_msgs, undelegations) = utils::get_undelegate_from_validator_msgs(
        undelegate_amount,
        coin_denom.clone(),
        validators_reg.validators,
    );
    msgs.extend(undelegate_msgs);

    let burn_msg = burn_token(
        delegator.to_string(),
        unbond_amount,
        liquidstaking_denom.clone(),
        params.cw20_address,
    );
    msgs.push(burn_msg.into());

    let unbond_coin = Coin {
        amount: unbond_amount.clone(),
        denom: liquidstaking_denom.clone(),
    };
    let id: u64 = increment_tokens(deps.storage).unwrap();
    let history = UnbondRecord {
        id,
        height: env.block.height,
        sender: sender.clone(),
        staker: the_staker.clone(),
        amount: unbond_coin,
        exchange_rate: current_exchange_rate,
        undelegate_amount: Coin {
            denom: coin_denom.clone(),
            amount: undelegate_amount,
        },
        undelegations,
        created: env.block.time,
        completion: env.block.time.plus_seconds(params.unbonding_time),
        released: false,
        released_time: Timestamp::from_nanos(000_000_000),
    };
    unbond_record().save(deps.storage, id, &history)?;

    // // update total bond, supply and exchange rate here
    state.total_bond_amount = total_bond_amount - undelegate_amount;
    state.total_lst_supply = state.total_lst_supply - unbond_amount;
    state.total_delegated_amount = delegated_amount - undelegate_amount;
    state.update_exchange_rate();
    STATE.save(deps.storage, &state)?;

    let unbond_event = UnbondEvent(
        sender.clone(),
        the_staker.clone(),
        unbond_amount.clone(),
        undelegate_amount.clone(),
        state.total_delegated_amount.clone(),
        total_bond_amount.clone(),
        state.total_lst_supply.clone(),
        current_exchange_rate,
    );

    let attrs = get_unbond_attrs(
        sender,
        the_staker,
        current_exchange_rate.to_string(),
        unbond_amount.to_string(),
        undelegate_amount.to_string(),
        state.total_delegated_amount.to_string(),
        state.total_bond_amount.to_string(),
        state.total_lst_supply.to_string(),
        coin_denom.clone(),
    );

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_messages(msgs)
        .add_event(unbond_event)
        .add_attributes(attrs);

    Ok(res)
}

pub fn redelegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;

    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom.clone();

    // make sure sender is the rewards contract
    if params.reward_address != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let delegator = env.contract.address;

    if info.funds.len() > 1usize {
        return Err(ContractError::InvalidAsset {});
    }

    let payment = Coin {
        amount: info
            .funds
            .iter()
            .find(|x| x.denom == coin_denom && x.amount > Uint128::zero())
            .ok_or_else(|| ContractError::NoAsset {})?
            .amount
            .clone(),
        denom: coin_denom.clone(),
    };

    let msgs = utils::get_delegate_to_validator_msgs(
        payment.amount,
        params.underlying_coin_denom.to_string(),
        validators_reg.validators.clone(),
    );

    let validators_list: Vec<String> = validators_reg
        .validators
        .iter()
        .map(|v| v.address.clone())
        .collect();

    // logic to mint token and update the supply and total_bond_amount
    let mut state = STATE.load(deps.storage)?;

    let delegated_amount = get_actual_total_delegated(
        deps.querier,
        delegator.to_string(),
        coin_denom.clone(),
        validators_list.clone(),
    );

    let total_bond_amount: Uint128;
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

    // after update exchange rate we update the state
    state.bond_counter = state.bond_counter + 1;
    state.total_bond_amount = total_bond_amount + payment.amount;
    state.total_lst_supply = total_lst_supply + mint_amount;
    state.total_delegated_amount += payment.amount;
    state.last_bond_time = env.block.time.nanos();
    state.update_exchange_rate();

    STATE.save(deps.storage, &state)?;

    let res: Response<TokenFactoryMsg> = Response::new().add_messages(msgs).add_attributes(vec![
        attr("action", "redelegate"),
        attr("from", info.sender.to_string()),
        attr("payment_amount", payment.amount.to_string()),
        attr("denom", coin_denom.to_string()),
        attr("exchange_rate", state.exchange_rate.to_string()),
    ]);

    Ok(res)
}

fn get_unbond_attrs(
    sender: String,
    the_staker: String,
    current_exchange_rate: String,
    unbond_amount: String,
    undelegate_amount: String,
    total_delegated_amount: String,
    total_bond_amount: String,
    total_lst_supply: String,
    coin_denom: String,
) -> Vec<Attribute> {
    return vec![
        attr("action", "unbond"),
        attr("sender", sender),
        attr("staker", the_staker),
        attr("current_exchange_rate", current_exchange_rate),
        attr("unbond_amount", unbond_amount),
        attr("undelegate_amount", undelegate_amount),
        attr("total_delegated_amount", total_delegated_amount),
        attr("total_bond_amount", total_bond_amount),
        attr("total_lst_supply", total_lst_supply),
        attr("denom", coin_denom),
    ];
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

pub fn process_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom;
    let sender = info.sender;
    let delegator = env.contract.address;
    let mut sub_msgs: Vec<SubMsg<TokenFactoryMsg>> = vec![];

    let mut attrs = vec![attr("action", "process_rewards"), attr("from", sender)];

    let mut total_amount: Uint128 = Uint128::zero();

    for validator in validators_reg.validators {
        let result: StdResult<Vec<DecCoin>> = deps
            .querier
            .query_delegation_rewards(delegator.clone(), validator.address.to_string());

        let mut payload = BondRewardsPayload {
            validator: validator.address.clone(),
            amount: Uint128::zero(),
        };

        if result.is_ok() {
            for reward in result.unwrap() {
                if reward.denom == coin_denom {
                    payload.amount = to_uint128(reward.amount.to_uint_floor())?;
                    total_amount += payload.amount;
                }
            }
        }

        let withdraw_reward_msg: CosmosMsg<TokenFactoryMsg> =
            CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
                validator: validator.address.to_string(),
            });

        if payload.amount != Uint128::zero() {
            let payload_bin = to_json_binary(&payload)?;

            let sub_msg: SubMsg<TokenFactoryMsg> =
                SubMsg::reply_always(withdraw_reward_msg, BOND_WITHDRAW_REWARD_REPLY_ID)
                    .with_payload(payload_bin)
                    .into();
            sub_msgs.push(sub_msg);
        }
        attrs.push(attr("amount", payload.amount.to_string()));
    }

    let ev = ProcessRewardsEvent(total_amount);
    let res: Response<TokenFactoryMsg> = Response::new()
        .add_attributes(attrs)
        .add_event(ev)
        .add_submessages(sub_msgs);

    Ok(res)
}

pub fn reset(
    deps: DepsMut,
    env: Env,
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

    unbond_record().clear(deps.storage);
    let msgs = get_unbond_all_messages(deps, env.contract.address)?;

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_messages(msgs)
        .add_attribute("action", "reset");

    Ok(res)
}

pub fn move_to_reward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let params = PARAMETERS.load(deps.storage)?;

    let balance = deps
        .querier
        .query_balance(env.contract.address, params.underlying_coin_denom)?;

    if balance.amount < Uint128::one() {
        return Err(ContractError::NotEnoughAvailableFund {});
    }

    let bank_msg = BankMsg::Send {
        to_address: params.reward_address.to_string(),
        amount: vec![balance.clone()],
    };
    let msg: CosmosMsg<TokenFactoryMsg> = CosmosMsg::Bank(bank_msg);

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_message(msg)
        .add_attribute("action", "move_to_reward_contract")
        .add_attribute("amount", balance.amount.to_string());

    Ok(res)
}

pub fn get_unbond_all_messages(
    deps: DepsMut,
    delegator: Addr,
) -> Result<Vec<CosmosMsg<TokenFactoryMsg>>, ContractError> {
    let delegations_resp = deps.querier.query_all_delegations(delegator);
    let params = PARAMETERS.load(deps.storage)?;
    let denom = params.underlying_coin_denom;

    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let mut msgs: Vec<CosmosMsg<TokenFactoryMsg>> = vec![];
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
        let undelegate_staking_msg: CosmosMsg<TokenFactoryMsg> =
            CosmosMsg::Staking(StakingMsg::Undelegate {
                validator: validator.address.to_string(),
                amount,
            });

        msgs.push(undelegate_staking_msg.into());
    }

    Ok(msgs)
}

/// Update the ownership of the contract.
#[allow(clippy::needless_pass_by_value)]
pub fn update_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action: cw_ownable::Action,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
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
    ucs03_channel: Option<String>,
    ucs03_relay_contract: Option<String>,
    unbonding_time: Option<u64>,
    cw20_address: Option<Addr>,
    reward_address: Option<Addr>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut params = PARAMETERS.load(deps.storage)?;

    params.underlying_coin_denom = underlying_coin_denom
        .clone()
        .unwrap_or_else(|| params.underlying_coin_denom);
    params.liquidstaking_denom = liquidstaking_denom
        .clone()
        .unwrap_or_else(|| params.liquidstaking_denom);
    params.ucs03_channel = ucs03_channel
        .clone()
        .unwrap_or_else(|| params.ucs03_channel);
    params.ucs03_relay_contract = ucs03_relay_contract
        .clone()
        .unwrap_or_else(|| params.ucs03_relay_contract);
    params.unbonding_time = unbonding_time
        .clone()
        .unwrap_or_else(|| params.unbonding_time);
    params.cw20_address = cw20_address.clone();
    params.reward_address = reward_address
        .clone()
        .unwrap_or_else(|| params.reward_address);

    let cw20_addr_string = match cw20_address {
        Some(cw20) => cw20.to_string(),
        None => "".to_string(),
    };
    let mut reward_address_str = "".to_string();

    let mut msgs: Vec<CosmosMsg<TokenFactoryMsg>> = vec![];

    if reward_address.is_some() {
        let msg: CosmosMsg<TokenFactoryMsg> =
            CosmosMsg::Distribution(DistributionMsg::SetWithdrawAddress {
                address: reward_address.clone().unwrap().to_string(),
            });
        msgs.push(msg);
        reward_address_str = reward_address.unwrap().to_string();
    }
    PARAMETERS.save(deps.storage, &params)?;

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_messages(msgs)
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
            "ucs03_channel",
            ucs03_channel.unwrap_or_else(|| "".to_string()),
        )
        .add_attribute(
            "ucs03_relay_contract",
            ucs03_relay_contract.unwrap_or_else(|| "".to_string()),
        )
        .add_attribute("cw20_address", cw20_addr_string)
        .add_attribute("reward_address", reward_address_str);

    Ok(res)
}

pub fn process_unbonding(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: u64,
    salt: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let params: crate::state::Parameters = PARAMETERS.load(deps.storage)?;
    let mut unbond_rec: crate::state::UnbondRecord = unbond_record().load(deps.storage, id)?;

    // query the undelegate_amount in contract balance
    let contract_address = env.clone().contract.address;
    let balance = deps
        .querier
        .query_balance(contract_address, params.underlying_coin_denom.clone())?;

    if balance.amount < unbond_rec.undelegate_amount.amount {
        return Err(ContractError::NotEnoughAvailableFund {});
    }

    // if exists, send to staker (it can be on same chain or other chain like evm/bera)
    let msg: CosmosMsg<TokenFactoryMsg> = {
        if unbond_rec.staker != unbond_rec.sender {
            let funds = vec![unbond_rec.undelegate_amount.clone()];
            let wasm_msg = utils::send_to_evm(
                env.clone(),
                params.ucs03_relay_contract.as_str().into(),
                params.ucs03_channel.parse::<u32>().unwrap(),
                Bytes::from_str(unbond_rec.staker.as_str()).unwrap(),
                params.underlying_coin_denom.clone(),
                unbond_rec.amount.amount,
                Bytes::from_str(params.underlying_coin_denom.as_str()).unwrap(),
                Uint256::zero(),
                funds,
                FixedBytes::from_str(salt.as_str()).unwrap(),
            )?;
            let msg: CosmosMsg<TokenFactoryMsg> = CosmosMsg::Wasm(wasm_msg);
            msg
        } else {
            let bank_msg = BankMsg::Send {
                to_address: unbond_rec.staker.clone(),
                amount: vec![unbond_rec.undelegate_amount.clone()],
            };
            let msg: CosmosMsg<TokenFactoryMsg> = CosmosMsg::Bank(bank_msg);
            msg
        }
    };

    let ev = ProcessUnbondingEvent(
        unbond_rec.staker.to_string(),
        unbond_rec.undelegate_amount.amount.clone(),
        unbond_rec.undelegate_amount.denom.clone(),
    );

    // set unbonding record to be released
    unbond_rec.released = true;
    unbond_rec.released_time = env.block.time;
    unbond_record().save(deps.storage, unbond_rec.id, &unbond_rec)?;

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_message(msg)
        .add_event(ev)
        .add_attribute("action", "transfer_unbonding")
        .add_attribute("staker", unbond_rec.staker)
        .add_attribute("amount", unbond_rec.undelegate_amount.amount)
        .add_attribute("denom", unbond_rec.undelegate_amount.denom);

    Ok(res)
}

/// Update the ownership of the contract.
#[allow(clippy::needless_pass_by_value)]
pub fn update_validators(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    validators: Vec<Validator>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if validators.len() < 1 {
        return Err(ContractError::EmptyValidator {});
    }

    let mut validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let prev_validators = validators_reg.validators.clone();
    validators_reg.validators = validators.clone();
    VALIDATORS_REGISTRY.save(deps.storage, &validators_reg)?;

    let msgs: Vec<CosmosMsg<TokenFactoryMsg>> = adjust_validators_delegation(
        deps,
        env.contract.address,
        prev_validators.clone(),
        validators.clone(),
    )?;

    let event = UpdateValidatorsEvent(info.sender.to_string(), prev_validators, validators);
    let res: Response<TokenFactoryMsg> = Response::new().add_messages(msgs).add_event(event);
    Ok(res)
}

pub fn adjust_validators_delegation(
    deps: DepsMut,
    delegator: Addr,
    prev_validators: Vec<Validator>,
    validators: Vec<Validator>,
) -> Result<Vec<CosmosMsg<TokenFactoryMsg>>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let denom = params.underlying_coin_denom;

    let (validator_delegation_map, total_delegated_amount) =
        utils::get_validator_delegation_map_with_total_bond(
            deps,
            delegator.to_string(),
            prev_validators,
        )?;

    let correct_validator_delegation_map =
        utils::get_validator_delegation_map_base_on_weight(validators, total_delegated_amount)?;

    let (surplus_validators, deficient_validators) = utils::get_surplus_deficit_validators(
        validator_delegation_map,
        correct_validator_delegation_map,
    );

    let msgs: Vec<CosmosMsg<TokenFactoryMsg>> =
        utils::get_restaking_msgs(surplus_validators, deficient_validators, denom);

    Ok(msgs)
}

// let undelegate_staking_msg: CosmosMsg<TokenFactoryMsg> = CosmosMsg::Staking(undelegate_msg);
// msgs.push(undelegate_staking_msg);

#[cfg(test)]
#[test]
fn validator_restaking_adjustment() {
    use std::collections::HashMap;

    let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();
    let mut correct_validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

    validator_delegation_map.insert("A".into(), Uint128::new(50000));
    validator_delegation_map.insert("B".into(), Uint128::new(50000));

    correct_validator_delegation_map.insert("B".into(), Uint128::new(30000));
    correct_validator_delegation_map.insert("C".into(), Uint128::new(30000));
    correct_validator_delegation_map.insert("D".into(), Uint128::new(40000));

    let (surplus, deficit) = utils::get_surplus_deficit_validators(
        validator_delegation_map,
        correct_validator_delegation_map,
    );

    let denom = "muno".to_string();
    let msgs = utils::get_restaking_msgs(surplus, deficit, denom);
    println!("msgs: {:?}", msgs);
}

#[test]
fn validator_restaking_adjustment_2() {
    use std::collections::HashMap;

    let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();
    let mut correct_validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

    validator_delegation_map.insert("A".into(), Uint128::new(50000));
    validator_delegation_map.insert("B".into(), Uint128::new(50000));
    validator_delegation_map.insert("C".into(), Uint128::new(50000));

    correct_validator_delegation_map.insert("B".into(), Uint128::new(75000));
    correct_validator_delegation_map.insert("C".into(), Uint128::new(75000));

    let (surplus, deficit) = utils::get_surplus_deficit_validators(
        validator_delegation_map,
        correct_validator_delegation_map,
    );

    let denom = "muno".to_string();
    let msgs = utils::get_restaking_msgs(surplus, deficit, denom);
    println!("msgs: {:?}", msgs);
}

#[test]
fn validator_restaking_adjustment_3() {
    use std::collections::HashMap;

    let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();
    let mut correct_validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

    validator_delegation_map.insert("A".into(), Uint128::new(30000));
    validator_delegation_map.insert("B".into(), Uint128::new(40000));
    validator_delegation_map.insert("C".into(), Uint128::new(30000));

    correct_validator_delegation_map.insert("B".into(), Uint128::new(25000));
    correct_validator_delegation_map.insert("C".into(), Uint128::new(25000));
    correct_validator_delegation_map.insert("D".into(), Uint128::new(50000));

    let (surplus, deficit) = utils::get_surplus_deficit_validators(
        validator_delegation_map,
        correct_validator_delegation_map,
    );

    let denom = "muno".to_string();
    let msgs = utils::get_restaking_msgs(surplus, deficit, denom);
    println!("\nmsgs: {:?}", msgs);
}

#[test]
fn validator_restaking_adjustment_4() {
    use std::collections::HashMap;

    let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();
    let mut correct_validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

    validator_delegation_map.insert("B".into(), Uint128::new(40000));
    validator_delegation_map.insert("C".into(), Uint128::new(30000));
    validator_delegation_map.insert("A".into(), Uint128::new(30000));

    correct_validator_delegation_map.insert("A".into(), Uint128::new(80000));
    correct_validator_delegation_map.insert("B".into(), Uint128::new(20000));

    let (surplus, deficit) = utils::get_surplus_deficit_validators(
        validator_delegation_map,
        correct_validator_delegation_map,
    );

    let denom = "muno".to_string();
    let msgs = utils::get_restaking_msgs(surplus, deficit, denom);
    println!("\nmsgs: {:?}", msgs);
}

#[test]
fn test_delegate_amount() {
    let weight: u32 = 1;
    let total_weight: u32 = 1;
    let ratio = Decimal::from_ratio(Uint128::from(weight), Uint128::from(total_weight));
    let amount = Uint128::from(10u32);
    let delegate_amount = utils::calculate_delegated_amount(amount, ratio);
    println!("delegate_amount: {}", delegate_amount);
}
