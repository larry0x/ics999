use {
    crate::{
        msg::{AccountKey, AccountResponse, ActiveChannelResponse, Config, DenomHashResponse},
        state::{ACCOUNTS, ACTIVE_CHANNELS, CONFIG, DENOM_TRACES},
        transfer::TraceItem,
    },
    cosmwasm_std::{Deps, IbcEndpoint, StdResult},
    cw_paginate::paginate_map,
    cw_storage_plus::Bound,
    ics999::Trace,
};

pub fn config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

pub fn denom_hash(trace: TraceItem) -> DenomHashResponse {
    DenomHashResponse {
        hash: trace.hash(),
    }
}

pub fn denom_trace(deps: Deps, denom: String) -> StdResult<Trace> {
    let trace = DENOM_TRACES.load(deps.storage, &denom)?;
    Ok(Trace {
        denom,
        base_denom: trace.base_denom,
        path:       trace.path,
    })
}

pub fn denom_traces(
    deps:        Deps,
    start_after: Option<String>,
    limit:       Option<u32>,
) -> StdResult<Vec<Trace>> {
    let start = start_after.as_ref().map(|denom| Bound::exclusive(denom.as_str()));
    paginate_map(&DENOM_TRACES, deps.storage, start, limit, |denom, trace| {
        Ok(Trace {
            denom,
            base_denom: trace.base_denom,
            path:       trace.path,
        })
    })
}

pub fn account(
    deps:       Deps,
    src:        IbcEndpoint,
    controller: String,
) -> StdResult<AccountResponse> {
    Ok(AccountResponse {
        address: ACCOUNTS.load(deps.storage, (&src.port_id, &src.channel_id, &controller))?.into(),
        src,
        controller,
    })
}

pub fn accounts(
    deps:        Deps,
    start_after: Option<AccountKey>,
    limit:       Option<u32>,
) -> StdResult<Vec<AccountResponse>> {
    let start = start_after
        .as_ref()
        .map(|AccountKey { src, controller }| {
            Bound::exclusive((src.port_id.as_str(), src.channel_id.as_str(), controller.as_str()))
        });

    paginate_map(&ACCOUNTS, deps.storage, start, limit, |(port_id, channel_id, controller), address| {
        Ok(AccountResponse {
            address: address.into(),
            src:     IbcEndpoint { port_id, channel_id },
            controller,
        })
    })
}

pub fn active_channel(deps: Deps, connection_id: String) -> StdResult<ActiveChannelResponse> {
    Ok(ActiveChannelResponse {
        endpoint: ACTIVE_CHANNELS.load(deps.storage, &connection_id)?,
        connection_id,
    })
}

pub fn active_channels(
    deps:        Deps,
    start_after: Option<String>,
    limit:       Option<u32>,
) -> StdResult<Vec<ActiveChannelResponse>> {
    let start = start_after.as_ref().map(|cid| Bound::exclusive(cid.as_str()));
    paginate_map(&ACTIVE_CHANNELS, deps.storage, start, limit, |connection_id, endpoint| {
        Ok(ActiveChannelResponse {
            connection_id,
            endpoint,
        })
    })
}
