use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use crate::{msg::Config, transfer::TraceItem};

pub const CONFIG: Item<Config> = Item::new("cfg");

// (channel_id, controller_addr) => account_addr
pub const ACCOUNTS: Map<(&str, &str), Addr> = Map::new("acct");

// denom => denom_trace
pub const DENOM_TRACES: Map<&str, TraceItem> = Map::new("dnm_trc");

// connection_id => channel_id
pub const ACTIVE_CHANNELS: Map<&str, String> = Map::new("act_chan");
