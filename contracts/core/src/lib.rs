#[cfg(feature = "entry")]
pub mod entries;
pub mod error;
pub mod execute;
pub mod handler;
pub mod ibc;
pub mod msg;
pub mod query;
pub mod state;

pub const CONTRACT_NAME: &str = "crates.io:one-core";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// reply IDs
const AFTER_ACTION: u64 = 1;
const AFTER_ALL_ACTIONS: u64 = 2;
const AFTER_CALLBACK: u64 = 3;
