#[cfg(feature = "entry")]
pub mod entries;
pub mod error;
pub mod execute;
pub mod ibc;
pub mod msg;
pub mod query;
pub mod state;

pub const CONTRACT_NAME: &str = "crates.io:cw-one";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
