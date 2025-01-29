use std::str::FromStr;

use crate::error::ContractError;
use crate::event::{
    BondEvent, ProcessRewardsEvent, ProcessUnbondingEvent, UnbondEvent, UpdateValidatorsEvent,
};
use crate::msg::{BondRewardsPayload, ExecuteRewardMsg, MigrateMsg, ZkgmMessage};
use crate::reply::BOND_WITHDRAW_REWARD_REPLY_ID;
use crate::state::{unbond_record, Validator, LOG, PARAMETERS, STATE, VALIDATORS_REGISTRY};
use crate::token_factory_api::TokenFactoryMsg;
use crate::utils::{
    self, delegation::get_actual_total_delegated, delegation::get_actual_total_reward,
    delegation::get_mock_total_reward, delegation::to_uint128,
};
use cosmwasm_std::{
    attr, from_json, to_json_binary, Addr, Attribute, BankMsg, Coin, CosmosMsg, DecCoin, Decimal,
    DepsMut, DistributionMsg, Env, MessageInfo, Response, StdResult, SubMsg, Uint128, Uint256,
    WasmMsg,
};
use unionlabs_primitives::{Bytes, FixedBytes};

pub fn bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staker: Option<String>,
    amount: Option<Uint128>,
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
    // transaction happen on same chain directly as the original staker/sender to contract is on same cosmos based chain
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

        let the_amount = amount.unwrap();

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

    let (msgs, sub_msgs, bond_data) = utils::delegation::process_bond(
        deps.storage,
        deps.querier,
        sender.to_string(),
        the_staker.clone(),
        delegator,
        payment.amount,
        env.block.time.nanos(),
        params,
        validators_reg,
        salt,
    )?;

    // create bond event here
    let bond_event = BondEvent(
        sender.to_string(),
        the_staker.clone(),
        payment.amount.clone(),
        bond_data.delegated_amount.clone(),
        bond_data.total_bond_amount.clone(),
        bond_data.total_supply,
        bond_data.exchange_rate,
    );

    if bond_data.mint_amount == Uint128::zero() {
        return Err(ContractError::InvalidMintAmount {});
    }

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_messages(msgs)
        .add_submessages(sub_msgs)
        .add_event(bond_event)
        .add_attributes(vec![
            attr("action", "bond"),
            attr("from", sender),
            attr("staker", the_staker.to_string()),
            attr("bond_amount", payment.amount.to_string()),
            attr("denom", coin_denom.to_string()),
            attr("minted", bond_data.mint_amount),
            attr("exchange_rate", bond_data.exchange_rate.to_string()),
        ]);

    Ok(res)
}

pub fn zkgm_unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staker: String,
    amount: Uint128,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let sender = info.sender.clone();
    let delegator = env.contract.address.clone();

    let (msgs, unbond_data) = utils::delegation::process_unbond(
        env.clone(),
        deps.storage,
        deps.querier,
        sender.to_string(),
        staker.clone(),
        delegator,
        amount,
        params,
        validators_reg,
    )?;

    // create bond event here
    let unbond_event = UnbondEvent(
        sender.to_string(),
        staker,
        amount,
        unbond_data.undelegate_amount,
        unbond_data.delegated_amount,
        unbond_data.delegated_amount + unbond_data.reward,
        unbond_data.total_supply,
        unbond_data.exchange_rate,
    );

    LOG.save(
        deps.storage,
        &format!("{}: {:?}", env.block.time, unbond_event),
    )?;

    let res: Response<TokenFactoryMsg> = Response::new().add_messages(msgs).add_event(unbond_event);

    Ok(res)
}

