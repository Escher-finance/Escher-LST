use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::Map;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

pub const PARAMETERS: Item<Parameters> = Item::new("parameters");
pub const STATE: Item<State> = Item::new("state");
pub const VALIDATORS_REGISTRY: Item<ValidatorsRegistry> = Item::new("validators_registry");
pub const BALANCE: Item<Balance> = Item::new("balance");

// Map of channel id to the quote token and lst quote token of destination chain
pub const QUOTE_TOKEN: Map<u32, QuoteToken> = Map::new("quote_token");

// Queue of validator reward for executing split reward
pub const SPLIT_REWARD_QUEUE: Item<Vec<String>> = Item::new("redelegate_batch");

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

#[test]
fn test_update_exchange_rate_should_never_yield_zero() {
    let decimal_zero = Decimal::zero();

    let mut state = State {
        exchange_rate: decimal_zero,
        total_bond_amount: Uint128::zero(),
        total_supply: Uint128::zero(),

        total_delegated_amount: Uint128::default(),
        bond_counter: u64::default(),
        last_bond_time: u64::default(),
    };

    // If `total_bond_amount` and `total_supply` are zero
    state.total_bond_amount = Uint128::zero();
    state.total_supply = Uint128::zero();
    state.update_exchange_rate();
    assert_ne!(state.exchange_rate, decimal_zero);

    // If only `total_bond_amount` is zero
    state.total_bond_amount = Uint128::zero();
    state.total_supply = Uint128::one();
    state.update_exchange_rate();
    assert_ne!(state.exchange_rate, decimal_zero);

    // If only `total_supply` is zero
    state.total_bond_amount = Uint128::one();
    state.total_supply = Uint128::zero();
    state.update_exchange_rate();
    assert_ne!(state.exchange_rate, decimal_zero);
}
