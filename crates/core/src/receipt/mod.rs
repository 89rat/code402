//! Cryptographic receipts: every paid call returns a receipt binding
//! request, tool, I/O hashes and timestamp into a single keccak commitment.

use alloy_primitives::{keccak256, B256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub request_id: String,
    pub tool: String,
    pub tool_version: String,
    pub input_hash: B256,
    pub output_hash: B256,
    pub timestamp_unix: u64,
}

/// keccak256 of the canonical JSON encoding of a value.
pub fn hash_json<T: Serialize>(v: &T) -> B256 {
    keccak256(serde_json::to_vec(v).expect("serde_json serialization is infallible for supported types"))
}

impl Receipt {
    /// keccak256 over the concatenated fields (length-prefixed strings,
    /// fixed-width hashes, big-endian timestamp) — deterministic and
    /// collision-resistant against field-boundary ambiguity.
    pub fn commitment(&self) -> B256 {
        let mut b = Vec::with_capacity(128);
        for s in [&self.request_id, &self.tool, &self.tool_version] {
            b.extend_from_slice(&(s.len() as u32).to_be_bytes());
            b.extend_from_slice(s.as_bytes());
        }
        b.extend_from_slice(self.input_hash.as_slice());
        b.extend_from_slice(self.output_hash.as_slice());
        b.extend_from_slice(&self.timestamp_unix.to_be_bytes());
        keccak256(&b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commitment_is_deterministic_and_sensitive() {
        let r1 = Receipt {
            request_id: "req-1".into(),
            tool: "uk-entity-validator".into(),
            tool_version: "1.0.0".into(),
            input_hash: hash_json(&serde_json::json!({"company_number":"12345678"})),
            output_hash: hash_json(&serde_json::json!({"valid":true})),
            timestamp_unix: 1_700_000_000,
        };
        let mut r2 = r1.clone();
        r2.timestamp_unix += 1;
        assert_eq!(r1.commitment(), r1.commitment());
        assert_ne!(r1.commitment(), r2.commitment());
    }
}
