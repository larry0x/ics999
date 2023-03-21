use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
    SubMsg, WasmMsg, Reply,
};

use crate::{error::ContractResult, msg::QueryMsg, CONTRACT_NAME, CONTRACT_VERSION};

const REPLY_ID: u64 = 69420;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: Empty,
) -> ContractResult<Response> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", info.sender))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: WasmMsg,
) -> ContractResult<Response> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(msg, REPLY_ID))
        .add_attribute("action", "execute"))
}

#[entry_point]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> ContractResult<Response> {
    match msg.id {
        // if the WasmMsg returned data, we need to forward it back to one-core
        //
        // NOTE: The `data` is protobuf-encoded MsgInstantiateContractResponse,
        // MsgExecuteContractResponse, etc. We don't decode them here. The ICA
        // controller is responsible for decoding it.
        REPLY_ID => {
            // reply on success so unwrap can't fail
            let Some(data) = msg.result.unwrap().data else {
                return Ok(Response::new());
            };

            Ok(Response::new().set_data(data))
        },
        id => unreachable!("unknown reply ID: `{id}`"),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_binary(&cw_ownable::get_ownership(deps.storage)?),
    }
}
