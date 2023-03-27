use cosmwasm_std::{
    from_slice, to_binary, Binary, DepsMut, Env, IbcBasicResponse, IbcMsg, IbcPacket, IbcTimeout,
    MessageInfo, Response, SubMsg, WasmMsg,
};
use one_types::{Action, PacketData, SenderExecuteMsg, Trace};
use token_factory::TokenFactoryMsg;

use crate::{
    error::ContractError,
    state::{ACTIVE_CHANNELS, DEFAULT_TIMEOUT_SECS, DENOM_TRACES},
    transfer::{burn, escrow},
    utils::Coins,
    AFTER_CALLBACK,
};

pub fn act(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    connection_id: String,
    actions: Vec<Action>,
    opt_timeout: Option<IbcTimeout>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let received_funds = Coins::from(info.funds);
    let mut sending_funds = Coins::empty();
    let mut msgs = vec![];
    let mut attrs = vec![];
    let mut traces = vec![];

    for action in &actions {
        if let Action::Transfer {
            amount,
            ..
        } = action
        {
            // if the denom has a trace stored, then the current chain is the
            // source.
            // the last element of the trace must be the current chain, but no
            // need to verify it here.
            match DENOM_TRACES.may_load(deps.storage, &amount.denom)? {
                // current chain is the sink -- burn voucher token
                Some(trace) => {
                    traces.push(Trace {
                        denom: amount.denom.clone(),
                        base_denom: trace.base_denom,
                        path: trace.path,
                    });
                    burn(amount.clone(), &info.sender, &mut msgs, &mut attrs);
                },

                // current chain is the source -- escrow
                None => {
                    escrow(amount, &mut attrs);
                },
            }

            sending_funds.add(amount.clone())?;
        }
    }

    if received_funds != sending_funds {
        return Err(ContractError::FundsMismatch {
            actual: received_funds,
            expected: sending_funds,
        });
    }

    let timeout = match opt_timeout {
        None => {
            let default_secs = DEFAULT_TIMEOUT_SECS.load(deps.storage)?;
            IbcTimeout::with_timestamp(env.block.time.plus_seconds(default_secs))
        },
        Some(to) => to,
    };

    Ok(Response::new()
        .add_messages(msgs)
        .add_message(IbcMsg::SendPacket {
            channel_id: ACTIVE_CHANNELS.load(deps.storage, &connection_id)?,
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
