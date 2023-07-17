use {
    cosmwasm_schema::cw_serde,
    cosmwasm_std::{Binary, IbcEndpoint, IbcOrder, Uint128},
};

// ---------------------------------- channel ----------------------------------

/// Expected channel packet ordering rule
pub const ORDER: IbcOrder = IbcOrder::Unordered;

/// Expected channel version string
pub const VERSION: &str = "ics999-1";

// ---------------------------------- packet -----------------------------------

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

#[cw_serde]
pub enum Action {
    /// Send one or more tokens to a recipient
    Transfer {
        denom: String,
        amount: Uint128,
        /// If not provided, default to the ICA controlled by the sender
        recipient: Option<String>,
    },

    /// Register an interchain account.
    ///
    /// The user provides a `RegisterOptions` data indicating how the account is
    /// to be registered. It can be one of two ways:
    /// - use the default account contract
    /// - the user to provide a custom "factory" contract which performs the
    ///   instantiation
    RegisterAccount(RegisterOptions),

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
pub enum RegisterOptions {
    /// Register the account with the default account contract.
    ///
    /// The only data that the user needs to provide is a salt (0 - 64 bytes)
    /// which is used in deriving the account address.
    Default {
        /// The interchain account's address is chosen deterministically using
        /// wasmd's Instantiate2 method.
        ///
        /// We need to prevent the attack where an attacker predicts the ICA's
        /// address ahead of time, and create an account with the same address.
        /// (this happened on Cosmos Hub which prevented Quicksilver from
        /// registering their ICA).
        ///
        /// To achieve this, we let the user pick the salt. If not given, use
        /// the controller address's UTF-8 bytes as the salt.
        salt: Option<Binary>,
    },

    /// If more sophisticated logics are needed for registering the account, the
    /// user may implement such logics as a "factory" contract.
    ///
    /// To register the account, the user provides the factory contract's
    /// address, and optional data to be provided to the factory.
    ///
    /// Ics999 contract will attempt to call the factory contract using the
    /// `FactoryExecuteMsg` defined below.
    CustomFactory {
        address: String,
        data:    Option<Binary>,
    },
}

// ------------------------------------ ack ------------------------------------

/// ICS-999 packet acknowledgement
///
/// Mostly based on the format recommended by ICS-4, but not exactly the same:
/// https://github.com/cosmos/ibc/tree/main/spec/core/ics-004-channel-and-packet-semantics#acknowledgement-envelope
#[cw_serde]
pub enum PacketAck {
    /// All actions were executed successfully. In this case, we return the
    /// result of each action.
    Success(Vec<ActionResult>),

    /// One of the actions failed to execute. In this case, the entire queue of
    /// actions is considered failed altogether. We inform the sender of the
    /// error message.
    ///
    /// NOTE: currently, wasmd redacts error messages due to concern of
    /// non-determinism: https://github.com/CosmWasm/wasmd/issues/759
    ///
    /// Therefore, although we return a String here, in reality it will only
    /// include the error code, not the message. It will look something like
    /// this:
    ///
    /// ```json
    /// {"failed":"codespace: wasm, code: 5"}
    /// ```
    Failed(String),
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

// ----------------------------------- trace -----------------------------------

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

// --------------------------- third party: factory ----------------------------

#[cw_serde]
pub enum FactoryExecuteMsg {
    Ics999(FactoryMsg),
}

#[cw_serde]
pub struct FactoryMsg {
    pub src:        IbcEndpoint,
    pub controller: String,
    pub data:       Option<Binary>,
}

#[cw_serde]
pub struct FactoryResponse {
    pub host: String,
}

// ---------------------------- third party: sender ----------------------------

#[cw_serde]
pub enum SenderExecuteMsg {
    Ics999(CallbackMsg),
}

#[cw_serde]
pub struct CallbackMsg {
    pub dest:     IbcEndpoint,
    pub sequence: u64,
    pub outcome:  PacketOutcome,
}

#[cw_serde]
pub enum PacketOutcome {
    Success(Vec<ActionResult>),
    Failed(String),
    Timeout {},
}

impl From<Option<PacketAck>> for PacketOutcome {
    fn from(maybe_ack: Option<PacketAck>) -> Self {
        match maybe_ack {
            Some(PacketAck::Success(results)) => PacketOutcome::Success(results),
            Some(PacketAck::Failed(error))    => PacketOutcome::Failed(error),
            None                              => PacketOutcome::Timeout {},
        }
    }
}

impl PacketOutcome {
    pub fn ty(&self) -> &str {
        match self {
            PacketOutcome::Success(_) => "success",
            PacketOutcome::Failed(_)  => "failed",
            PacketOutcome::Timeout {} => "timeout",
        }
    }
}
