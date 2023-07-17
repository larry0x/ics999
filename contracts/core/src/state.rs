use {
    crate::{msg::Config, transfer::TraceItem},
    cosmwasm_std::{Addr, IbcEndpoint},
    cw_storage_plus::{Item, Map},
};

pub const CONFIG: Item<Config> = Item::new("cfg");

// (port_id, channel_id, controller_addr) => account_addr
pub const ACCOUNTS: Map<(&str, &str, &str), Addr> = Map::new("acc");

// denom => denom_trace
pub const DENOM_TRACES: Map<&str, TraceItem> = Map::new("dt");

// connection_id => ibc_endpoint
pub const ACTIVE_CHANNELS: Map<&str, IbcEndpoint> = Map::new("actchan");
