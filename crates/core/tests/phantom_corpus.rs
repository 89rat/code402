//! Phantom regression corpus (RECONCILER-SPEC §6.7, PANEL.md kaizen rule:
//! "every defect becomes a vector"). The 132 rows are REAL phantom settles
//! from the 1000-settle stress — money moved on-chain, our record timed out,
//! the G7 cron reconciled. Payers anonymized (salted sha256, deterministic).
//! If the resolver ever stops landing these in settled_reconciled with the
//! exact on-chain tx, this file fails.

use m2m_core::payment::reconciler::{
    encode_authorization_state_call, evidence_from_reads, resolve, ConsumingLog, Resolution,
};
use m2m_core::payment::x402v2::PaymentPayload;

#[derive(serde::Deserialize)]
struct Corpus {
    phantoms: Vec<Phantom>,
    payload_bearing_settled: Vec<PayloadRow>,
}

#[derive(serde::Deserialize)]
struct Phantom {
    payer_anon: String,
    nonce: String,
    tx_hash: String,
}

#[derive(serde::Deserialize)]
struct PayloadRow {
    payer_anon: String,
    nonce: String,
    payment_payload: String,
}

fn corpus() -> Corpus {
    let raw = std::fs::read_to_string("../../tests/fixtures/phantom-corpus.json")
        .expect("corpus fixture missing (export from staging D1)");
    serde_json::from_str(&raw).expect("corpus fixture must be valid JSON")
}

/// Every real phantom resolves to settled_reconciled carrying the exact
/// on-chain tx — the G2 entitlement contract over the production defect set.
#[test]
fn corpus_phantoms_resolve_to_settled_reconciled_with_tx() {
    let c = corpus();
    assert!(!c.phantoms.is_empty(), "corpus must not be empty");
    for p in &c.phantoms {
        let ev = evidence_from_reads(true, Some(ConsumingLog::Used), Some(p.tx_hash.clone()), None, 0, 30);
        match resolve(&ev, 0) {
            Resolution::SettledReconciled { tx_hash } => {
                assert_eq!(tx_hash, p.tx_hash, "tx must round-trip for nonce {}", p.nonce);
            }
            other => panic!("phantom {} resolved to {other:?}", p.nonce),
        }
    }
}

/// Every real phantom nonce is chain-readable: the authorizationState calldata
/// encodes (shape the sweep's eth_call depends on).
#[test]
fn corpus_nonces_encode_for_chain_reads() {
    let c = corpus();
    for p in &c.phantoms {
        let calldata = encode_authorization_state_call("0x857b06519e91e3a54538791bDbb0E22373e36b66", &p.nonce)
            .unwrap_or_else(|e| panic!("nonce {} failed to encode: {e}", p.nonce));
        assert_eq!(calldata.len(), 2 + 8 + 64 + 64);
    }
}

/// The re-drive precondition holds on real stored payloads: they parse as
/// PaymentPayload and their validBefore is machine-readable (the sweep's
/// expiry branch and re-drive margin depend on this).
#[test]
fn corpus_payloads_parse_and_carry_valid_before() {
    let c = corpus();
    assert!(!c.payload_bearing_settled.is_empty(), "payload sample must not be empty");
    for r in &c.payload_bearing_settled {
        let pp: PaymentPayload = serde_json::from_str(&r.payment_payload)
            .unwrap_or_else(|e| panic!("payload for {} must parse: {e}", r.nonce));
        assert!(
            pp.payload.authorization.valid_before_unix().is_ok(),
            "validBefore must parse for {}",
            r.nonce
        );
    }
}

/// Idempotency + absorbing law over the corpus: re-resolving is a no-op
/// verdict and settled_reconciled never transitions.
#[test]
fn corpus_resolution_is_idempotent_and_absorbing() {
    use m2m_core::payment::reconciler::can_transition_from;
    let c = corpus();
    for p in c.phantoms.iter().take(25) {
        let ev = evidence_from_reads(true, Some(ConsumingLog::Used), Some(p.tx_hash.clone()), None, 0, 30);
        let a = resolve(&ev, 0);
        let b = resolve(&ev, 0);
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        assert!(!can_transition_from("settled_reconciled"));
    }
}
