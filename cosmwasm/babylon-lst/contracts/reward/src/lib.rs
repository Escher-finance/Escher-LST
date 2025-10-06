#![cfg_attr(not(test), warn(clippy::unwrap_used))]
#![allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]

pub mod contract;
mod error;
pub mod event;
pub mod execute;
pub mod helpers;
pub mod msg;
pub mod state;
pub use crate::error::ContractError;

#[cfg(test)]
pub mod tests;
