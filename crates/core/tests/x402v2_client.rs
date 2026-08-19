//! C1 client-core tests: 402 parsing (v2 + real-v1), policy selection (I2),
//! dry-run pipeline (signs nothing), signed pipeline round-tripping through
//! the merchant's own structural gate + prefilter (the mirror principle),
//! receipt parsing, nonce ledger key.

use alloy_primitives::U256;
use m2m_core::payment::x402v2::PaymentRequired;
use m2m_core::payment::x402v2_client::*;

fn staging_pr() -> PaymentRequired {
    let raw = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/payment-required.json"),
    )
    .expect("vector");
    let mut pr: PaymentRequired = serde_json::from_str(&raw).expect("vector decode");
    // make it issuance-shaped (reserved keys) for selection/signing
    let mut a = pr.accepts[0].clone();
    a.extra = Some(serde_json::json!({
        "name": "USDC", "version": "2",
        "assetTransferMethod": "eip3009", "paymentFlow": "upfront"
    }));
    pr.accepts[0] = a;
    pr
}

fn policy() -> SelectionPolicy {
    SelectionPolicy {
        allowed_networks: vec!["eip155:84532".to_string()],
        allowed_assets: vec!["0x036CbD53842c5426634e7929541eC2318f3dCF7e".to_string()],
        max_amount: U256::from(1_000_000u64),
    }
}

#[test]
fn c1_parses_v2_header_form() {
    let pr = staging_pr();
    let b64 = m2m_core::payment::x402v2::encode_payment_required(&pr).expect("encode");
    let back = parse_v2_payment_required(&b64).expect("parse");
    assert_eq!(back, pr);
}

#[test]
fn c1_parses_real_v1_body_form() {
    // real x402 v1: JSON body, x402Version 1, accepts[] (not the bespoke
    // code402 legacy dialect, which is intentionally not understood)
    let v1_body = serde_json::json!({
        "x402Version": 1,
        "error": "payment required",
        "accepts": [{
            "scheme": "exact",
            "network": "eip155:84532",
            "amount": "10000",
            "asset": "0x036CbD53842c5426634e7929541eC2318f3dCF7e",
            "payTo": "0x3bca128282a1de2f74efc16fa44a32a6f88a72ff",
            "maxTimeoutSeconds": 60,
            "extra": {"name": "USDC", "version": "2"}
        }]
    });
    let pr = parse_v1_payment_required(&v1_body.to_string()).expect("v1 parse");
    assert_eq!(pr.accepts[0].amount, "10000");
    // v1 maps to internal v2 representation for a single downstream pipeline
    assert_eq!(pr.x402_version, 2);
}

#[test]
fn c1_policy_denies_by_default() {
    let mut pr = staging_pr();
    // wrong network -> deny
    let p = policy();
    assert!(p.select(&pr).is_ok(), "matching requirement selected");
    pr.accepts[0].network = "eip155:1".to_string();
    assert!(p.select(&pr).is_err(), "wrong network denied");
    // price above ceiling -> deny
    let mut pr2 = staging_pr();
    pr2.accepts[0].amount = "2000000".to_string();
    assert!(p.select(&pr2).is_err(), "above ceiling denied");
    // wrong asset -> deny
    let mut pr3 = staging_pr();
    pr3.accepts[0].asset = "0x0000000000000000000000000000000000000001".to_string();
    assert!(p.select(&pr3).is_err(), "wrong asset denied");
}

#[test]
fn c1_dry_run_signs_nothing() {
    let pr = staging_pr();
    let req = policy().select(&pr).expect("select");
    let auth = build_authorization(&AuthorizationParams {
        payer: "0x857b06519E91e3A54538791bDbb0E22373e36b66".parse().expect("addr"),
        pay_to: req.pay_to_addr().expect("payto"),
        value: req.amount_u256().expect("amount"),
        nonce: [7u8; 32],
        valid_after_unix: 1,
        valid_before_unix: 4102444800, // 2100
    });
    match sign_payment(req, &auth, &Signer::DryRun, pr.extensions.clone()).expect("dry run") {
        SignedPayment::WouldPay { requirement, authorization } => {
            assert_eq!(requirement.amount, "10000");
            assert_eq!(authorization.nonce.len(), 66);
        }
        SignedPayment::Signed { .. } => panic!("DryRun must never produce a signature"),
    }
}

