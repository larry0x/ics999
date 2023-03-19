use cosmwasm_std::{Addr, BlockInfo, DepsMut, Response};

use crate::{error::ContractResult, msg::InstantiateMsg, state::ACCOUNT_CODE_ID};

pub fn init(deps: DepsMut, owner: &str, msg: InstantiateMsg) -> ContractResult<Response> {
    let ownership = cw_ownable::initialize_owner(deps.storage, deps.api, Some(owner))?;

    ACCOUNT_CODE_ID.save(deps.storage, &msg.account_code_id)?;

    // TODO: instantaite the transfer contract

    Ok(Response::new()
        .add_attribute("action", "init")
        .add_attributes(ownership.into_attributes()))
}

pub fn update_ownership(
    deps: DepsMut,
    block: &BlockInfo,
    sender: &Addr,
    action: cw_ownable::Action,
) -> ContractResult<Response> {
    let new_ownership = cw_ownable::update_ownership(deps, block, sender, action)?;

    Ok(Response::new()
        .add_attribute("action", "update_ownership")
        .add_attributes(new_ownership.into_attributes()))
}
