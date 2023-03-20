use cosmwasm_std::{
    to_binary, ChannelResponse, DepsMut, Env, IbcBasicResponse, IbcChannel, IbcChannelCloseMsg,
    IbcChannelOpenResponse, IbcOrder, IbcQuery, IbcReceiveResponse, PortIdResponse, QuerierWrapper,
    QueryRequest, Response, Storage, SubMsgResponse, SubMsgResult,
};
use one_types::{Acknowledgment, Packet};

use crate::{
    error::{ContractError, ContractResult},
    handler::{Handler, HANDLER},
    state::{ACCOUNTS, ACTIVE_CHANNELS},
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
    // find the connection ID corresponding to the sender channel
    let connection_id = connection_of_channel(&deps.querier, &channel_id)?;

    // load the sender's interchain account
    let host = ACCOUNTS.may_load(deps.storage, (&connection_id, &packet.sender))?;

    // reverse the order of actions, so that we can use pop() to fetch the first
    // action from the queue
    packet.actions.reverse();

    let handler = Handler {
        connection_id,
        controller: packet.sender,
        host,
        action: None,
        pending_actions: packet.actions,
        results: vec![],
    };

    handler.handle_next_action(deps, env).map(into_ibc_receive_resp)
}

pub fn after_action(deps: DepsMut, env: Env, res: SubMsgResult) -> ContractResult<Response> {
    match res {
        SubMsgResult::Ok(SubMsgResponse {
            // we don't include events in the acknowledgement, because events
            // are not part of the block result, i.e. not reached consensus by
            // validators. there is no guarantee that events are deterministic
            // (see one of the Juno chain halt exploits).
            //
            // in princle, contracts should only have access to data that have
            // reached consensus by validators.
            events: _,
            data,
        }) => {
            let mut handler = HANDLER.load(deps.storage)?;

            // parse the result of the previous action
            handler.add_result(data)?;

            // handle the next action
            //
            // if there is no more action to be executed, this will wite the ack
            // and return
            handler.handle_next_action(deps, env)
        },

        SubMsgResult::Err(err) => {
            // delete the handler as it's no longer needed
            HANDLER.remove(deps.storage);

            // return an err ack
            Ok(Response::new()
                .add_attribute("action", "action_failed")
                .add_attribute("error", &err)
                // wasmd will save this data as the packet ack
                .set_data(to_binary(&Acknowledgment::Err(err))?))
        },
    }
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

/// Convert a Response to an IbcReceiveResponse
fn into_ibc_receive_resp(resp: Response) -> IbcReceiveResponse {
    IbcReceiveResponse::new()
        .add_submessages(resp.messages)
        .add_attributes(resp.attributes)
        .add_events(resp.events)
}
