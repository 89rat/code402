//! Cryptographic receipts (XDR-1 v0.2): every paid call returns a receipt
//! binding request, tool, I/O hashes, timestamp, and the payment reference
//! (the 402-challenge nonce) into a domain-separated keccak commitment.
//! Spec: x402-receipt-spec v0.2 (JCS-pinned I/O hashes, low-s signatures).

use alloy_primitives::{keccak256, B256};
use serde::{Deserialize, Serialize};

pub mod jcs;

/// Receipt spec tag — signed inside the commitment (XDR-1 §4).
pub const SPEC: &str = "xdr-1/0.2";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub request_id: String,
    pub tool: String,
    pub tool_version: String,
    pub input_hash: B256,
    pub output_hash: B256,
    pub timestamp_unix: u64,
    /// The `nonce` of this request's 402 challenge / EIP-3009 authorization —
    /// binds the receipt to ONE payment (XDR-1 v0.2). Legacy v0 receipts
    /// deserialize with the zero default and verify via `commitment_v0`.
    #[serde(default = "b256_zero")]
    pub payment_ref: B256,
}

fn b256_zero() -> B256 {
    B256::ZERO
}

/// keccak256(jcs(v)) — the XDR-1 I/O hash rule (RFC 8785 canonicalization).
/// Floats cannot appear in validated tool I/O; reaching the fallback would
/// mean an unvalidated value crossed this boundary — hash a poison tag
/// instead of silently diverging from the canonical rule.
pub fn hash_json<T: Serialize>(v: &T) -> B256 {
    match serde_json::to_value(v) {
        Ok(val) => match jcs::jcs_hash(&val) {
            Ok(h) => h,
            Err(e) => keccak256(format!("xdr1:jcs-error:{e}").as_bytes()),
        },
        Err(e) => keccak256(format!("xdr1:serde-error:{e}").as_bytes()),
    }
}

impl Receipt {
    /// XDR-1 v0.2 commitment: domain-separated ("XDR-1" || 0x00),
    /// length-prefixed strings, fixed-width hashes/timestamp/payment_ref,
    /// spec-tagged. Deterministic and collision-resistant against
    /// field-boundary ambiguity; a v0.2 signature can never be replayed
    /// into a protocol that ecrecovers bare digests.
    pub fn commitment(&self) -> B256 {
        let mut b = Vec::with_capacity(160);
        b.extend_from_slice(b"XDR-1");
        b.push(0x00);
        for s in [&self.request_id, &self.tool, &self.tool_version] {
            b.extend_from_slice(&(s.len() as u32).to_be_bytes());
            b.extend_from_slice(s.as_bytes());
        }
        b.extend_from_slice(self.input_hash.as_slice());
        b.extend_from_slice(self.output_hash.as_slice());
        b.extend_from_slice(&self.timestamp_unix.to_be_bytes());
        b.extend_from_slice(self.payment_ref.as_slice());
        b.extend_from_slice(&[SPEC.len() as u8]);
        b.extend_from_slice(SPEC.as_bytes());
        keccak256(&b)
    }

