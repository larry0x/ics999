use cosmwasm_std::{to_binary, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo, Response};
use one_types::{Action, Packet};

use crate::{
    error::ContractResult,
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

    Ok(Response::new()
        .add_message(IbcMsg::SendPacket {
            channel_id: ACTIVE_CHANNELS.load(deps.storage, &connection_id)?,
            data: to_binary(&Packet {
                sender: info.sender.into(),
                actions,
            })?,
            timeout: IbcTimeout::with_timestamp(
                env.block.time.plus_seconds(timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS)),
            ),
        })
        .add_attribute("action", "act"))
}
