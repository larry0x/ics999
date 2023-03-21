use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Coin, WasmMsg, IbcOrder};

/// Expected channel packet ordering rule
pub const ORDER: IbcOrder = IbcOrder::Unordered;

/// Expected channel version string
pub const VERSION: &str = "ics999-1";

#[cw_serde]
pub struct PacketData {
    /// The account who sends this packet
    pub sender: String,

    /// Actions to take.
    /// The actions will be executed in order and atomically.
    pub actions: Vec<Action>,

    // TODO: add a `reply_on` parameter to let the sender specify under which
    // situations to give a callback (default to "never")
}

// ICS-4 recommand acknowldgement envelop format:
// https://github.com/cosmos/ibc/tree/main/spec/core/ics-004-channel-and-packet-semantics#acknowledgement-envelope
#[cw_serde]
pub enum Acknowledgment {
    /// All actions were executed successfully. In this case, we return the
    /// result of each action.
    ///
    /// ICS-4 recommends a raw binary here, but we choose to use `Vec<ActionResult>`
    /// so that it's easier to consume by the sender contract
    Result(Vec<ActionResult>),

    /// One of the actions failed to execute. In this case, the entire queue of
    /// actions is considered to be failed. We inform the sender contract of the
    /// failure.
    ///
    /// Ideally, we would also like to inform the sender contract of the
    /// specific error message. Unfortunately, the error message is redacted in
    /// submesssage replies: https://github.com/CosmWasm/wasmd/issues/759
    Error(String),
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
