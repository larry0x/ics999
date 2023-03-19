use cosmwasm_std::Addr;
use cw_storage_plus::{Map, Item};

/// Address of the one-transfer contract
pub const TRANSFER: Item<Addr> = Item::new("transfer");

/// Code ID of the one-account contract
pub const ACCOUNT_CODE_ID: Item<u64> = Item::new("acc_cid");

/// Interchain accounts associated with each controller
///
/// (connection_id, controller_addr) => account_addr
pub const ACCOUNTS: Map<(&str, &str), Addr> = Map::new("acct");

/// The open active channel associated with each connection.
/// Used to enforce one unique ICS-999 channel per connection.
///
/// connection_id => channel_id
pub const ACTIVE_CHANNELS: Map<&str, String> = Map::new("act_chan");
