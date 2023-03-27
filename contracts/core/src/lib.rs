pub mod action;
// #[cfg(not(feature = "library"))]
pub mod contract;
pub mod error;
pub mod execute;
pub mod ibc;
pub mod msg;
pub mod query;
pub mod state;
pub mod utils;
pub mod transfer;

pub const CONTRACT_NAME: &str = "crates.io:one-core";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// reply IDs
const AFTER_ACTION: u64 = 1111;
const AFTER_ALL_ACTIONS: u64 = 2222;
const AFTER_CALLBACK: u64 = 3333;
