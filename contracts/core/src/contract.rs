use {
    crate::{
        controller,
        error::{Error, Result},
        handshake, host,
        msg::{AccountKey, Config, ExecuteMsg, QueryMsg},
        query,
        state::CONFIG,
        AFTER_ACTION, AFTER_ALL_ACTIONS, AFTER_CALLBACK, CONTRACT_NAME, CONTRACT_VERSION,
    },
    cosmwasm_std::{
        entry_point, to_binary, Binary, Deps, DepsMut, Env, IbcBasicResponse, IbcChannelCloseMsg,
        IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcPacketAckMsg,
        IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo, Reply, Response,
        StdResult,
    },
};

#[entry_point]
pub fn instantiate(deps: DepsMut, _: Env, _: MessageInfo, cfg: Config) -> Result<Response> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new())
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> Result<Response> {
    match msg {
        ExecuteMsg::Act {
            connection_id,
            actions,
            timeout,
        } => {
            if actions.is_empty() {
                return Err(Error::EmptyActionQueue);
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
                return Err(Error::Unauthorized);
            }

            host::handle(deps, env, src, dest, controller, actions, traces)
        },
    }
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response> {
    match msg.id {
        AFTER_ACTION => host::after_action(deps, env, msg.result),
        AFTER_ALL_ACTIONS => host::after_all_actions(msg.result),
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
        QueryMsg::Account(AccountKey {
            src,
            controller,
        }) => to_binary(&query::account(deps, src, controller)?),
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
    _:    Env,
    msg:  IbcChannelOpenMsg,
) -> Result<IbcChannelOpenResponse> {
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
    _:    Env,
    msg:  IbcChannelConnectMsg,
) -> Result<IbcBasicResponse> {
    handshake::open_connect(deps, msg.channel(), msg.counterparty_version())
}

#[entry_point]
pub fn ibc_channel_close(_: DepsMut, _: Env, msg: IbcChannelCloseMsg) -> Result<IbcBasicResponse> {
    handshake::close(msg)
}

#[entry_point]
pub fn ibc_packet_receive(
    _:   DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse> {
    host::packet_receive(env, msg.packet)
}

#[entry_point]
pub fn ibc_packet_ack(deps: DepsMut, env: Env, msg: IbcPacketAckMsg) -> Result<IbcBasicResponse> {
    controller::packet_lifecycle_complete(
        deps,
        env,
        msg.original_packet,
        Some(msg.acknowledgement.data),
    )
}

#[entry_point]
pub fn ibc_packet_timeout(
    deps: DepsMut,
    env:  Env,
    msg:  IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse> {
    controller::packet_lifecycle_complete(deps, env, msg.packet, None)
}
