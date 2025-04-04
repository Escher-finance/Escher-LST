pub mod contract;
mod error;
pub mod execute;

pub mod event;
pub mod helpers;
pub mod instantiate;
pub mod msg;
pub mod query;
pub mod reply;
pub mod state;
pub mod tests;
pub mod utils;
pub use crate::error::ContractError;
// Include the generated code
pub mod proto {
    include!("gen/mod.rs");
}
