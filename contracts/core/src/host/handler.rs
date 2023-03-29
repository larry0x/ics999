use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    instantiate2_address, to_binary, Addr, BankMsg, Binary, Coin, ContractResult, DepsMut, Empty,
    Env, IbcEndpoint, Response, StdResult, Storage, SubMsg, SystemResult, WasmMsg,
};
use cw_storage_plus::Item;
use cw_utils::parse_execute_response_data;
use one_types::{Action, ActionResult, Trace};
use token_factory::{construct_denom, TokenFactoryMsg, TokenFactoryQuery};

use crate::{
    error::ContractError,
    state::{ACCOUNTS, ACCOUNT_CODE_ID, DENOM_TRACES},
    transfer::{assert_free_denom_creation, denom_exists, TraceItem},
    utils::default_salt,
    AFTER_ACTION,
};

const HANDLER: Item<Handler> = Item::new("handler");

/// An ICS-999 packet contains one or more `Action`'s that need to be executed
/// one at a time and atomically.
///
/// Handler is an object that contains necessary states and methods for
/// executing the actions. It also implements serde traits so that it can be
/// saved/loaded from the contract store.
#[cw_serde]
pub(super) struct Handler {
    /// The chain where the packet was sent from, i.e. the controller chain
    pub src: IbcEndpoint,

    /// The current chain, i.e. the host chain
    pub dest: IbcEndpoint,

    /// The account who sent the packet on the sender chain
    pub controller: String,

    /// The interchain account controlled by the sender
    pub host: Option<Addr>,

    /// Traces of all tokens being transferred in the packet
    pub traces: Vec<Trace>,

    /// Index of the current action being executed, starting from 0.
    /// Used only for event logging.
    pub action_index: u64,

    /// The action is to be executed at the current step.
    /// None means all actions have finished executing.
    pub action: Option<Action>,

    /// The actions that are to be executed later, in reverse order.
    pub pending_actions: Vec<Action>,

    /// The results from executing the earlier actions
    ///
    /// At the end of each step, the response data is parsed and pushed into
    /// this queue.
    ///
    /// Once all actions have finished executing, this enture queue is returned
    /// in the packet acknowledgement.
    pub results: Vec<ActionResult>,
}

impl Handler {
    pub fn create(
        store: &dyn Storage,
        src: IbcEndpoint,
        dest: IbcEndpoint,
        controller: String,
        mut actions: Vec<Action>,
        traces: Vec<Trace>,
    ) -> StdResult<Self> {
        // load the controller's ICA host, which may or may not have already
        // been instantiated
        let host = ACCOUNTS.may_load(store, (&dest.channel_id, &controller))?;

        // reverse the actions, so that we can use pop() to grab the 1st action
        actions.reverse();

        Ok(Self {
            src,
            dest,
            controller,
            host,
            traces,
            action_index: 0,
            action: None,
            pending_actions: actions,
            results: vec![],
        })
    }

    pub fn load(store: &dyn Storage) -> StdResult<Self> {
        HANDLER.load(store)
    }

    fn save(&self, store: &mut dyn Storage) -> StdResult<()> {
        HANDLER.save(store, self)
    }

    fn remove(store: &mut dyn Storage) {
        HANDLER.remove(store)
    }

