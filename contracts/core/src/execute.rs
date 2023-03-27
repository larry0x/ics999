use cosmwasm_std::{to_binary, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo, Response};
use one_types::{Action, PacketData, Trace};
use token_factory::TokenFactoryMsg;

use crate::{
    utils::Coins,
    error::ContractError,
    msg::InstantiateMsg,
    state::{ACCOUNT_CODE_ID, ACTIVE_CHANNELS, DEFAULT_TIMEOUT_SECS, DENOM_TRACES},
    transfer::{burn, escrow},
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
                    traces.push(Trace {
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
