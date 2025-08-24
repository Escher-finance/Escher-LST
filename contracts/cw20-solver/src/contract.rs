#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::error::ContractError;
use crate::execute;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw20_base::contract::instantiate(
        deps.branch(),
        env,
        info,
        cw20_base::msg::InstantiateMsg {
            name: msg.name,
            symbol: msg.symbol,
            decimals: msg.decimals,
            initial_balances: msg.initial_balances,
            mint: msg.mint,
            marketing: msg.marketing,
        },
    )?;
    cw_ownable::initialize_owner(
        deps.storage,
        deps.api,
        msg.owner.as_ref().map(|o| o.as_str()),
    )?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let sender = info.sender.clone();
    match msg {
        ExecuteMsg::UpdateOwnership(action) => execute::update_ownership(deps, env, sender, action),
        //
        // ZKGM SOLVER
        //
        ExecuteMsg::SetFungibleCounterparty { .. } => unimplemented!(),
        ExecuteMsg::DoSolve { .. } => unimplemented!(),
        //
        // CW20 BASE
        //
        ExecuteMsg::UpdateMinter { new_minter } => Ok(cw20_base::contract::execute_update_minter(
            deps, env, info, new_minter,
        )?),
        ExecuteMsg::Mint { recipient, amount } => Ok(cw20_base::contract::execute_mint(
            deps, env, info, recipient, amount,
        )?),
        ExecuteMsg::Transfer { recipient, amount } => Ok(cw20_base::contract::execute_transfer(
            deps, env, info, recipient, amount,
        )?),
        ExecuteMsg::Burn { amount } => {
            Ok(cw20_base::contract::execute_burn(deps, env, info, amount)?)
        }
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => Ok(cw20_base::contract::execute_send(
            deps, env, info, contract, amount, msg,
        )?),
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(cw20_base::allowances::execute_increase_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(cw20_base::allowances::execute_decrease_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => Ok(cw20_base::allowances::execute_transfer_from(
            deps, env, info, owner, recipient, amount,
        )?),
        ExecuteMsg::BurnFrom { owner, amount } => Ok(cw20_base::allowances::execute_burn_from(
            deps, env, info, owner, amount,
        )?),
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => Ok(cw20_base::allowances::execute_send_from(
            deps, env, info, owner, contract, amount, msg,
        )?),
        ExecuteMsg::UpdateMarketing {
            project,
            description,
            marketing,
        } => Ok(cw20_base::contract::execute_update_marketing(
            deps,
            env,
            info,
            project,
            description,
            marketing,
        )?),
        ExecuteMsg::UploadLogo(logo) => Ok(cw20_base::contract::execute_upload_logo(
            deps, env, info, logo,
        )?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[cfg(test)]
mod tests {}
