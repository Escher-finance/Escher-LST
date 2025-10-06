#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]
pub mod contract;
mod error;
pub mod execute;

pub mod event;
pub mod helpers;
pub mod ibc;
/// Various identifier types used throughout the IBC stack.
pub mod instantiate;
pub mod msg;
pub mod query;
pub mod reply;
pub mod state;
pub mod tests;
pub mod types;
pub mod utils;
pub mod zkgm;
pub use crate::error::ContractError;
// Include the generated code
pub mod proto {
    include!("gen/mod.rs");
}
