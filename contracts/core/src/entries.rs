use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, IbcBasicResponse, IbcChannelCloseMsg,
    IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo, Reply, Response,
    StdResult,
};

use crate::{
    error::ContractError,
    execute, ibc,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    query, AFTER_ACTION, AFTER_ALL_ACTIONS, CONTRACT_NAME, CONTRACT_VERSION,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    execute::init(deps, msg)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Act {
            connection_id,
            actions,
            callback,
            timeout,
        } => execute::act(deps, env, info, connection_id, actions, callback, timeout),
        ExecuteMsg::Handle {
            connection_id,
            controller,
            actions,
        } => {
            // only the contract itself can invoke this method
            if info.sender != env.contract.address {
                return Err(ContractError::Unauthorized);
            }

            execute::handle(deps, env, connection_id, controller, actions)
        },
    }
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        // after finished executing a single action - update the result, and
        // move on to the next action
        AFTER_ACTION => execute::after_action(deps, env, msg.result),

        // after finished executing all actions - write ack and clear up state
        AFTER_ALL_ACTIONS => ibc::after_all_actions(msg.result),

        id => unreachable!("unknown reply ID: `{id}`"),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query::config(deps)?),
        QueryMsg::Account {
            connection_id,
            controller,
        } => to_binary(&query::account(deps, connection_id, controller)?),
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
        } => ibc::open_init(deps, channel),
        IbcChannelOpenMsg::OpenTry {
            channel,
            counterparty_version,
        } => ibc::open_try(deps, channel, counterparty_version),
    }
}

#[entry_point]
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> Result<IbcBasicResponse, ContractError> {
    ibc::open_connect(deps, msg.channel(), msg.counterparty_version())
}

#[entry_point]
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> Result<IbcBasicResponse, ContractError> {
    ibc::close(msg)
}

#[entry_point]
pub fn ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    ibc::packet_receive(deps, env, msg.packet)
}

#[entry_point]
pub fn ibc_packet_ack(
    _deps: DepsMut,
    _env: Env,
    _ack: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // TODO
    Ok(IbcBasicResponse::new())
}

#[entry_point]
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // TODO
    Ok(IbcBasicResponse::new())
}
