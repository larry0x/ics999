use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    attr, instantiate2_address, to_binary, Addr, Attribute, Binary, ContractResult, CosmosMsg,
    DepsMut, Empty, Env, QueryRequest, Response, StdResult, Storage, SubMsg, SystemResult, WasmMsg,
    WasmQuery,
};
use cw_storage_plus::Item;
use cw_utils::{parse_execute_response_data, parse_instantiate_response_data};
use one_types::{Action, ActionResult};

use crate::{
    error::ContractError,
    state::{ACCOUNTS, ACCOUNT_CODE_ID},
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
pub struct Handler {
    /// The connection the packet was sent from
    pub connection_id: String,

    /// The account who sent the packet on the sender chain
    pub controller: String,

    /// The interchain account controlled by the sender
    pub host: Option<Addr>,

    /// The action is to be executed at the current step.
    /// None means all actions have finished executing.
    pub action: Option<Action>,

    /// The actions that are to be executed later, in reverse order.
    ///
    /// At the beginning of each step, we pop the last element and put it in
    /// `self.action`.
    pub pending_actions: Vec<Action>,

    /// The results from executing the earlier actions
    ///
    /// At the end of each step, the response data is parsed and pushed into
    /// this queue.
    ///
    /// Once all actions have finished executing, this enture queue is included
    /// in the packet acknowledgement.
    ///
    /// NOTE: we don't include events in the acknowledgement, because events
    /// are not part of the block result, i.e. not reached consensus by
    /// validators. there is no guarantee that events are deterministic
    /// (see one of the Juno chain halt exploits).
    ///
    /// in princle, contracts should only have access to data that have reached
    /// consensus by validators.
    pub results: Vec<ActionResult>,
}

impl Handler {
    pub fn create(
        store: &dyn Storage,
        connection_id: String,
        controller: String,
        mut actions: Vec<Action>,
    ) -> StdResult<Self> {
        // load the controller's ICA host, which may or may not have already
        // been instantiated
        let host = ACCOUNTS.may_load(store, (&connection_id, &controller))?;

        // reverse the actions, so that we can use pop() to grab the 1st action
        actions.reverse();

        Ok(Self {
            connection_id,
            controller,
            host,
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
        deps: DepsMut,
        env: Env,
    ) -> Result<Response, ContractError> {
        // fetch the first action in the queue
        self.action = self.pending_actions.pop();

        // if there is no more action to execute
        // delete handler state from contract store, return the results as data
        // in the response
        let Some(action) = &self.action else {
            Handler::remove(deps.storage);

            return Ok(Response::new()
                .set_data(to_binary(&self.results)?)
                .add_attributes(self.into_attributes()));
        };

        // convert the action to the appropriate CosmosMsg
        let msg: CosmosMsg = match action {
            Action::Transfer {
                amount: _,
                recipient: _,
            } => {
                todo!("fungible token transfer is not implemented yet");
            },

            Action::QueryRaw {
                contract,
                key,
            } => {
                let value = deps.querier.query_wasm_raw(contract, key.clone())?;

                self.results.push(ActionResult::QueryRaw {
                    value: value.map(Binary),
                });

                return self.handle_next_action(deps, env);
            },

            Action::QuerySmart {
                contract,
                msg,
            } => {
                let query_req = QueryRequest::<Empty>::Wasm(WasmQuery::Smart {
                    contract_addr: contract.clone(),
                    msg: msg.clone(),
                });
                let query_res = deps.querier.raw_query(&to_binary(&query_req)?);

                let SystemResult::Ok(ContractResult::Ok(response)) = query_res else {
                    return Err(ContractError::SmartQueryFailed);
                };

                self.results.push(ActionResult::QuerySmart {
                    response,
                });

                return self.handle_next_action(deps, env);
            },

            Action::RegisterAccount {
                salt,
            } => {
                // only one ICA per controller allowed
                if self.host.is_some() {
                    return Err(ContractError::AccountExists {
                        connection_id: self.connection_id,
                        controller: self.controller,
                    })?;
                }

                // if a salt is not provided, by default use the connection ID
                // and controller account's UTF-8 bytes
                let salt = salt
                    .clone()
                    .unwrap_or_else(|| default_salt(&self.connection_id, &self.controller));

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

                ACCOUNTS.save(deps.storage, (&self.connection_id, &self.controller), &addr)?;

                self.host = Some(addr);

                WasmMsg::Instantiate2 {
                    admin: Some(env.contract.address.into()),
                    code_id,
                    label: format!("one-account/{}/{}", self.connection_id, self.controller),
                    msg: to_binary(&Empty {})?,
                    funds: vec![],
                    salt,
                }
                .into()
            },

            Action::Execute(wasm_msg) => {
                let Some(addr) = &self.host else {
                    return Err(ContractError::AccountNotFound {
                        connection_id: self.connection_id,
                        controller: self.controller,
                    });
                };

                let funds = {
                    // TODO: convert funds to their corresponding ibc denoms
                    vec![]
                };

                WasmMsg::Execute {
                    contract_addr: addr.into(),
                    msg: to_binary(&wasm_msg)?,
                    funds,
                }
                .into()
            },
        };

        self.save(deps.storage)?;

        Ok(Response::new()
            .add_attributes(self.into_attributes())
            // note that we use reply_on_success here, meaning any one action
            // fail wil lead to the entire execute::handle method call to fail.
            // this this atomicity - a desired property
            .add_submessage(SubMsg::reply_on_success(msg, AFTER_ACTION)))
    }

    /// After an action has been executed, parse the response
    pub fn handle_result(&mut self, data: Option<Binary>) -> Result<(), ContractError> {
        // the action that was executed
        let action = self.action.as_ref().expect("missing active action");

        // we deserialize the data based on which type of action that was handled
        match action {
            Action::Transfer {
                amount: _,
                recipient: _,
            } => {
                todo!("fungible token transfer is not implemented yet");
            },

            Action::RegisterAccount {
                ..
            } => {
                let data = data.expect("missing instantaite response data");
                // technically this should be Instantiate2 response, but it's
                // the same as the normal instantite response so this should work
                let instantiate_res = parse_instantiate_response_data(&data)?;

                self.results.push(ActionResult::RegisterAccount {
                    address: instantiate_res.contract_address,
                });
            },

            Action::Execute(_) => {
                let data = data.expect("missing wasm execute response data");
                let execute_res = parse_execute_response_data(&data)?;

                self.results.push(ActionResult::Execute {
                    data: execute_res.data,
                });
            },

            _ => unreachable!("query actions should not have reply"),
        }

        Ok(())
    }

    fn into_attributes(self) -> Vec<Attribute> {
        vec![
            attr("connection_id", self.connection_id),
            attr("controller", self.controller),
            attr("host", self.host.map_or_else(|| "null".to_string(), |addr| addr.to_string())),
            attr("actions_left", self.pending_actions.len().to_string()),
        ]
    }
}

/// Generate a salt to be used in Instantiate2, if the user does not provide one.
///
/// The salt is the UTF-8 bytes of the connection ID and controller address,
/// concatenated. This ensures unique salt for each {connection, controller} pair.
fn default_salt(connection_id: &str, controller: &str) -> Binary {
    let mut bytes = vec![];
    bytes.extend(connection_id.as_bytes());
    bytes.extend(controller.as_bytes());
    bytes.into()
}
