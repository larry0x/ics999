use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, IbcBasicResponse, IbcChannelCloseMsg,
    IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo, Reply, Response,
    StdResult,
};
use token_factory::{TokenFactoryMsg, TokenFactoryQuery};

use crate::{
    controller,
    error::ContractError,
    handshake, host,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    query,
    state::{ACCOUNT_CODE_ID, DEFAULT_TIMEOUT_SECS},
    AFTER_ACTIONS, AFTER_EXECUTE, AFTER_CALLBACK, CONTRACT_NAME, CONTRACT_VERSION,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    ACCOUNT_CODE_ID.save(deps.storage, &msg.account_code_id)?;
    DEFAULT_TIMEOUT_SECS.save(deps.storage, &msg.default_timeout_secs)?;

    Ok(Response::new())
}

#[entry_point]
pub fn execute(
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    match msg {
        ExecuteMsg::Act {
            connection_id,
            actions,
            timeout,
        } => {
            if actions.is_empty() {
                return Err(ContractError::EmptyActionQueue);
            }

            controller::act(deps, env, info, connection_id, actions, timeout)
        },
        ExecuteMsg::Handle {
            src,
            dest,
            controller,
            actions,
            traces,
        } => {
            if info.sender != env.contract.address {
                return Err(ContractError::Unauthorized);
            }

            host::handle(deps, env, src, dest, controller, actions, traces)
        },
    }
}

#[entry_point]
pub fn reply(
    deps: DepsMut<TokenFactoryQuery>,
    env: Env,
    msg: Reply,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    match msg.id {
        AFTER_EXECUTE => host::after_execute(deps, env, msg.result),
        AFTER_ACTIONS => host::after_actions(msg.result),
        AFTER_CALLBACK => controller::after_callback(msg.result.is_ok()),
        id => unreachable!("unknown reply ID: `{id}`"),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query::config(deps)?),
        QueryMsg::DenomHash {
            trace,
        } => to_binary(&query::denom_hash(trace)),
        QueryMsg::DenomTrace {
            denom,
        } => to_binary(&query::denom_trace(deps, denom)?),
        QueryMsg::DenomTraces {
            start_after,
            limit,
        } => to_binary(&query::denom_traces(deps, start_after, limit)?),
        QueryMsg::Account {
            channel_id,
            controller,
        } => to_binary(&query::account(deps, channel_id, controller)?),
        QueryMsg::Accounts {
            start_after,
            limit,
        } => to_binary(&query::accounts(deps, start_after, limit)?),
        QueryMsg::ActiveChannel {
            connection_id,
        } => to_binary(&query::active_channel(deps, connection_id)?),
        QueryMsg::ActiveChannels {
            start_after,
            limit,
        } => to_binary(&query::active_channels(deps, start_after, limit)?),
    }
}

#[entry_point]
pub fn ibc_channel_open(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<IbcChannelOpenResponse, ContractError> {
    match msg {
        IbcChannelOpenMsg::OpenInit {
            channel,
        } => handshake::open_init(deps, channel),
        IbcChannelOpenMsg::OpenTry {
            channel,
            counterparty_version,
        } => handshake::open_try(deps, channel, counterparty_version),
    }
}

#[entry_point]
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> Result<IbcBasicResponse, ContractError> {
    handshake::open_connect(deps, msg.channel(), msg.counterparty_version())
}

#[entry_point]
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> Result<IbcBasicResponse, ContractError> {
    handshake::close(msg)
}

#[entry_point]
pub fn ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    host::packet_receive(deps, env, msg.packet)
}

#[entry_point]
pub fn ibc_packet_ack(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse<TokenFactoryMsg>, ContractError> {
    controller::packet_lifecycle_complete(deps, msg.original_packet, Some(msg.acknowledgement.data))
}

#[entry_point]
pub fn ibc_packet_timeout(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse<TokenFactoryMsg>, ContractError> {
    controller::packet_lifecycle_complete(deps, msg.packet, None)
}
