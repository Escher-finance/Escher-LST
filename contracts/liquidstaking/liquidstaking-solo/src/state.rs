use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::Map;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

pub const PARAMETERS: Item<Parameters> = Item::new("parameters");
pub const STATE: Item<State> = Item::new("state");
pub const VALIDATORS_REGISTRY: Item<ValidatorsRegistry> = Item::new("validators_registry");
pub const LOG: Item<String> = Item::new("log");
pub const CONFIG: Item<Config> = Item::new("config");

pub const REWARD_BALANCE: Item<Uint128> = Item::new("reward_balance");

// Map of channel id to the quote token and lst quote token of destination chain
pub const QUOTE_TOKEN: Map<u32, QuoteToken> = Map::new("quote_token");

// mint and burn queue of staking token
pub const SUPPLY_QUEUE: Item<SupplyQueue> = Item::new("supply_queue");

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
    pub underlying_coin_denom: String,
    pub liquidstaking_denom: String,
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
}

impl State {
    pub fn update_exchange_rate(&mut self) {
        if self.total_bond_amount != Uint128::new(0) && self.total_supply != Uint128::new(0) {
            self.exchange_rate = Decimal::from_ratio(self.total_bond_amount, self.total_supply);
        }
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
    pub undelegate_amount: Uint128,
    pub created: Timestamp,
    pub released_height: u64,
    pub released: bool,
}
pub struct UnbondRecordIndexes<'a> {
    pub staker: MultiIndex<'a, String, UnbondRecord, u64>,
    pub released: MultiIndex<'a, String, UnbondRecord, u64>,
    pub staker_released: MultiIndex<'a, String, UnbondRecord, u64>,
}

impl<'a> IndexList<UnbondRecord> for UnbondRecordIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<UnbondRecord>> + '_> {
        let v: Vec<&dyn Index<UnbondRecord>> =
            vec![&self.staker, &self.released, &self.staker_released];
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
pub struct Config {
    pub lst_contract_address: Addr,
    pub fee_receiver: Addr,
    pub fee_rate: Decimal,
    pub coin_denom: String,
}

/// the queued staking token mint amount
#[cw_serde]
pub struct MintQueue {
    pub amount: Uint128,
    pub block: u64,
}

/// the queued staking token burn amount
#[cw_serde]
pub struct BurnQueue {
    pub amount: Uint128,
    pub block: u64,
}

/// the minted and burned amount that is not counted yet for exchange rate calculation that will be reset to zero every hour
#[cw_serde]
pub struct SupplyQueue {
    /// the mint amount that is not added for total supply, so total supply should be substracted with this mint amount value
    /// to get the total supply calculation for exchange rate
    pub mint: Vec<MintQueue>,
    /// the burn amount that is not substracted from real total supply, so total supply should be added with this burn amount value
    /// to get the total supply calculation for exchange rate
    pub burn: Vec<BurnQueue>,
    /// epooch period in seconds
    pub epoch_period: u32,
}
