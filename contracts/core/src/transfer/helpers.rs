use cosmwasm_std::{attr, Attribute, BankMsg, Coin, CosmosMsg, IbcEndpoint, QuerierWrapper};
use osmosis_std::types::{
    cosmos::base::v1beta1::Coin as ProtoCoin, osmosis::tokenfactory::v1beta1 as tokenfactory,
};

use ics999::Trace;

use crate::{error::ContractError, transfer::TraceItem, utils::Coins};

/// This is called when a user sends funds to the contract to be transferred to
/// another chain. ("Outgoing" in the sense that the coin is going out to
/// another chain.)
pub fn handle_outgoing_coin(
    contract: impl Into<String>,
    coin: Coin,
    trace: TraceItem,
    src: &IbcEndpoint,
    traces: &mut Vec<Trace>,
    expect_funds: &mut Coins,
    msgs: &mut Vec<CosmosMsg>,
    attrs: &mut Vec<Attribute>,
) -> Result<(), ContractError> {
    if trace.sender_is_source(src) {
        escrow(&coin, attrs);
    } else {
        // note that we burn from the contract address instead of from
        // info.sender
        // this is because the token to be burned should have already
        // been sent to the contract address along with the executeMsg
        burn(contract, coin.clone(), msgs, attrs);
    }

    if !contains_denom(&traces, &coin.denom) {
        traces.push(trace.into_full_trace(&coin.denom));
    }

    expect_funds.add(coin)?;

    Ok(())
}

/// This is called when the contract receives funds from another chain and needs
/// to transfer them to a user. ("Incoming" in the sense that the coin came in
/// from another chain.)
pub fn handle_incoming_coin(
    contract: impl Into<String>,
    recipient: impl Into<String>,
    coin: Coin,
    trace: TraceItem,
    src: &IbcEndpoint,
    msgs: &mut Vec<CosmosMsg>,
    attrs: &mut Vec<Attribute>,
) {
    if trace.sender_is_source(src) {
        release(coin, recipient, msgs, attrs);
    } else {
        mint(contract, recipient, coin,  msgs, attrs);
    }
}

fn mint(
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

fn burn(
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

fn release(
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

fn escrow(coin: &Coin, attrs: &mut Vec<Attribute>) {
    attrs.push(attr("coin", coin.to_string()));
    attrs.push(attr("action", "escrow"));
}

fn contains_denom(traces: &[Trace], denom: &str) -> bool {
    traces.iter().any(|trace| trace.denom == *denom)
}

/// Combine a creator address and a subdenom into the tokenfactory full denom
pub fn construct_denom(creator: &str, subdenom: &str) -> String {
    format!("factory/{creator}/{subdenom}")
}

/// Convert a cosmwasm_std::Coin into a /cosmos.base.v1beta1.coin
pub fn into_proto_coin(coin: Coin) -> ProtoCoin {
    ProtoCoin {
        denom: coin.denom,
        amount: coin.amount.to_string(),
    }
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
