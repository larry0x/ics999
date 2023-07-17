use cosmwasm_std::{Deps, StdResult};
use cw_paginate::paginate_map;
use cw_storage_plus::Bound;

use ics999::Trace;

use crate::{
    msg::{AccountResponse, ActiveChannelResponse, Config, DenomHashResponse},
    state::{ACCOUNTS, ACTIVE_CHANNELS, CONFIG, DENOM_TRACES},
    transfer::TraceItem,
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
    channel_id: String,
    controller: String,
) -> StdResult<AccountResponse> {
    Ok(AccountResponse {
        address: ACCOUNTS.load(deps.storage, (&channel_id, &controller))?.into(),
        channel_id,
        controller,
    })
}

pub fn accounts(
    deps:        Deps,
    start_after: Option<(String, String)>,
    limit:       Option<u32>,
) -> StdResult<Vec<AccountResponse>> {
    let start = start_after.as_ref().map(|(cid, con)| Bound::exclusive((cid.as_str(), con.as_str())));
    paginate_map(&ACCOUNTS, deps.storage, start, limit, |(channel_id, controller), address| {
        Ok(AccountResponse {
            channel_id,
            controller,
            address: address.into(),
        })
    })
}

pub fn active_channel(deps: Deps, connection_id: String) -> StdResult<ActiveChannelResponse> {
    Ok(ActiveChannelResponse {
        channel_id: ACTIVE_CHANNELS.load(deps.storage, &connection_id)?,
        connection_id,
    })
}

pub fn active_channels(
    deps:        Deps,
    start_after: Option<String>,
    limit:       Option<u32>,
) -> StdResult<Vec<ActiveChannelResponse>> {
    let start = start_after.as_ref().map(|cid| Bound::exclusive(cid.as_str()));
    paginate_map(&ACTIVE_CHANNELS, deps.storage, start, limit, |connection_id, channel_id| {
        Ok(ActiveChannelResponse {
            connection_id,
            channel_id,
        })
    })
}
