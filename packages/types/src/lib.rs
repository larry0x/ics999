use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Coin, WasmMsg};

pub const CHANNEL_VERSION: &str = "ics999-1";

#[cw_serde]
pub struct Packet {
    /// The account who sends this packet
    sender: String,

    /// Actions to take.
    /// The actions will be executed in order and atomically.
    actions: Vec<Action>,
}

#[cw_serde]
pub enum Action {
    /// Send one or more tokens to a recipient
    Transfer {
        amount: Vec<Coin>,
        /// If not provided, default to the interchain account controlled by the
        /// sender.
        recipient: Option<String>,
    },

    /// Query a raw key-value pair at a contract
    QueryRaw {
        contract: String,
        key: Binary,
    },

    /// Performs a smart contract at a contract
    QuerySmart {
        contract: String,
        msg: Binary,
    },

    /// Register an interchain account
    RegisterAccount {
        /// Wasm bytecode to use for the interchain account.
        /// If not provided, will use the default.
        code_id: Option<u64>,
    },

    /// Instructs the interchain account to execute a wasm message
    Execute(WasmMsg),
}
