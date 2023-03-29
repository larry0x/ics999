use cosmwasm_std::{
    from_slice, to_binary, Binary, Coin, CustomQuery, Deps, DepsMut, Env, IbcBasicResponse,
    IbcEndpoint, IbcMsg, IbcPacket, IbcTimeout, MessageInfo, Response, StdResult, Storage, SubMsg,
    WasmMsg,
};
use one_types::{Action, PacketAck, PacketData, SenderExecuteMsg};
use token_factory::TokenFactoryMsg;

use crate::{
    error::ContractError,
    state::{ACTIVE_CHANNELS, DEFAULT_TIMEOUT_SECS, DENOM_TRACES},
    transfer::{burn, escrow, mint, release, TraceItem},
    utils::{query_port, Coins},
    AFTER_CALLBACK,
};

pub fn act<Q: CustomQuery>(
    deps: DepsMut<Q>,
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
    let localhost = localhost(deps.as_ref(), &connection_id)?;

    // go through all transfer actions, either escrow or burn the coins based on
    // whether the current chain is the source or the sink.
    // also, compose the traces which will be included in the packet.
    for action in &actions {
        if let Action::Transfer { denom, amount, .. } = action {
            let trace = trace_of(deps.storage, &denom)?;

            let coin = Coin {
                denom: denom.clone(),
                amount: *amount,
            };

            if trace.sender_is_source(&localhost) {
                escrow(&coin, &mut attrs);
            } else {
                burn(coin.clone(), &info.sender, &mut msgs, &mut attrs);
            }

            traces.push(trace.into_full_trace(&denom));
            sending_funds.add(coin)?;
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
    }))
}

pub fn packet_lifecycle_complete(
    deps: DepsMut,
    packet: IbcPacket,
    ack_bin: Option<Binary>,
) -> Result<IbcBasicResponse<TokenFactoryMsg>, ContractError> {
    let mut msgs = vec![];
    let mut attrs = vec![];

    // deserialize the original packet
    let packet_data: PacketData = from_slice(&packet.data)?;

    // deserialize the ack
    let ack = ack_bin.map(|bin| from_slice(&bin)).transpose()?;

    // process refund if the packet timed out or failed
    if should_refund(&ack) {
        for action in &packet_data.actions {
            if let Action::Transfer { denom, amount, .. } = action {
                let trace = trace_of(deps.storage, &denom)?;

                let coin = Coin {
                    denom: denom.clone(),
                    amount: *amount,
                };

                // do the reverse of what was done in `act`
                // if the tokens were escrowed, then release them
                // if the tokens were burned, then mint them
                if trace.sender_is_source(&packet.src) {
                    release(coin, &packet_data.sender, &mut msgs, &mut attrs);
                } else {
                    mint(coin, &packet_data.sender, &mut msgs, &mut attrs);
                }
            }
        }
    }

    Ok(IbcBasicResponse::new()
        .add_attribute("method", "packet_lifecycle_complete")
        .add_attribute("channel_id", &packet.src.channel_id)
        .add_attribute("sequence", packet.sequence.to_string())
        .add_attribute("acknowledged", ack.is_some().to_string())
        .add_attribute("sender", &packet_data.sender)
        .add_attributes(attrs)
        .add_messages(msgs)
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

// this method must succeed whether the callback was successful or not
// if the callback failed, we simply log it here
pub fn after_callback(success: bool) -> Result<Response<TokenFactoryMsg>, ContractError> {
    Ok(Response::new()
        .add_attribute("method", "after_callback")
        .add_attribute("success", success.to_string()))
}

/// Find the trace associated with a denom.
///
/// If there isn't a trace stored for this denom, then the current chain must be
/// the source. In this case, initialize a new trace with the current chain
/// being the first and only step in the path.
fn trace_of(store: &dyn Storage, denom: &str) -> StdResult<TraceItem> {
    Ok(DENOM_TRACES
        .may_load(store, &denom)?
        .unwrap_or_else(|| TraceItem::new(&denom)))
}

fn localhost<Q: CustomQuery>(deps: Deps<Q>, connection_id: &str) -> StdResult<IbcEndpoint> {
    Ok(IbcEndpoint {
        port_id: query_port(&deps.querier)?,
        channel_id: ACTIVE_CHANNELS.load(deps.storage, connection_id)?,
    })
}

fn should_refund(ack: &Option<PacketAck>) -> bool {
    match ack {
        // packet timed out -- refund
        None => true,

        // packet acknowledged but errored -- refund
        Some(PacketAck::Error(_)) => true,

        // packet acknowledged and succeeded -- no refund
        Some(PacketAck::Results(_)) => false,
    }
}
