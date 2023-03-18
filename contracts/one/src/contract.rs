#[cfg(feature = "entry")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::{
    error::ContractResult,
    execute,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

pub const CONTRACT_NAME: &str = "crates.io:cw-one";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(feature = "entry", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> ContractResult<Response> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    execute::init(deps, info.sender.as_str())
}

#[cfg_attr(feature = "entry", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::UpdateOwnership(action) => execute::update_ownership(
            deps,
            &env.block,
            &info.sender,
            action,
        ),
    }
}

#[cfg_attr(feature = "entry", entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_binary(&cw_ownable::get_ownership(deps.storage)?),
    }
}
