use cosmwasm_std::{
    from_slice, to_binary, ChannelResponse, DepsMut, Env, IbcBasicResponse, IbcChannel,
    IbcChannelCloseMsg, IbcChannelOpenResponse, IbcOrder, IbcPacket, IbcQuery, IbcReceiveResponse,
    PortIdResponse, QuerierWrapper, QueryRequest, Response, Storage, SubMsg, SubMsgResponse,
    SubMsgResult, WasmMsg,
};
use cw_utils::parse_execute_response_data;
use one_types::{PacketAck, PacketData};

use crate::{error::ContractError, msg::ExecuteMsg, state::ACTIVE_CHANNELS, AFTER_ALL_ACTIONS};

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
        .add_attribute("action", "open_connect")
        .add_attribute("connection_id", &channel.connection_id)
        .add_attribute("port_id", &channel.endpoint.port_id)
        .add_attribute("channel_id", &channel.endpoint.channel_id))
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

pub fn packet_receive(
    deps: DepsMut,
    env: Env,
    packet: IbcPacket,
) -> Result<IbcReceiveResponse, ContractError> {
    // find the connection ID corresponding to the sender channel
    let connection_id = connection_of_channel(&deps.querier, &packet.src.channel_id)?;

    // deserialize packet data
    let pd: PacketData = from_slice(&packet.data)?;

    // we don't add an ack in this response
    // the ack will be added in after_all_actions reply (see below)
    Ok(IbcReceiveResponse::new()
        .add_attribute("action", "packet_receive")
        .add_attribute("connection_id", &connection_id)
        .add_attribute("channel_id", &packet.src.channel_id)
        .add_attribute("sequence", packet.sequence.to_string())
        .add_submessage(SubMsg::reply_always(
            WasmMsg::Execute {
                contract_addr: env.contract.address.into(),
                msg: to_binary(&ExecuteMsg::Handle {
                    connection_id,
                    controller: pd.sender,
                    actions: pd.actions,
                })?,
                funds: vec![],
            },
            AFTER_ALL_ACTIONS,
        )))
}

pub fn after_all_actions(res: SubMsgResult) -> Result<Response, ContractError> {
    let ack = match &res {
        // all actions were successful - write an Success ack
        SubMsgResult::Ok(SubMsgResponse {
            data,
            ..
        }) => {
            let execute_res_bin = data.as_ref().expect("missing execute response data");
            let execute_res = parse_execute_response_data(execute_res_bin)?;

            let action_res_bin = execute_res.data.expect("missing action results data");
            let action_res = from_slice(&action_res_bin)?;

            PacketAck::Result(action_res)
        },

        // one of actions failed - write an Error ack
        SubMsgResult::Err(err) => PacketAck::Error(err.clone()),
    };

    Ok(Response::new()
        .add_attribute("action", "after_all_actions")
        .add_attribute("success", res.is_ok().to_string())
        // wasmd will interpret this data field as the ack, overriding the ack
        // emitted in the ibc_packet_receive entry point
        .set_data(to_binary(&ack)?))
}

fn validate_order_and_version(
    order: &IbcOrder,
    version: &str,
    counterparty_version: Option<&str>,
) -> Result<(), ContractError> {
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

fn assert_unique_channel(store: &dyn Storage, connection_id: &str) -> Result<(), ContractError> {
    if ACTIVE_CHANNELS.has(store, connection_id) {
        return Err(ContractError::ChannelExists {
            connection_id: connection_id.into(),
        });
    }

    Ok(())
}

/// Query the connection ID associated with the specified channel
fn connection_of_channel(
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