#[test]
fn c1_signed_pipeline_passes_merchant_gate_and_prefilter() {
    // THE mirror test (design-logic §5): our client's payment must clear our
    // own merchant structural gate + EOA prefilter.
    let pr = staging_pr();
    let req = policy().select(&pr).expect("select");
    let sk = k256::ecdsa::SigningKey::from_slice(
        &hex::decode("4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318")
            .expect("hex"),
    )
    .expect("key");
    let vk = k256::ecdsa::VerifyingKey::from(&sk);
    let p = vk.to_encoded_point(false);
    let h = alloy_primitives::keccak256(&p.as_bytes()[1..]);
    let payer = alloy_primitives::Address::from_slice(&h[12..]);

    let auth = build_authorization(&AuthorizationParams {
        payer,
        pay_to: req.pay_to_addr().expect("payto"),
        value: req.amount_u256().expect("amount"),
        nonce: [9u8; 32],
        valid_after_unix: 1,
        valid_before_unix: 4102444800,
    });
    let signed = match sign_payment(req, &auth, &Signer::Eoa(sk), pr.extensions.clone()).expect("sign") {
        SignedPayment::Signed { payload, b64 } => {
            // wire roundtrip: our own decode accepts our encode
            let back: m2m_core::payment::x402v2::PaymentPayload =
                m2m_core::payment::x402v2::decode_payment_payload(&b64).expect("decode own");
            (payload, back)
        }
        SignedPayment::WouldPay { .. } => panic!("Eoa signer must sign"),
    };
    let (payload, decoded) = signed;
    // merchant side: structural gate + prefilter with the SAME requirement
    use m2m_core::payment::x402v2::{StructuralContext, structural_gate};
    use m2m_core::payment::x402v2_verify::{prefilter, VerifyOutcome};
    let ctx = StructuralContext {
        expected: req,
        route_url: "https://api.example.com/premium-data",
        now_unix: 1_740_672_100,
    };
    if let Err(e) = structural_gate(&decoded, &ctx) { panic!("gate: {e:?}"); }
    assert!(matches!(
        prefilter(&decoded, req),
        VerifyOutcome::LocalPass { .. }
    ));
    let _ = payload;
}

#[test]
fn c1_parses_receipt_and_nonce_key_stable() {
    let sr = m2m_core::payment::x402v2::SettleResponse {
        success: true,
        error_reason: None,
        payer: Some("0x857b06519E91e3A54538791bDbb0E22373e36b66".into()),
        transaction: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".into(),
        network: "eip155:84532".into(),
        amount: Some("10000".into()),
        extensions: None,
    };
    let b64 = m2m_core::payment::x402v2::encode_settle_response(&sr).expect("encode");
    let back = parse_settle_response(&b64).expect("receipt parse");
    assert!(back.success);

    let pr = staging_pr();
    let req = policy().select(&pr).expect("select");
    let auth = build_authorization(&AuthorizationParams {
        payer: "0x857b06519E91e3A54538791bDbb0E22373e36b66".parse().expect("a"),
        pay_to: req.pay_to_addr().expect("p"),
        value: req.amount_u256().expect("v"),
        nonce: [3u8; 32],
        valid_after_unix: 0,
        valid_before_unix: 4102444800,
    });
    let k1 = nonce_key(&auth).expect("key");
    let k2 = nonce_key(&auth).expect("key");
    assert_eq!(k1, k2, "nonce ledger key deterministic");
}

#[test]
fn c1_random_nonce_is_32_random_bytes() {
    let a = random_nonce();
    let b = random_nonce();
    assert_eq!(a.len(), 32);
    assert_ne!(a, b, "nonces must not repeat");
}
