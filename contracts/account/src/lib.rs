#[cfg(feature = "entry")]
pub mod entries;
pub mod error;
pub mod msg;

pub const CONTRACT_NAME: &str = "crates.io:one-account";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
