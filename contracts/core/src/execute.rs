use cosmwasm_std::{
    to_binary, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo, Response, SubMsgResult,
};
use one_types::{Action, PacketData};

use crate::{
    error::ContractError,
    handler::Handler,
    msg::InstantiateMsg,
    state::{ACCOUNT_CODE_ID, ACTIVE_CHANNELS},
};

// should this be a configurable parameter instead?
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 600;

pub fn init(deps: DepsMut, msg: InstantiateMsg) -> Result<Response, ContractError> {
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
    callback: bool,
    timeout: IbcTimeout,
) -> Result<Response, ContractError> {
    // TODO: validate received coin amount
    // TODO: make sure the action queue is not empty
    // TODO: save callback

    Ok(Response::new()
        .add_message(IbcMsg::SendPacket {
            channel_id: ACTIVE_CHANNELS.load(deps.storage, &connection_id)?,
            data: to_binary(&PacketData {
                sender: info.sender.into(),
                actions,
            })?,
            timeout,
        })
        .add_attribute("action", "act"))
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    connection_id: String,
    controller: String,
    actions: Vec<Action>,
) -> Result<Response, ContractError> {
    let handler = Handler::create(deps.storage, connection_id, controller, actions)?;
    handler.handle_next_action(deps, env)
}

pub fn after_action(deps: DepsMut, env: Env, res: SubMsgResult) -> Result<Response, ContractError> {
    let mut handler = Handler::load(deps.storage)?;
    handler.handle_result(res.unwrap().data)?; // reply on success so unwrap can't fail
    handler.handle_next_action(deps, env)
}
