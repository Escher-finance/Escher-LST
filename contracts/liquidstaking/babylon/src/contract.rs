use cosmwasm_std::{
    entry_point, from_json, CosmosMsg, Decimal, DepsMut, DistributionMsg, Env, MessageInfo,
    Response, StdError, Uint128,
};
use cw2::set_contract_version;
use on_zkgm_call_proxy::OnProxyOnZkgmCall;

use crate::{
    error::ContractError,
    execute,
    instantiate::create_reward,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, RemoteExecuteMsg},
    state::{
        unbond_record, Config, OldParameters, Parameters, ParametersV0_1_194, State, Status,
        SupplyQueue, ValidatorsRegistry, WithdrawReward, CONFIG, PARAMETERS, PENDING_BATCH_ID,
        QUOTE_TOKEN, REWARD_BALANCE, SPLIT_REWARD_QUEUE, STATE, STATUS, SUPPLY_QUEUE,
        VALIDATORS_REGISTRY, WITHDRAW_REWARD_QUEUE,
    },
    utils::{
        batch::{batches, Batch},
        validation::{validate_quote_tokens, validate_validators},
    },
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:liquidstaking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let binding = info.sender.to_string();
    let owner = Some(binding.as_ref());
    cw_ownable::initialize_owner(deps.storage, deps.api, owner)?;

    REWARD_BALANCE.save(deps.storage, &Uint128::new(0))?;

    validate_validators(&msg.validators)?;

    let reg = ValidatorsRegistry {
        validators: msg.validators,
    };

    VALIDATORS_REGISTRY.save(deps.storage, &reg)?;

    let reward_config = Config {
        lst_contract_address: env.clone().contract.address,
        fee_receiver: msg.fee_receiver.clone(),
        fee_rate: msg.fee_rate,
        coin_denom: msg.underlying_coin_denom.clone(),
    };
    CONFIG.save(deps.storage, &reward_config)?;

    let mut reward_address = env.contract.address.clone();
    let msgs: Vec<CosmosMsg> = if msg.use_external_reward.unwrap_or(false) {
        // create reward contract message to instantiate reward contract that will receive staking reward
        let (reward_msg, reward_addr) = create_reward(
            &deps,
            &env,
            msg.salt,
            msg.reward_code_id,
            env.clone().contract.address,
            msg.fee_receiver.clone(),
            msg.fee_rate,
            msg.underlying_coin_denom.clone(),
        )?;
        let set_withdraw_msg: CosmosMsg =
            CosmosMsg::Distribution(DistributionMsg::SetWithdrawAddress {
                address: reward_addr.to_string(),
            });
        reward_address = reward_addr;
        vec![reward_msg, set_withdraw_msg]
    } else {
        vec![]
    };

    let params = Parameters {
        underlying_coin_denom: msg.underlying_coin_denom,
        liquidstaking_denom: msg.liquidstaking_denom,
        ucs03_relay_contract: msg.ucs03_relay_contract,
        unbonding_time: msg.unbonding_time,
        cw20_address: msg.cw20_address,
        reward_address,
        fee_rate: msg.fee_rate,
        fee_receiver: msg.fee_receiver,
        batch_period: msg.batch_period,
        min_bond: msg.min_bond,
        min_unbond: msg.min_unbond,
        batch_limit: msg.batch_limit,
        transfer_fee: msg.transfer_fee,
        transfer_handler: msg.transfer_handler,
        zkgm_token_minter: msg.zkgm_token_minter,
        zkgm_proxy_contract: msg.zkgm_proxy_contract,
    };
    PARAMETERS.save(deps.storage, &params)?;

    let state = State {
        exchange_rate: Decimal::one(),
        total_delegated_amount: Uint128::new(0),
        total_bond_amount: Uint128::new(0),
        total_supply: Uint128::new(0),
        bond_counter: 0,
        last_bond_time: 0,
    };
    STATE.save(deps.storage, &state)?;

    let status = Status {
        bond_is_paused: false,
        unbond_is_paused: false,
    };
    STATUS.save(deps.storage, &status)?;

    SPLIT_REWARD_QUEUE.save(
        deps.storage,
        &WithdrawReward {
            target_amount: Uint128::zero(),
            withdrawed_amount: Uint128::zero(),
        },
    )?;

    validate_quote_tokens(&msg.quote_tokens)?;

    for quote_token in msg.quote_tokens {
        QUOTE_TOKEN.save(deps.storage, quote_token.channel_id, &quote_token)?;
    }

    // set the supply queue
    let supply_queue = SupplyQueue {
        mint: vec![],
        burn: vec![],
        epoch_period: msg.epoch_period.unwrap_or(360),
    };
    SUPPLY_QUEUE.save(deps.storage, &supply_queue)?;

    let pending_batch = Batch::new(
        1,
        Uint128::zero(),
        env.block.time.seconds() + params.batch_period,
    );
    batches().save(deps.storage, pending_batch.id, &pending_batch)?;
    PENDING_BATCH_ID.save(deps.storage, &pending_batch.id)?;

    STATUS.save(
        deps.storage,
        &Status {
            bond_is_paused: false,
            unbond_is_paused: false,
        },
    )?;

    WITHDRAW_REWARD_QUEUE.save(deps.storage, &vec![])?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_messages(msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Bond {
            slippage,
            expected,
            recipient,
            recipient_channel_id,
            salt,
        } => execute::bond(
            deps,
            env,
            info.clone(),
            slippage,
            expected,
            recipient,
            recipient_channel_id,
            salt,
            info.sender.to_string(), // local bond will set sender as staker
        ),
        ExecuteMsg::Receive(cw20_msg) => execute::receive(deps, env, info, cw20_msg),
        ExecuteMsg::SubmitBatch {} => execute::submit_batch(deps, env, info),
        ExecuteMsg::ProcessRewards {} => execute::process_rewards(deps, env, info),
        ExecuteMsg::ProcessBatchWithdrawal { id, salt } => {
            execute::process_batch_withdrawal(deps, env, info, id, salt)
        }
        ExecuteMsg::SetBatchReceivedAmount { id, amount } => {
            execute::set_batch_received_amount(deps, env, info, id, amount)
        }
        ExecuteMsg::UpdateOwnership(action) => execute::update_ownership(deps, env, info, action),
        ExecuteMsg::UpdateValidators { validators } => {
            execute::update_validators(deps, env, info, validators)
        }
        ExecuteMsg::SetParameters {
            underlying_coin_denom,
            liquidstaking_denom,
            ucs03_relay_contract,
            unbonding_time,
            cw20_address,
            reward_address,
            fee_receiver,
            fee_rate,
            batch_period,
            epoch_period,
            min_bond,
            min_unbond,
            batch_limit,
            transfer_handler,
            transfer_fee,
            zkgm_token_minter,
        } => execute::set_parameters(
            deps,
            env,
            info,
            underlying_coin_denom,
            liquidstaking_denom,
            ucs03_relay_contract,
            unbonding_time,
            cw20_address,
            reward_address,
            fee_receiver,
            fee_rate,
            batch_period,
            epoch_period,
            min_bond,
            min_unbond,
            batch_limit,
            transfer_handler,
            transfer_fee,
            zkgm_token_minter,
        ),
        ExecuteMsg::UpdateQuoteToken {
            channel_id,
            quote_token,
        } => execute::update_quote_token(deps, env, info, channel_id, quote_token),
        ExecuteMsg::Redelegate {} => execute::redelegate(deps, env, info),
        ExecuteMsg::OnZkgm {
            caller: _caller,
            path: _path,
            source_channel_id: _source_channel_id,
            destination_channel_id,
            sender,
            message,
            relayer: _relayer,
            relayer_msg: _relayer_msg,
        } => execute::on_zkgm(deps, env, info, destination_channel_id, sender, message),
        ExecuteMsg::MigrateReward { code_id } => execute::migrate_reward(deps, env, info, code_id),
        ExecuteMsg::SplitReward {} => execute::split_reward(deps, env, info),
        ExecuteMsg::SetConfig {
            lst_contract_address,
            fee_receiver,
            fee_rate,
            coin_denom,
        } => execute::set_config(
            deps,
            env,
            info,
            lst_contract_address,
            fee_receiver,
            fee_rate,
            coin_denom,
        ),
        ExecuteMsg::SetStatus(new_status) => execute::set_status(deps, info, new_status),
        ExecuteMsg::SetChain { chain } => execute::set_chain(deps, info, chain),
        ExecuteMsg::RemoveChain { channel_id } => execute::remove_chain(deps, info, channel_id),
        ExecuteMsg::NormalizeReward {} => execute::normalize_reward(deps, env),
        ExecuteMsg::Inject { amount } => execute::inject(deps, env, info, amount),
        ExecuteMsg::AddIbcChannel {
            ibc_channel_id,
            prefix,
        } => execute::add_ibc_channel(deps, info, ibc_channel_id, prefix),
        ExecuteMsg::RemoveIbcChannel { ibc_channel_id } => {
            execute::remove_ibc_channel(deps, info, ibc_channel_id)
        }
        ExecuteMsg::OnProxyOnZkgmCall(OnProxyOnZkgmCall { on_zkgm_msg, msg }) => {
            let params = PARAMETERS.load(deps.storage)?;
            // only zkgm proxy contract is allowed to call this OnProxyOnZkgmCall
            if params.zkgm_proxy_contract != info.sender {
                return Err(ContractError::Unauthorized {
                    sender: info.sender.clone(),
                });
            }

            match from_json::<RemoteExecuteMsg>(msg)? {
                RemoteExecuteMsg::Bond {
                    slippage,
                    expected,
                    recipient,
                    recipient_channel_id,
                    salt,
                } => execute::bond(
                    deps,
                    env,
                    info,
                    slippage,
                    expected,
                    recipient,
                    recipient_channel_id,
                    salt,
                    on_zkgm_msg.sender.to_string(),
                ),
                RemoteExecuteMsg::Unbond {
                    amount,
                    recipient,
                    recipient_channel_id,
                } => execute::unbond(deps, env, info, amount, recipient, recipient_channel_id),
            }
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let check_status = deps.storage.get(b"status");
    if check_status.is_none() {
        STATUS.save(
            deps.storage,
            &Status {
                bond_is_paused: false,
                unbond_is_paused: false,
            },
        )?;
    }

    if CONTRACT_VERSION == "0.1.157" {
        let Some(old_data) = deps.storage.get(b"parameters") else {
            return Err(ContractError::Std(StdError::generic_err("no parameters")));
        };
        // Deserialize it from the old format
        let old_param_result: Result<OldParameters, StdError> = cosmwasm_std::from_json(&old_data);

        if let Ok(old_param) = old_param_result {
            let zkgm_token_minter = match msg.zkgm_token_minter {
                Some(minter) => minter,
                None => env.contract.address.to_string(),
            };

            let new_params = ParametersV0_1_194 {
                underlying_coin_denom: old_param.underlying_coin_denom,
                liquidstaking_denom: old_param.liquidstaking_denom,
                ucs03_relay_contract: old_param.ucs03_relay_contract,
                unbonding_time: old_param.unbonding_time,
                cw20_address: old_param.cw20_address,
                reward_address: old_param.reward_address,
                fee_rate: old_param.fee_rate,
                fee_receiver: old_param.fee_receiver,
                batch_period: old_param.batch_period,
                min_bond: old_param.min_bond,
                min_unbond: old_param.min_unbond,
                batch_limit: old_param.batch_limit,
                transfer_handler: old_param.transfer_handler,
                transfer_fee: old_param.transfer_fee,
                zkgm_token_minter,
            };

            // Serialize the new dataya
            let new_data = cosmwasm_std::to_json_vec(&new_params)?;
            deps.storage.set(b"parameters", &new_data);
        }
    }

    if CONTRACT_VERSION == "0.1.163" {
        migrate_unbond_record_v0_1_163(deps.storage)?;
    }

    let reward_queue_res = deps.storage.get(b"withdraw_reward_queue");
    if reward_queue_res.is_none() {
        WITHDRAW_REWARD_QUEUE.save(deps.storage, &vec![])?;
    }

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("version", CONTRACT_VERSION)
        .add_attribute("contract_name", CONTRACT_NAME))
}

