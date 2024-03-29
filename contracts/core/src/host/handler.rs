use {
    crate::{
        error::{Error, Result},
        state::{ACCOUNTS, CONFIG, DENOM_TRACES},
        transfer::{assert_free_denom_creation, construct_denom, into_proto_coin, TraceItem},
        AFTER_ACTION,
    },
    cosmwasm_schema::cw_serde,
    cosmwasm_std::{
        from_binary, instantiate2_address, to_binary, Addr, BankMsg, Binary, Coin, Deps, DepsMut,
        Empty, Env, IbcEndpoint, QueryRequest, Response, StdResult, Storage, SubMsg, Uint128,
        WasmMsg, WasmQuery,
    },
    cw_storage_plus::Item,
    cw_utils::parse_execute_response_data,
    ics999::{
        Action, ActionResult, FactoryExecuteMsg, FactoryMsg, FactoryResponse, RegisterOptions,
        Trace,
    },
    osmosis_std::types::osmosis::tokenfactory::v1beta1 as tokenfactory,
    sha2::{Digest, Sha256},
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
    counterparty_endpoint: IbcEndpoint,
    endpoint:              IbcEndpoint,
    controller:            String,
    host:                  Option<Addr>,
    traces:                Vec<Trace>,
    action:                Option<Action>,
    pending_actions:       Vec<Action>,
    results:               Vec<ActionResult>,
}

impl Handler {
    pub fn create(
        store:                 &dyn Storage,
        counterparty_endpoint: IbcEndpoint,
        endpoint:              IbcEndpoint,
        controller:            String,
        mut actions:           Vec<Action>,
        traces:                Vec<Trace>,
    ) -> StdResult<Self> {
        // load the controller's ICA host, which may or may not have already
        // been instantiated
        let host = ACCOUNTS.may_load(store, (&endpoint.port_id, &endpoint.channel_id, &controller))?;

        // reverse the actions, so that we can use pop() to grab the 1st action
        actions.reverse();

        Ok(Self {
            counterparty_endpoint,
            endpoint,
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
        mut deps: DepsMut,
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
                denom,
                amount,
                recipient,
            } => self.handle_transfer(response, deps.branch(), env, denom, amount, recipient)?,

            Action::RegisterAccount(RegisterOptions::Default {
                salt,
            }) => self.handle_register_account_default(response, deps.branch(), env, salt)?,

            Action::RegisterAccount(RegisterOptions::CustomFactory {
                address,
                data,
            }) => self.handle_register_account_custom_factory(response, address, data)?,

            Action::Query(msg) => {
                response = self.handle_query(response, deps.as_ref(), msg)?;
                return self.handle_next_action(deps, env, Some(response));
            },

            Action::Execute(msg) => self.handle_execute(response, msg)?,
        };

        self.save(deps.storage)?;

        Ok(response)
    }

    fn handle_transfer(
        &mut self,
        mut response: Response,
        deps:         DepsMut,
        env:          Env,
        src_denom:    String,
        amount:       Uint128,
        recipient:    Option<String>,
    ) -> Result<Response> {
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
            None => self.get_host().cloned()?,

            // if the sender does specify a recipient, simply validate
            // the address
            Some(r) => deps.api.addr_validate(&r)?,
        };

