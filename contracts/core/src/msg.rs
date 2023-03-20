use cosmwasm_schema::{cw_serde, QueryResponses};
use one_types::Action;

#[cw_serde]
pub struct InstantiateMsg {
    /// Code ID of the one-account contract
    pub account_code_id: u64,

    /// Code ID of the one-transfer contract
    pub transfer_code_id: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Execute one or more actions via the specified connection
    Act {
        connection_id: String,
        actions: Vec<Action>,
        /// How many seconds from how will the packet timeout
        timeout_seconds: Option<u64>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Contract configuration
    #[returns(ConfigResponse)]
    Config {},

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
