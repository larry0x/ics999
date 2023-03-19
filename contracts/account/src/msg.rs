use cosmwasm_schema::{cw_serde, QueryResponses};
use cw_ownable::cw_ownable_query;

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