    /// Execute the next action in the queue. Saved the updated handler state.
    pub fn handle_next_action(
        mut self,
        deps: DepsMut<TokenFactoryQuery>,
        env: Env,
        response: Option<Response<TokenFactoryMsg>>,
    ) -> Result<Response<TokenFactoryMsg>, ContractError> {
        let mut response = response.unwrap_or_else(|| self.default_handle_action_response());

        // grab the first action in the queue
        self.action = self.pending_actions.pop();

        // if there is no more action to execute
        // delete handler state from contract store, return the results as data
        // in the response
        let Some(action) = &self.action else {
            Handler::remove(deps.storage);

            return Ok(response.set_data(to_binary(&self.results)?));
        };

        // convert the action to the appropriate msgs and event attributes
        let response = match action.clone() {
            Action::Transfer {
                denom: src_denom,
                amount,
                recipient,
            } => {
                response = response.add_attribute("action", "transfer");

                let mut trace: TraceItem = self
                    .traces
                    .iter()
                    .find(|trace| trace.denom == src_denom)
                    .ok_or(ContractError::TraceNotFound {
                        denom: src_denom,
                    })?
                    .into();

                let recipient = match recipient {
                    // if the sender doesn't specify the recipient, default to
                    // their interchain account
                    // error if the sender does not already own an ICA
                    None => self.host.clone().ok_or_else(|| ContractError::AccountNotFound {
                        channel_id: self.dest.channel_id.clone(),
                        controller: self.controller.clone(),
                    })?,

                    // if the sender does specify a recipient, simply validate
                    // the address
                    Some(r) => deps.api.addr_validate(&r)?,
                };

                if trace.sender_is_source(&self.src) {
                    // append current chain to the path
                    trace.path.push(self.dest.clone());

                    // derive the ibc denom
                    let subdenom = trace.hash().to_hex();
                    let denom = construct_denom(&self.dest.port_id, &subdenom);
                    let new_token = !denom_exists(&deps.querier, &denom);

                    // if the denom does not exist yet -- create the denom and
                    // save the trace to store
                    if new_token {
                        DENOM_TRACES.save(deps.storage, &denom, &trace)?;

                        // we can only create the denom if denom creation fee
                        // is zero
                        assert_free_denom_creation(&deps.querier)?;

                        response = response.add_message(TokenFactoryMsg::CreateDenom {
                            subdenom,
                        });
                    }

                    self.results.push(ActionResult::Transfer {
                        denom: denom.clone(),
                        new_token,
                        recipient: recipient.to_string(),
                    });

                    response.add_submessage(SubMsg::reply_on_success(
                        TokenFactoryMsg::MintTokens {
                            denom,
                            amount,
                            mint_to_address: recipient.into(),
                        },
                        AFTER_ACTION,
                    ))
                } else {
                    // pop the sender chain from the path
                    trace.path.pop();

                    // derive the ibc denom
                    let subdenom = trace.hash().to_hex();
                    let denom = construct_denom(&self.dest.port_id, &subdenom);

                    self.results.push(ActionResult::Transfer {
                        denom: denom.clone(),
                        new_token: false,
                        recipient: recipient.to_string(),
                    });

                    let coin = Coin {
                        denom,
                        amount,
                    };

                    response.add_submessage(SubMsg::reply_on_success(
                        BankMsg::Send {
                            to_address: recipient.into(),
                            amount: vec![coin],
                        },
                        AFTER_ACTION,
                    ))
                }
            },

            Action::RegisterAccount {
                salt,
            } => {
                // only one ICA per controller allowed
                if self.host.is_some() {
                    return Err(ContractError::AccountExists {
                        channel_id: self.dest.channel_id,
                        controller: self.controller,
                    })?;
                }

                // if a salt is not provided, by default use:
                // sha256(channel_id_bytes | controller_addr_bytes)
                let salt = salt.unwrap_or_else(|| default_salt(&self.dest.channel_id, &self.controller));

                // load the one-account contract's code ID and checksum, which is
                // used in Instantiate2 to determine the contract address
                let code_id = ACCOUNT_CODE_ID.load(deps.storage)?;
                let code_res = deps.querier.query_wasm_code_info(code_id)?;

                // predict the contract address
                let addr_raw = instantiate2_address(
                    &code_res.checksum,
                    &deps.api.addr_canonicalize(env.contract.address.as_str())?,
                    &salt,
                )?;
                let addr = deps.api.addr_humanize(&addr_raw)?;

                ACCOUNTS.save(deps.storage, (&self.dest.channel_id, &self.controller), &addr)?;

                self.results.push(ActionResult::RegisterAccount {
                    address: addr.to_string(),
                });

                self.host = Some(addr);

                response
                    .add_attribute("action", "register_account")
                    .add_submessage(SubMsg::reply_on_success(
                        WasmMsg::Instantiate2 {
                            admin: Some(env.contract.address.into()),
                            code_id,
                            label: format!("one-account/{}/{}", self.dest.channel_id, self.controller),
                            msg: to_binary(&Empty {})?,
                            funds: vec![],
                            salt,
                        },
                        AFTER_ACTION,
                    ))
            },

            Action::Execute(cosmos_msg) => {
                let Some(addr) = &self.host else {
                    return Err(ContractError::AccountNotFound {
                        channel_id: self.dest.channel_id,
                        controller: self.controller,
                    });
                };

                response
                    .add_attribute("action", "execute")
                    .add_submessage(SubMsg::reply_on_success(
                        WasmMsg::Execute {
                            contract_addr: addr.into(),
                            msg: to_binary(&cosmos_msg)?,
                            funds: vec![],
                        },
                        AFTER_ACTION,
                    ))
            },

            Action::Query(query_req) => {
                let query_res = deps.querier.raw_query(&to_binary(&query_req)?);

                let SystemResult::Ok(ContractResult::Ok(query_res_bin)) = query_res else {
                    return Err(ContractError::QueryFailed);
                };

                self.results.push(ActionResult::Query {
                    response: query_res_bin,
                });

                response = response.add_attribute("action", "query");

                return self.handle_next_action(deps, env, Some(response));
            },
        };

        self.save(deps.storage)?;

        Ok(response)
    }

    /// After an `Execute` action has been completed, parse the response
    pub fn after_action(&mut self, data: Option<Binary>) -> Result<(), ContractError> {
        // the action that was executed
        let action = self.action.as_ref().expect("missing active action");

        // we only need to parse the result if the action is an msg execution
        if let Action::Execute(_) = action {
            let data = data.expect("missing wasm execute response data");
            let execute_res = parse_execute_response_data(&data)?;

            self.results.push(ActionResult::Execute {
                data: execute_res.data,
            });
        }

        Ok(())
    }

    fn default_handle_action_response<T>(&self) -> Response<T> {
        Response::new()
            .add_attribute("method", "handle_next_action")
            .add_attribute("action_index", self.action_index.to_string())
    }
}
