use crate::event::SplitRewardEvent;
use crate::{error::ContractError, msg::ExecuteLstMsg};

use crate::helpers;
use crate::state::CONFIG;
use cosmwasm_std::{
    attr, Addr, Attribute, BankMsg, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response,
    Uint128,
};

pub fn split_reward(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let config = CONFIG.load(deps.storage)?;
    // only liquid staking contract able to call this function
    if info.sender != config.lst_contract_address {
        return Err(ContractError::Unauthorized {});
    }

    // first need to get this contract balance
    let contract_addr: Addr = env.contract.address;
    let balance = deps
        .querier
        .query_balance(contract_addr, config.coin_denom.clone())?;

    let mut msgs: Vec<CosmosMsg> = vec![];

    if balance.amount == Uint128::zero() {
        return Err(ContractError::NoBalance {});
    }

    let mut attrs: Vec<Attribute> = vec![
        attr("action", "split_reward"),
        attr("fee_rate", format!("{:?}", config.fee_rate)),
        attr("amount", balance.amount.to_string()),
        attr("fee_receiver", config.fee_receiver.to_string()),
    ];
    let (redelegate, fee) =
        helpers::split_revenue(balance.amount, config.fee_rate, config.coin_denom);

    // Send the fee to revenue receiver
    let bank_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: config.fee_receiver.to_string(),
        amount: vec![fee.clone()],
    });

    msgs.push(bank_msg);

    // Redelegate by call the LST Contract and attach the funds
    let lst: helpers::LstTemplateContract =
        helpers::LstTemplateContract(config.lst_contract_address);
    let execute_msg = lst.call(ExecuteLstMsg::Redelegate {}, vec![redelegate.clone()])?;
    msgs.push(execute_msg);

    attrs.push(attr("redelegate_amount", redelegate.amount.to_string()));
    attrs.push(attr("fee_amount", fee.amount.to_string()));

    let event = SplitRewardEvent(
        config.fee_rate,
        balance.amount,
        redelegate.amount,
        fee.amount,
    );

    // transfer the fee to revenue receiver
    Ok(Response::new().add_messages(msgs).add_event(event))
}

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

    config.lst_contract_address = lst_contract_address
        .clone()
        .unwrap_or_else(|| config.lst_contract_address);
    config.fee_receiver = fee_receiver.clone().unwrap_or_else(|| config.fee_receiver);
    config.fee_rate = fee_rate.clone().unwrap_or_else(|| config.fee_rate);
    config.coin_denom = coin_denom.clone().unwrap_or_else(|| config.coin_denom);
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

/// Update the ownership of the contract.
#[allow(clippy::needless_pass_by_value)]
pub fn update_ownership(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    if action == cw_ownable::Action::RenounceOwnership {
        return Err(ContractError::OwnershipCannotBeRenounced);
    };

    let res: Response = Response::new().add_attribute("action", "update_ownership");

    Ok(res)
}

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
