use {
    crate::{
        error::{Error, Result},
        state::ACTIVE_CHANNELS,
    },
    cosmwasm_std::{
        DepsMut, IbcBasicResponse, IbcChannel, IbcChannelCloseMsg, IbcChannelOpenResponse,
        IbcOrder, Storage,
    },
    ics999,
};

pub fn open_init(
    deps:    DepsMut,
    channel: IbcChannel,
) -> Result<IbcChannelOpenResponse> {
    validate_order_and_version(&channel.order, &channel.version, None)?;

    // only one active ICS-999 channel per connection
    assert_unique_channel(deps.storage, &channel.connection_id)?;

    // no need to validate counterparty version at this step, because we don't
    // know what it is yet
    //
    // return None means we don't want to set the version to a different value
    Ok(None)
}

pub fn open_try(
    deps:                 DepsMut,
    channel:              IbcChannel,
    counterparty_version: String,
) -> Result<IbcChannelOpenResponse> {
    validate_order_and_version(&channel.order, &channel.version, Some(&counterparty_version))?;

    assert_unique_channel(deps.storage, &channel.connection_id)?;

    Ok(None)
}

pub fn open_connect(
    deps:                 DepsMut,
    channel:              &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<IbcBasicResponse> {
    validate_order_and_version(&channel.order, &channel.version, counterparty_version)?;

    ACTIVE_CHANNELS.save(deps.storage, &channel.connection_id, &channel.endpoint.channel_id)?;

    Ok(IbcBasicResponse::new()
        .add_attribute("method", "open_connect")
        .add_attribute("connection_id", &channel.connection_id)
        .add_attribute("port_id", &channel.endpoint.port_id)
        .add_attribute("channel_id", &channel.endpoint.channel_id))
}

fn validate_order_and_version(
    order:                &IbcOrder,
    version:              &str,
    counterparty_version: Option<&str>,
) -> Result<()> {
    if *order != ics999::ORDER {
        return Err(Error::IncorrectOrder {
            actual:   order.clone(),
            expected: ics999::ORDER,
        });
    }

    if version != ics999::VERSION {
        return Err(Error::IncorrectVersion {
            actual:   version.into(),
            expected: ics999::VERSION.into(),
        });
    }

    if let Some(cp_version) = counterparty_version {
        if cp_version != ics999::VERSION {
            return Err(Error::IncorrectVersion {
                actual:   cp_version.into(),
                expected: ics999::VERSION.into(),
            });
        }
    }

    Ok(())
}

fn assert_unique_channel(store: &dyn Storage, connection_id: &str) -> Result<()> {
    if ACTIVE_CHANNELS.has(store, connection_id) {
        return Err(Error::ChannelExists {
            connection_id: connection_id.into(),
        });
    }

    Ok(())
}

pub fn close(msg: IbcChannelCloseMsg) -> Result<IbcBasicResponse> {
    match msg {
        // we do not expect an ICS-999 channel to be closed
        IbcChannelCloseMsg::CloseInit {
            ..
        } => Err(Error::UnexpectedChannelClosure),

        // If we're here, something has gone catastrophically wrong on our
        // counterparty chain. Per the CloseInit handler above, this contract
        // should never allow its channel to be closed.
        //
        // Note: Erroring here would prevent our side of the channel closing,
        // leading to a situation where the counterparty thinks the channel is
        // closed, but we think it's still open. To avoid this inconsistency,
        // we must let the tx go through.
        //
        // We probably should delete the ACTIVE_CHANNEL, since the channel is
        // now closed... However, as we're in a catastrophic situation that
        // requires admin intervention anyways, let's leave this to the admin.
        IbcChannelCloseMsg::CloseConfirm {
            ..
        } => Ok(IbcBasicResponse::new()),
    }
}

// ----------------------------------- Tests -----------------------------------

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::{mock_dependencies, MOCK_CONTRACT_ADDR},
        IbcEndpoint,
    };

    use super::*;

    fn mock_ibc_endpoint() -> IbcEndpoint {
        IbcEndpoint {
            port_id:    format!("wasm.{MOCK_CONTRACT_ADDR}"),
            channel_id: "channel-0".into(),
        }
    }

    fn mock_ibc_channel() -> IbcChannel {
        IbcChannel::new(
            mock_ibc_endpoint(),
            mock_ibc_endpoint(),
            ics999::ORDER,
            ics999::VERSION,
            "connection-0",
        )
    }

    #[test]
    fn proper_open_init() {
        let mut deps = mock_dependencies();

        // valid channel
        {
            let res = open_init(deps.as_mut(), mock_ibc_channel()).unwrap();
            assert_eq!(res, None);
        }

        // incorrect ordering
        {
            let mut channel = mock_ibc_channel();
            channel.order = IbcOrder::Ordered;

            let err = open_init(deps.as_mut(), channel).unwrap_err();
            assert!(matches!(err, Error::IncorrectOrder { .. }));
        }

        // incorrect version
        {
            let mut channel = mock_ibc_channel();
            channel.version = "ics20".into();

            let err = open_init(deps.as_mut(), channel).unwrap_err();
            assert!(matches!(err, Error::IncorrectVersion { .. }));
        }

        // channel already exists for the connection
        {
            let channel = mock_ibc_channel();

            ACTIVE_CHANNELS
                .save(deps.as_mut().storage, &channel.connection_id, &"channel-123".into())
                .unwrap();

            let err = open_init(deps.as_mut(), channel).unwrap_err();
            assert!(matches!(err, Error::ChannelExists { .. }));
        }
    }

    #[test]
    fn proper_open_try() {
        let mut deps = mock_dependencies();

        // valid channel
        {
            let res = open_try(deps.as_mut(), mock_ibc_channel(), ics999::VERSION.into()).unwrap();
            assert_eq!(res, None);
        }

        // incorrect countarparty version
        {
            let err = open_try(deps.as_mut(), mock_ibc_channel(), "ics20".into()).unwrap_err();
            assert!(matches!(err, Error::IncorrectVersion { .. }));
        }
    }

    #[test]
    fn proper_open_connect() {
        let mut deps = mock_dependencies();

        let channel = mock_ibc_channel();

        let res = open_connect(deps.as_mut(), &channel, Some(ics999::VERSION)).unwrap();
        assert!(res.messages.is_empty());

        let active_channel = ACTIVE_CHANNELS.load(deps.as_ref().storage, &channel.connection_id).unwrap();
        assert_eq!(active_channel, channel.endpoint.channel_id);
    }

    #[test]
    fn rejecting_channel_close() {
        let err = close(IbcChannelCloseMsg::CloseInit {
            channel: mock_ibc_channel(),
        })
        .unwrap_err();
        assert_eq!(err, Error::UnexpectedChannelClosure);
    }
}
