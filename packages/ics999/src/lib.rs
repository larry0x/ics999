use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, IbcEndpoint, IbcOrder, Uint128};

/// Expected channel packet ordering rule
pub const ORDER: IbcOrder = IbcOrder::Unordered;

/// Expected channel version string
pub const VERSION: &str = "ics999-1";

/// ICS-999 packet data structure
#[cw_serde]
pub struct PacketData {
    /// The account who sends this packet
    pub sender: String,

    /// Actions to take.
    /// The actions will be executed in order and atomically.
    pub actions: Vec<Action>,

    /// Traces of each token that is being transferred.
    /// Receiver chain uses this to determine whether it's the sender or sink.
    /// Must include ALL tokens that are being transferred.
    pub traces: Vec<Trace>,
}

/// ICS-999 packet acknowledgement
///
/// Related: ICS-4 recommand acknowldgement envelop format:
/// https://github.com/cosmos/ibc/tree/main/spec/core/ics-004-channel-and-packet-semantics#acknowledgement-envelope
#[cw_serde]
pub enum PacketAck {
    /// All actions were executed successfully. In this case, we return the
    /// result of each action.
    ///
    /// ICS-4 recommends a raw binary here, but we choose to use `Vec<ActionResult>`
    /// so that it's easier to consume by the sender contract
    Results(Vec<ActionResult>),

    /// One of the actions failed to execute. In this case, the entire queue of
    /// actions is considered to be failed. We inform the sender contract of the
    /// failure.
    ///
    /// NOTE: currently, wasmd redacts error messages due to concern of
    /// non-determinism: https://github.com/CosmWasm/wasmd/issues/759
    ///
    /// Therefore, although we return a String here, in reality it will only
    /// include the error code, not the message. It will look something like
    /// this:
    ///
    /// ```json
    /// {"error":"codespace: wasm, code: 5"}
    /// ```
    Error(String),
}

#[cw_serde]
pub enum Action {
    /// Send one or more tokens to a recipient
    Transfer {
        denom: String,
        amount: Uint128,
        /// If not provided, default to the ICA controlled by the sender
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

    /// Call the ICA contract's execute entry point.
    ///
    /// The message is to be in raw binary format. The ICA contract is
    /// responsible for implementing logics to interpret and handle this message.
    Execute(Binary),

    /// Call the ICA contract's query entry point.
    ///
    /// The message is to be in raw binary format. The ICA contract is
    /// responsible for implementing logics to interpret and handle this message.
    Query(Binary),
}

#[cw_serde]
pub enum ActionResult {
    /// Result of a successfully executed `transfer` action.
    Transfer {
        /// IBC denom of the coin that was transferred
        denom: String,

        /// Whether a new token was created using the tokenfactory module as the
        /// result of this transfer
        new_token: bool,

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

    /// Result of a successful query
    Query {
        /// The querying contract is responsible for decoding the response
        response: Binary,
    },
}

/// Trace includes the token's original denom and the path it had travelled to
/// arrive at the current chain.
///
/// It is used to derive the voucher denom in such a way that there's a unique
/// voucher denom for each token and each path.
#[cw_serde]
pub struct Trace {
    /// The token's denom on the packet sender chain
    pub denom: String,

    /// The token's original denom
    pub base_denom: String,

    /// The path the token took to arrived to the current chain.
    ///
    /// At each stop, the chain is appended to the end of the array. For example,
    /// consider a token being transferred via this path:
    ///
    ///   chainA --> chainB --> chainC
    ///
    /// - on chain B, the path is \[A\]
    /// - on chain C, the path is \[A, B\]
    ///
    /// Note, this is different from ICS-20, where the latest chain is prefixed
    /// (instead of appended) to the beginning of the trace.
    pub path: Vec<IbcEndpoint>,
}

/// If the sender contract wishes to receive a callback after the completion of
/// a packet lifecycle, it must implement this execute message.
#[cw_serde]
pub enum SenderExecuteMsg {
    /// Called by ICS-999 core contract after the completion of a packet
    /// lifecycle (acknowledged or timed out)
    PacketCallback {
        channel_id: String,
        sequence: u64,
        /// The packet acknowledgement. None if the packet has timed out.
        ack: Option<PacketAck>,
    },
}
