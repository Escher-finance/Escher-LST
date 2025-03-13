/// MsgWrappedDelegate is the message for delegating stakes
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgWrappedDelegate {
    #[prost(message, optional, tag = "1")]
    pub msg: ::core::option::Option<super::super::super::cosmos::staking::v1beta1::MsgDelegate>,
}
/// MsgWrappedDelegate is the response to the MsgWrappedDelegate message
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgWrappedDelegateResponse {}
/// MsgWrappedUndelegate is the message for undelegating stakes
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgWrappedUndelegate {
    #[prost(message, optional, tag = "1")]
    pub msg: ::core::option::Option<super::super::super::cosmos::staking::v1beta1::MsgUndelegate>,
}
/// MsgWrappedUndelegateResponse is the response to the MsgWrappedUndelegate
/// message
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgWrappedUndelegateResponse {}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
/// MsgWrappedBeginRedelegate is the message for moving bonded stakes from a
/// validator to another validator
pub struct MsgWrappedBeginRedelegate {
    #[prost(message, optional, tag = "1")]
    pub msg:
        ::core::option::Option<super::super::super::cosmos::staking::v1beta1::MsgBeginRedelegate>,
}
