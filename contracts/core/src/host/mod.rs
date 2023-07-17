mod handler;

use {
    self::handler::Handler,
    crate::{error::Result, msg::ExecuteMsg, AFTER_ALL_ACTIONS},
    cosmwasm_std::{
        from_slice, to_binary, DepsMut, Env, IbcEndpoint, IbcPacket, IbcReceiveResponse, Response,
        SubMsg, SubMsgResponse, SubMsgResult, WasmMsg,
    },
    cw_utils::parse_execute_response_data,
    ics999::{Action, PacketAck, PacketData, Trace},
};

pub fn packet_receive(env: Env, packet: IbcPacket) -> Result<IbcReceiveResponse> {
    // deserialize packet data
    let pd: PacketData = from_slice(&packet.data)?;

    // we don't add an ack in this response
    // the ack will be added in after_all_actions reply (see below)
    Ok(IbcReceiveResponse::new()
        .add_attribute("method", "packet_receive")
        .add_attribute("port_id", &packet.dest.port_id)
        .add_attribute("channel_id", &packet.dest.channel_id)
        .add_attribute("sequence", packet.sequence.to_string())
        .add_submessage(SubMsg::reply_always(
            WasmMsg::Execute {
                contract_addr: env.contract.address.into(),
                msg: to_binary(&ExecuteMsg::Handle {
                    counterparty_endpoint: packet.src,
                    endpoint:              packet.dest,
                    controller:            pd.controller,
                    actions:               pd.actions,
                    traces:                pd.traces,
                })?,
                funds: vec![],
            },
            AFTER_ALL_ACTIONS,
        )))
}

pub fn handle(
    deps:       DepsMut,
    env:        Env,
    src:        IbcEndpoint,
    dest:       IbcEndpoint,
    controller: String,
    actions:    Vec<Action>,
    traces:     Vec<Trace>,
) -> Result<Response> {
    let handler = Handler::create(deps.storage, src, dest, controller, actions, traces)?;
    handler.handle_next_action(deps, env, None)
}

pub fn after_action(mut deps: DepsMut, env: Env, res: SubMsgResult) -> Result<Response> {
    let mut handler = Handler::load(deps.storage)?;
    handler.after_action(deps.branch(), res.unwrap().data)?; // reply on success so unwrap can't fail
    handler.handle_next_action(deps, env, None)
}

pub fn after_all_actions(res: SubMsgResult) -> Result<Response> {
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

            PacketAck::Success(action_res)
        },

        // one of actions failed - write an Error ack
        SubMsgResult::Err(err) => PacketAck::Failed(err.clone()),
    };

    Ok(Response::new()
        .add_attribute("method", "after_actions")
        .add_attribute("success", res.is_ok().to_string())
        // wasmd will interpret this data field as the ack, overriding the ack
        // emitted in the ibc_packet_receive entry point
        .set_data(to_binary(&ack)?))
}
