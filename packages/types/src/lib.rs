use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Coin, WasmMsg, IbcOrder};

/// Expected channel packet ordering rule
pub const ORDER: IbcOrder = IbcOrder::Unordered;

/// Expected channel version string
pub const VERSION: &str = "ics999-1";

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
    /// The response data produced by each action, if all actions were executed
    /// successfully
    Ok(Vec<ActionResult>),

    /// The error message, if any action failed to execute
    Err(String),
}

#[cw_serde]
pub enum Action {
    /// Send one or more tokens to a recipient
    Transfer {
        /// The amount of coins to transfer.
        ///
        /// NOTE: Use the denoms on the sender chain. The one-core contract on
        /// the receiver chain will translate the denoms to the appropriate IBC
        /// denoms.
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
pub enum ActionResult {
    /// Result of a successfully executed `transfer` action.
    Transfer {
        /// The amount that was transferred (the denoms substituted with the IBC
        /// denoms on the receiver chain)
        amount: Vec<Coin>,

        /// The recipient address (in case the sender did not provide an address,
        /// they can get it here)
        recipient: String,
    },

    /// Result of a successfully executed `register_account` action.
    RegisterAccount {
        /// The address of the account that was registered
        address: String,
    },

    /// Result of a successfully executed `execute` action.
    Execute {
        /// The data returned by the ICA contract
        data: Option<Binary>,
    },
}
