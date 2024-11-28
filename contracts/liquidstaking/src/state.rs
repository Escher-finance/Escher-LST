use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

use crate::msg::UndelegationRecord;

pub const PARAMETERS: Item<Parameters> = Item::new("parameters");
pub const STATE: Item<State> = Item::new("state");
pub const VALIDATORS_REGISTRY: Item<ValidatorsRegistry> = Item::new("validators_registry");
pub const BALANCE: Item<Balance> = Item::new("balance");
pub const LOG: Item<String> = Item::new("log");

#[cw_serde]
pub struct Balance {
    pub amount: Uint128,
    pub last_updated: u64,
}

#[cw_serde]
pub struct State {
    pub exchange_rate: Decimal,
    // total native token plus staking rewards
    pub total_bond_amount: Uint128,
    // total native token that is delegated, include rewards
    pub total_delegated_amount: Uint128,
    // total liquid staking token that is issued
    pub total_lst_supply: Uint128,
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
    pub ucs01_channel: String,
    pub ucs01_relay_contract: String,
    pub fee_rate: Decimal,
    pub revenue_receiver: String,
    pub unbonding_time: u64,
    // cw20 contract address
    pub cw20_address: Option<Addr>,
}

impl State {
    pub fn update_exchange_rate(&mut self) {
        if self.total_bond_amount != Uint128::new(0) && self.total_lst_supply != Uint128::new(0) {
            self.exchange_rate = Decimal::from_ratio(self.total_bond_amount, self.total_lst_supply);
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
    pub amount: Coin,
    pub undelegate_amount: Coin,
    pub exchange_rate: Decimal,
    pub undelegations: Vec<UndelegationRecord>,
    pub created: Timestamp,
    pub completion: Timestamp,
    pub released: bool,
    pub released_time: Timestamp,
}

pub struct UnbondRecordIndexes<'a> {
    pub staker: MultiIndex<'a, String, UnbondRecord, u64>,
    pub sender: MultiIndex<'a, String, UnbondRecord, u64>,
    pub released: MultiIndex<'a, String, UnbondRecord, u64>,
    pub staker_released: MultiIndex<'a, String, UnbondRecord, u64>,
    pub height: MultiIndex<'a, String, UnbondRecord, u64>,
}

impl<'a> IndexList<UnbondRecord> for UnbondRecordIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<UnbondRecord>> + '_> {
        let v: Vec<&dyn Index<UnbondRecord>> =
            vec![&self.staker, &self.sender, &self.released, &self.height];
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
        sender: MultiIndex::new(
            |_pk, d: &UnbondRecord| d.sender.clone(),
            UNBOND_RECORD_NAMESPACE,
            "unbond_record__sender",
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
        height: MultiIndex::new(
            |_pk, d: &UnbondRecord| d.height.to_string(),
            UNBOND_RECORD_NAMESPACE,
            "unbond_record__height",
        ),
    };
    IndexedMap::new(UNBOND_RECORD_NAMESPACE, indexes)
}
