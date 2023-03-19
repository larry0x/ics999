use cosmwasm_std::{
    DepsMut, IbcBasicResponse, IbcChannel, IbcChannelOpenResponse, IbcOrder, Storage,
};

use crate::{
    error::{ContractError, ContractResult},
    state::ACTIVE_CHANNELS,
};

pub fn open_init(deps: DepsMut, channel: IbcChannel) -> ContractResult<IbcChannelOpenResponse> {
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
) -> ContractResult<IbcChannelOpenResponse> {
    validate_order_and_version(&channel.order, &channel.version, Some(&counterparty_version))?;

    assert_unique_channel(deps.storage, &channel.connection_id)?;

    Ok(None)
}

pub fn open_connect(
    deps: DepsMut,
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> ContractResult<IbcBasicResponse> {
    validate_order_and_version(&channel.order, &channel.version, counterparty_version)?;

    ACTIVE_CHANNELS.save(deps.storage, &channel.connection_id, &channel.endpoint.channel_id)?;

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "open_connect")
        .add_attribute("connection_id", &channel.connection_id)
        .add_attribute("port_id", &channel.endpoint.port_id)
        .add_attribute("channel_id", &channel.endpoint.channel_id))
}

fn validate_order_and_version(
    order: &IbcOrder,
    version: &str,
    counterparty_version: Option<&str>,
) -> ContractResult<()> {
    if *order != one_types::ORDER {
        return Err(ContractError::IncorrectOrder {
            actual: order.clone(),
            expected: one_types::ORDER,
        });
    }

    if version != one_types::VERSION {
        return Err(ContractError::IncorrectVersion {
            actual: version.into(),
            expected: one_types::VERSION.into(),
        });
    }

    if let Some(cp_version) = counterparty_version {
        if cp_version != one_types::VERSION {
            return Err(ContractError::IncorrectVersion {
                actual: cp_version.into(),
                expected: one_types::VERSION.into(),
            });
        }
    }

    Ok(())
}

fn assert_unique_channel(store: &dyn Storage, connection_id: &str) -> ContractResult<()> {
    if ACTIVE_CHANNELS.has(store, connection_id) {
        return Err(ContractError::ChannelExists {
            connection_id: connection_id.into(),
        });
    }

    Ok(())
}
