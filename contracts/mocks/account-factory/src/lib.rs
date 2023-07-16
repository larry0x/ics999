use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    entry_point, from_binary, Binary, Deps, DepsMut, Empty, Env, IbcEndpoint, MessageInfo, Reply,
    Response, StdError, SubMsg, SubMsgResponse, SubMsgResult, WasmMsg,
};
use cw_storage_plus::Item;
use ics999::{FactoryExecuteMsg, FactoryMsg};

pub const CONFIG: Item<Config> = Item::new("cfg");

const AFTER_INSTANTIATE: u64 = 1;

#[cw_serde]
pub struct Config {
    pub one_core:           String,
    pub allowed_src:        IbcEndpoint,
    pub allowed_controller: String,
}

#[cw_serde]
pub struct InstantiateData {
    pub code_id:         u64,
    pub instantiate_msg: Binary,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("sender is not ics999 core contract")]
    NotIcs999,

    #[error("not allowed to register from this connection")]
    NotAllowedSource,

    #[error("not allowed to register with this controller account")]
    NotAllowedController,

    #[error("instantiate data not provided")]
    MissingInstantiateData,

    #[error("failed to extract instantiate response data from reply")]
    MissingInstantiateResponse,
}

pub type Result<T> = core::result::Result<T, Error>;

#[entry_point]
pub fn instantiate(deps: DepsMut, _: Env, _: MessageInfo, cfg: Config) -> Result<Response> {
    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env:  Env,
    info: MessageInfo,
    msg:  FactoryExecuteMsg,
) -> Result<Response> {
    match msg {
        FactoryExecuteMsg::Ics999(FactoryMsg { src, controller, data }) => {
            let cfg = CONFIG.load(deps.storage)?;

            if info.sender != cfg.one_core {
                return Err(Error::NotIcs999);
            }

            if src != cfg.allowed_src {
                return Err(Error::NotAllowedSource);
            }

            if controller != cfg.allowed_controller {
                return Err(Error::NotAllowedController);
            }

            let Some(data_bytes) = data else {
                return Err(Error::MissingInstantiateData);
            };

            let InstantiateData { code_id, instantiate_msg } = from_binary(&data_bytes)?;

            Ok(Response::new().add_submessage(SubMsg::reply_on_success(
                WasmMsg::Instantiate {
                    code_id,
                    msg:   instantiate_msg,
                    funds: vec![],
                    admin: Some(env.contract.address.into()),
                    label: "mock-label".into(),
                },
                AFTER_INSTANTIATE,
            )))
        }
    }
}

#[entry_point]
pub fn reply(_: DepsMut, _: Env, reply: Reply) -> Result<Response> {
    match reply.id {
        AFTER_INSTANTIATE => {
            let SubMsgResult::Ok(SubMsgResponse { data: Some(data), .. }) = reply.result else {
                return Err(Error::MissingInstantiateResponse);
            };

            Ok(Response::new().set_data(data))
        },
        id => unreachable!("unexpected reply id: {id}"),
    }
}

#[entry_point]
pub fn query(_: Deps, _: Env, _: Empty) -> Result<Binary> {
    unreachable!("this contract does not implement any query method");
}
