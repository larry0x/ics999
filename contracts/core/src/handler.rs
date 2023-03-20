use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    attr, instantiate2_address, to_binary, Addr, Attribute, Binary, DepsMut, Empty, Env, Response,
    SubMsg, WasmMsg,
};
use cw_storage_plus::Item;
use cw_utils::{parse_execute_response_data, parse_instantiate_response_data};
use one_types::{Acknowledgment, Action, ActionResult};

use crate::{
    error::{ContractError, ContractResult},
    state::{ACCOUNTS, ACCOUNT_CODE_ID},
};

pub const HANDLER: Item<Handler> = Item::new("handler");

pub const HANDLE_REPLY_ID: u64 = 1;

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
    pub results: Vec<ActionResult>,
}

impl Handler {
    /// Execute the next action in the queue. Saved the updated handler state.
    pub fn handle_next_action(mut self, deps: DepsMut, env: Env) -> ContractResult<Response> {
        // fetch the first action in the queue
        self.action = self.pending_actions.pop();

        // if there is no more action to execute
        let Some(action) = &self.action else {
            // delete the handler state from contract store
            HANDLER.remove(deps.storage);

            // compose the acknowledgement
            let ack = Acknowledgment::Ok(self.results);
            let ack_bin = to_binary(&ack)?;

            return Ok(Response::new().set_data(ack_bin));
        };

        let msg = match action {
            Action::Transfer {
                amount: _,
                recipient: _,
            } => {
                todo!("fungible token transfer is not implemented yet");
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
            },
        };

        HANDLER.save(deps.storage, &self)?;

        Ok(Response::new()
            .add_submessage(SubMsg::reply_always(msg, HANDLE_REPLY_ID))
            .add_attributes(self.into_attributes()))
    }

    /// After an action has been executed, parse the response
    pub fn add_result(&mut self, data: Option<Binary>) -> ContractResult<()> {
        // the action that was executed
        let action = self.action.as_ref().expect("missing active action");

        // we deserialize the data based on which type of action that was handled
        match action {
            Action::Transfer {
                amount: _,
                recipient: _,
            } => {
                todo!("fungible token transfer is not implemented yet")
            },

            Action::RegisterAccount {
                ..
            } => {
                let data = data.expect("missing instantaite response data");
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
