mod helpers;
mod msg;
mod query;
mod types;

pub use helpers::*;
pub use msg::*;
pub use query::*;
pub use types::*;

pub type Deps<'a> = cosmwasm_std::Deps<'a, TokenFactoryQuery>;
pub type DepsMut<'a> = cosmwasm_std::DepsMut<'a, TokenFactoryQuery>;
pub type Response = cosmwasm_std::Response<TokenFactoryMsg>;
pub type QueryRequest = cosmwasm_std::QueryRequest<TokenFactoryQuery>;
pub type QuerierWrapper<'a> = cosmwasm_std::QuerierWrapper<'a, TokenFactoryQuery>;

pub const DENOM_PREFIX: &str = "factory";
