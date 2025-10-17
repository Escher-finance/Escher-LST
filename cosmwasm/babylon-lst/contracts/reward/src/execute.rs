use cosmwasm_std::{
    Addr, Attribute, BankMsg, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, Uint128,
    attr,
};

use crate::{
    error::ContractError,
    event::{SplitRewardEvent, UpdateConfigEvent},
    helpers,
    msg::{Balance, ExecuteLstMsg, LSTQueryMsg},
    state::CONFIG,
};

/// Errors:
/// - Returns `ContractError::NoBalance` when there is no available reward to split.
pub fn split_reward(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let config = CONFIG.load(deps.storage)?;

    // first need to get balance state that is stored on liquid staking contract
    let contract_addr: Addr = env.contract.address;
    let msg = LSTQueryMsg::RewardBalance {};

    let reward_contract_balance = deps
        .querier
        .query_balance(contract_addr, config.coin_denom.clone())?;

    if reward_contract_balance.amount == Uint128::zero() {
        return Err(ContractError::NoBalance {});
    }

    let lst_contract_addr: Addr = config.lst_contract_address;
    let lst_reward_balance: Balance = deps
        .querier
        .query_wasm_smart(lst_contract_addr.clone(), &msg)?;

    let mut balance_to_split = lst_reward_balance.amount;

    if reward_contract_balance.amount < balance_to_split {
        balance_to_split = reward_contract_balance.amount;
    }

    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut attrs: Vec<Attribute> = vec![
        attr("action", "split_reward"),
        attr("fee_rate", format!("{:?}", config.fee_rate)),
        attr("amount", balance_to_split.to_string()),
        attr("fee_receiver", config.fee_receiver.to_string()),
        attr("time", format!("{}", env.block.time.nanos())),
    ];
    let (redelegate, fee) =
        helpers::split_revenue(balance_to_split, config.fee_rate, &config.coin_denom);

    // Send the fee to revenue receiver
    let bank_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: config.fee_receiver.to_string(),
        amount: vec![fee.clone()],
    });

    msgs.push(bank_msg);

    // Redelegate by call the LST Contract and attach the funds
    let lst: helpers::LstTemplateContract = helpers::LstTemplateContract(lst_contract_addr);
    let execute_msg = lst.call(ExecuteLstMsg::Redelegate {}, vec![redelegate.clone()])?;
    msgs.push(execute_msg);

    attrs.push(attr("redelegate_amount", redelegate.amount.to_string()));
    attrs.push(attr("fee_amount", fee.amount.to_string()));

    let event = SplitRewardEvent(
        config.fee_rate,
        balance_to_split,
        redelegate.amount,
        fee.amount,
        env.block.time,
    );

    // transfer the fee to revenue receiver
    Ok(Response::new()
        .add_messages(msgs)
        .add_event(event)
        .add_attributes(attrs))
}

/// Errors:
/// - Returns `ContractError::InvalidFeeRate` when `fee_rate` is greater than one.
pub fn set_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    lst_contract_address: Option<Addr>,
    fee_receiver: Option<Addr>,
    fee_rate: Option<Decimal>,
    coin_denom: Option<String>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    let mut config = CONFIG.load(deps.storage)?;

    if let Some(fee_rate) = fee_rate
        && fee_rate > Decimal::one()
    {
        return Err(ContractError::InvalidFeeRate {});
    }

    config.lst_contract_address = lst_contract_address
        .clone()
        .unwrap_or(config.lst_contract_address);
    config.fee_receiver = fee_receiver.clone().unwrap_or(config.fee_receiver);
    config.fee_rate = fee_rate.unwrap_or(config.fee_rate);
    config.coin_denom = coin_denom.unwrap_or(config.coin_denom);
    CONFIG.save(deps.storage, &config)?;

    let event = UpdateConfigEvent(
        config.lst_contract_address.clone(),
        config.fee_receiver.clone(),
        config.fee_rate,
        config.coin_denom.clone(),
    );

    let attrs = Vec::from([
        attr("action", "set_config"),
        attr("lst_contract_address", config.lst_contract_address),
        attr("fee_receiver", config.fee_receiver),
        attr("fee_rate", config.fee_rate.to_string()),
        attr("coin_denom", config.coin_denom),
    ]);

    Ok(Response::new().add_attributes(attrs).add_event(event))
}

/// Errors:
/// - Returns `ContractError::NoBalance` when there is no available balance to transfer.
pub fn transfer_to_owner(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let config = CONFIG.load(deps.storage)?;

    // first need to get this contract balance
    let contract_addr: Addr = env.contract.address;
    let balance = deps
        .querier
        .query_balance(contract_addr, config.coin_denom.clone())?;

    if balance.amount == Uint128::zero() {
        return Err(ContractError::NoBalance {});
    }

    // Send the fee to revenue receiver
    let bank_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![balance],
    });

    let res: Response = Response::new().add_message(bank_msg);

    Ok(res)
}
