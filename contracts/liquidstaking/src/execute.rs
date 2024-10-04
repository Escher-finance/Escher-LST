use crate::error::ContractError;
use crate::msg::MintTokensPayload;
use crate::relay::send_to_evm;
use crate::reply::MINT_TOKENS_REPLY_ID;
use crate::state::{increment_tokens, unbond_history, UnbondHistory, Config, CONFIG, PARAMETERS, STATE, VALIDATORS_REGISTRY};
use crate::token_factory_api::TokenFactoryMsg;
use crate::utils::{
    calculate_native_token_from_staking_token, calculate_staking_token_from_rate, get_actual_total_bonded, get_actual_total_reward,
    get_mock_total_reward,
};
use cosmwasm_std::{
    attr, to_json_binary, Addr, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response,
    StakingMsg, SubMsg, Uint128, Timestamp
};

pub fn bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    source: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom;
    let sender = info.sender;

    // coin must have be sent along with transaction and it should be in underlying coin denom
    if info.funds.len() > 1usize {
        return Err(ContractError::InvalidAsset {});
    }

    // coin must have be sent along with transaction and it should be in underlying coin denom
    let payment = info
        .funds
        .iter()
        .find(|x| x.denom == coin_denom && x.amount > Uint128::zero())
        .ok_or_else(|| ContractError::NoAsset {})?;

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

    if !cfg!(test) {
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
            source,
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
        .add_attributes(vec![
            attr("action", "mint"),
            attr("from", sender),
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
    source: String,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let validators_reg = VALIDATORS_REGISTRY.load(deps.storage)?;
    let coin_denom = params.liquidstaking_denom;
    let sender = info.sender;

    // coin must have be sent along with transaction and it should be in underlying coin denom
    if info.funds.len() > 1usize {
        return Err(ContractError::InvalidAsset {});
    }

    // coin must have be sent along with transaction and it should be in underlying coin denom
    let payment = info
        .funds
        .iter()
        .find(|x| x.denom == coin_denom && x.amount > Uint128::zero())
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
    state.exchange_rate = Decimal::from_ratio(total_bond_amount, state.total_lst_supply);

    // calculate native token undelegated amount from liquid staking payment amount 
    let native_token_undelegated_amount = calculate_native_token_from_staking_token(payment.amount.clone(), state.exchange_rate);

    let undelegate_amount = native_token_undelegated_amount / total_validators;
    let remaining_amount = native_token_undelegated_amount % total_validators;

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

    let burn_msg = TokenFactoryMsg::BurnTokens {
        denom: coin_denom.clone(),
        amount: payment.amount,
        burn_from_address: delegator.to_string(),
    };

    msgs.push(burn_msg.into());
    
    let id = increment_tokens(deps.storage).unwrap();
    let unbond_amount = Coin {
        amount: native_token_undelegated_amount.clone(),
        denom: coin_denom.clone(),
    };
    let history = UnbondHistory {
        id,
        sender: source,
        amount: unbond_amount,
        exchange_rate: state.exchange_rate,
        unbond_time: env.block.time, 
        released: false,
        released_time: Timestamp::from_nanos(000_000_000),
    };
    unbond_history().save(deps.storage, id, &history)?;
    
    // update total bond, supply and exchange rate here
    state.total_bond_amount = state.total_bond_amount - native_token_undelegated_amount;
    state.total_lst_supply = state.total_lst_supply - payment.amount;
    state.update_exchange_rate();
    STATE.save(deps.storage, &state)?;

    let res: Response<TokenFactoryMsg> = Response::new()
    .add_messages(msgs)
    .add_attributes(vec![
        attr("action", "undelegate"),
        attr("from", sender),
        attr("undelegate_amount", undelegate_amount.to_string()),
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

pub fn set_owner(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: Addr,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

    config.owner = new_owner.to_string();
    CONFIG.save(deps.storage, &config)?;

    let res: Response<TokenFactoryMsg> = Response::new()
        .add_attribute("action", "set_owner")
        .add_attribute("owner", new_owner.to_string());
    Ok(res)
}

pub fn set_token_admin(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
    new_admin: Addr,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

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
