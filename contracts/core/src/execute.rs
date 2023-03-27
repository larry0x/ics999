use cosmwasm_std::{
    attr, coin, to_binary, Addr, Attribute, BankMsg, Coin, CosmosMsg, DepsMut, Env, IbcMsg,
    IbcTimeout, MessageInfo, QuerierWrapper, Response, Uint128,
};
use one_types::{Action, PacketData, DenomTraceItem};
use token_factory::{construct_denom, TokenFactoryMsg, TokenFactoryQuery};

use crate::{
    utils::Coins,
    error::ContractError,
    msg::InstantiateMsg,
    state::{ACCOUNT_CODE_ID, ACTIVE_CHANNELS, DEFAULT_TIMEOUT_SECS, DENOM_TRACES},
};

pub fn init(deps: DepsMut, msg: InstantiateMsg) -> Result<Response, ContractError> {
    ACCOUNT_CODE_ID.save(deps.storage, &msg.account_code_id)?;
    DEFAULT_TIMEOUT_SECS.save(deps.storage, &msg.default_timeout_secs)?;

    Ok(Response::new())
}

pub fn act(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    connection_id: String,
    actions: Vec<Action>,
    opt_timeout: Option<IbcTimeout>,
) -> Result<Response<TokenFactoryMsg>, ContractError> {
    let received_funds = Coins::from(info.funds);
    let mut sending_funds = Coins::empty();
    let mut msgs = vec![];
    let mut attrs = vec![];
    let mut traces = vec![];

    for action in &actions {
        if let Action::Transfer { amount, .. } = action {
            // if the denom has a trace stored, then the current chain is the
            // source.
            // the last element of the trace must be the current chain, but no
            // need to verify it here.
            match DENOM_TRACES.may_load(deps.storage, &amount.denom)? {
                // current chain is the sink -- burn voucher token
                Some(trace) => {
                    traces.push(DenomTraceItem {
                        denom: amount.denom.clone(),
                        base_denom: trace.base_denom,
                        path: trace.path,
                    });
                    burn(amount.clone(), &info.sender, &mut msgs, &mut attrs);
                },

                // current chain is the source -- escrow
                None => {
                    escrow(amount, &mut attrs);
                },
            }

            sending_funds.add(amount.clone())?;
        }
    }

    if received_funds != sending_funds {
        return Err(ContractError::FundsMismatch {
            actual: received_funds,
            expected: sending_funds,
        });
    }

    let timeout = match opt_timeout {
        None => {
            let default_secs = DEFAULT_TIMEOUT_SECS.load(deps.storage)?;
            IbcTimeout::with_timestamp(env.block.time.plus_seconds(default_secs))
        },
        Some(to) => to,
    };

    Ok(Response::new()
        .add_messages(msgs)
        .add_message(IbcMsg::SendPacket {
            channel_id: ACTIVE_CHANNELS.load(deps.storage, &connection_id)?,
            data: to_binary(&PacketData {
                sender: info.sender.into(),
                actions,
                traces,
            })?,
            timeout,
        })
        .add_attribute("action", "act")
        .add_attributes(attrs))
}

fn create_and_mint(
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

fn mint(
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

fn burn(
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

fn escrow(coin: &Coin, attrs: &mut Vec<Attribute>) {
    attrs.push(attr("coin", coin.to_string()));
    attrs.push(attr("action", "escrow"));
}

// fn burn(
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

// fn escrow(coin: Coin, res: Response) -> Result<Response, ContractError> {
//     Ok(res
//         .add_attribute("coin", coin.to_string())
//         .add_attribute("action", "escrow"))
// }

fn release(coin: Coin, to: &Addr, res: Response) -> Result<Response, ContractError> {
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
