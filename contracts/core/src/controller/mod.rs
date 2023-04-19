use cosmwasm_std::{
    from_slice, to_binary, Addr, Attribute, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    IbcBasicResponse, IbcEndpoint, IbcMsg, IbcPacket, IbcTimeout, MessageInfo, Response, StdResult,
    Storage, SubMsg, WasmMsg,
};

use ics999::{Action, PacketAck, PacketData, RelayerFee, SenderExecuteMsg, Trace};

use crate::{
    error::ContractError,
    state::{ACTIVE_CHANNELS, DEFAULT_TIMEOUT_SECS, DENOM_TRACES},
    transfer::{burn, escrow, mint, release, TraceItem},
    utils::{query_port, Coins},
    AFTER_CALLBACK,
};

pub fn act(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    connection_id: String,
    actions: Vec<Action>,
    timeout: Option<IbcTimeout>,
    relayer_fee: RelayerFee,
) -> Result<Response, ContractError> {
    let received_funds = Coins::from(info.funds);
    let mut expect_funds = Coins::empty();
    let mut msgs = vec![];
    let mut attrs = vec![];
    let mut traces: Vec<Trace> = vec![];

    // find the current chain's port and channel IDs
    let localhost = localhost(deps.as_ref(), &connection_id)?;

    // go through all transfer actions, either escrow or burn the coins based on
    // whether the current chain is the source or the sink.
    // also, compose the traces which will be included in the packet.
    for action in &actions {
        if let Action::Transfer { denom, amount, .. } = action {
            let trace = trace_of(deps.storage, denom)?;

            let coin = Coin {
                denom: denom.clone(),
                amount: *amount,
            };

            if trace.sender_is_source(&localhost) {
                escrow(&coin, &mut attrs);
            } else {
                // note that we burn from the contract address instead of from
                // info.sender
                // this is because the token to be burned should have already
                // been sent to the contract address along with the executeMsg
                burn(&env.contract.address, coin.clone(), &mut msgs, &mut attrs);
            }

            if !contains_denom(&traces, denom) {
                traces.push(trace.into_full_trace(denom));
            }

            expect_funds.add(coin)?;
        }
    }

    // handle the destination relayer fee
    //
    // destination relayer refers to the account who submits the RecvPacket
    // message on the destination chain.
    //
    // different from ICS-29, the relayer will receive the fee on the des chain.
    // for this to work, the fee token's trace must be included in the packet.
    if let Some(fee) = &relayer_fee.dest {
        if !contains_denom(&traces, &fee.denom) {
            let trace = trace_of(deps.storage, &fee.denom)?;
            traces.push(trace.into_full_trace(&fee.denom));
        }

        expect_funds.add(fee.clone())?;
    }

    // handle the source relayer fee
    //
    // source relayer refers to the account who submits either the Ack or
    // Timeout message on the source chain.
    //
    // the packet does not need to include the trace for this token.
    if let Some(fee) = &relayer_fee.src {
        expect_funds.add(fee.clone())?;
    }

    // the total amount of coins the user has sent to the contract must equal
    // the amount they want to transfer via IBC + fees to be paid
    if received_funds != expect_funds {
        return Err(ContractError::FundsMismatch {
            actual: received_funds,
            expected: expect_funds,
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
        .add_attributes(attrs)
        .add_messages(msgs)
        .add_message(IbcMsg::SendPacket {
            channel_id: localhost.channel_id,
            data: to_binary(&PacketData {
                sender: info.sender.into(),
                actions,
                traces,
                relayer_fee,
            })?,
            timeout,
    }))
}

pub fn packet_lifecycle_complete(
    deps: DepsMut,
    env: Env,
    packet: IbcPacket,
    ack_bin: Option<Binary>,
    relayer: Addr,
) -> Result<IbcBasicResponse, ContractError> {
    let mut msgs = vec![];
    let mut attrs = vec![];

    // deserialize the original packet
    let packet_data: PacketData = from_slice(&packet.data)?;

    // deserialize the ack
    let ack = ack_bin.as_ref().map(|bin| from_slice(bin)).transpose()?;

    // process refund if the packet timed out or failed
    if should_refund(&ack) {
        for action in &packet_data.actions {
            if let Action::Transfer { denom, amount, .. } = action {
                let trace = trace_of(deps.storage, denom)?;

                let coin = Coin {
                    denom: denom.clone(),
                    amount: *amount,
                };

                refund(
                    &env.contract.address,
                    &packet_data.sender,
                    coin,
                    &trace,
                    &packet.src,
                    &mut msgs,
                    &mut attrs,
                );
            }
        }
    }

    // if the packet timed out, refund the destination relayer fee
    if let Some(fee) = &packet_data.relayer_fee.dest {
        if ack_bin.is_none() {
            refund(
                &env.contract.address,
                &packet_data.sender,
                fee.clone(),
                &trace_of(deps.storage, &fee.denom)?,
                &packet.src,
                &mut msgs,
                &mut attrs,
            );
        }
    }

    // pay the source relayer fee
    if let Some(fee) = &packet_data.relayer_fee.src {
        refund(
            &env.contract.address,
            relayer,
            fee.clone(),
            &trace_of(deps.storage, &fee.denom)?,
            &packet.src,
            &mut msgs,
            &mut attrs,
        );
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
pub fn after_callback(success: bool) -> Result<Response, ContractError> {
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
        .may_load(store, denom)?
        .unwrap_or_else(|| TraceItem::new(denom)))
}

fn contains_denom(traces: &[Trace], denom: &str) -> bool {
    traces.iter().any(|trace| trace.denom == *denom)
}

fn localhost(deps: Deps, connection_id: &str) -> StdResult<IbcEndpoint> {
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

fn refund(
    contract: impl Into<String>,
    sender: impl Into<String>,
    coin: Coin,
    trace: &TraceItem,
    src: &IbcEndpoint,
    msgs: &mut Vec<CosmosMsg>,
    attrs: &mut Vec<Attribute>,
) {
    if trace.sender_is_source(src) {
        release(coin, sender, msgs, attrs);
    } else {
        mint(contract, sender, coin,  msgs, attrs);
    }
}

// ----------------------------------- Tests -----------------------------------

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Uint128,
    };

    use super::*;

    #[test]
    fn asserting_funds() {
        struct TestCase {
            sending_funds: Vec<Coin>,
            should_ok: bool,
        }

        // this contains the correct amount of coins expected to be sent
        let actions = vec![
            Action::Transfer {
                denom: "uatom".into(),
                amount: Uint128::new(10000),
                recipient: None,
            },
            Action::Transfer {
                denom: "uosmo".into(),
                amount: Uint128::new(23456),
                recipient: None,
            },
            Action::Transfer {
                denom: "uatom".into(),
                amount: Uint128::new(2345),
                recipient: Some("pumpkin".into()),
            },
        ];

        let testcases = [
            // no fund sent
            TestCase {
                sending_funds: vec![],
                should_ok: false,
            },

            // only 1 coin sent
            TestCase {
                sending_funds: vec![
                    Coin {
                        denom: "uatom".into(),
                        amount: Uint128::new(12345),
                    },
                ],
                should_ok: false,
            },

            // two coins sent but incorrect amount
            TestCase {
                sending_funds: vec![
                    Coin {
                        denom: "uatom".into(),
                        amount: Uint128::new(12345),
                    },
                    Coin {
                        denom: "uosmo".into(),
                        amount: Uint128::new(12345),
                    },
                ],
                should_ok: false,
            },

            // extra coins sent
            TestCase {
                sending_funds: vec![
                    Coin {
                        denom: "uatom".into(),
                        amount: Uint128::new(12345),
                    },
                    Coin {
                        denom: "uosmo".into(),
                        amount: Uint128::new(23456),
                    },
                    Coin {
                        denom: "ujuno".into(),
                        amount: Uint128::new(34567),
                    },
                ],
                should_ok: false,
            },

            // correct funds sent
            TestCase {
                sending_funds: vec![
                    Coin {
                        denom: "uatom".into(),
                        amount: Uint128::new(12345),
                    },
                    Coin {
                        denom: "uosmo".into(),
                        amount: Uint128::new(23456),
                    },
                ],
                should_ok: true,
            },
        ];

        for testcase in testcases {
            let mut deps = mock_dependencies();

            let mock_connection_id = "connection-0";
            let mock_active_channel_id = "channel-0";
            let mock_default_timeout_secs = 300;

            DEFAULT_TIMEOUT_SECS
                .save(deps.as_mut().storage, &mock_default_timeout_secs)
                .unwrap();
            ACTIVE_CHANNELS
                .save(deps.as_mut().storage, mock_connection_id, &mock_active_channel_id.into())
                .unwrap();

            let result = act(
                deps.as_mut(),
                mock_env(),
                mock_info("larry", &testcase.sending_funds),
                mock_connection_id.into(),
                actions.clone(),
                None,
                RelayerFee {
                    dest: None,
                    src: None,
                },
            );

            if testcase.should_ok {
                assert!(result.is_ok());
            } else {
                assert!(matches!(result, Err(ContractError::FundsMismatch { .. })));
            }
        }
    }

    #[test]
    fn sending_packet() {
        // TODO
    }

    #[test]
    fn receiving_packet() {
        // TODO
    }
}
