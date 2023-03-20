use cosmwasm_std::{
    to_binary, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo, Response, SubMsgResult,
};
use one_types::{Action, PacketData};

use crate::{
    error::ContractResult,
    handler::Handler,
    msg::InstantiateMsg,
    state::{ACCOUNT_CODE_ID, ACTIVE_CHANNELS},
};

// should this be a configurable parameter instead?
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 600;

pub fn init(deps: DepsMut, msg: InstantiateMsg) -> ContractResult<Response> {
    ACCOUNT_CODE_ID.save(deps.storage, &msg.account_code_id)?;

    // TODO: instantaite the transfer contract

    Ok(Response::new().add_attribute("action", "init"))
}

pub fn act(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    connection_id: String,
    actions: Vec<Action>,
    timeout_seconds: Option<u64>,
) -> ContractResult<Response> {
    // TODO: validate received coin amount
    // TODO: make sure the action queue is not empty

    Ok(Response::new()
        .add_message(IbcMsg::SendPacket {
            channel_id: ACTIVE_CHANNELS.load(deps.storage, &connection_id)?,
            data: to_binary(&PacketData {
                sender: info.sender.into(),
                actions,
            })?,
            timeout: IbcTimeout::with_timestamp(
                env.block.time.plus_seconds(timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS)),
            ),
        })
        .add_attribute("action", "act"))
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    connection_id: String,
    controller: String,
    actions: Vec<Action>,
) -> ContractResult<Response> {
    let handler = Handler::create(deps.storage, connection_id, controller, actions)?;
    handler.handle_next_action(deps, env)
}

pub fn after_action(deps: DepsMut, env: Env, res: SubMsgResult) -> ContractResult<Response> {
    let mut handler = Handler::load(deps.storage)?;
    handler.handle_result(res.unwrap().data)?; // reply on success so unwrap can't fail
    handler.handle_next_action(deps, env)
}