pub fn zkgm_bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staker: String,
    amount: Uint128,
    salt: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom.clone();
    let sender = info.sender;
    let delegator = env.contract.address;

    let (msgs, sub_msgs, bond_data) = utils::delegation::process_bond(
        deps.storage,
        deps.querier,
        sender.to_string(),
        staker.clone(),
        delegator,
        amount,
        env.block.time.nanos(),
        params,
        validators_reg,
        salt,
    )?;

    // create bond event here
    let bond_event = BondEvent(
        sender.to_string(),
        staker.clone(),
        amount.clone(),
        bond_data.delegated_amount.clone(),
        bond_data.total_bond_amount.clone(),
        bond_data.total_supply,
        bond_data.exchange_rate,
    );

    LOG.save(
        deps.storage,
        &format!("{}: {:?}", env.block.time, bond_event),
    )?;

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_messages(msgs)
        .add_submessages(sub_msgs)
        .add_event(bond_event)
        .add_attributes(vec![
            attr("action", "bond"),
            attr("from", sender),
            attr("staker", staker.to_string()),
            attr("bond_amount", amount.to_string()),
            attr("denom", coin_denom.to_string()),
            attr("minted", bond_data.mint_amount),
            attr("exchange_rate", bond_data.exchange_rate.to_string()),
        ]);

    Ok(res)
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
    let coin_denom = params.underlying_coin_denom.to_string();
    let liquidstaking_denom = params.liquidstaking_denom.to_string();
    let sender = info.sender.to_string();
    let the_staker: String = staker.unwrap_or_else(|| sender.to_string());
    let delegator = env.contract.address.clone();

    let unbond_amount: Uint128;
    if cfg!(not(nonunion)) {
        if amount.is_none() {
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
            unbond_amount = amount.unwrap();
        }
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

    let (msgs, unbond_data) = utils::delegation::process_unbond(
        env.clone(),
        deps.storage,
        deps.querier,
        sender.to_string(),
        the_staker.clone(),
        delegator,
        unbond_amount,
        params,
        validators_reg,
    )?;

    // create bond event here
    let unbond_event = UnbondEvent(
        sender.to_string(),
        the_staker.clone(),
        unbond_amount,
        unbond_data.undelegate_amount,
        unbond_data.delegated_amount,
        unbond_data.delegated_amount + unbond_data.reward,
        unbond_data.total_supply,
        unbond_data.exchange_rate,
    );

    LOG.save(
        deps.storage,
        &format!("{}: {:?}", env.block.time, unbond_event),
    )?;

    let attrs = get_unbond_attrs(
        sender,
        the_staker,
        unbond_data.exchange_rate.to_string(),
        unbond_amount.to_string(),
        unbond_data.undelegate_amount.to_string(),
        unbond_data.delegated_amount.to_string(),
        (unbond_data.delegated_amount + unbond_data.reward).to_string(),
        unbond_data.total_supply.to_string(),
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

    let delegator = env.contract.address.clone();

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

    let msgs = utils::delegation::get_delegate_to_validator_msgs(
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

    // after update exchange rate we update the state
    state.bond_counter = state.bond_counter + 1;
    state.total_bond_amount = total_bond_amount + payment.amount;
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
    state.total_supply = Uint128::new(0);
    state.total_delegated_amount = Uint128::new(0);
    state.last_bond_time = 0;
    state.exchange_rate = Decimal::one();
    STATE.save(deps.storage, &state)?;

    unbond_record().clear(deps.storage);
    let msgs = utils::delegation::get_unbond_all_messages(deps, env.contract.address)?;

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_messages(msgs)
        .add_attribute("action", "reset");

    Ok(res)
}

pub fn transfer_to_owner(
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

    let owner = cw_ownable::get_ownership(deps.storage)?;

    let bank_msg = BankMsg::Send {
        to_address: owner.owner.unwrap().to_string(),
        amount: vec![balance.clone()],
    };
    let msg: CosmosMsg<TokenFactoryMsg> = CosmosMsg::Bank(bank_msg);

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_message(msg)
        .add_attribute("action", "transfer_to_owner")
        .add_attribute("amount", balance.amount.to_string());

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
    ucs03_channel: Option<u32>,
    ucs03_relay_contract: Option<String>,
    unbonding_time: Option<u64>,
    cw20_address: Option<Addr>,
    reward_address: Option<Addr>,
    quote_token: Option<String>,
    lst_quote_token: Option<String>,
    fee_receiver: Option<Addr>,
    fee_rate: Option<Decimal>,
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
    params.quote_token = quote_token.clone().unwrap_or_else(|| params.quote_token);
    params.lst_quote_token = lst_quote_token
        .clone()
        .unwrap_or_else(|| params.lst_quote_token);
    params.fee_receiver = fee_receiver.clone().unwrap_or_else(|| params.fee_receiver);
    params.fee_rate = fee_rate.clone().unwrap_or_else(|| params.fee_rate);

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

    // change the fee receiver and fee rate on reward contract
    if fee_receiver.is_some() || fee_rate.is_some() {
        let msg = ExecuteRewardMsg::SetConfig {
            fee_receiver: fee_receiver,
            fee_rate: fee_rate,
        };
        let msg_bin = to_json_binary(&msg)?;
        let msg: CosmosMsg<TokenFactoryMsg> = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: params.reward_address.to_string(),
            msg: msg_bin,
            funds: vec![],
        });
        msgs.push(msg);
    }

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
            ucs03_channel.unwrap_or_else(|| 0).to_string(),
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
            let wasm_msg = utils::protocol::ucs03_transfer(
                env.clone(),
                params.ucs03_relay_contract.as_str().into(),
                params.ucs03_channel,
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

pub fn transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Coin,
    receiver: String,
    ucs03_channel_id: u32,
    ucs03_contract: String,
    quote_token: String,
    salt: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let params = PARAMETERS.load(deps.storage)?;

    let funds = vec![amount.clone()];
    let wasm_msg: WasmMsg = utils::protocol::ucs03_transfer(
        env,
        ucs03_contract,
        ucs03_channel_id,
        Bytes::from_str(receiver.as_str()).unwrap(),
        params.underlying_coin_denom.clone(),
        amount.amount,
        Bytes::from_str(quote_token.as_str()).unwrap(),
        Uint256::from(amount.amount),
        funds,
        FixedBytes::from_str(salt.as_str()).unwrap(),
    )?;
    let msg: CosmosMsg<TokenFactoryMsg> = CosmosMsg::Wasm(wasm_msg);

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_message(msg)
        .add_attribute("action", "transfer")
        .add_attribute("receiver", receiver.to_string())
        .add_attribute("amount", amount.amount.to_string())
        .add_attribute("denom", amount.denom);
    Ok(res)
}

pub fn on_zkgm(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: u32,
    sender: Bytes,
    message: Bytes,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let msg_bytes = message.as_ref();
    let payload: ZkgmMessage = from_json(msg_bytes)?;
    let msg = format!(
        "onzgkm time:{} channel_id:{} sender:{} message:{:?}",
        env.block.time, channel_id, sender, payload
    );
    LOG.save(deps.storage, &msg)?;

    match payload {
        ZkgmMessage::Bond { amount, salt } => {
            return zkgm_bond(deps, env, info, format!("{}", sender), amount, salt)
        }
        ZkgmMessage::Unbond { amount } => {
            return zkgm_unbond(deps, env, info, format!("{}", sender), amount)
        }
    }
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

    let msgs: Vec<CosmosMsg<TokenFactoryMsg>> = utils::delegation::adjust_validators_delegation(
        deps,
        env.contract.address,
        prev_validators.clone(),
        validators.clone(),
    )?;

    let event = UpdateValidatorsEvent(info.sender.to_string(), prev_validators, validators);
    let res: Response<TokenFactoryMsg> = Response::new().add_messages(msgs).add_event(event);
    Ok(res)
}

// let undelegate_staking_msg: CosmosMsg<TokenFactoryMsg> = CosmosMsg::Staking(undelegate_msg);
// msgs.push(undelegate_staking_msg);

pub fn migrate_reward(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    code_id: u64,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let migrate = MigrateMsg {};
    let msg_bin = to_json_binary(&migrate)?;
    let migrate_msg: CosmosMsg<TokenFactoryMsg> = CosmosMsg::Wasm(WasmMsg::Migrate {
        contract_addr: params.reward_address.to_string(),
        new_code_id: code_id,
        msg: msg_bin,
    });

    let res: Response<TokenFactoryMsg> = Response::new().add_message(migrate_msg);
    Ok(res)
}

pub fn transfer_reward(deps: DepsMut) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let msg = ExecuteRewardMsg::TransferToOwner {};
    let msg_bin = to_json_binary(&msg)?;
    let msg: CosmosMsg<TokenFactoryMsg> = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: params.reward_address.to_string(),
        msg: msg_bin,
        funds: vec![],
    });

    let res: Response<TokenFactoryMsg> = Response::new().add_message(msg);
    Ok(res)
}