/// Migrate the old unbond record to the new record with recipient and recipient_channel_id properties
pub fn migrate_unbond_record_v0_1_163(
    storage: &mut dyn cosmwasm_std::Storage,
) -> Result<(), ContractError> {
    let old_unbond_records: Vec<(u64, crate::state::OldUnbondRecord)> =
        crate::state::old_unbond_record()
            .range(storage, None, None, cosmwasm_std::Order::Ascending)
            .map(|item| {
                item.map_err(|_| {
                    ContractError::Std(StdError::generic_err("Failed to load old unbond records"))
                })
            })
            .collect::<Result<_, _>>()?;

    for (id, old_record) in old_unbond_records {
        let new_record = crate::state::UnbondRecord {
            id,
            staker: old_record.staker,
            amount: old_record.amount,
            channel_id: old_record.channel_id,
            batch_id: old_record.batch_id,
            height: old_record.height,
            sender: old_record.sender,
            released_height: old_record.released_height,
            released: old_record.released,
            recipient: None,
            recipient_channel_id: None,
        };

        unbond_record().save(storage, id, &new_record)?;
    }

    Ok(())
}

#[test]
fn test_migrate_unbond_record_v0_1_163() {
    let mut deps = cosmwasm_std::testing::mock_dependencies();
    let sender = "sender".to_string();
    let staker = "staker".to_string();
    let unstake_amount = Uint128::new(10000);
    let pending_batch_id = 1;
    let token_count = 5;
    let channel_id = Some(1);
    // Populate old storage
    let old_store = crate::state::old_unbond_record();

    for i in 1..10 {
        let data = crate::state::OldUnbondRecord {
            id: i,
            height: 1000 + i,
            sender: sender.clone(),
            staker: staker.clone(),
            channel_id,
            amount: unstake_amount,
            released_height: 0,
            released: i > (token_count / 2),
            batch_id: pending_batch_id,
        };
        old_store.save(&mut deps.storage, i, &data).unwrap();
    }

    // Run migration
    migrate_unbond_record_v0_1_163(deps.as_mut().storage).unwrap();

    // Verify new storage
    let new_store = unbond_record();
    let new_data: crate::state::UnbondRecord = new_store.load(&deps.storage, 1).unwrap();
    assert_eq!(new_data.id, 1);
    assert_eq!(new_data.height, 1001);
    assert_eq!(new_data.recipient, None);

    let new_data: crate::state::UnbondRecord = new_store.load(&deps.storage, 8).unwrap();
    assert_eq!(new_data.id, 8);
    assert_eq!(new_data.height, 1008);
    assert_eq!(new_data.recipient, None);

    println!("{:#?}", new_data);
}