    /// Legacy v0 construction (untagged, no payment_ref, no domain separator)
    /// — verification of receipts issued before 2026-08-19 only.
    pub fn commitment_v0(&self) -> B256 {
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

    fn sample() -> Receipt {
        Receipt {
            request_id: "req-1".into(),
            tool: "uk-entity-validator".into(),
            tool_version: "1.0.0".into(),
            input_hash: hash_json(&serde_json::json!({"company_number":"12345678"})),
            output_hash: hash_json(&serde_json::json!({"valid":true})),
            timestamp_unix: 1_700_000_000,
            payment_ref: B256::ZERO,
        }
    }

    #[test]
    fn commitment_is_deterministic_and_sensitive() {
        let r1 = sample();
        let mut r2 = r1.clone();
        r2.timestamp_unix += 1;
        assert_eq!(r1.commitment(), r1.commitment());
        assert_ne!(r1.commitment(), r2.commitment());
        // payment_ref is signed: changing the bound payment changes the digest
        let mut pr = [0u8; 32];
        pr[0] = 1;
        let mut r3 = r1.clone();
        r3.payment_ref = B256::from(pr);
        assert_ne!(r1.commitment(), r3.commitment());
    }

    /// XDR-1 v0.2 §7 test vector — a conforming implementation MUST
    /// reproduce every value exactly (proves the JCS canonicalizer, the
    /// domain-separated commitment, and the signature rules).
    #[test]
    fn spec_vector_v02_reproduces_exactly() {
        let mut pr = [0u8; 32];
        pr[31] = 1; // payment_ref = 0x...01 per the vector
        let r = Receipt {
            request_id: "req-1".into(),
            tool: "uk-entity-validator".into(),
            tool_version: "1.0.0".into(),
            input_hash: jcs::jcs_hash(&serde_json::json!({"company_number":"12345678"})).unwrap(),
            output_hash: jcs::jcs_hash(&serde_json::json!({"valid":true})).unwrap(),
            timestamp_unix: 1_700_000_000,
            payment_ref: B256::from(pr),
        };
        // hashes
        assert_eq!(
            alloy_primitives::hex::encode(r.input_hash),
            "903d7dd8de69f0a4618c92477ca60cb692fe0103aaa5fe8b3f1703914e2f67f5",
            "input_hash must be keccak256(jcs(input))"
        );
        assert_eq!(
            alloy_primitives::hex::encode(r.output_hash),
            "e0a7d443f12051fd841e5d532d4f88126687a97c25a8b0deb88632d60f61f88b",
            "output_hash must be keccak256(jcs(output))"
        );
        // commitment
        assert_eq!(
            alloy_primitives::hex::encode(r.commitment()),
            "0f2e25c4f2736bf8db95f01f99ce0593ded8a4f6b6c14ee6a697f2e3b41e89c5",
            "v0.2 commitment must match the spec vector"
        );
        // signature (65 bytes r||s||v): low-s, v ∈ {0,1}, recovers to the
        // anvil#0 address, and matches the published vector exactly.
        let mut key = [0u8; 32];
        key[31] = 1; // anvil/hardhat account #0 — the spec's public test key
        let sk = k256::ecdsa::SigningKey::from_slice(&key).unwrap();
        let (sig, rid) = sk.sign_prehash_recoverable(r.commitment().as_slice()).unwrap();
        let mut s65 = sig.to_bytes().to_vec();
        s65.push(rid.to_byte());
        assert_eq!(
            alloy_primitives::hex::encode(&s65),
            "5000b8e2a3cfa0f57cdc907422a4fa7025ef4948066200561cf76ffd8a5506400f7e26497ee2f98ccee82d095bf38e2013a8a6dded41210de0a59c809545ef3500",
            "signature must match the spec vector"
        );
        let recovered = crate::payment::eip712::recover_address(&r.commitment(), &s65)
            .expect("recover");
        assert_eq!(
            alloy_primitives::hex::encode(recovered),
            "7e5f4552091a69125d5dfcb7b8c2659029395bdf",
            "recovered signer must be the anvil#0 address"
        );
    }

    /// Legacy v0 receipts remain verifiable under the untagged construction.
    #[test]
    fn v0_commitment_unchanged_for_legacy_receipts() {
        let r = sample();
        let expected = {
            // the pre-v0.2 construction, spelled out independently
            let mut b = Vec::new();
            for s in ["req-1", "uk-entity-validator", "1.0.0"] {
                b.extend_from_slice(&(s.len() as u32).to_be_bytes());
                b.extend_from_slice(s.as_bytes());
            }
            b.extend_from_slice(r.input_hash.as_slice());
            b.extend_from_slice(r.output_hash.as_slice());
            b.extend_from_slice(&1_700_000_000u64.to_be_bytes());
            keccak256(&b)
        };
        assert_eq!(r.commitment_v0(), expected);
        assert_ne!(r.commitment(), r.commitment_v0());
    }
}