pub fn burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let params = PARAMETERS.load(deps.storage)?;

    let msg = utils::token::burn_token(
        env.contract.address.to_string(),
        amount,
        params.liquidstaking_denom.clone(),
        None,
    );

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_message(msg)
        .add_attribute("action", "burn")
        .add_attribute("denom", params.liquidstaking_denom)
        .add_attribute("amount", amount.to_string());
    Ok(res)
}

#[cfg(test)]
// #[test]
// fn validator_restaking_adjustment() {
//     use std::collections::HashMap;

//     let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();
//     let mut correct_validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

//     validator_delegation_map.insert("A".into(), Uint128::new(50000));
//     validator_delegation_map.insert("B".into(), Uint128::new(50000));

//     correct_validator_delegation_map.insert("B".into(), Uint128::new(30000));
//     correct_validator_delegation_map.insert("C".into(), Uint128::new(30000));
//     correct_validator_delegation_map.insert("D".into(), Uint128::new(40000));

//     let (surplus, deficit) = utils::delegation::get_surplus_deficit_validators(
//         validator_delegation_map,
//         correct_validator_delegation_map,
//     );

//     let denom = "muno".to_string();
//     let msgs = utils::delegation::get_restaking_msgs(surplus, deficit, denom);
//     println!("msgs: {:?}", msgs);
// }

