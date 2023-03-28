mod handler;

use cosmwasm_std::{
    from_slice, to_binary, DepsMut, Env, IbcPacket, IbcReceiveResponse, Response, SubMsg,
    SubMsgResponse, SubMsgResult, WasmMsg,
};
use cw_utils::parse_execute_response_data;
use one_types::{Action, PacketAck, PacketData};
use token_factory::TokenFactoryMsg;

use crate::{error::ContractError, msg::ExecuteMsg, utils::connection_of_channel, AFTER_ALL_ACTIONS};

use self::handler::Handler;

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
        .add_attribute("method", "packet_receive")
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

pub fn handle(
    deps: DepsMut,
    env: Env,
    connection_id: String,
    controller: String,
    actions: Vec<Action>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let handler = Handler::create(deps.storage, connection_id, controller, actions)?;
    handler.handle_next_action(deps, env)
}

pub fn after_action(
    deps: DepsMut,
    env: Env,
    res: SubMsgResult,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let mut handler = Handler::load(deps.storage)?;
    handler.handle_result(res.unwrap().data)?; // reply on success so unwrap can't fail
    handler.handle_next_action(deps, env)
}

pub fn after_all_actions(res: SubMsgResult) -> Result<Response<TokenFactoryMsg>, ContractError> {
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

            PacketAck::Results(action_res)
        },

        // one of actions failed - write an Error ack
        SubMsgResult::Err(err) => PacketAck::Error(err.clone()),
    };

    Ok(Response::new()
        .add_attribute("method", "after_all_actions")
        .add_attribute("success", res.is_ok().to_string())
        // wasmd will interpret this data field as the ack, overriding the ack
        // emitted in the ibc_packet_receive entry point
        .set_data(to_binary(&ack)?))
}
