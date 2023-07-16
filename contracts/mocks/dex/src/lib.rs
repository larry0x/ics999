use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    coins, entry_point, to_binary, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult,
};
use cw_storage_plus::Item;
use cw_utils::PaymentError;

pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct Config {
    denom_in:  String,
    denom_out: String,
}

pub type InstantiateMsg = Config;

#[cw_serde]
pub enum ExecuteMsg {
    Swap {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},
}

#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Payment(#[from] PaymentError),
}

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _: Env,
    _: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    CONFIG.save(deps.storage, &msg)?;

    Ok(Response::new())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Swap {} => {
            let cfg = CONFIG.load(deps.storage)?;
            let amount_in = cw_utils::must_pay(&info, &cfg.denom_in)?;

            Ok(Response::new().add_message(BankMsg::Send {
                to_address: info.sender.into(),
                // send back denom_out of the same amount
                amount: coins(amount_in.u128(), cfg.denom_out),
            }))
        },
    }
}

#[entry_point]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&CONFIG.load(deps.storage)?),
    }
}
