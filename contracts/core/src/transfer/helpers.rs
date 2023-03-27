use cosmwasm_std::{
    attr, coin, Addr, Attribute, BankMsg, Coin, CosmosMsg, QuerierWrapper, Response, Uint128,
};
use token_factory::{construct_denom, TokenFactoryMsg, TokenFactoryQuery};

use crate::error::ContractError;

pub fn create_and_mint(
    querier: &QuerierWrapper<TokenFactoryQuery>,
    creator: &Addr,
    subdenom: String,
    amount: Uint128,
    to: &Addr,
    res: Response<TokenFactoryMsg>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    // we can only create the denom if denom create fee is zero
    let tf_params = token_factory::query_params(querier)?;
    if !tf_params.params.denom_creation_fee.is_empty() {
        return Err(ContractError::NonZeroTokenCreationFee);
    }

    mint(
        coin(amount.u128(), construct_denom(creator.as_str(), &subdenom)),
        to,
        res.add_message(TokenFactoryMsg::CreateDenom {
            subdenom,
        }),
    )
}

pub fn mint(
    coin: Coin,
    to: &Addr,
    res: Response<TokenFactoryMsg>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    Ok(res
        .add_attribute("coin", coin.to_string())
        .add_attribute("action", "mint")
        .add_message(TokenFactoryMsg::MintTokens {
            denom: coin.denom,
            amount: coin.amount,
            mint_to_address: to.into(),
        }))
}

pub fn burn(
    coin: Coin,
    from: &Addr,
    msgs: &mut Vec<CosmosMsg<TokenFactoryMsg>>,
    attrs: &mut Vec<Attribute>,
) {
    attrs.push(attr("coin", coin.to_string()));
    attrs.push(attr("action", "burn"));
    msgs.push(TokenFactoryMsg::BurnTokens {
        denom: coin.denom,
        amount: coin.amount,
        burn_from_address: from.into(),
    }
    .into());
}

pub fn escrow(coin: &Coin, attrs: &mut Vec<Attribute>) {
    attrs.push(attr("coin", coin.to_string()));
    attrs.push(attr("action", "escrow"));
}

// pub fn burn(
//     coin: Coin,
//     from: &Addr,
//     res: Response<TokenFactoryMsg>,
// ) -> Result<Response<TokenFactoryMsg>, ContractError> {
//     Ok(res
//         .add_attribute("coin", coin.to_string())
//         .add_attribute("action", "burn")
//         .add_message(TokenFactoryMsg::BurnTokens {
//             denom: coin.denom,
//             amount: coin.amount,
//             burn_from_address: from.into(),
//         }))
// }

// pub fn escrow(coin: Coin, res: Response) -> Result<Response, ContractError> {
//     Ok(res
//         .add_attribute("coin", coin.to_string())
//         .add_attribute("action", "escrow"))
// }

pub fn release(coin: Coin, to: &Addr, res: Response) -> Result<Response, ContractError> {
    Ok(res
        .add_attribute("coin", coin.to_string())
        .add_attribute("action", "release")
        .add_message(BankMsg::Send {
            to_address: to.into(),
            amount: vec![coin],
        }))
}

/// Check whether a tokenfactory denom exists.
///
/// We do this by attempting to query the denom's metadata. If it errors, we
/// assume the token doesn't exist.
///
/// This approach ignores other possible errors such as serde errors, but I
/// can't think of a better method.
fn tf_denom_exists(querier: &QuerierWrapper<TokenFactoryQuery>, denom: &str) -> bool {
    token_factory::query_metadata(querier, denom).is_ok()
}
