#[cfg(not(feature = "library"))]
pub mod contract;
pub mod error;
pub mod msg;

pub const CONTRACT_NAME:    &str = "crates.io:one-account";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const REPLY_ID: u64 = 69420;
