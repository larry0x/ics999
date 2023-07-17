use {
    cosmwasm_schema::cw_serde,
    cosmwasm_std::{HexBinary, IbcEndpoint},
    ics999::Trace,
    ripemd::{Digest, Ripemd160},
};

/// Similar to one_types::Trace ("full trace"), but without the `denom` field
/// (which will be used as the key in contract storage). Also implements some
/// helper methods.
#[cw_serde]
pub struct TraceItem {
    pub base_denom: String,
    pub path:       Vec<IbcEndpoint>,
}

impl From<&Trace> for TraceItem {
    fn from(trace: &Trace) -> Self {
        Self {
            base_denom: trace.base_denom.clone(),
            path:       trace.path.clone(),
        }
    }
}

impl TraceItem {
    /// Create a new trace item with an empty path
    pub fn new(base_denom: &str) -> Self {
        Self {
            base_denom: base_denom.to_owned(),
            path:       vec![],
        }
    }

    /// Combine the trace item with the denom on the current chain to form the
    /// full trace.
    pub fn into_full_trace(self, denom: &str) -> Trace {
        Trace {
            denom:      denom.to_owned(),
            base_denom: self.base_denom,
            path:       self.path,
        }
    }

    /// Hash the trace. The resulting hash is used as the subdenom of the
    /// voucher token.
    ///
    /// We use RIPEMD-160 instead of SHA-256 because with the latter, the token
    /// factory denom will be longer than cosmos-sdk's max allowed denom length.
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

    /// The reverse of receiver_chain_is_source.
    ///
    /// We make this function public instead because it's easier to wrap head
    /// around (ibc-go does the same).
    ///
    /// NOTE: Regardless of where is function is run -- the sender chain or the
    /// receiver chain -- it always takes the sender chain IBC endpoint (`src`)!
    pub fn sender_is_source(&self, src: &IbcEndpoint) -> bool {
        !self.receiver_is_source(src)
    }

    /// Given the sender endpoint of a packet, return:
    /// - true, if the denom originally came from the receiving chain, or
    /// - false, if otherwise.
    ///
    /// The receiver is the source, if the path is not empty, and the channel
    /// connecting the receiving chain is the very last step in the path.
    fn receiver_is_source(&self, src: &IbcEndpoint) -> bool {
        let Some(last_step) = self.path.last() else {
            return false;
        };

        src == last_step
    }
}

// ----------------------------------- Tests -----------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deriving_hash() {
        let trace = TraceItem {
            base_denom: "ujuno".into(),
            path: vec![
                IbcEndpoint {
                    port_id:    "transfer".into(),
                    channel_id: "channel-0".into(),
                },
                IbcEndpoint {
                    port_id:    "ics999".into(),
                    channel_id: "channel-12345".into(),
                },
            ],
        };
        assert_eq!(trace.hash().to_hex(), "88a388f8b33bf58238ed9600360c471707db9eab");
    }

    #[test]
    fn determining_source() {
        let mock_src = IbcEndpoint {
            port_id:    "ics999".into(),
            channel_id: "channel_0".into(),
        };

        // if path is empty, then sender is source
        {
            let trace = TraceItem {
                base_denom: "uatom".into(),
                path:       vec![],
            };
            assert!(trace.sender_is_source(&mock_src));
        }

        // if path is not empty, but the very last step is not the receiver
        // chain, then sender is source
        {
            let trace = TraceItem {
                base_denom: "uatom".into(),
                path: vec![
                    IbcEndpoint {
                        port_id:    "test".into(),
                        channel_id: "channel-1".into(),
                    },
                ],
            };
            assert!(trace.sender_is_source(&mock_src));
        }

        // if path is not empty, and the very last step is the receiver chain,
        // then receiver is the source
        {
            let trace = TraceItem {
                base_denom: "uatom".into(),
                path: vec![
                    IbcEndpoint {
                        port_id:    "test".into(),
                        channel_id: "channel-1".into(),
                    },
                    mock_src.clone(),
                ],
            };
            assert!(trace.receiver_is_source(&mock_src));
        }
    }
}
