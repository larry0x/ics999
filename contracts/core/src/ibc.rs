use cosmwasm_std::{
    instantiate2_address, to_binary, Binary, ChannelResponse, DepsMut, Empty, Env, IbcBasicResponse,
    IbcChannel, IbcChannelCloseMsg, IbcChannelOpenResponse, IbcOrder, IbcQuery, IbcReceiveResponse,
    PortIdResponse, QuerierWrapper, QueryRequest, Storage, SubMsg, WasmMsg,
};
use one_types::{Action, Packet};

use crate::{
    error::{ContractError, ContractResult},
    state::{ACCOUNTS, ACCOUNT_CODE_ID, ACTIVE_CHANNELS, CURRENT_PACKET},
};

pub const ACTION_REPLY_ID: u64 = 1;

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

pub fn close(msg: IbcChannelCloseMsg) -> ContractResult<IbcBasicResponse> {
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

pub fn packet_receive(
    deps: DepsMut,
    env: Env,
    channel_id: String,
    mut packet: Packet,
) -> ContractResult<IbcReceiveResponse> {
    let connection_id = connection_of_channel(&deps.querier, &channel_id)?;

    let ica = ACCOUNTS.may_load(deps.storage, (&connection_id, &packet.sender))?;

    // take out the first action (which we execute right now)
    // the rest of the actions are to be executed in the reply
    let action = packet.actions.remove(0);

    let msg = match action {
        Action::Transfer {
            amount: _,
            recipient: _,
        } => todo!("fungible token transfer is not implemented yet"),

        Action::RegisterAccount {
            salt,
        } => {
            // only one ICA per controller allowed
            if ica.is_some() {
                return Err(ContractError::AccountExists {
                    connection_id,
                    controller: packet.sender,
                })?;
            }

            // if a salt is not provided, use the controller account's UTF-8
            // bytes by default
            let salt = salt.unwrap_or_else(|| default_salt(&connection_id, &packet.sender));

            // load the one-account contract's code ID and checksum, which is
            // used in Instantiate2 to determine the contract address
            let code_id = ACCOUNT_CODE_ID.load(deps.storage)?;
            let code_res = deps.querier.query_wasm_code_info(code_id)?;

            // predict the contract address
            let addr_raw = instantiate2_address(
                &code_res.checksum,
                &deps.api.addr_canonicalize(env.contract.address.as_str())?,
                &salt,
            )?;
            let addr = deps.api.addr_humanize(&addr_raw)?;

            ACCOUNTS.save(deps.storage, (&connection_id, &packet.sender), &addr)?;

            WasmMsg::Instantiate2 {
                admin: Some(env.contract.address.into()),
                code_id,
                label: format!("one-account/{connection_id}/{}", packet.sender),
                msg: to_binary(&Empty {})?,
                funds: vec![],
                salt,
            }
        },

        Action::Execute(wasm_msg) => {
            let Some(addr) = ica else {
                return Err(ContractError::AccountNotFound {
                    connection_id,
                    controller: packet.sender,
                });
            };

            let funds = {
                // TODO: convert funds to their corresponding ibc denoms
                vec![]
            };

            WasmMsg::Execute {
                contract_addr: addr.into(),
                msg: to_binary(&wasm_msg)?,
                funds,
            }
        },
    };

    // save the rest of the action queue, to be handled during reply
    CURRENT_PACKET.save(deps.storage, &packet)?;

    Ok(IbcReceiveResponse::new()
        .add_submessage(SubMsg::reply_always(msg, ACTION_REPLY_ID))
        .add_attribute("action", "packet_receive")
        .add_attribute("sender", packet.sender)
        .add_attribute("actions_left", packet.actions.len().to_string()))
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

/// Query the connection ID associated with the specified channel
fn connection_of_channel(querier: &QuerierWrapper, channel_id: &str) -> ContractResult<String> {
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

/// Generate a salt to be used in Instantiate2, if the user does not provide one.
///
/// The salt is the UTF-8 bytes of the connection ID and controller address,
/// concatenated. This ensures unique salt for each {connection, controller} pair.
fn default_salt(connection_id: &str, controller: &str) -> Binary {
    let mut bytes = vec![];
    bytes.extend(connection_id.as_bytes());
    bytes.extend(controller.as_bytes());
    bytes.into()
}
