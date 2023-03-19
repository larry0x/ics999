use cosmwasm_schema::write_api;
use cosmwasm_std::{Empty, WasmMsg};

use one_account::msg::QueryMsg;

fn main() {
    write_api! {
        instantiate: Empty,
        execute: WasmMsg,
        query: QueryMsg,
    };
}
