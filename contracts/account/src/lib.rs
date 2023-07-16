use cosmwasm_std::{
    entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo,
    QueryRequest, Reply, Response, SubMsg,
};

pub type InstantiateMsg = Empty;
pub type ExecuteMsg     = CosmosMsg<Empty>;
pub type QueryMsg       = QueryRequest<Empty>;

pub const CONTRACT_NAME:    &str = "crates.io:one-account";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const REPLY_ID: u64 = 69420;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    Ownership(#[from] cw_ownable::OwnershipError),

    #[error("query failed due to system error: {0}")]
    QuerySystem(#[from] cosmwasm_std::SystemError),

    #[error("query failed due to contract error: {0}")]
    QueryContract(String),

    #[error("submessage failed to execute: {0}")]
    SubMsgFailed(String),

    #[error("unknown reply id: {0}")]
    UnknownReplyId(u64),
}

type Result<T> = core::result::Result<T, Error>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _:    Env,
    info: MessageInfo,
    _:    InstantiateMsg,
) -> Result<Response> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, _: Env, info: MessageInfo, msg: ExecuteMsg) -> Result<Response> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(msg, REPLY_ID))
        .add_attribute("method", "execute"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _: Env, msg: Reply) -> Result<Response> {
    match msg.id {
        // if the submsg returned data, we need to forward it back to one-core
        //
        // NOTE: The `data` is protobuf-encoded MsgInstantiateContractResponse,
        // MsgExecuteContractResponse, etc. We don't decode them here. The ICA
        // controller is responsible for decoding it.
        REPLY_ID => {
            let mut res = Response::new();

            // this submsg is reply on success, so we expect it to succeed
            let submsg_res = msg.result.into_result().map_err(Error::SubMsgFailed)?;
            if let Some(data) = submsg_res.data {
                res = res.set_data(data);
            }

            Ok(res)
        },
        id => Err(Error::UnknownReplyId(id)),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> Result<Binary> {
    deps.querier
        .raw_query(&to_binary(&msg)?)
        .into_result()?
        .into_result()
        .map_err(Error::QueryContract)
}

// ----------------------------------- Tests -----------------------------------

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coins,
        testing::{mock_dependencies, mock_env, mock_info},
        BankMsg, CosmosMsg, SubMsgResult, SubMsgResponse,
    };
    use cw_ownable::OwnershipError;

    use super::*;

    #[test]
    fn proper_execute() {
        let mut deps = mock_dependencies();

        let cosmos_msg: CosmosMsg = BankMsg::Send {
            to_address: "larry".into(),
            amount: coins(88888, "uastro"),
        }
        .into();

        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("one-core", &[]),
            InstantiateMsg {},
        )
        .unwrap();

        // not owner
        {
            let err = execute(
                deps.as_mut(),
                mock_env(),
                mock_info("larry", &[]),
                cosmos_msg.clone(),
            )
            .unwrap_err();
            assert_eq!(err, Error::Ownership(OwnershipError::NotOwner));
        }

        // owner
        {
            let res = execute(
                deps.as_mut(),
                mock_env(),
                mock_info("one-core", &[]),
                cosmos_msg.clone(),
            )
            .unwrap();
            assert_eq!(res.messages, vec![SubMsg::reply_on_success(cosmos_msg, REPLY_ID)]);
        }
    }

    #[test]
    fn proper_reply() {
        let mut deps = mock_dependencies();

        // no data
        {
            let res = reply(
                deps.as_mut(),
                mock_env(),
                Reply {
                    id: REPLY_ID,
                    result: SubMsgResult::Ok(SubMsgResponse {
                        events: vec![],
                        data: None,
                    }),
                },
            )
            .unwrap();
            assert_eq!(res.data, None);
        }

        // with data
        {
            let data = b"hello";

            let res = reply(
                deps.as_mut(),
                mock_env(),
                Reply {
                    id: REPLY_ID,
                    result: SubMsgResult::Ok(SubMsgResponse {
                        events: vec![],
                        data: Some(data.into()),
                    }),
                },
            )
            .unwrap();
            assert_eq!(res.data, Some(data.into()));
        }
    }
}
