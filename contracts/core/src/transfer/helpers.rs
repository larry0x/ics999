use cosmwasm_std::{attr, Attribute, BankMsg, Coin, CosmosMsg, QuerierWrapper};
use token_factory::{TokenFactoryMsg, TokenFactoryQuery};

use crate::error::ContractError;

pub fn create_denom(
    subdenom: String,
    msgs: &mut Vec<CosmosMsg<TokenFactoryMsg>>,
    attrs: &mut Vec<Attribute>,
) {
    attrs.push(attr("msg", "create_denom"));
    attrs.push(attr("subdenom", &subdenom));
    msgs.push(
        TokenFactoryMsg::CreateDenom {
            subdenom,
        }
        .into(),
    );
}

pub fn mint(
    coin: Coin,
    to: impl Into<String>,
    msgs: &mut Vec<CosmosMsg<TokenFactoryMsg>>,
    attrs: &mut Vec<Attribute>,
) {
    attrs.push(attr("coin", coin.to_string()));
    attrs.push(attr("action", "mint"));
    msgs.push(
        TokenFactoryMsg::MintTokens {
            denom: coin.denom,
            amount: coin.amount,
            mint_to_address: to.into(),
        }
        .into(),
    );
}

pub fn burn(
    coin: Coin,
    from: impl Into<String>,
    msgs: &mut Vec<CosmosMsg<TokenFactoryMsg>>,
    attrs: &mut Vec<Attribute>,
) {
    attrs.push(attr("coin", coin.to_string()));
    attrs.push(attr("action", "burn"));
    msgs.push(
        TokenFactoryMsg::BurnTokens {
            denom: coin.denom,
            amount: coin.amount,
            burn_from_address: from.into(),
        }
        .into(),
    );
}

pub fn release<T>(
    coin: Coin,
    to: impl Into<String>,
    msgs: &mut Vec<CosmosMsg<T>>,
    attrs: &mut Vec<Attribute>,
) {
    attrs.push(attr("coin", coin.to_string()));
    attrs.push(attr("action", "release"));
    msgs.push(
        BankMsg::Send {
            to_address: to.into(),
            amount: vec![coin],
        }
        .into(),
    );
}

pub fn escrow(coin: &Coin, attrs: &mut Vec<Attribute>) {
    attrs.push(attr("coin", coin.to_string()));
    attrs.push(attr("action", "escrow"));
}

/// Check whether a tokenfactory denom exists.
///
/// We do this by attempting to query the denom's metadata. If it errors, we
/// assume the token doesn't exist.
///
/// This approach ignores other possible errors such as serde errors, but I
/// can't think of a better method.
pub fn denom_exists(querier: &QuerierWrapper<TokenFactoryQuery>, denom: &str) -> bool {
    token_factory::query_metadata(querier, denom).is_ok()
}

/// Assert that denom creation fee is zero.
///
/// We don't have the money to pay the fee. If the fee is non-zero then we
/// simply refuse to complete the transfer.
pub fn assert_free_denom_creation(
    querier: &QuerierWrapper<TokenFactoryQuery>,
) -> Result<(), ContractError> {
    let params = token_factory::query_params(querier)?;

    if !params.params.denom_creation_fee.is_empty() {
        return Err(ContractError::NonZeroTokenCreationFee);
    }

    Ok(())
}
