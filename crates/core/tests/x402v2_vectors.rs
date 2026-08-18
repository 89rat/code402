//! Stage 1 conformance tests: golden vectors derived VERBATIM from the
//! vendored spec examples (specs/x402/ @ pinned commit), codec roundtrips,
//! negative vectors, structural-gate behavior, and offline SPEC-VERSION
//! drift detection.
//!
//! If a change here is needed to make the spec examples pass, the change is
//! wrong — the vendored spec text is the authority (SPEC-VERSION rule 3).

use m2m_core::payment::x402v2::*;

fn vectors_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/x402")
}

fn read_vec(name: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors").join(name),
    )
    .expect("vector file")
}

// ---------------------------------------------------------------------------
// Golden roundtrips: spec JSON -> struct -> JSON must be BYTE-IDENTICAL
// (serde_json with struct field order == spec field order), and the b64
// codec must roundtrip byte-identically.
// ---------------------------------------------------------------------------

#[test]
fn vector_payment_required_roundtrips() {
    let raw = read_vec("payment-required.json");
    let pr: PaymentRequired = serde_json::from_str(&raw).expect("spec example must decode");
    pr.validate().expect("spec example is spec-valid");
    // Spec example omits §6.1 reserved keys => NOT valid under our stricter
    // issuance rules (mechanism defaults are for other servers, not us).
    assert!(pr.validate_for_issue().is_err(), "spec example lacks reserved keys; validate_for_issue must reject");
    let re = serde_json::to_string(&pr).expect("re-encode");
    assert_eq!(re, raw.trim(), "byte-exact JSON roundtrip");
    let b64 = encode_b64_json(&pr).expect("b64");
    let back: PaymentRequired = decode_b64_json(&b64).expect("b64 decode");
    assert_eq!(back, pr, "b64 roundtrip");
}

#[test]
fn vector_payment_payload_roundtrips() {
    let raw = read_vec("payment-payload.json");
    let pp: PaymentPayload = serde_json::from_str(&raw).expect("spec example must decode");
    let auth = &pp.payload.authorization;
    assert_eq!(auth.nonce_bytes().expect("nonce"), {
        let mut b = [0u8; 32];
        hex_to_bytes_auth(&auth.nonce, &mut b);
        b
    });
    assert_eq!(auth.valid_after_unix().expect("ts"), 1_740_672_089);
    assert_eq!(auth.valid_before_unix().expect("ts"), 1_740_672_154);
    assert_eq!(
        auth.value_u256().expect("amount").to_string(),
        "10000"
    );
    let re = serde_json::to_string(&pp).expect("re-encode");
    assert_eq!(re, raw.trim(), "byte-exact JSON roundtrip");
}

// NOTE: settle-response.json uses the §7.2/§5.3.2 TABLE field order
// (success, payer, transaction, network). The §5.3.1 EXAMPLE orders payer
// last — spec self-inconsistency; the table order is canonical for our
// serializer. Do not "fix" this to match the example.
#[test]
fn vector_settle_response_roundtrips() {
    let raw = read_vec("settle-response.json");
    let sr: SettleResponse = serde_json::from_str(&raw).expect("spec example must decode");
    sr.validate().expect("spec example is spec-valid");
    let re = serde_json::to_string(&sr).expect("re-encode");
    assert_eq!(re, raw.trim(), "byte-exact JSON roundtrip");
    let b64 = encode_settle_response(&sr).expect("encode");
    let back: SettleResponse = decode_b64_json(&b64).expect("decode");
    assert_eq!(back, sr);
}

fn hex_to_bytes_auth(s: &str, out: &mut [u8; 32]) {
    let s = s.strip_prefix("0x").unwrap_or(s);
    for (i, c) in s.as_bytes().chunks(2).enumerate() {
        let hi = (c[0] as char).to_digit(16).unwrap_or(0) as u8;
        let lo = (c[1] as char).to_digit(16).unwrap_or(0) as u8;
        out[i] = (hi << 4) | lo;
    }
}

// ---------------------------------------------------------------------------
// Negative vectors — every malformed input class the G4 gate must reject
// WITHOUT panicking (payment-path panic = worker abort).
// ---------------------------------------------------------------------------

fn base_payload() -> PaymentPayload {
    let raw = read_vec("payment-payload.json");
    serde_json::from_str(&raw).expect("vector")
}

fn gate_ctx<'a>(expected: &'a PaymentRequirements, now: u64) -> StructuralContext<'a> {
    StructuralContext { expected, route_url: None, now_unix: now }
}

