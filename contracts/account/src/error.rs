#[derive(Debug, PartialEq, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    Ownership(#[from] cw_ownable::OwnershipError),

    #[error("query failed due to system error: {0}")]
    QuerySystem(#[from] cosmwasm_std::SystemError),

    #[error("query failed due to contract error: {0}")]
    QueryContract(String),

    #[error("submessage failed to execute: {0}")]
    SubMsgFailed(String),

    #[error("unknown reply id: {0}")]
    UnknownReplyId(u64),
}

pub(crate) type Result<T> = core::result::Result<T, Error>;
