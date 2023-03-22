use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const TRANSFER: Item<Addr> = Item::new("transfer");

pub const ACCOUNT_CODE_ID: Item<u64> = Item::new("acc_cid");

pub const DEFAULT_TIMEOUT_SECS: Item<u64> = Item::new("def_to_secs");

// (connection_id, controller_addr) => account_addr
pub const ACCOUNTS: Map<(&str, &str), Addr> = Map::new("acct");

// connection_id => channel_id
pub const ACTIVE_CHANNELS: Map<&str, String> = Map::new("act_chan");
