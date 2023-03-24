use cosmwasm_schema::cw_serde;
use cosmwasm_std::{HexBinary, IbcEndpoint};
use ripemd::{Digest, Ripemd160};

/// DenomTrace includes the token's original denom and the path it had travelled
/// to arrive at the current chain. It is used to derive the voucher denom in
/// such a way that there's a unique voucher denom for each token and each path.
#[cw_serde]
pub struct DenomTrace {
    /// The token's original denom
    pub base_denom: String,

    /// The path the token took to arrived to the current chain.
    ///
    /// At each stop, the chain is appended to the end of the array. For example,
    /// consider a token being transferred via this path:
    ///
    ///   chainA --> chainB --> chainC
    ///
    /// - on chain B, the path is \[A\]
    /// - on chain C, the path is \[A, B\]
    ///
    /// Note, this is different from ICS-20, where the latest chain is prefixed
    /// (instead of appended) to the beginning of the trace.
    pub path: Vec<IbcEndpoint>,
}

impl DenomTrace {
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
