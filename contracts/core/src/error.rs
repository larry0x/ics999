use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;

#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownership(#[from] OwnershipError),
}

pub type ContractResult<T> = Result<T, ContractError>;
