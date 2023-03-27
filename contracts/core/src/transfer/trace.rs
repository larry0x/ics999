use cosmwasm_schema::cw_serde;
use cosmwasm_std::{HexBinary, IbcEndpoint};
use ripemd::{Digest, Ripemd160};

/// Similar to one_types::Trace, but without the `denom` field (which will be
/// used as the key in contract storage). Also implements some helper methods.
#[cw_serde]
pub struct TraceItem {
    pub base_denom: String,
    pub path: Vec<IbcEndpoint>,
}

impl TraceItem {
    pub fn new(base_denom: impl Into<String>) -> Self {
        Self {
            base_denom: base_denom.into(),
            path: vec![],
        }
    }

    /// Hash the trace. The resulting hash is used as the subdenom of the
    /// voucher token.
    ///
    /// We use RIPEMD-160 instead of SHA-256 because with the latter, the token
    /// factory denom will longer than cosmos-sdk's max allowed denom length.
    /// - max length: 128 characters
    /// - with SHA-256: 137 chars
    /// - with RIPEMD-160: 113 chars
    pub fn hash(&self) -> HexBinary {
        let mut hasher = Ripemd160::new();
        hasher.update(self.base_denom.as_bytes());
        for step in &self.path {
            hasher.update(step.port_id.as_bytes());
            hasher.update(step.channel_id.as_bytes());
        }
        hasher.finalize().to_vec().into()
    }

    /// Return whether the current chain ("localhost") is the source chain.
    ///
    /// If localhost is the last step in the path, then it's a sink. If it's not
    /// a sink, its a source.
    pub fn is_source(&self, localhost: &IbcEndpoint) -> bool {
        let Some(last_step) = self.path.last() else {
            return false;
        };

        localhost == last_step
    }
}
