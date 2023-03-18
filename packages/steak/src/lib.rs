/// Message type definitions of the Steak Hub contract
pub mod hub {
    use cosmwasm_schema::{cw_serde, QueryResponses};
    use cw_ownable::{cw_ownable_execute, cw_ownable_query};

    #[cw_serde]
    pub struct InstantiateMsg {
        /// The account to be appointed the contract owner
        pub owner: String,
    }

    #[cw_ownable_execute]
    #[cw_serde]
    pub enum ExecuteMsg {}

    #[cw_ownable_query]
    #[cw_serde]
    #[derive(QueryResponses)]
    pub enum QueryMsg {}
}
