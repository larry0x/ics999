use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::CustomQuery;

use crate::{Metadata, Params};

#[cw_serde]
#[derive(QueryResponses)]
pub enum TokenFactoryQuery {
    /// Given a subdenom minted by a contract via `TokenFactoryMsg::MintTokens`,
    /// returns the full denom as used by `BankMsg::Send`.
    #[returns(FullDenomResponse)]
    FullDenom {
        creator_addr: String,
        subdenom: String,
    },

    /// Return the admin of a denom, if the denom is a Token Factory denom.
    #[returns(AdminResponse)]
    Admin {
        denom: String,
    },

    /// Return the metadata of a denom
    #[returns(MetadataResponse)]
    Metadata {
        denom: String,
    },

    /// Return the list of denoms created by the specified account
    #[returns(DenomsByCreatorResponse)]
    DenomsByCreator {
        creator: String,
    },

    /// Return tokenfactory module's params
    #[returns(ParamsResponse)]
    Params {},
}

// we don't need to impl From<TokenFactoryQuery> for QueryRequest because it's
// already implemented by the CustomQuery trait.
//
// weirdly tho, this is not the case for CustomMsg. maybe time for a PR
impl CustomQuery for TokenFactoryQuery {}

#[cw_serde]
pub struct FullDenomResponse {
    pub denom: String,
}

#[cw_serde]
pub struct AdminResponse {
    pub admin: String,
}

#[cw_serde]
pub struct MetadataResponse {
    pub metadata: Option<Metadata>,
}

#[cw_serde]
pub struct DenomsByCreatorResponse {
    pub denoms: Vec<String>,
}

#[cw_serde]
pub struct ParamsResponse {
    pub params: Params,
}
