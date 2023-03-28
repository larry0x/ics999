use cosmwasm_std::{
    from_slice, to_binary, Binary, DepsMut, Env, IbcBasicResponse, IbcMsg, IbcPacket, IbcTimeout,
    MessageInfo, Response, SubMsg, WasmMsg, IbcEndpoint,
};
use one_types::{Action, PacketData, SenderExecuteMsg};
use token_factory::TokenFactoryMsg;

use crate::{
    error::ContractError,
    state::{ACTIVE_CHANNELS, DEFAULT_TIMEOUT_SECS, DENOM_TRACES},
    transfer::{burn, escrow, TraceItem},
    utils::{Coins, query_port},
    AFTER_CALLBACK,
};

pub fn act(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    connection_id: String,
    actions: Vec<Action>,
    timeout: Option<IbcTimeout>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let received_funds = Coins::from(info.funds);
    let mut sending_funds = Coins::empty();
    let mut msgs = vec![];
    let mut attrs = vec![];
    let mut traces = vec![];

    // find the current chain's port and channel IDs
    let localhost = IbcEndpoint {
        port_id: query_port(&deps.querier)?,
        channel_id: ACTIVE_CHANNELS.load(deps.storage, &connection_id)?,
    };

    // go through all transfer actions, either escrow or burn the coins based on
    // whether the current chain is the source or the sink.
    // also, compose the traces which will be included in the packet.
    for action in &actions {
        if let Action::Transfer {
            amount,
            ..
        } = action
        {
            // load the denom's trace
            // if there isn't a trace stored for this denom, then the current
            // chain must be the source. in this case we initialize a new trace
            let trace = DENOM_TRACES
                .may_load(deps.storage, &amount.denom)?
                .unwrap_or_else(|| TraceItem::new(&amount.denom, &localhost));

            if trace.is_source(&localhost) {
                // current chain is the sink -- burn voucher token
                burn(amount.clone(), &info.sender, &mut msgs, &mut attrs);
            } else {
                // current chain is the source -- escrow
                escrow(amount, &mut attrs);
            }

            traces.push(trace.into_full_trace(&amount.denom));
            sending_funds.add(amount.clone())?;
        }
    }

    // the total amount of coins the user has sent to the contract must equal
    // the amount they want to transfer via IBC
    if received_funds != sending_funds {
        return Err(ContractError::FundsMismatch {
            actual: received_funds,
            expected: sending_funds,
        });
    }

    // if the user does not specify a timeout, we use the default
    let timeout = match timeout {
        None => {
            let default_secs = DEFAULT_TIMEOUT_SECS.load(deps.storage)?;
            IbcTimeout::with_timestamp(env.block.time.plus_seconds(default_secs))
        },
        Some(to) => to,
    };

    Ok(Response::new()
        .add_messages(msgs)
        .add_message(IbcMsg::SendPacket {
            channel_id: localhost.channel_id,
            data: to_binary(&PacketData {
                sender: info.sender.into(),
                actions,
                traces,
            })?,
            timeout,
        })
        .add_attribute("action", "act")
        .add_attributes(attrs))
}

pub fn packet_lifecycle_complete(
    packet: IbcPacket,
    ack_bin: Option<Binary>,
) -> Result<IbcBasicResponse, ContractError> {
    // deserialize the original packet
    let packet_data: PacketData = from_slice(&packet.data)?;

    // deserialize the ack
    let ack = ack_bin.map(|bin| from_slice(&bin)).transpose()?;

    // TODO: refund escrowed tokens if the packet failed or timed out

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "packet_lifecycle_complete")
        .add_attribute("channel_id", &packet.src.channel_id)
        .add_attribute("sequence", packet.sequence.to_string())
        .add_attribute("acknowledged", ack.is_some().to_string())
        .add_attribute("sender", &packet_data.sender)
        .add_submessage(SubMsg::reply_always(
            WasmMsg::Execute {
                contract_addr: packet_data.sender,
                msg: to_binary(&SenderExecuteMsg::PacketCallback {
                    channel_id: packet.src.channel_id,
                    sequence: packet.sequence,
                    ack,
                })?,
                funds: vec![],
            },
            AFTER_CALLBACK,
        )))
}

pub fn after_callback(success: bool) -> Result<Response<TokenFactoryMsg>, ContractError> {
    Ok(Response::new()
        .add_attribute("action", "after_callback")
        .add_attribute("success", success.to_string()))
}