// #[test]
// fn validator_restaking_adjustment_2() {
//     use std::collections::HashMap;

//     let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();
//     let mut correct_validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

//     validator_delegation_map.insert("A".into(), Uint128::new(50000));
//     validator_delegation_map.insert("B".into(), Uint128::new(50000));
//     validator_delegation_map.insert("C".into(), Uint128::new(50000));

//     correct_validator_delegation_map.insert("B".into(), Uint128::new(75000));
//     correct_validator_delegation_map.insert("C".into(), Uint128::new(75000));

//     let (surplus, deficit) = utils::delegation::get_surplus_deficit_validators(
//         validator_delegation_map,
//         correct_validator_delegation_map,
//     );

//     let denom = "muno".to_string();
//     let msgs = utils::delegation::get_restaking_msgs(surplus, deficit, denom);
//     println!("msgs: {:?}", msgs);
// }

// #[test]
// fn validator_restaking_adjustment_3() {
//     use std::collections::HashMap;

//     let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();
//     let mut correct_validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

//     validator_delegation_map.insert("A".into(), Uint128::new(30000));
//     validator_delegation_map.insert("B".into(), Uint128::new(40000));
//     validator_delegation_map.insert("C".into(), Uint128::new(30000));

//     correct_validator_delegation_map.insert("B".into(), Uint128::new(25000));
//     correct_validator_delegation_map.insert("C".into(), Uint128::new(25000));
//     correct_validator_delegation_map.insert("D".into(), Uint128::new(50000));

//     let (surplus, deficit) = utils::delegation::get_surplus_deficit_validators(
//         validator_delegation_map,
//         correct_validator_delegation_map,
//     );

