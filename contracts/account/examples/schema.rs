use cosmwasm_schema::write_api;
use cosmwasm_std::{CosmosMsg, Empty};
use one_account::QueryMsg;

fn main() {
    write_api! {
        instantiate: Empty,
        execute: CosmosMsg,
        query: QueryMsg,
    };
}
