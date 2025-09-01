use cosmwasm_std::{Addr, StdError, StdResult};
use depolama::{KeyCodec, Prefix, Store, ValueCodec, value::ValueCodecViaEncoding};
use unionlabs_encoding::Bincode;
use unionlabs_primitives::{ByteArrayExt, Bytes};

use crate::types::{
    AccountingState, Batch, BatchId, Config, PendingOwner, ProtocolFeeConfig, UnstakeRequest,
    UnstakeRequestKey,
};

pub enum Stopped {}

impl Store for Stopped {
    const PREFIX: Prefix = Prefix::new(b"stopped");
    type Key = ();
    type Value = bool;
}

impl ValueCodecViaEncoding for Stopped {
    type Encoding = Bincode;
}

pub enum ConfigStore {}

impl Store for ConfigStore {
    const PREFIX: Prefix = Prefix::new(b"config");
    type Key = ();
    type Value = Config;
}

impl ValueCodecViaEncoding for ConfigStore {
    type Encoding = Bincode;
}

pub enum ProtocolFeeConfigStore {}

impl Store for ProtocolFeeConfigStore {
    const PREFIX: Prefix = Prefix::new(b"protocol_fee_config");
    type Key = ();
    type Value = ProtocolFeeConfig;
}

impl ValueCodecViaEncoding for ProtocolFeeConfigStore {
    type Encoding = Bincode;
}

pub enum AccountingStateStore {}

impl Store for AccountingStateStore {
    const PREFIX: Prefix = Prefix::new(b"state");
    type Key = ();
    type Value = AccountingState;
}

impl ValueCodecViaEncoding for AccountingStateStore {
    type Encoding = Bincode;
}

pub enum Monitors {}

impl Store for Monitors {
    const PREFIX: Prefix = Prefix::new(b"monitors");
    type Key = ();
    type Value = Vec<String>;
}

impl ValueCodecViaEncoding for Monitors {
    type Encoding = Bincode;
}

/// The address of the [`UCS03-ZKGM`] contract on this chain.
///
/// [`UCS03-ZKGM`]: https://docs.union.build/ucs/03
pub enum Zkgm {}

impl Store for Zkgm {
    const PREFIX: Prefix = Prefix::new(b"zkgm");
    type Key = ();
    type Value = Addr;
}

impl ValueCodec<Addr> for Zkgm {
    fn encode_value(value: &Addr) -> Bytes {
        value.as_bytes().into()
    }

    fn decode_value(raw: &Bytes) -> StdResult<Addr> {
        String::from_utf8(raw.to_vec())
            .map(Addr::unchecked)
            .map_err(|e| StdError::generic_err(format!("invalid value: {e}")))
    }
}

pub enum Admin {}

impl Store for Admin {
    const PREFIX: Prefix = Prefix::new(b"admin");
    type Key = ();
    type Value = Addr;
}

impl ValueCodec<Addr> for Admin {
    fn encode_value(value: &Addr) -> Bytes {
        value.as_bytes().into()
    }

    fn decode_value(raw: &Bytes) -> StdResult<Addr> {
        String::from_utf8(raw.to_vec())
            .map(Addr::unchecked)
            .map_err(|e| StdError::generic_err(format!("invalid value: {e}")))
    }
}

/// The address of the [`on-zkgm-call-proxy`] contract. This is checked when executing the [`OnProxyOnZkgmCall`](on_zkgm_call_proxy::OnProxyOnZkgmCall) message.
///
/// [`on-zkgm-call-proxy`]: https://github.com/unionlabs/union/tree/758d66edd45a47861773a1ca74ef9e8a2ea24961/cosmwasm/on-zkgm-call-proxy
pub enum OnZkgmCallProxy {}

impl Store for OnZkgmCallProxy {
    const PREFIX: Prefix = Prefix::new(b"on_zkgm_call_proxy");
    type Key = ();
    type Value = Addr;
}

impl ValueCodec<Addr> for OnZkgmCallProxy {
    fn encode_value(value: &Addr) -> Bytes {
        value.as_bytes().into()
    }

    fn decode_value(raw: &Bytes) -> StdResult<Addr> {
        String::from_utf8(raw.to_vec())
            .map(Addr::unchecked)
            .map_err(|e| StdError::generic_err(format!("invalid value: {e}")))
    }
}

/// Address of the account that is performing the delegation.
///
/// This contract is an implementation of [TODO: LINK CONTRACT CODE HERE]
pub enum StakerAddress {}

impl Store for StakerAddress {
    const PREFIX: Prefix = Prefix::new(b"staker_address");
    type Key = ();
    type Value = Addr;
}

impl ValueCodec<Addr> for StakerAddress {
    fn encode_value(value: &Addr) -> Bytes {
        value.as_bytes().into()
    }

