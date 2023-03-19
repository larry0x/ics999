use cosmwasm_std::{DepsMut, Response};

use crate::{error::ContractResult, msg::InstantiateMsg, state::ACCOUNT_CODE_ID};

pub fn init(deps: DepsMut, msg: InstantiateMsg) -> ContractResult<Response> {
    ACCOUNT_CODE_ID.save(deps.storage, &msg.account_code_id)?;

    // TODO: instantaite the transfer contract

    Ok(Response::new()
        .add_attribute("action", "init"))
}