#[test]
fn gate_accepts_well_formed_payload() {
    let pp = base_payload();
    let now = 1_740_672_100; // inside validity window, margin satisfied
    assert!(structural_gate(&pp, &gate_ctx(&pp.accepted, now)).is_ok());
}

#[test]
fn gate_rejects_amount_tamper() {
    let mut pp = base_payload();
    let now = 1_740_672_100;
    // client swaps amount in echoed requirement (classic exact cheat)
    let tampered = {
        let mut t = pp.accepted.clone();
        t.amount = "1".to_string();
        t
    };
    let _ = tampered; // echo mismatch is caught by field compare below
    pp.payload.authorization.value = "1".to_string();
    let r = structural_gate(&pp, &gate_ctx(&pp.accepted, now));
    assert!(matches!(r, Err(X402Error::ExactAmountMismatch(_, _))));
}

#[test]
fn gate_rejects_echo_mismatch() {
    let mut pp = base_payload();
    let now = 1_740_672_100;
    pp.accepted.pay_to = "0x0000000000000000000000000000000000000001".to_string();
    let expected = {
        let raw = read_vec("payment-required.json");
        let pr: PaymentRequired = serde_json::from_str(&raw).expect("vector");
        pr.accepts[0].clone()
    };
    let r = structural_gate(&pp, &gate_ctx(&expected, now));
    assert!(r.is_err(), "echoed requirement differing from issued must fail");
}

#[test]
fn gate_rejects_valid_before_inside_margin() {
    let pp = base_payload();
    // now = validBefore - 10s < margin of 30s
    let now = 1_740_672_154 - 10;
    let r = structural_gate(&pp, &gate_ctx(&pp.accepted, now));
    assert!(matches!(r, Err(X402Error::ValidBeforeMargin(_, _))));
}

#[test]
fn gate_rejects_short_nonce_and_signature() {
    let mut pp = base_payload();
    let now = 1_740_672_100;
    pp.payload.authorization.nonce = "0x00".to_string();
    assert!(matches!(
        structural_gate(&pp, &gate_ctx(&pp.accepted, now)),
        Err(X402Error::BadNonce(_))
    ));
    let mut pp2 = base_payload();
    pp2.payload.signature = "0x1234".to_string();
    assert!(matches!(
        structural_gate(&pp2, &gate_ctx(&pp2.accepted, now)),
        Err(X402Error::BadSignature(_))
    ));
}

#[test]
fn gate_rejects_numeric_timestamps_and_wrong_version() {
    let mut raw = read_vec("payment-payload.json");
    // validAfter as JSON number instead of string (v1-era shape)
    raw = raw.replace("\"validAfter\":\"1740672089\"", "\"validAfter\":1740672089");
    let pp: Result<PaymentPayload, _> = serde_json::from_str(&raw);
    assert!(pp.is_err(), "numeric timestamps must fail deserialization (String field)");

    let mut pp2 = base_payload();
    pp2.x402_version = 1;
    let now = 1_740_672_100;
    assert!(matches!(
        structural_gate(&pp2, &gate_ctx(&pp2.accepted, now)),
        Err(X402Error::WrongVersion(1))
    ));
}

#[test]
fn gate_passes_6492_envelopes_rejects_short() {
    // EIP-6492 envelope: 65-byte sig + magic suffix — longer than 65 bytes.
    // Must PASS the structural gate (facilitator verifies it downstream, G4).
    let mut pp = base_payload();
    let now = 1_740_672_100;
    let inner = pp.payload.signature.clone();
    let inner = inner.strip_prefix("0x").unwrap_or(&inner);
    pp.payload.signature = format!("{inner}6492649264926492649264926492649264926492649264926492649264926492");
    assert!(
        structural_gate(&pp, &gate_ctx(&pp.accepted, now)).is_ok(),
        "6492-style long envelope must pass through to facilitator"
    );
    // too-short signature must fail
    let mut pp2 = base_payload();
    pp2.payload.signature = format!("0x{}", "ab".repeat(64)); // 64 bytes < 65
    assert!(matches!(
        structural_gate(&pp2, &gate_ctx(&pp2.accepted, now)),
        Err(X402Error::BadSignature(_))
    ));
    // odd-length hex must fail
    let mut pp3 = base_payload();
    pp3.payload.signature = format!("0x{}", "a".repeat(131)); // 131 nibbles = odd
    assert!(matches!(
        structural_gate(&pp3, &gate_ctx(&pp3.accepted, now)),
        Err(X402Error::BadSignature(_))
    ));
}

