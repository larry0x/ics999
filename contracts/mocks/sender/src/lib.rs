use std::fmt;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    WasmMsg,
};
use cw_paginate::paginate_map;
use cw_storage_plus::{Bound, Item, Map};
use one_types::{Action, PacketAck};

pub const ONE_CORE: Item<Addr> = Item::new("one_core");

// we save the outcome of the packet in contract store during callbacks
// we then verify the outcomes are correct
//
// (channel_id, sequence) => PacketOutcome
pub const OUTCOMES: Map<(&str, u64), PacketOutcome> = Map::new("outcomes");

#[cw_serde]
pub enum PacketOutcome {
    Successful,
    Failed,
    TimedOut,
}

impl fmt::Display for PacketOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PacketOutcome::Successful => "successful",
            PacketOutcome::Failed => "failed",
            PacketOutcome::TimedOut => "timed_out",
        };
        write!(f, "{s}")
    }
}

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
    #[returns(OutcomeResponse)]
    Outcome {
        channel_id: String,
        sequence: u64,
    },

    /// Iterate all stored packet acknowledgements
    #[returns(Vec<OutcomeResponse>)]
    Outcomes {
        start_after: Option<(String, u64)>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct OutcomeResponse {
    channel_id: String,
    sequence: u64,
    outcome: PacketOutcome,
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
                        callback: true, // yes we always want the callback
                        timeout: None,  // use the default timeout set by one-core
                    })?,
                    funds: vec![],
                }))
        },

        ExecuteMsg::PacketCallback {
            channel_id,
            sequence,
            ack: ack_opt,
        } => {
            let outcome = match ack_opt {
                Some(ack) => match ack {
                    PacketAck::Results(_) => PacketOutcome::Successful,
                    PacketAck::Error(_) => PacketOutcome::Failed,
                },
                None => PacketOutcome::TimedOut,
            };

            OUTCOMES.save(deps.storage, (&channel_id, sequence), &outcome)?;

            Ok(Response::new()
                .add_attribute("method", "packet_callback")
                .add_attribute("channel_id", channel_id)
                .add_attribute("sequence", sequence.to_string())
                .add_attribute("outcome", outcome.to_string()))
        },
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Outcome {
            channel_id,
            sequence,
        } => {
            let res = OutcomeResponse {
                outcome: OUTCOMES.load(deps.storage, (&channel_id, sequence))?,
                channel_id,
                sequence,
            };
            to_binary(&res)
        },

        QueryMsg::Outcomes {
            start_after,
            limit,
        } => {
            let start = start_after
                .as_ref()
                .map(|(chan_id, seq)| Bound::exclusive((chan_id.as_str(), *seq)));
            let res = paginate_map(
                &OUTCOMES,
                deps.storage,
                start,
                limit,
                |(channel_id, sequence), outcome| -> StdResult<_> {
                    Ok(OutcomeResponse {
                        channel_id,
                        sequence,
                        outcome,
                    })
                },
            )?;
            to_binary(&res)
        },
    }
}
