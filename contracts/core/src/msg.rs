use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, HexBinary, IbcTimeout};
use one_types::{Action, DenomTrace};

#[cw_serde]
pub struct InstantiateMsg {
    /// Code ID of the one-account contract
    pub account_code_id: u64,

    /// Code ID of the one-transfer contract
    pub transfer_code_id: u64,

    /// A salt used for instantiating the transfer contract
    pub transfer_salt: Binary,

    /// The default timeout (in seconds) if the user does not provide a timeout
    /// timestamp
    pub default_timeout_secs: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Send a packet consisting of a series of actions
    Act {
        /// The connection via which to send the actions.
        /// The contract will query the appropriate channel.
        connection_id: String,

        /// One or more actions to take
        actions: Vec<Action>,

        /// Whether to request a callback on the completion of packet lifecycle
        callback: bool,

        /// How many seconds from how will the packet timeout
        /// TODO: make this optional
        timeout: Option<IbcTimeout>,
    },

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
        connection_id: String,
        controller: String,
        actions: Vec<Action>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Contract configuration
    #[returns(ConfigResponse)]
    Config {},

    /// Compute the denom hash of a given denom trace
    #[returns(HexBinary)]
    DenomHash {
        trace: DenomTrace,
    },

    /// Query the denom trace associated with a given denom hash
    #[returns(DenomTraceResponse)]
    DenomTrace {
        denom: String,
    },

    /// Iterate all known denom traces
    #[returns(Vec<DenomTraceResponse>)]
    DenomTraces {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Interchain account controlled by a specific controller
    #[returns(AccountResponse)]
    Account {
        connection_id: String,
        controller: String,
    },

    /// Iterate all interchain accounts
    #[returns(Vec<AccountResponse>)]
    Accounts {
        start_after: Option<(String, String)>,
        limit: Option<u32>,
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
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub account_code_id: u64,
    pub transfer: String,
    pub default_timeout_secs: u64,
}

#[cw_serde]
pub struct DenomTraceResponse {
    pub denom: String,
    pub trace: DenomTrace,
}

#[cw_serde]
pub struct AccountResponse {
    pub connection_id: String,
    pub controller: String,
    pub address: String,
}

#[cw_serde]
pub struct ActiveChannelResponse {
    pub connection_id: String,
    pub channel_id: String,
}
