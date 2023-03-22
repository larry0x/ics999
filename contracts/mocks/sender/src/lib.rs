use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, IbcTimeout, MessageInfo, Response,
    StdResult, Timestamp, WasmMsg,
};
use cw_paginate::paginate_map;
use cw_storage_plus::{Bound, Item, Map};
use one_types::{Action, PacketAck};

pub const ONE_CORE: Item<Addr> = Item::new("one_core");

// (channel_id, sequence) => ack
pub const ACKS: Map<(&str, u64), PacketAck> = Map::new("acks");

#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the one-core contract
    one_core: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Send some actions to a remote chain via one-core
    Send {
        connection_id: String,
        actions: Vec<Action>,
    },

    /// Respond to packet ack or timeout. Required by one-core.
    PacketCallback {
        channel_id: String,
        sequence: u64,
        ack: Option<PacketAck>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Query a single packet acknowledgement
    #[returns(AckResponse)]
    Ack {
        channel_id: String,
        sequence: u64,
    },

    /// Iterate all stored packet acknowledgements
    #[returns(Vec<AckResponse>)]
    Acks {
        start_after: Option<(String, u64)>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct AckResponse {
    channel_id: String,
    sequence: u64,
    ack: PacketAck,
}

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let one_core_addr = deps.api.addr_validate(&msg.one_core)?;
    ONE_CORE.save(deps.storage, &one_core_addr)?;

    Ok(Response::new())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Send {
            connection_id,
            actions,
        } => {
            let one_core_addr = ONE_CORE.load(deps.storage)?;

            Ok(Response::new()
                .add_attribute("method", "send")
                .add_attribute("connection_id", &connection_id)
                .add_attribute("num_actions", actions.len().to_string())
                .add_message(WasmMsg::Execute {
                    contract_addr: one_core_addr.into(),
                    msg: to_binary(&one_core::msg::ExecuteMsg::Act {
                        connection_id,
                        actions,
                        callback: true,
                        timeout: IbcTimeout::with_timestamp(Timestamp::from_seconds(9999999999)), // FIXME: change to None
                    })?,
                    funds: vec![],
                }))
        },

        ExecuteMsg::PacketCallback {
            channel_id,
            sequence,
            ack: ack_opt,
        } => {
            if let Some(ack) = &ack_opt {
                ACKS.save(deps.storage, (&channel_id, sequence), ack)?;
            }

            Ok(Response::new()
                .add_attribute("method", "packet_callback")
                .add_attribute("channel_id", channel_id)
                .add_attribute("sequence", sequence.to_string())
                .add_attribute("acknowledged", ack_opt.is_some().to_string()))
        },
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ack {
            channel_id,
            sequence,
        } => {
            let res = AckResponse {
                ack: ACKS.load(deps.storage, (&channel_id, sequence))?,
                channel_id,
                sequence,
            };
            to_binary(&res)
        },
        QueryMsg::Acks {
            start_after,
            limit,
        } => {
            let start = start_after
                .as_ref()
                .map(|(chan_id, seq)| Bound::exclusive((chan_id.as_str(), *seq)));
            let res = paginate_map(
                &ACKS,
                deps.storage,
                start,
                limit,
                |(channel_id, sequence), ack| -> StdResult<_> {
                    Ok(AckResponse {
                        channel_id,
                        sequence,
                        ack,
                    })
                },
            )?;
            to_binary(&res)
        },
    }
}
