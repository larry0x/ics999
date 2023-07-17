use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    instantiate2_address, to_binary, Addr, BankMsg, Binary, Coin, DepsMut, Empty, Env, IbcEndpoint,
    QueryRequest, Response, StdResult, Storage, SubMsg, WasmMsg, WasmQuery,
};
use cw_storage_plus::Item;
use cw_utils::{parse_execute_response_data, parse_instantiate_response_data};
use osmosis_std::types::osmosis::tokenfactory::v1beta1 as tokenfactory;

use ics999::{Action, ActionResult, RegisterOptions, Trace, FactoryExecuteMsg, FactoryMsg};

use crate::{
    error::{Error, Result},
    state::{ACCOUNTS, CONFIG, DENOM_TRACES},
    transfer::{assert_free_denom_creation, construct_denom, into_proto_coin, TraceItem},
    utils::default_salt,
    AFTER_ACTION,
    AFTER_CUSTOM_FACTORY,
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
        store:       &dyn Storage,
        src:         IbcEndpoint,
        dest:        IbcEndpoint,
        controller:  String,
        mut actions: Vec<Action>,
        traces:      Vec<Trace>,
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
            action:          None,
            pending_actions: actions,
            results:         vec![],
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
        deps:     DepsMut,
        env:      Env,
        response: Option<Response>,
    ) -> Result<Response> {
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
                    .ok_or(Error::TraceNotFound {
                        denom: src_denom,
                    })?
                    .into();

                let recipient = match recipient {
                    // if the sender doesn't specify the recipient, default to
                    // their interchain account
                    // error if the sender does not already own an ICA
                    None => self.host.clone().ok_or_else(|| Error::AccountNotFound {
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
                    let denom = construct_denom(env.contract.address.as_str(), &subdenom);

                    let new_token = !DENOM_TRACES.has(deps.storage, &denom);

                    // if the denom does not exist yet -- create the denom and
                    // save the trace to store
                    if new_token {
                        DENOM_TRACES.save(deps.storage, &denom, &trace)?;

                        // we can only create the denom if denom creation fee
                        // is zero
                        assert_free_denom_creation(&deps.querier)?;

                        response = response.add_message(tokenfactory::MsgCreateDenom {
                            sender: env.contract.address.to_string(),
                            subdenom,
                        });
                    }

                    self.results.push(ActionResult::Transfer {
                        denom:     denom.clone(),
                        new_token,
                        recipient: recipient.to_string(),
                    });

                    let coin = Coin {
                        denom,
                        amount,
                    };

                    // tokenfactory only supports minting to the sender
                    // therefore we first mint to ourself, then transfer to the recipient
                    response
                        .add_message(tokenfactory::MsgMint {
                            sender:          env.contract.address.clone().into(),
                            mint_to_address: env.contract.address.into(),
                            amount:          Some(into_proto_coin(coin.clone())),
                        })
                        .add_submessage(SubMsg::reply_on_success(
                            BankMsg::Send {
                                to_address: recipient.into(),
                                amount:     vec![coin],
                            },
                            AFTER_ACTION,
                        ))
                } else {
                    // pop the sender chain from the path
                    trace.path.pop();

                    // derive the ibc denom
                    let denom = if trace.path.is_empty() {
                        trace.base_denom
                    } else {
                        let subdenom = trace.hash().to_hex();
                        construct_denom(env.contract.address.as_str(), &subdenom)
                    };

                    self.results.push(ActionResult::Transfer {
                        denom:     denom.clone(),
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
                            amount:     vec![coin],
                        },
                        AFTER_ACTION,
                    ))
                }
            },

            Action::RegisterAccount(options) => {
                // match the type of registration flow
                match options {
                    RegisterOptions::Default { salt } => {
                        // only one ICA per controller allowed
                        if self.host.is_some() {
                            return Err(Error::AccountExists {
                                channel_id: self.dest.channel_id,
                                controller: self.controller,
                            })?;
                        }

                        // if a salt is not provided, by default use:
                        // sha256(channel_id_bytes | controller_addr_bytes)
                        let salt = salt.unwrap_or_else(|| default_salt(&self.dest.channel_id, &self.controller));

                        // load the one-account contract's code ID and checksum, which is
                        // used in Instantiate2 to determine the contract address
                        let cfg = CONFIG.load(deps.storage)?;
                        let code_res = deps.querier.query_wasm_code_info(cfg.default_account_code_id)?;

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
                                    code_id: cfg.default_account_code_id,
                                    msg:     to_binary(&Empty {})?,
                                    funds:   vec![],
                                    admin:   Some(env.contract.address.into()),
                                    label:   format!("one-account/{}/{}", self.dest.channel_id, self.controller),
                                    salt,
                                },
                                AFTER_ACTION,
                            ))
                    },
                    RegisterOptions::CustomFactory { address, data } => {
                        response
                            .add_attribute("action", "register_account")
                            .add_submessage(SubMsg::reply_on_success(
                                WasmMsg::Execute {
                                    contract_addr: address,
                                    msg: to_binary(&FactoryExecuteMsg::Ics999(FactoryMsg {
                                        src:        self.src.clone(),
                                        controller: self.controller.clone(),
                                        data,
                                    }))?,
                                    funds: vec![],
                                },
                                AFTER_CUSTOM_FACTORY,
                            ))
                    },
                }
            },

            Action::Execute(msg) => {
                let Some(addr) = &self.host else {
                    return Err(Error::AccountNotFound {
                        channel_id: self.dest.channel_id,
                        controller: self.controller,
                    });
                };

                response
                    .add_attribute("action", "execute")
                    .add_submessage(SubMsg::reply_on_success(
                        WasmMsg::Execute {
                            contract_addr: addr.into(),
                            msg,
                            funds: vec![],
                        },
                        AFTER_ACTION,
                    ))
            },

            Action::Query(msg) => {
                let Some(addr) = &self.host else {
                    return Err(Error::AccountNotFound {
                        channel_id: self.dest.channel_id,
                        controller: self.controller,
                    });
                };

                let query_req = to_binary(&QueryRequest::<Empty>::Wasm(WasmQuery::Smart {
                    contract_addr: addr.into(),
                    msg,
                }))?;

                let query_res = deps
                    .querier
                    .raw_query(&query_req)
                    .into_result()?
                    .into_result()
                    .map_err(Error::QueryContract)?;

                self.results.push(ActionResult::Query {
                    response: query_res,
                });

                response = response.add_attribute("action", "query");

                return self.handle_next_action(deps, env, Some(response));
            },
        };

        self.save(deps.storage)?;

        Ok(response)
    }

    /// After an `Execute` action has been completed, parse the response
    pub fn after_action(&mut self, deps: DepsMut, data: Option<Binary>) -> Result<()> {
        // the action that was executed
        let action = self.action.as_ref().expect("missing active action");

        // we only need to parse the result if the action is an msg execution
        if let Action::Execute(_) = action {
            // note that the contract being executed does not necessarily return
            // any data
            let data = data
                .map(|bin| parse_execute_response_data(&bin))
                .transpose()?
                .and_then(|res| res.data);

            self.results.push(ActionResult::Execute {
                data,
            });
        } else if let Action::RegisterAccount(RegisterOptions::CustomFactory { .. }) = action {
            // TODO: We could consider moving this into a separate reply_id
            // assert we have data
            let bin = data.ok_or(Error::FactoryResponseDataMissing)?;

            // We parse the response data from the custom factory (which is expected to be an MsgInstantiateContractResponse)
            // to get the ICA address.
            let account_address_str = parse_instantiate_response_data(&bin)?
                .contract_address;

            let addr = deps.api.addr_validate(&account_address_str)?;

            // save the address
            ACCOUNTS.save(deps.storage, (&self.dest.channel_id, &self.controller), &addr)?;

            self.results.push(ActionResult::RegisterAccount {
                address: addr.to_string(),
            });

            self.host = Some(addr);
        }

        Ok(())
    }

    fn default_handle_action_response<T>(&self) -> Response<T> {
        Response::new()
            .add_attribute("method", "handle_next_action")
            .add_attribute("actions_left", self.pending_actions.len().to_string())
    }
}
