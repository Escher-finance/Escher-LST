use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, StdResult, Storage, Uint128};
use cw_storage_plus::Map;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

pub const PARAMETERS: Item<Parameters> = Item::new("parameters");
pub const STATE: Item<State> = Item::new("state");
pub const STATUS: Item<Status> = Item::new("status");
pub const VALIDATORS_REGISTRY: Item<ValidatorsRegistry> = Item::new("validators_registry");
pub const REWARD_BALANCE: Item<Uint128> = Item::new("reward_balance");

// Map of channel id to the quote token and lst quote token of destination chain
pub const QUOTE_TOKEN: Map<u32, QuoteToken> = Map::new("quote_token");
pub const PENDING_BATCH_ID: Item<u64> = Item::new("pending_batch_id");

// Queue of validator reward for executing split reward
pub const SPLIT_REWARD_QUEUE: Item<WithdrawReward> = Item::new("split_reward_queue");

// Map of supported ucs03 chains with ucs03 channel_id as key
pub const CHAINS: Map<u32, Chain> = Map::new("chains");

#[cw_serde]
pub struct Status {
    pub bond_is_paused: bool,
    pub unbond_is_paused: bool,
}

#[cw_serde]
pub struct WithdrawReward {
    pub withdrawed_amount: Uint128,
    pub target_amount: Uint128,
}

#[cw_serde]
pub struct Balance {
    pub amount: Uint128,
}

#[cw_serde]
pub struct State {
    pub exchange_rate: Decimal,
    // total native token plus staking rewards
    pub total_bond_amount: Uint128,
    // total native token that is delegated, include rewards
    pub total_delegated_amount: Uint128,
    // total liquid staking token that is issued
    pub total_supply: Uint128,
    // bond_counter how many times bond is called
    pub bond_counter: u64,
    // last_bond_time
    pub last_bond_time: u64,
}

#[cw_serde]
pub struct Validator {
    pub address: String,
    pub weight: u64,
}

#[cw_serde]
pub struct ValidatorsRegistry {
    pub validators: Vec<Validator>,
}

// Parameter is required data to instantiate and run contract
#[cw_serde]
pub struct Parameters {
    /// native coin denom like ubbn, emuno, etc.
    pub underlying_coin_denom: String,
    /// native coin denom symbol that will be used for ucs03 transfer like ubbn, emuno, etc.
    pub underlying_coin_denom_symbol: String,
    /// liquid staking cw20 denom name that will be used for ucs03 transfer like ebbn, emuno, etc.
    pub liquidstaking_denom: String,
    /// liquid staking cw20 denom symbol that will be used for ucs03 transfer like eBABY
    pub liquidstaking_denom_symbol: String,
    pub ucs03_relay_contract: String,
    pub unbonding_time: u64,
    // liquid_staking denom/cw20 contract address
    pub cw20_address: Addr,
    // reward contract address
    pub reward_address: Addr,
    // fee fee_rate
    pub fee_rate: Decimal,
    // fee receiver
    pub fee_receiver: Addr,
    // batch period range in seconds to execute batch
    pub batch_period: u64,
    // min bond/stake amount
    pub min_bond: Uint128,
    // min unbond/unstake amount
    pub min_unbond: Uint128,
    // limit per batch
    // this is the max number of unbonding records that can be processed in one batch
    pub batch_limit: u32,
    // handler of cw20 staking token transfer, as ucs03 fee payer address and also minted cw20 staking token receiver
    pub transfer_handler: String,
    // ucs03 transfer fee from babylon to other
    pub transfer_fee: Uint128,
    // zkgm token_minter address as cw20 allowance spender
    pub zkgm_token_minter: String,
}

impl State {
    pub fn update_exchange_rate(&mut self) {
        let zero = Uint128::zero();
        self.exchange_rate = if self.total_bond_amount != zero && self.total_supply != zero {
            Decimal::from_ratio(self.total_bond_amount, self.total_supply)
        } else {
            Decimal::one()
        };
    }
}

pub const TOKEN_COUNT: Item<u64> = Item::new("num_tokens");

pub fn increment_tokens(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = num_tokens(storage)? + 1;
    TOKEN_COUNT.save(storage, &val)?;
    Ok(val)
}

pub fn num_tokens(storage: &mut dyn Storage) -> StdResult<u64> {
    Ok(TOKEN_COUNT.may_load(storage)?.unwrap_or_default())
}

#[cw_serde]
pub struct UnbondRecord {
    pub id: u64,
    pub height: u64,
    pub sender: String,
    pub staker: String,
    pub channel_id: Option<u32>,
    pub amount: Uint128,
    pub released_height: u64,
    pub released: bool,
    pub batch_id: u64,
    pub recipient: Option<String>,
    pub recipient_channel_id: Option<u32>,
}

pub struct UnbondRecordIndexes<'a> {
    pub staker: MultiIndex<'a, String, UnbondRecord, u64>,
    pub released: MultiIndex<'a, String, UnbondRecord, u64>,
    pub staker_released: MultiIndex<'a, String, UnbondRecord, u64>,
    pub batch: MultiIndex<'a, String, UnbondRecord, u64>,
}

impl<'a> IndexList<UnbondRecord> for UnbondRecordIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<UnbondRecord>> + '_> {
        let v: Vec<&dyn Index<UnbondRecord>> = vec![
            &self.staker,
            &self.released,
            &self.staker_released,
            &self.batch,
        ];
        Box::new(v.into_iter())
    }
}

const UNBOND_RECORD_NAMESPACE: &str = "unbond_record";

pub fn unbond_record<'a>() -> IndexedMap<u64, UnbondRecord, UnbondRecordIndexes<'a>> {
    let indexes = UnbondRecordIndexes {
        staker: MultiIndex::new(
            |_pk, d: &UnbondRecord| d.staker.clone(),
            UNBOND_RECORD_NAMESPACE,
            "unbond_record__staker",
        ),
        released: MultiIndex::new(
            |_pk, d: &UnbondRecord| d.released.to_string(),
            UNBOND_RECORD_NAMESPACE,
            "unbond_record__released",
        ),
        staker_released: MultiIndex::new(
            |_pk, d: &UnbondRecord| format!("{}-{}", d.staker, d.released),
            UNBOND_RECORD_NAMESPACE,
            "unbond_record__staker_released",
        ),
        batch: MultiIndex::new(
            |_pk, d: &UnbondRecord| d.batch_id.to_string(),
            UNBOND_RECORD_NAMESPACE,
            "unbond_record__batch",
        ),
    };
    IndexedMap::new(UNBOND_RECORD_NAMESPACE, indexes)
}

#[cw_serde]
pub struct QuoteToken {
    pub channel_id: u32,
    pub quote_token: String,
    pub lst_quote_token: String,
}

#[cw_serde]
pub struct Chain {
    pub name: String,
    pub chain_id: String,
    pub ucs03_channel_id: u32,
    pub prefix: String,
}
