use crate::error::ContractError;
use crate::msg::{BondRewardsPayload, ExecuteRewardMsg};
use crate::state::{WithdrawReward, PARAMETERS, REWARD_BALANCE, SPLIT_REWARD_QUEUE};
use cosmwasm_std::{
    attr, entry_point, from_json, to_json_binary, Attribute, CosmosMsg, DepsMut, Env, Reply,
    Response, SubMsg, Uint128, WasmMsg,
};

pub const MINT_CW20_TOKENS_REPLY_ID: u64 = 124;
pub const PROCESS_WITHDRAW_REWARD_REPLY_ID: u64 = 125;
pub const SPLIT_REWARD_REPLY_ID: u64 = 126;

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    if !msg.result.is_ok() {
        let err = msg.result.unwrap_err();
        return Err(ContractError::ReplyError {
            message: err.to_string(),
        });
    }

    match msg.id {
        PROCESS_WITHDRAW_REWARD_REPLY_ID => on_process_rewards(deps, env, msg),
        SPLIT_REWARD_REPLY_ID => on_split_reward(deps, env, msg),
        _ => Ok(Response::new()),
    }
}

/// Call split reward to reward contract after withdraw reward
fn on_process_rewards(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let params = PARAMETERS.load(deps.storage)?;
    let payload: BondRewardsPayload = from_json(msg.payload)?;

    let mut msgs: Vec<SubMsg> = vec![];
    let mut attrs: Vec<Attribute> = vec![];

    // increment the reward balance on this contract as result of withdraw reward
    let mut reward_balance = REWARD_BALANCE.load(deps.storage)?;
    reward_balance += payload.amount;
    REWARD_BALANCE.save(deps.storage, &reward_balance)?;

    let mut split_reward_queue = SPLIT_REWARD_QUEUE.load(deps.storage)?;
    split_reward_queue.withdrawed_amount += payload.amount;
    SPLIT_REWARD_QUEUE.save(deps.storage, &split_reward_queue)?;

    if split_reward_queue.withdrawed_amount == split_reward_queue.target_amount {
        // if total amount that should be withdrawed is reached, we need to split reward
        let msg = ExecuteRewardMsg::SplitReward {};
        let msg_bin = to_json_binary(&msg)?;
        let split_reward_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: params.reward_address.to_string(),
            msg: msg_bin,
            funds: vec![],
        });

        attrs = vec![
            attr("action", "execute_split_reward"),
            attr("reward_contract", params.reward_address.clone().to_string()),
        ];

        // reset SPLIT_REWARD_QUEUE as we already call the split reward and total amount that should be withdrawed is reached
        SPLIT_REWARD_QUEUE.save(
            deps.storage,
            &WithdrawReward {
                target_amount: Uint128::zero(),
                withdrawed_amount: Uint128::zero(),
            },
        )?;

        let sub_msg: SubMsg = SubMsg::reply_always(split_reward_msg, SPLIT_REWARD_REPLY_ID);
        msgs.push(sub_msg);
    }

    let res: Response = Response::new().add_submessages(msgs).add_attributes(attrs);

    Ok(res)
}

/// Handle split reward call to reward contract reply
fn on_split_reward(deps: DepsMut, _env: Env, _msg: Reply) -> Result<Response, ContractError> {
    // reset reward balance after split reward call to "contract that handle split reward" success
    REWARD_BALANCE.save(deps.storage, &Uint128::new(0))?;

    let res: Response = Response::new().add_attribute("action", "on_split_reward");

    Ok(res)
}
