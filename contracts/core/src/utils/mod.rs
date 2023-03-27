mod coins;

use cosmwasm_std::{
    Binary, ChannelResponse, IbcQuery, PortIdResponse, QuerierWrapper, QueryRequest,
};
use sha2::{Digest, Sha256};

use crate::error::ContractError;

pub use self::coins::Coins;

/// Generate a salt to be used in Instantiate2, if the user does not provide one.
///
/// The salt is sha256 hash of the connection ID and controller address.
/// This entures:
/// - unique for each {connection_id, controller} pair
/// - not exceed the 64 byte max length
pub fn default_salt(connection_id: &str, controller: &str) -> Binary {
    let mut hasher = Sha256::new();
    hasher.update(connection_id.as_bytes());
    hasher.update(controller.as_bytes());
    hasher.finalize().to_vec().into()
}

/// Query the connection ID associated with the specified channel
pub fn connection_of_channel(
    querier: &QuerierWrapper,
    channel_id: &str,
) -> Result<String, ContractError> {
    let chan_res: ChannelResponse = querier.query(&QueryRequest::Ibc(IbcQuery::Channel {
        channel_id: channel_id.into(),
        port_id: None, // default to the contract's own port
    }))?;

    let Some(chan) = chan_res.channel else {
        let port_res: PortIdResponse = querier.query(&QueryRequest::Ibc(IbcQuery::PortId {}))?;
        return Err(ContractError::ChannelNotFound {
            port_id: port_res.port_id,
            channel_id: channel_id.into(),
        });
    };

    Ok(chan.connection_id)
}
