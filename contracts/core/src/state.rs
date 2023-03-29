use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use crate::transfer::TraceItem;

pub const ACCOUNT_CODE_ID: Item<u64> = Item::new("acc_cid");

pub const DEFAULT_TIMEOUT_SECS: Item<u64> = Item::new("def_to_secs");

// (channel_id, controller_addr) => account_addr
pub const ACCOUNTS: Map<(&str, &str), Addr> = Map::new("acct");

// denom => denom_trace
pub const DENOM_TRACES: Map<&str, TraceItem> = Map::new("dnm_trc");

// connection_id => channel_id
pub const ACTIVE_CHANNELS: Map<&str, String> = Map::new("act_chan");
