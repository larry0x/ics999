use cosmwasm_std::{CosmosMsg, Empty, QueryRequest};

// this contract does not need any parameter for instantiation
// it will store the deployer as the owner
pub type InstantiateMsg = Empty;

// this contract takes a CosmosMsg and simply executes it
// note: only the owner can execute
// note: no support for custom bindings. use StargateMsg or fork this contract
pub type ExecuteMsg = CosmosMsg<Empty>;

// this contract takes a QueryRequest, performs the query, and directly returns
// the binary response without attempting to deserializing it
// note: no support for custom bindings. use StargateQuery or fork this contract
pub type QueryMsg = QueryRequest<Empty>;
