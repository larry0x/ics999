use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Coin, WasmMsg, IbcOrder, SubMsgResponse};

/// Expected channel packet ordering rule
pub const ORDER: IbcOrder = IbcOrder::Unordered;

/// Expected channel version string
pub const VERSION: &str = "ics999-1";

#[cw_serde]
pub enum Action {
    /// Send one or more tokens to a recipient
    Transfer {
        amount: Vec<Coin>,
        /// If not provided, default to the interchain account controlled by the
        /// sender.
        recipient: Option<String>,
    },

    /// Register an interchain account
    RegisterAccount {
        /// The interchain account's address is chosen deterministically using
        /// wasmd's Instantiate2 method.
        ///
        /// We need to prevent the attack where an attacker predicts the ICA's
        /// address ahead of time, and create an account with the same address.
        /// (this happened on Cosmos Hub which prevented Quicksilver from
        /// registering their ICA)
        ///
        /// To achieve this, we let the user pick the salt. If not given, use
        /// the controller address's UTF-8 bytes as the salt.
        salt: Option<Binary>,
    },

    /// Instructs the interchain account to execute a wasm message
    Execute(WasmMsg),
}

#[cw_serde]
pub struct Packet {
    /// The account who sends this packet
    pub sender: String,

    /// Actions to take.
    /// The actions will be executed in order and atomically.
    pub actions: Vec<Action>,
}

#[cw_serde]
pub enum Acknowledgment {
    /// The response produced by each action, if all actions are successfully
    /// executed
    Ok(Vec<SubMsgResponse>),

    /// The error message, if any action fails
    Err(String),
}
