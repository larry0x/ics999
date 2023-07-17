use cosmwasm_std::IbcEndpoint;

use {
    crate::utils::Coins,
    cosmwasm_std::{IbcOrder, Instantiate2AddressError, OverflowError, StdError},
    cw_utils::{ParseReplyError, PaymentError},
};

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Overflow(#[from] OverflowError),

    #[error(transparent)]
    Instantiate2Address(#[from] Instantiate2AddressError),

    #[error(transparent)]
    Payment(#[from] PaymentError),

    #[error(transparent)]
    ParseReply(#[from] ParseReplyError),

    #[error("query failed due to system error: {0}")]
    QuerySystem(#[from] cosmwasm_std::SystemError),

    #[error("query failed due to contract error: {0}")]
    QueryContract(String),

    #[error("action queue cannot be empty")]
    EmptyActionQueue,

    #[error("account factory failed to return instantiate data in its response")]
    FactoryResponseDataMissing,

    #[error("cannot create voucher token because token create fee is non-zero")]
    NonZeroTokenCreationFee,

    #[error("unauthorized")]
    Unauthorized,

    #[error("ICS-999 channel may not be closed")]
    UnexpectedChannelClosure,

    #[error("packet does not contain the trace for denom `{denom}`")]
    TraceNotFound {
        denom: String,
    },

    #[error("incorrect amount of funds sent: expecting `{expected}`, found `{actual}`")]
    FundsMismatch {
        actual:   Coins,
        expected: Coins,
    },

    #[error("incorrect IBC channel order: expecting `{expected:?}`, found `{actual:?}`")]
    IncorrectOrder {
        actual:   IbcOrder,
        expected: IbcOrder,
    },

    #[error("incorrect IBC channel version: expecting `{expected}`, found `{actual}`")]
    IncorrectVersion {
        actual:   String,
        expected: String,
    },

    #[error("an open ICS-999 channel already exists on connection `{connection_id}`")]
    ChannelExists {
        connection_id: String,
    },

    #[error("an interchain account already exists for endpoint `{endpoint:?}`, and controller `{controller}`")]
    AccountExists {
        endpoint:   IbcEndpoint,
        controller: String,
    },

    #[error("no interchain account found at endpoint `{endpoint:?}`, and controller `{controller}`")]
    AccountNotFound {
        endpoint:   IbcEndpoint,
        controller: String,
    },
}

pub(crate) type Result<T> = core::result::Result<T, Error>;
