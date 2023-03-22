use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response,
    StdError, StdResult, SubMsg, WasmMsg,
};
use cw_ownable::{cw_ownable_query, OwnershipError};

pub const CONTRACT_NAME: &str = "crates.io:one-account";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const REPLY_ID: u64 = 69420;

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // nothing to query other than ownership
}

#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownership(#[from] OwnershipError),
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: Empty,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: WasmMsg,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(msg, REPLY_ID))
        .add_attribute("action", "execute"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        // if the submsg returned data, we need to forward it back to one-core
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_binary(&cw_ownable::get_ownership(deps.storage)?),
    }
}