#[test]
fn gate_rejects_resource_url_mismatch() {
    let mut pp = base_payload();
    let now = 1_740_672_100;
    let ctx = StructuralContext {
        expected: &pp.accepted,
        route_url: Some("https://api.example.com/other-route"),
        now_unix: now,
    };
    assert!(matches!(
        structural_gate(&pp, &ctx),
        Err(X402Error::ResourceUrlMismatch(_))
    ));
}

#[test]
fn codec_rejects_non_canonical_and_oversized() {
    // valid b64 with trailing whitespace = non-canonical
    let pp = base_payload();
    let b64 = encode_b64_json(&pp).expect("encode");
    let padded = format!("{} ", b64);
    let r: Result<PaymentPayload, _> = decode_b64_json(&padded);
    assert!(matches!(r, Err(X402Error::NotCanonicalBase64)));

    // oversized: cap enforced before any parsing
    let big = "A".repeat(MAX_HEADER_B64_BYTES + 1);
    let r2: Result<PaymentPayload, _> = decode_b64_json(&big);
    assert!(matches!(r2, Err(X402Error::HeaderTooLarge(_))));
}

#[test]
fn amounts_reject_floats_negatives_overflow() {
    for bad in ["1.5", "-1", "1e3", "", "0x10", " 42"] {
        let mut pp = base_payload();
        pp.accepted.amount = bad.to_string();
        assert!(pp.accepted.amount_u256().is_err(), "{bad:?} must not parse");
    }
    let huge = "1".repeat(79); // > 2^256
    let mut pp = base_payload();
    pp.accepted.amount = huge;
    assert!(matches!(pp.accepted.amount_u256(), Err(X402Error::AmountOverflow(_))));
}

#[test]
fn settle_response_pending_requires_tx() {
    let raw = read_vec("settle-response.json");
    let mut sr: SettleResponse = serde_json::from_str(&raw).expect("vector");
    sr.success = false;
    sr.error_reason = Some("settlement_pending".to_string());
    sr.transaction = String::new();
    assert!(sr.validate().is_err(), "settlement_pending MUST carry a tx hash");

    sr.error_reason = Some("insufficient_funds".to_string());
    assert!(sr.validate().is_ok(), "other failures may have empty tx");

    let mut sr2: SettleResponse = serde_json::from_str(&raw).expect("vector");
    sr2.transaction = String::new(); // success without tx is invalid
    assert!(sr2.validate().is_err());
}

#[test]
fn issuance_requires_reserved_keys() {
    let raw = read_vec("payment-required.json");
    let mut pr: PaymentRequired = serde_json::from_str(&raw).expect("vector");
    // spec example (no reserved keys) must fail OUR issuance validation
    assert!(pr.validate_for_issue().is_err());
    // declaring both reserved keys must pass
    let mut req = pr.accepts[0].clone();
    req.extra = Some(serde_json::json!({
        "name": "USDC", "version": "2",
        "assetTransferMethod": "eip3009", "paymentFlow": "upfront"
    }));
    req.validate_issued().expect("declared keys pass");
    pr.accepts[0] = req;
    pr.validate_for_issue().expect("issuance-valid");
    encode_payment_required(&pr).expect("encodable");
}

// ---------------------------------------------------------------------------
// Offline SPEC-VERSION drift detection (Stage-0 audit nice-to-have, adopted)
// ---------------------------------------------------------------------------

#[test]
fn spec_version_hashes_match_vendored_files() {
    use sha2::{Digest, Sha256};
    let dir = vectors_dir();
    let sv = std::fs::read_to_string(dir.join("SPEC-VERSION")).expect("SPEC-VERSION");
    let mut checked = 0;
    for line in sv.lines() {
        let line = line.trim();
        if let Some((key, val)) = line.strip_prefix("sha256_").and_then(|l| l.split_once('=')) {
            let file = key.trim();
            let want = val.trim();
            let bytes = std::fs::read(dir.join(file)).unwrap_or_else(|_| panic!("{file} missing"));
            let got = hex::encode(Sha256::digest(&bytes));
            assert_eq!(got, want, "{file} drifted from SPEC-VERSION pin");
            checked += 1;
        }
    }
    assert_eq!(checked, 5, "expected 5 pinned spec files");
}
