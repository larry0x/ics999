mod coins;

use {
    crate::error::{Error, Result},
    cosmwasm_std::{
        Binary, ChannelResponse, IbcQuery, PortIdResponse, QuerierWrapper, QueryRequest,
    },
    sha2::{Digest, Sha256},
};

pub use self::coins::Coins;

/// Generate a salt to be used in Instantiate2, if the user does not provide one.
///
/// The salt is sha256 hash of the connection ID and controller address.
/// This entures:
/// - unique for each {channel_id, controller} pair
/// - not exceed the 64 byte max length
pub fn default_salt(channel_id: &str, controller: &str) -> Binary {
    let mut hasher = Sha256::new();
    hasher.update(channel_id.as_bytes());
    hasher.update(controller.as_bytes());
    hasher.finalize().to_vec().into()
}

/// Query the connection ID associated with the specified channel
pub fn connection_of_channel(
    querier:    &QuerierWrapper,
    channel_id: &str,
) -> Result<String> {
    let chan_res: ChannelResponse = querier.query(&QueryRequest::Ibc(IbcQuery::Channel {
        channel_id: channel_id.into(),
        port_id:    None, // default to the contract's own port
    }))?;

    let Some(chan) = chan_res.channel else {
        return Err(Error::ChannelNotFound {
            port_id:    query_port(querier)?,
            channel_id: channel_id.into(),
        });
    };

    Ok(chan.connection_id)
}

/// Query the port ID bound to the current contract.
///
/// Ideally we can simply to querier.query_port but this function isn't
/// available yet.
pub fn query_port(querier: &QuerierWrapper) -> Result<String> {
    querier.query::<PortIdResponse>(&QueryRequest::Ibc(IbcQuery::PortId {}))
        .map(|res| res.port_id)
        .map_err(Into::into)
}
