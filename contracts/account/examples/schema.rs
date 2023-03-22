use cosmwasm_schema::write_api;
use cosmwasm_std::{Empty, CosmosMsg};

use one_account::QueryMsg;

fn main() {
    write_api! {
        instantiate: Empty,
        execute: CosmosMsg,
        query: QueryMsg,
    };
}