//     let denom = "muno".to_string();
//     let msgs = utils::delegation::get_restaking_msgs(surplus, deficit, denom);
//     println!("\nmsgs: {:?}", msgs);
// }

// #[test]
// fn validator_restaking_adjustment_4() {
//     use std::collections::HashMap;

//     let mut validator_delegation_map: HashMap<String, Uint128> = HashMap::new();
//     let mut correct_validator_delegation_map: HashMap<String, Uint128> = HashMap::new();

//     validator_delegation_map.insert("B".into(), Uint128::new(40000));
//     validator_delegation_map.insert("C".into(), Uint128::new(30000));
//     validator_delegation_map.insert("A".into(), Uint128::new(30000));

//     correct_validator_delegation_map.insert("A".into(), Uint128::new(80000));
//     correct_validator_delegation_map.insert("B".into(), Uint128::new(20000));

//     let (surplus, deficit) = utils::delegation::get_surplus_deficit_validators(
//         validator_delegation_map,
//         correct_validator_delegation_map,
//     );

//     let denom = "muno".to_string();
//     let msgs = utils::delegation::get_restaking_msgs(surplus, deficit, denom);
//     println!("\nmsgs: {:?}", msgs);
// }

// #[test]
// fn test_delegate_amount() {
//     let weight: u32 = 1;
//     let total_weight: u32 = 1;
//     let ratio = Decimal::from_ratio(Uint128::from(weight), Uint128::from(total_weight));
//     let amount = Uint128::from(10u32);
//     let delegate_amount = utils::delegation::calculate_delegated_amount(amount, ratio);
//     println!("delegate_amount: {}", delegate_amount);
// }
#[test]
fn test_calculate_native_token() {
    let staking_token = Uint128::from(10000u32);
    //60926366
    let exchange_rate =
        Decimal::from_ratio(Uint128::from(5350444044771u128), Uint128::from(30000u128));

    println!("exchange_rate: {}", exchange_rate);

    let undelegate_amount: Uint128 =
        utils::calc::calculate_native_token_from_staking_token(staking_token, exchange_rate);
    println!("undelegate_amount: {}", undelegate_amount);
}

#[test]
fn exchange_rate_calculation() {
    let total_bond = Uint128::new(100);

    let a = Uint128::new(10);
    let b = Uint128::new(50);
    let exchange_rate = Decimal::from_ratio(a, b);
    println!("{:?} / {:?}", total_bond, exchange_rate);

    let token = utils::calc::calculate_staking_token_from_rate(total_bond, exchange_rate);

    println!("token: {:?}", token);
    assert_eq!(token, Uint128::new(500));

    // - Rewards for 4 days: 1000 Union * 0.0274% * 4 = 1.096 Union
    // - Total staked Union + rewards (U + R): 1001.096 Union
    // - Total LUnion (L): 1000 LUnion

    // - New exchange rate: 1001.096 / 1000 = 1.001096 Union per LUnion
    // - Bob receives: 500 / 1.001096 = 499.45 LUnion

    let a = Uint128::new(1001096);
    let b = Uint128::new(1000000);
    let new_exchange_rate = Decimal::from_ratio(a, b);

    let bond_amount = Uint128::new(500000000);
    let mint_amount =
        utils::calc::calculate_staking_token_from_rate(bond_amount, new_exchange_rate);
    assert_eq!(mint_amount, Uint128::new(499452599));
    println!("mint_amount: {:?}", mint_amount);
}

// #[test]
// fn exchange_unbond_rate_calculation() {
//     let staking_token = Uint128::new(100);

//     let a = Uint128::new(110);
//     let b = Uint128::new(100);
//     let exchange_rate = Decimal::from_ratio(a, b);

//     let token =
//         utils::calc::calculate_native_token_from_staking_token(staking_token, exchange_rate);
//     assert_eq!(token, Uint128::new(110));
// }