        if trace.sender_is_source(&self.counterparty_endpoint) {
            // append current chain to the path
            trace.path.push(self.endpoint.clone());

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
            Ok(response
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
                )))
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

            Ok(response.add_submessage(SubMsg::reply_on_success(
                BankMsg::Send {
                    to_address: recipient.into(),
                    amount:     vec![coin],
                },
                AFTER_ACTION,
            )))
        }
    }

    fn handle_register_account_default(
        &mut self,
        response: Response,
        deps:     DepsMut,
        env:      Env,
        salt:     Option<Binary>,
    ) -> Result<Response> {
        // only one ICA per controller allowed
        self.assert_no_host()?;

        // if a salt is not provided, by default use:
        // sha256(channel_id_bytes | controller_addr_bytes)
        let salt = salt.unwrap_or_else(|| self.default_salt());

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

        ACCOUNTS.save(
            deps.storage,
            (&self.endpoint.port_id, &self.endpoint.channel_id, &self.controller),
            &addr,
        )?;

        self.results.push(ActionResult::RegisterAccount { address: addr.to_string() });
        self.host = Some(addr);

        Ok(response
            .add_attribute("action", "register_account")
            .add_submessage(SubMsg::reply_on_success(
                WasmMsg::Instantiate2 {
                    code_id: cfg.default_account_code_id,
                    msg:     to_binary(&Empty {})?,
                    funds:   vec![],
                    admin:   Some(env.contract.address.into()),
                    label:   format!("one-account/{}/{}", self.endpoint.channel_id, self.controller),
                    salt,
                },
                AFTER_ACTION,
            )))
    }

    fn handle_register_account_custom_factory(
        &self,
        response: Response,
        factory:  String,
        data:     Option<Binary>,
    ) -> Result<Response> {
        // only one ICA per controller allowed
        self.assert_no_host()?;

        Ok(response
            .add_attribute("action", "register_account")
            .add_submessage(SubMsg::reply_on_success(
                WasmMsg::Execute {
                    contract_addr: factory,
                    msg: to_binary(&FactoryExecuteMsg::Ics999(FactoryMsg {
                        endpoint:   self.endpoint.clone(),
                        controller: self.controller.clone(),
                        data,
                    }))?,
                    funds: vec![],
                },
                AFTER_ACTION,
            )))
    }

    fn handle_query(
        &mut self,
        response: Response,
        deps:     Deps,
        msg:      Binary,
    ) -> Result<Response> {
        let addr = self.get_host()?;

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

        self.results.push(ActionResult::Query { response: query_res });

        Ok(response.add_attribute("action", "query"))
    }

    fn handle_execute(&self, response: Response, msg: Binary) -> Result<Response> {
        let addr = self.get_host()?;

        Ok(response
            .add_attribute("action", "execute")
            .add_submessage(SubMsg::reply_on_success(
                WasmMsg::Execute {
                    contract_addr: addr.into(),
                    msg,
                    funds: vec![],
                },
                AFTER_ACTION,
            )))
    }

    fn assert_no_host(&self) -> Result<()> {
        if self.host.is_some() {
            return Err(Error::AccountExists {
                endpoint:   self.endpoint.clone(),
                controller: self.controller.clone(),
            })?;
        }

        Ok(())
    }

    fn get_host(&self) -> Result<&Addr> {
        self.host.as_ref().ok_or_else(|| Error::AccountNotFound {
            endpoint:   self.endpoint.clone(),
            controller: self.controller.clone(),
        })
    }

    /// After an `Execute` action has been completed, parse the response
    pub fn after_action(&mut self, deps: DepsMut, data: Option<Binary>) -> Result<()> {
        // the action that was executed
        let action = self.action.as_ref().expect("missing active action");

        if let Action::Execute(_) = action {
            return self.after_execute(data);
        }

        if let Action::RegisterAccount(RegisterOptions::CustomFactory { .. }) = action {
            return self.after_register_account_custom_factory(deps, data);
        }

        Ok(())
    }

    fn after_execute(&mut self, data: Option<Binary>) -> Result<()> {
        // note that the contract being executed does not necessarily return
        // any data
        let data = data
            .map(|bin| parse_execute_response_data(&bin))
            .transpose()?
            .and_then(|res| res.data);

        self.results.push(ActionResult::Execute { data });

        Ok(())
    }

    fn after_register_account_custom_factory(
        &mut self,
        deps: DepsMut,
        data: Option<Binary>,
    ) -> Result<()> {
        let execute_res_bytes = data.ok_or(Error::FactoryResponseDataMissing)?;
        let execute_res = parse_execute_response_data(&execute_res_bytes)?;

        let factory_res_bytes = execute_res.data.ok_or(Error::FactoryResponseDataMissing)?;
        let factory_res: FactoryResponse = from_binary(&factory_res_bytes)?;

        let addr = deps.api.addr_validate(&factory_res.address)?;

        ACCOUNTS.save(
            deps.storage,
            (&self.endpoint.port_id, &self.endpoint.channel_id, &self.controller),
            &addr,
        )?;

        self.results.push(ActionResult::RegisterAccount { address: addr.to_string() });
        self.host = Some(addr);

        Ok(())
    }

    fn default_handle_action_response<T>(&self) -> Response<T> {
        Response::new()
            .add_attribute("method", "handle_next_action")
            .add_attribute("actions_left", self.pending_actions.len().to_string())
    }

    /// Generate a salt to be used in Instantiate2, if the user does not provide one.
    ///
    /// The salt is sha256 hash of the connection ID and controller address.
    /// This entures:
    /// - unique for each {port_id, channel_id, controller} pair
    /// - not exceed the 64 byte max length
    fn default_salt(&self) -> Binary {
        let mut hasher = Sha256::new();
        hasher.update(self.endpoint.port_id.as_bytes());
        hasher.update(self.endpoint.channel_id.as_bytes());
        hasher.update(self.controller.as_bytes());
        hasher.finalize().to_vec().into()
    }
}
