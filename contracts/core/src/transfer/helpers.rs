use cosmwasm_std::{attr, Attribute, BankMsg, Coin, CosmosMsg, QuerierWrapper, StdResult};
use osmosis_std::types::{
    cosmos::base::v1beta1::Coin as ProtoCoin, osmosis::tokenfactory::v1beta1 as tokenfactory,
};

use crate::error::ContractError;

pub fn mint(
    sender: impl Into<String>,
    to: impl Into<String>,
    coin: Coin,
    msgs: &mut Vec<CosmosMsg>,
    attrs: &mut Vec<Attribute>,
) {
    attrs.push(attr("coin", coin.to_string()));
    attrs.push(attr("action", "mint"));
    msgs.push(
        tokenfactory::MsgMint {
            sender: sender.into(),
            amount: Some(into_proto_coin(coin.clone())),
        }
        .into(),
    );
    msgs.push(
        BankMsg::Send {
            to_address: to.into(),
            amount: vec![coin],
        }
        .into(),
    );
}

pub fn burn(
    sender: impl Into<String>,
    coin: Coin,
    msgs: &mut Vec<CosmosMsg>,
    attrs: &mut Vec<Attribute>,
) {
    attrs.push(attr("coin", coin.to_string()));
    attrs.push(attr("action", "burn"));
    msgs.push(
        tokenfactory::MsgBurn {
            sender: sender.into(),
            amount: Some(into_proto_coin(coin)),
        }
        .into(),
    );
}

pub fn release(
    coin: Coin,
    to: impl Into<String>,
    msgs: &mut Vec<CosmosMsg>,
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

pub fn construct_denom(creator: &str, subdenom: &str) -> String {
    format!("factory/{creator}/{subdenom}")
}

pub fn into_proto_coin(coin: Coin) -> ProtoCoin {
    ProtoCoin {
        denom: coin.denom,
        amount: coin.amount.to_string(),
    }
}

/// Check whether a tokenfactory denom exists.
///
/// We do this by attempting to query the denom's metadata. If it errors, we
/// assume the token doesn't exist.
///
/// This approach ignores other possible errors such as serde errors, but I
/// can't think of a better method.
pub fn denom_exists(querier: &QuerierWrapper, denom: &str) -> StdResult<bool> {
    Ok(tokenfactory::TokenfactoryQuerier::new(querier)
        .denom_authority_metadata(denom.into())?
        .authority_metadata
        .is_some())
}

/// Assert that denom creation fee is zero.
///
/// We don't have the money to pay the fee. If the fee is non-zero then we
/// simply refuse to complete the transfer.
pub fn assert_free_denom_creation(querier: &QuerierWrapper) -> Result<(), ContractError> {
    let fee = tokenfactory::TokenfactoryQuerier::new(querier)
        .params()?
        .params
        .expect("params response does not contain params")
        .denom_creation_fee;

    if !fee.is_empty() {
        return Err(ContractError::NonZeroTokenCreationFee);
    }

    Ok(())
}
