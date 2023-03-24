use cosmwasm_std::{QuerierWrapper, StdResult};

use crate::*;

/// Construct a full denom from a creator address and a subdenom
pub fn construct_denom(creator: &str, subdenom: &str) -> String {
    format!("{DENOM_PREFIX}/{creator}/{subdenom}")
}

/// Deconstruct a full denom into the creator address and the subdenom.
/// None if the denom does not have the correct format.
pub fn deconstruct_denom(denom: &str) -> Option<(&str, &str)> {
    let Some((prefix, creator_and_subdenom)) = denom.split_once('/') else {
        return None;
    };

    if prefix != DENOM_PREFIX {
        return None;
    }

    creator_and_subdenom.split_once('/')
}

pub fn query_full_denom(
    querier: &QuerierWrapper<TokenFactoryQuery>,
    creator_addr: impl Into<String>,
    subdenom: impl Into<String>,
) -> StdResult<FullDenomResponse> {
    querier.query(
        &TokenFactoryQuery::FullDenom {
            creator_addr: creator_addr.into(),
            subdenom: subdenom.into(),
        }
        .into(),
    )
}

pub fn query_admin(
    querier: &QuerierWrapper<TokenFactoryQuery>,
    denom: impl Into<String>,
) -> StdResult<AdminResponse> {
    querier.query(
        &TokenFactoryQuery::Admin {
            denom: denom.into(),
        }
        .into(),
    )
}

pub fn query_metadata(
    querier: &QuerierWrapper<TokenFactoryQuery>,
    denom: impl Into<String>,
) -> StdResult<MetadataResponse> {
    querier.query(
        &TokenFactoryQuery::Metadata {
            denom: denom.into(),
        }
        .into(),
    )
}

pub fn query_denoms_by_creator(
    querier: &QuerierWrapper<TokenFactoryQuery>,
    creator: impl Into<String>,
) -> StdResult<DenomsByCreatorResponse> {
    querier.query(
        &TokenFactoryQuery::DenomsByCreator {
            creator: creator.into(),
        }
        .into(),
    )
}

pub fn query_params(querier: &QuerierWrapper<TokenFactoryQuery>) -> StdResult<ParamsResponse> {
    querier.query(&TokenFactoryQuery::Params {}.into())
}
