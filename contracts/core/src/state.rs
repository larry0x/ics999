use cosmwasm_std::Addr;
use cw_storage_plus::{Map, Item};
use one_types::Packet;

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

/// The packet that is being processed.
///
/// A packet may contain one or more actions, which we execute one at a time.
/// Each time, we load the packet from storage, pop the first action, and store
/// the rest. We repeat this until the action queue is empty.
pub const CURRENT_PACKET: Item<Packet> = Item::new("curr_pkt");
