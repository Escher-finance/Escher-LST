use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

pub const PARAMETERS: Item<Parameters> = Item::new("parameters");
pub const STATE: Item<State> = Item::new("state");
pub const VALIDATORS_REGISTRY: Item<ValidatorsRegistry> = Item::new("validators_registry");
pub const CONFIG: Item<Config> = Item::new("config");
pub const BALANCE: Item<Balance> = Item::new("balance");
pub const LOG: Item<String> = Item::new("log");

#[cw_serde]
pub struct Balance {
    pub amount: Uint128,
    pub last_updated: u64,
}

#[cw_serde]
pub struct Validator {
    pub address: String,
}

#[cw_serde]
pub struct Config {
    pub owner: String,
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

// Config is configuration that still possible to change
#[cw_serde]
pub struct ValidatorsRegistry {
    pub validators: Vec<Validator>,
}

// Parameter is not changeable configuration
#[cw_serde]
pub struct Parameters {
    pub underlying_coin_denom: String,
    pub liquidstaking_denom: String,
    pub ucs01_channel: String,
    pub ucs01_relay_contract: String,
}

impl State {
    pub fn update_exchange_rate(&mut self) {
        self.exchange_rate = Decimal::from_ratio(self.total_bond_amount, self.total_lst_supply);
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
pub struct UnbondHistory {
    pub id: u64,
    pub sender: String,
    pub source: String,
    pub amount: Coin,
    pub exchange_rate: Decimal,
    pub unbond_time: Timestamp,
    pub released: bool,
    pub released_time: Timestamp,
}

pub struct UnbondHistoryIndexes<'a> {
    pub source: MultiIndex<'a, String, UnbondHistory, u64>,
    pub sender: MultiIndex<'a, String, UnbondHistory, u64>,
    pub released: MultiIndex<'a, String, UnbondHistory, u64>,
    pub source_released: MultiIndex<'a, String, UnbondHistory, u64>,
}

impl<'a> IndexList<UnbondHistory> for UnbondHistoryIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<UnbondHistory>> + '_> {
        let v: Vec<&dyn Index<UnbondHistory>> = vec![&self.source, &self.sender, &self.released];
        Box::new(v.into_iter())
    }
}

const UNBOND_HISTORY_NAMESPACE: &str = "unbond_history";

pub fn unbond_history<'a>() -> IndexedMap<u64, UnbondHistory, UnbondHistoryIndexes<'a>> {
    let indexes = UnbondHistoryIndexes {
        source: MultiIndex::new(
            |_pk, d: &UnbondHistory| d.source.clone(),
            UNBOND_HISTORY_NAMESPACE,
            "unbond_history__source",
        ),
        sender: MultiIndex::new(
            |_pk, d: &UnbondHistory| d.sender.clone(),
            UNBOND_HISTORY_NAMESPACE,
            "unbond_history__sender",
        ),
        released: MultiIndex::new(
            |_pk, d: &UnbondHistory| format!("{}", d.released.to_string()),
            UNBOND_HISTORY_NAMESPACE,
            "unbond_history__released",
        ),
        source_released: MultiIndex::new(
            |_pk, d: &UnbondHistory| format!("{}-{}", d.source.to_string(), d.released.to_string()),
            UNBOND_HISTORY_NAMESPACE,
            "unbond_history__source_released",
        ),
    };
    IndexedMap::new(UNBOND_HISTORY_NAMESPACE, indexes)
}
