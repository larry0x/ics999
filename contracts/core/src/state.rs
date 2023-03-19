use cw_storage_plus::Map;

/// The open active channel associated with each connection.
/// Used to enforce one unique ICS-999 channel per connection.
//
// connection_id => channel_id
pub const ACTIVE_CHANNELS: Map<&str, String> = Map::new("act_chan");
