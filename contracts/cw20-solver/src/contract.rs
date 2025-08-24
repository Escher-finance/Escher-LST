#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::error::ContractError;
use crate::execute;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::ZKGM;

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
    ZKGM.save(deps.storage, &msg.zkgm)?;
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
        ExecuteMsg::SetFungibleCounterparty {
            path,
            channel_id,
            base_token,
            counterparty_beneficiary,
        } => execute::set_fungible_counterparty(
            sender,
            deps,
            path,
            channel_id,
            base_token,
            counterparty_beneficiary,
        ),
        ExecuteMsg::DoSolve {
            packet,
            order,
            path,
            caller,
            relayer,
            relayer_msg,
            intent,
        } => execute::do_solve(packet, order, path, caller, relayer, relayer_msg, intent),
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        //
        // ZKGM SOLVER
        //
        QueryMsg::IsSolver {} => to_json_binary(&()),
        QueryMsg::AllowMarketMakers {} => to_json_binary(&false),
        //
        // CW20 BASE
        //
        QueryMsg::Minter {} => to_json_binary(&cw20_base::contract::query_minter(deps)?),
        QueryMsg::Balance { address } => {
            to_json_binary(&cw20_base::contract::query_balance(deps, address)?)
        }
        QueryMsg::TokenInfo {} => to_json_binary(&cw20_base::contract::query_token_info(deps)?),
        QueryMsg::Allowance { owner, spender } => to_json_binary(
            &cw20_base::allowances::query_allowance(deps, owner, spender)?,
        ),
        QueryMsg::AllAllowances {
            owner,
            start_after,
            limit,
        } => to_json_binary(&cw20_base::enumerable::query_owner_allowances(
            deps,
            owner,
            start_after,
            limit,
        )?),
        QueryMsg::AllSpenderAllowances {
            spender,
            start_after,
            limit,
        } => to_json_binary(&cw20_base::enumerable::query_spender_allowances(
            deps,
            spender,
            start_after,
            limit,
        )?),
        QueryMsg::AllAccounts { start_after, limit } => to_json_binary(
            &cw20_base::enumerable::query_all_accounts(deps, start_after, limit)?,
        ),
        QueryMsg::MarketingInfo {} => {
            to_json_binary(&cw20_base::contract::query_marketing_info(deps)?)
        }
        QueryMsg::DownloadLogo {} => {
            to_json_binary(&cw20_base::contract::query_download_logo(deps)?)
        }
    }
}

#[cfg(test)]
mod tests {}
