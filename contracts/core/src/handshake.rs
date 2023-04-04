use cosmwasm_std::{
    DepsMut, IbcBasicResponse, IbcChannel, IbcChannelCloseMsg, IbcChannelOpenResponse, IbcOrder,
    Storage,
};

use ics999;

use crate::{error::ContractError, state::ACTIVE_CHANNELS};

pub fn open_init(
    deps: DepsMut,
    channel: IbcChannel,
) -> Result<IbcChannelOpenResponse, ContractError> {
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
    deps: DepsMut,
    channel: IbcChannel,
    counterparty_version: String,
) -> Result<IbcChannelOpenResponse, ContractError> {
    validate_order_and_version(&channel.order, &channel.version, Some(&counterparty_version))?;

    assert_unique_channel(deps.storage, &channel.connection_id)?;

    Ok(None)
}

pub fn open_connect(
    deps: DepsMut,
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<IbcBasicResponse, ContractError> {
    validate_order_and_version(&channel.order, &channel.version, counterparty_version)?;

    ACTIVE_CHANNELS.save(deps.storage, &channel.connection_id, &channel.endpoint.channel_id)?;

    Ok(IbcBasicResponse::new()
        .add_attribute("method", "open_connect")
        .add_attribute("connection_id", &channel.connection_id)
        .add_attribute("port_id", &channel.endpoint.port_id)
        .add_attribute("channel_id", &channel.endpoint.channel_id))
}

fn validate_order_and_version(
    order: &IbcOrder,
    version: &str,
    counterparty_version: Option<&str>,
) -> Result<(), ContractError> {
    if *order != ics999::ORDER {
        return Err(ContractError::IncorrectOrder {
            actual: order.clone(),
            expected: ics999::ORDER,
        });
    }

    if version != ics999::VERSION {
        return Err(ContractError::IncorrectVersion {
            actual: version.into(),
            expected: ics999::VERSION.into(),
        });
    }

    if let Some(cp_version) = counterparty_version {
        if cp_version != ics999::VERSION {
            return Err(ContractError::IncorrectVersion {
                actual: cp_version.into(),
                expected: ics999::VERSION.into(),
            });
        }
    }

    Ok(())
}

fn assert_unique_channel(store: &dyn Storage, connection_id: &str) -> Result<(), ContractError> {
    if ACTIVE_CHANNELS.has(store, connection_id) {
        return Err(ContractError::ChannelExists {
            connection_id: connection_id.into(),
        });
    }

    Ok(())
}

pub fn close(msg: IbcChannelCloseMsg) -> Result<IbcBasicResponse, ContractError> {
    match msg {
        // we do not expect an ICS-999 channel to be closed
        IbcChannelCloseMsg::CloseInit {
            ..
        } => Err(ContractError::UnexpectedChannelClosure),

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
