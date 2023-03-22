use cosmwasm_std::{IbcOrder, Instantiate2AddressError, StdError};
use cw_utils::{ParseReplyError, PaymentError};

#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Instantiate2Address(#[from] Instantiate2AddressError),

    #[error(transparent)]
    Payment(#[from] PaymentError),

    #[error(transparent)]
    ParseReply(#[from] ParseReplyError),

    #[error("incorrect IBC channel order: expecting `{expected:?}`, found `{actual:?}`")]
    IncorrectOrder {
        actual: IbcOrder,
        expected: IbcOrder,
    },

    #[error("incorrect IBC channel version: expecting `{expected}`, found `{actual}`")]
    IncorrectVersion {
        actual: String,
        expected: String,
    },

    #[error("an open ICS-999 channel already exists on connection `{connection_id}`")]
    ChannelExists {
        connection_id: String,
    },

    #[error("no channel found at port `{port_id}` with channel id `{channel_id}`")]
    ChannelNotFound {
        port_id: String,
        channel_id: String,
    },

    #[error("an interchain account already exists for connection `{connection_id}` and controller `{controller}`")]
    AccountExists {
        connection_id: String,
        controller: String,
    },

    #[error("no interchain account found at connection `{connection_id}` and controller `{controller}`")]
    AccountNotFound {
        connection_id: String,
        controller: String,
    },

    #[error("action queue cannot be empty")]
    EmptyActionQueue,

    #[error("query failed")]
    QueryFailed,

    #[error("unauthorized")]
    Unauthorized,

    #[error("ICS-999 channel may not be closed")]
    UnexpectedChannelClosure,
}
