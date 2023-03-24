use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

#[cw_serde]
pub struct Metadata {
    pub description: String,

    /// The list of DenomUnit's for a given coin
    pub denom_units: Vec<DenomUnit>,

    /// The base denom (should be the DenomUnit with exponent = 0)
    pub base: String,

    /// The suggested denom that should be displayed in clients
    pub display: String,

    /// The name of the token (e.g. Cosmos Atom)
    pub name: String,

    /// The token symbol usually shown on exchanges (eg: ATOM).
    /// This can be the same as the display.
    pub symbol: String,
}

#[cw_serde]
pub struct DenomUnit {
    /// The string name of the given denom unit (e.g. uatom)
    pub denom: String,

    /// Power of 10 exponent that one must raise the `base_denom` to in order to
    /// equal the given DenomUnit's denom.
    ///
    /// 1 denom = 1^exponent base_denom
    ///
    /// E.g. with a base_denom of uatom, one can create a DenomUnit of 'atom'
    /// with exponent = 6, thus: 1 atom = 10^6 uatom.
    pub exponent: u32,
}

#[cw_serde]
pub struct Params {
    pub denom_creation_fee: Vec<Coin>,
}
