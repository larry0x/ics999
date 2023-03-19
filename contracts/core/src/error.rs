use cosmwasm_std::{IbcOrder, StdError};

#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

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

    #[error("ICS-999 channel may not be closed")]
    UnexpectedChannelClosure,
}

pub type ContractResult<T> = Result<T, ContractError>;
