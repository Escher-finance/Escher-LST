use crate::error::ContractError;
use crate::msg::{BondRewardsPayload, MintTokensPayload, TransferMsg, Ucs01RelayExecuteMsg};
use crate::state::{Balance, Parameters, BALANCE, LOG, PARAMETERS};
use cosmwasm_std::{
    entry_point, from_json, to_json_binary, Coin, CosmosMsg, DepsMut, Env, Reply, Response,
    StakingMsg, StdError, Uint128, WasmMsg,
};

pub const MINT_TOKENS_REPLY_ID: u64 = 123;
pub const BOND_WITHDRAW_REWARD_REPLY_ID: u64 = 124;

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    if !msg.result.is_ok() {
        let err = msg.result.unwrap_err();
        return Err(ContractError::ReplyError {
            message: err.to_string(),
        });
    }

    match msg.id {
        MINT_TOKENS_REPLY_ID => return on_mint_tokens(deps, env, msg),
        BOND_WITHDRAW_REWARD_REPLY_ID => return on_bond_rewards(deps, env, msg),
        _ => return Ok(Response::new()),
    };
}

fn on_mint_tokens(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    let params: Parameters = PARAMETERS.load(deps.storage)?;
    let lst_balance = deps.querier.query_balance(
        env.contract.address.to_string(),
        params.liquidstaking_denom.clone(),
    )?;

    let balance = Balance {
        amount: lst_balance.amount,
        last_updated: env.block.time.nanos(),
    };
    BALANCE.save(deps.storage, &balance)?;

    let responses = msg.result.unwrap().msg_responses;
    let mut log = format!("responses_count: {} ", responses.len());
    for response in responses {
        log += format!("{} ", &response.type_url).as_str();
    }

    let payload: MintTokensPayload = from_json(msg.payload)?;
    log += format!("transfer to: {} amount: {}", payload.staker, payload.amount).as_str();
    LOG.save(deps.storage, &log)?;
    return transfer(deps, payload.amount, payload.staker);
}

pub fn transfer(
    deps: DepsMut,
    amount: Uint128,
    receiver: String,
) -> Result<Response, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;

    let coin_amount = Coin {
        amount,
        denom: params.liquidstaking_denom,
    };

    let funds = vec![coin_amount.clone()];
    let msg: CosmosMsg = send_to_evm(
        params.ucs01_relay_contract,
        params.ucs01_channel,
        receiver.to_string(),
        funds,
    )?
    .into();

    let res: Response = Response::new()
        .add_message(msg)
        .add_attribute("action", "transfer")
        .add_attribute("receiver", receiver.to_string())
        .add_attribute("amount", coin_amount.amount.to_string())
        .add_attribute("denom", coin_amount.denom);
    Ok(res)
}

pub fn send_to_evm(
    contract_addr: String,
    channel: String,
    receiver: String,
    funds: Vec<Coin>,
) -> Result<CosmosMsg, StdError> {
    let relay_transfer_msg: Ucs01RelayExecuteMsg = Ucs01RelayExecuteMsg::Transfer(TransferMsg {
        channel,
        receiver,
        memo: "Send back to EVM".to_string(),
        timeout: None,
    });
    let transfer_relay_msg = to_json_binary(&relay_transfer_msg)?;

    let msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        msg: transfer_relay_msg,
        funds,
    })
    .into();

    Ok(msg)
}

fn on_bond_rewards(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let coin_denom = params.underlying_coin_denom;
    let payload: BondRewardsPayload = from_json(msg.payload)?;
    let amount = Coin {
        amount: payload.amount.clone(),
        denom: coin_denom.to_string(),
    };

    let staking_msg: CosmosMsg = CosmosMsg::Staking(StakingMsg::Delegate {
        validator: payload.validator.to_string(),
        amount,
    });

    let res: Response = Response::new()
        .add_message(staking_msg)
        .add_attribute("action", "stake_rewards")
        .add_attribute("validator", payload.validator.to_string())
        .add_attribute("amount", payload.amount.to_string());
    Ok(res)
}
