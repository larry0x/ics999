use cosmwasm_schema::cw_serde;
use cosmwasm_std::{HexBinary, IbcEndpoint};
use one_types::Trace;
use ripemd::{Digest, Ripemd160};

/// Similar to one_types::Trace ("full trace"), but without the `denom` field
/// (which will be used as the key in contract storage). Also implements some
/// helper methods.
#[cw_serde]
pub struct TraceItem {
    pub base_denom: String,
    pub path: Vec<IbcEndpoint>,
}

impl From<&Trace> for TraceItem {
    fn from(trace: &Trace) -> Self {
        Self {
            base_denom: trace.base_denom.clone(),
            path: trace.path.clone(),
        }
    }
}

impl TraceItem {
    /// Create a new trace item with an empty path
    pub fn new(base_denom: &str) -> Self {
        Self {
            base_denom: base_denom.to_owned(),
            path: vec![],
        }
    }

    /// Combine the trace item with the denom on the current chain to form the
    /// full trace.
    pub fn into_full_trace(self, denom: &str) -> Trace {
        Trace {
            denom: denom.to_owned(),
            base_denom: self.base_denom,
            path: self.path,
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
