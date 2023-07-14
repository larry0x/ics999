use cosmwasm_std::{
    entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo,
    QueryRequest, Reply, Response, StdError, SubMsg, SystemError,
};
use cw_ownable::OwnershipError;

pub const CONTRACT_NAME:    &str = "crates.io:one-account";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const REPLY_ID: u64 = 69420;

// this contract does not need any parameter for instantiation
// it will store the deployer as the owner
pub type InstantiateMsg = Empty;

// this contract takes a CosmosMsg and simply executes it
// note: only the owner can execute
// note: no support for custom bindings. use StargateMsg or fork this contract
pub type ExecuteMsg = CosmosMsg<Empty>;

// this contract takes a QueryRequest, performs the query, and directly returns
// the binary response without attempting to deserializing it
// note: no support for custom bindings. use StargateQuery or fork this contract
pub type QueryMsg = QueryRequest<Empty>;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownership(#[from] OwnershipError),

    #[error("query failed due to system error: {0}")]
    QuerySystem(#[from] SystemError),

    #[error("query failed due to contract error: {0}")]
    QueryContract(String),
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _:    Env,
    info: MessageInfo,
    _:    Empty,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _:    Env,
    info: MessageInfo,
    msg:  ExecuteMsg,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(msg, REPLY_ID))
        .add_attribute("method", "execute"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _: Env, msg: Reply) -> Result<Response, ContractError> {
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
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    deps.querier
        .raw_query(&to_binary(&msg)?)
        .into_result()?
        .into_result()
        .map_err(ContractError::QueryContract)
}

// ----------------------------------- Tests -----------------------------------

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coins,
        testing::{mock_dependencies, mock_env, mock_info},
        BankMsg, SubMsgResult, SubMsgResponse,
    };

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
            Empty {},
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
            assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner));
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
