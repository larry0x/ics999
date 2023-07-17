use {
    crate::transfer::TraceItem,
    cosmwasm_schema::{cw_serde, QueryResponses},
    cosmwasm_std::{HexBinary, IbcEndpoint, IbcTimeout},
    ics999::{Action, Trace},
};

#[cw_serde]
pub struct Config {
    /// Code ID of the one-account contract
    pub default_account_code_id: u64,

    /// The default timeout (in seconds) if the user does not provide a timeout
    /// timestamp
    pub default_timeout_secs: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    // ----------------------- USED ON CONTROLLER CHAIN ------------------------

    /// Send a packet consisting of a series of actions
    Act {
        connection_id: String,
        actions:       Vec<Action>,
        timeout:       Option<IbcTimeout>,
    },

    // ------------------------ USED ON THE HOST CHAIN -------------------------

    /// Execute a series of actions received in a packet.
    ///
    /// Can only be invoked by the contract itself.
    ///
    /// NOTE: We have to create an execute method for this instead of handling
    /// the actions in the `ibc_packet_receive` entry point, because only this
    /// way we can achieve atomicity - one action fails means all actions fail,
    /// and no state changes from any action (even those that succeeded) will be
    /// committed.
    Handle {
        counterparty_endpoint: IbcEndpoint,
        endpoint:              IbcEndpoint,
        controller:            String,
        actions:               Vec<Action>,
        traces:                Vec<Trace>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Contract configuration
    #[returns(Config)]
    Config {},

    /// Compute the denom hash of a given denom trace
    #[returns(DenomHashResponse)]
    DenomHash {
        trace: TraceItem,
    },

    /// Query the denom trace associated with a given denom hash
    #[returns(Trace)]
    DenomTrace {
        denom: String,
    },

    /// Iterate all known denom traces
    #[returns(Vec<Trace>)]
    DenomTraces {
        start_after: Option<String>,
        limit:       Option<u32>,
    },

    /// Interchain account controlled by a specific controller
    #[returns(AccountResponse)]
    Account(AccountKey),

    /// Iterate all interchain accounts
    #[returns(Vec<AccountResponse>)]
    Accounts {
        start_after: Option<AccountKey>,
        limit:       Option<u32>,
    },

    /// Active channel associated with a connection
    #[returns(ActiveChannelResponse)]
    ActiveChannel {
        connection_id: String,
    },

    /// Iterate active channels on all connections
    #[returns(Vec<ActiveChannelResponse>)]
    ActiveChannels {
        start_after: Option<String>,
        limit:       Option<u32>,
    },
}

#[cw_serde]
pub struct DenomHashResponse {
    pub hash: HexBinary,
}

#[cw_serde]
pub struct AccountKey {
    pub src:        IbcEndpoint,
    pub controller: String,
}

#[cw_serde]
pub struct AccountResponse {
    pub src:        IbcEndpoint,
    pub controller: String,
    pub address:    String,
}

#[cw_serde]
pub struct ActiveChannelResponse {
    pub connection_id: String,
    pub endpoint:      IbcEndpoint,
}