    fn decode_value(raw: &Bytes) -> StdResult<Addr> {
        String::from_utf8(raw.to_vec())
            .map(Addr::unchecked)
            .map_err(|e| StdError::generic_err(format!("invalid value: {e}")))
    }
}

/// The address of the LST CW20 contract.
pub enum LstAddress {}

impl Store for LstAddress {
    const PREFIX: Prefix = Prefix::new(b"lst_address");
    type Key = ();
    type Value = Addr;
}

impl ValueCodec<Addr> for LstAddress {
    fn encode_value(value: &Addr) -> Bytes {
        value.as_bytes().into()
    }

    fn decode_value(raw: &Bytes) -> StdResult<Addr> {
        String::from_utf8(raw.to_vec())
            .map(Addr::unchecked)
            .map_err(|e| StdError::generic_err(format!("invalid value: {e}")))
    }
}

pub enum Batches {}

impl Store for Batches {
    const PREFIX: Prefix = Prefix::new(b"batches");
    type Key = BatchId;
    type Value = Batch;
}

// big endian for iteration to work correctly
impl KeyCodec<BatchId> for Batches {
    fn encode_key(key: &BatchId) -> Bytes {
        key.to_be_bytes().into()
    }

    fn decode_key(raw: &Bytes) -> StdResult<BatchId> {
        BatchId::try_from_be_bytes(raw)
    }
}

impl ValueCodecViaEncoding for Batches {
    type Encoding = Bincode;
}

pub enum PendingBatchId {}

impl Store for PendingBatchId {
    const PREFIX: Prefix = Prefix::new(b"pending_batch_id");
    type Key = ();
    type Value = BatchId;
}

// big endian for iteration to work correctly
impl ValueCodec<BatchId> for PendingBatchId {
    fn encode_value(value: &BatchId) -> Bytes {
        value.to_be_bytes().into()
    }

    fn decode_value(raw: &Bytes) -> StdResult<BatchId> {
        BatchId::try_from_be_bytes(raw)
    }
}

pub enum UnstakeRequests {}

impl Store for UnstakeRequests {
    const PREFIX: Prefix = Prefix::new(b"unstake_requests");

    type Key = UnstakeRequestKey;

    type Value = UnstakeRequest;
}

impl KeyCodec<UnstakeRequestKey> for UnstakeRequests {
    fn encode_key(key: &UnstakeRequestKey) -> Bytes {
        [
            key.batch_id.get().get().to_be_bytes().as_slice(),
            key.staker_hash.get().as_slice(),
        ]
        .concat()
        .into()
    }

    fn decode_key(raw: &Bytes) -> StdResult<UnstakeRequestKey> {
        raw.try_into()
            .map_err(|_| {
                StdError::generic_err(format!(
                    "invalid key: expected 40 bytes, found {}: {raw}",
                    raw.len(),
                ))
            })
            .and_then(|arr: [u8; 40]| {
                Ok(UnstakeRequestKey {
                    batch_id: BatchId::from_be_bytes(arr.array_slice::<0, 8>())?,
                    staker_hash: arr.array_slice::<8, 32>().into(),
                })
            })
    }
}

impl ValueCodecViaEncoding for UnstakeRequests {
    type Encoding = Bincode;
}

/// Compliment to [`UnstakeRequests`], but keyed by the staker hash.
pub enum UnstakeRequestsByStakerHash {}

impl Store for UnstakeRequestsByStakerHash {
    const PREFIX: Prefix = Prefix::new(b"unstake_requests_by_staker_hash");

    type Key = UnstakeRequestKey;

    type Value = UnstakeRequest;
}

impl KeyCodec<UnstakeRequestKey> for UnstakeRequestsByStakerHash {
    fn encode_key(key: &UnstakeRequestKey) -> Bytes {
        [
            key.staker_hash.get().as_slice(),
            key.batch_id.get().get().to_be_bytes().as_slice(),
        ]
        .concat()
        .into()
    }

    fn decode_key(raw: &Bytes) -> StdResult<UnstakeRequestKey> {
        raw.try_into()
            .map_err(|_| {
                StdError::generic_err(format!(
                    "invalid key: expected 40 bytes, found {}: {raw}",
                    raw.len(),
                ))
            })
            .and_then(|arr: [u8; 40]| {
                Ok(UnstakeRequestKey {
                    batch_id: BatchId::from_be_bytes(arr.array_slice::<32, 8>())?,
                    staker_hash: arr.array_slice::<0, 32>().into(),
                })
            })
    }
}

impl ValueCodecViaEncoding for UnstakeRequestsByStakerHash {
    type Encoding = Bincode;
}

pub enum PendingOwnerStore {}

impl Store for PendingOwnerStore {
    const PREFIX: Prefix = Prefix::new(b"pending_owner");
    type Key = ();
    type Value = PendingOwner;
}

impl ValueCodecViaEncoding for PendingOwnerStore {
    type Encoding = Bincode;
}
