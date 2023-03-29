#[cfg(not(feature = "library"))]
pub mod contract;
pub mod controller;
pub mod error;
pub mod handshake;
pub mod host;
pub mod msg;
pub mod query;
pub mod state;
pub mod transfer;
pub mod utils;

pub const CONTRACT_NAME: &str = "crates.io:one-core";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// reply IDs
const AFTER_EXECUTE: u64 = 1111;
const AFTER_ACTIONS: u64 = 2222;
const AFTER_CALLBACK: u64 = 3333;
