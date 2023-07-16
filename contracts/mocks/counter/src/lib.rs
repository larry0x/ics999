use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Coin, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StdError, StdResult,
};
use cw_storage_plus::Item;

pub const NUMBER: Item<u64> = Item::new("number");

#[cw_serde]
pub enum ExecuteMsg {
    /// Increment the number by 1
    Increment {},

    /// Attempt to increment the number by 1, but intentionally fail by the end.
    ///
    /// Used to test that state changes effected by failed submessages will not
    /// be committed.
    IncrementButFail {},
}

#[cw_serde]
pub struct IncrementResult {
    pub new_number: u64,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Query the current number stored in the contract
    #[returns(NumberResponse)]
    Number {},
}

#[cw_serde]
pub struct NumberResponse {
    pub number: u64,
}

#[entry_point]
pub fn instantiate(deps: DepsMut, _: Env, _: MessageInfo, _: Empty) -> StdResult<Response> {
    NUMBER.save(deps.storage, &0)?;

    Ok(Response::new())
}

#[entry_point]
pub fn execute(deps: DepsMut, _: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Increment {} => {
            let new_number = NUMBER.update(deps.storage, |number| -> StdResult<_> {
                Ok(number + 1)
            })?;

            let data = to_binary(&IncrementResult {
                new_number,
            })?;

            Ok(Response::new()
                .set_data(data)
                .add_attribute("new_number", new_number.to_string())
                .add_attribute("user", info.sender)
                .add_attribute("funds", stringify_funds(&info.funds)))
        },
        ExecuteMsg::IncrementButFail {} => {
            // attempt to increment the number, but we throw an error later so
            // this should have no effect
            NUMBER.update(deps.storage, |number| -> StdResult<_> {
                Ok(number + 1)
            })?;

            Err(StdError::generic_err("intentional error instructed by user"))
        },
    }
}

fn stringify_funds(funds: &[Coin]) -> String {
    if funds.is_empty() {
        return "[]".into();
    }

    funds.iter().map(|coin| coin.to_string()).collect::<Vec<_>>().join(",")
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Number {} => {
            let number = NUMBER.load(deps.storage)?;
            to_binary(&NumberResponse {
                number,
            })
        },
    }
}
