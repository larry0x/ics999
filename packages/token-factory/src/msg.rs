use cosmwasm_schema::cw_serde;
use cosmwasm_std::{CosmosMsg, CustomMsg, Uint128};

use crate::Metadata;

#[cw_serde]
pub enum TokenFactoryMsg {
    /// Create a new factory denom, of denomination `factory/{contract-address}/{subdenom}`.
    ///
    /// Subdenom can be of length at most 44 characters, in `[0-9a-zA-Z./]`.
    ///
    /// The {contract-address, subdenom} pair must be unique.
    ///
    /// The created denom's admin is the creating contract address, but this
    /// admin can be changed using the `change_admin` binding.
    CreateDenom {
        subdenom: String,
    },

    /// Change the admin for a factory denom.
    ///
    /// If the `new_admin_address` is empty, the denom has no admin.
    ChangeAdmin {
        denom: String,
        new_admin_address: Option<String>,
    },

    /// Mint tokens to an address.
    MintTokens {
        denom: String,
        amount: Uint128,
        mint_to_address: String,
    },

    /// Burn tokens from an address.
    BurnTokens {
        denom: String,
        amount: Uint128,
        burn_from_address: String,
    },

    /// Sets the metadata on a denom.
    SetMetadata {
        denom: String,
        metadata: Metadata,
    },
}

impl From<TokenFactoryMsg> for CosmosMsg<TokenFactoryMsg> {
    fn from(tf_msg: TokenFactoryMsg) -> Self {
        CosmosMsg::Custom(tf_msg)
    }
}

impl CustomMsg for TokenFactoryMsg {}
