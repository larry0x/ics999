use cosmwasm_schema::{cw_serde, QueryResponses};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub struct InstantiateMsg {
    /// Code ID of the one-account contract
    pub account_code_id: u64,

    /// Code ID of the one-transfer contract
    pub transfer_code_id: u64,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
