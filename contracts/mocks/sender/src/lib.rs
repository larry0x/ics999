use {
    cosmwasm_schema::{cw_serde, QueryResponses},
    cosmwasm_std::{
        coin, entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, IbcEndpoint, MessageInfo,
        OverflowError, Response, StdResult, WasmMsg,
    },
    cw_paginate::paginate_map,
    cw_storage_plus::{Bound, Item, Map},
    ics999::{Action, CallbackMsg, PacketOutcome},
    one_core::utils::Coins,
};

pub const ONE_CORE: Item<Addr> = Item::new("one_core");

// we save the outcome of the packet in contract store during callbacks
// we then verify the outcomes are correct
//
// (port_id, channel_id, sequence) => outcome
pub const OUTCOMES: Map<(&str, &str, u64), PacketOutcome> = Map::new("outcomes");

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
        actions:       Vec<Action>,
    },

    /// Respond to packet ack or timeout. Required by one-core.
    Ics999(CallbackMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Query a single packet acknowledgement
    #[returns(OutcomeResponse)]
    Outcome(OutcomeKey),

    /// Iterate all stored packet acknowledgements
    #[returns(Vec<OutcomeResponse>)]
    Outcomes {
        start_after: Option<OutcomeKey>,
        limit:       Option<u32>,
    },
}

#[cw_serde]
pub struct OutcomeKey {
    pub dest:     IbcEndpoint,
    pub sequence: u64,
}

#[cw_serde]
pub struct OutcomeResponse {
    pub dest:     IbcEndpoint,
    pub sequence: u64,
    pub outcome:  PacketOutcome,
}

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _:    Env,
    _:    MessageInfo,
    msg:  InstantiateMsg,
) -> StdResult<Response> {
    let one_core_addr = deps.api.addr_validate(&msg.one_core)?;
    ONE_CORE.save(deps.storage, &one_core_addr)?;

    Ok(Response::new())
}

#[entry_point]
pub fn execute(deps: DepsMut, _: Env, _: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Send {
            connection_id,
            actions,
        } => {
            let one_core_addr = ONE_CORE.load(deps.storage)?;

            // compute the total amount of coins to be sent to one-core
            // must equal to the sum of all the amounts in transfer actions
            let funds = actions.iter().try_fold(
                Coins::empty(),
                |mut funds, action| -> Result<_, OverflowError> {
                    if let Action::Transfer { denom, amount, .. } = action {
                        funds.add(coin(amount.u128(), denom))?;
                    }
                    Ok(funds)
                },
            )?;

            Ok(Response::new()
                .add_attribute("method", "send")
                .add_attribute("connection_id", &connection_id)
                .add_attribute("num_actions", actions.len().to_string())
                .add_message(WasmMsg::Execute {
                    contract_addr: one_core_addr.into(),
                    msg: to_binary(&one_core::msg::ExecuteMsg::Act {
                        connection_id,
                        actions,
                        timeout: None, // use the default timeout set by one-core
                    })?,
                    funds: funds.into(),
                }))
        },

        ExecuteMsg::Ics999(CallbackMsg {
            endpoint,
            sequence,
            outcome,
        }) => {
            OUTCOMES.save(deps.storage, (&endpoint.port_id, &endpoint.channel_id, sequence), &outcome)?;

            Ok(Response::new()
                .add_attribute("method", "packet_callback")
                .add_attribute("port_id", endpoint.port_id)
                .add_attribute("channel_id", endpoint.channel_id)
                .add_attribute("sequence", sequence.to_string())
                .add_attribute("outcome", outcome.ty()))
        },
    }
}

#[entry_point]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Outcome(OutcomeKey {
            dest,
            sequence,
        }) => {
            let res = OutcomeResponse {
                outcome: OUTCOMES.load(deps.storage, (&dest.port_id, &dest.channel_id, sequence))?,
                dest,
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
                .map(|OutcomeKey { dest, sequence }| {
                    Bound::exclusive((dest.port_id.as_str(), dest.channel_id.as_str(), *sequence))
                });

            let res = paginate_map(
                &OUTCOMES,
                deps.storage,
                start,
                limit,
                |(port_id, channel_id, sequence), outcome| -> StdResult<_> {
                    Ok(OutcomeResponse {
                        dest: IbcEndpoint { port_id, channel_id },
                        sequence,
                        outcome,
                    })
                },
            )?;

            to_binary(&res)
        },
    }
}
