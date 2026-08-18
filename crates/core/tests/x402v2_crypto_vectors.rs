//! Stage 2 crypto conformance — TS-generated fixtures through the Rust
//! prefilter (the TS→Rust direction). Each fixture embeds its own
//! expectation (`local_pass` | `local_reject` | `pass_through`).
//!
//! The reverse direction (Rust signs → TS verifies) lives in
//! tests/vectors/gen/verify-rust-vectors.mjs, run by tests/fuzz/run.sh.

use m2m_core::payment::x402v2::{
    Authorization, ExactEvmPayload, PaymentPayload, PaymentRequirements,
};
use m2m_core::payment::x402v2_verify::{prefilter, VerifyOutcome};

fn vec_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/crypto")
}

#[derive(serde::Deserialize)]
struct Fixture {
    name: String,
    #[allow(dead_code)]
    note: String,
    requirement: PaymentRequirements,
    authorization: Authorization,
    signature: String,
    expected: String,
}

fn load_all() -> Vec<Fixture> {
    let mut out = Vec::new();
    let mut names: Vec<_> = std::fs::read_dir(vec_dir())
        .expect("crypto vectors dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".json"))
        .collect();
    names.sort();
    for n in names {
        let raw = std::fs::read_to_string(vec_dir().join(&n)).expect("fixture");
        let mut f: Fixture = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("{n}: fixture must decode: {e}"));
        f.name = n;
        out.push(f);
    }
    out
}

fn payload_from(f: &Fixture) -> PaymentPayload {
    PaymentPayload {
        x402_version: 2,
        resource: None,
        accepted: f.requirement.clone(),
        payload: ExactEvmPayload {
            signature: f.signature.clone(),
            authorization: f.authorization.clone(),
        },
        extensions: None,
    }
}

#[test]
fn ts_generated_vectors_pass_the_prefilter() {
    let fixtures = load_all();
    assert!(fixtures.len() >= 10, "expected >= 10 crypto fixtures");
    let mut passed = 0;
    for f in &fixtures {
        let outcome = prefilter(&payload_from(f), &f.requirement);
        match f.expected.as_str() {
            "local_pass" => assert!(
                matches!(outcome, VerifyOutcome::LocalPass { .. }),
                "{}: expected LocalPass, got {:?}",
                f.name,
                outcome
            ),
            "local_reject" => assert!(
                matches!(outcome, VerifyOutcome::LocalReject(_)),
                "{}: expected LocalReject, got {:?}",
                f.name,
                outcome
            ),
            "pass_through" => assert!(
                outcome == VerifyOutcome::PassThrough,
                "{}: expected PassThrough, got {:?}",
                f.name,
                outcome
            ),
            other => panic!("{}: unknown expectation {other:?}", f.name),
        }
        passed += 1;
    }
    println!("crypto fixtures verified: {passed}");
}

#[test]
fn domain_divergence_is_real() {
    // sanity: the two pass-fixtures must carry genuinely different domain
    // separators (name AND chain AND token all differ) — proving the pass
    // results aren't an artifact of a shared domain.
    let fixtures = load_all();
    let sep = |name: &str| -> [u8; 32] {
        let f = fixtures
            .iter()
            .find(|f| f.name.starts_with(name))
            .unwrap_or_else(|| panic!("fixture {name} missing"));
        m2m_core::payment::x402v2_verify::domain_separator_from_requirement(&f.requirement)
            .expect("domain sep")
            .0
    };
    let sepolia = sep("sepolia_usdc_pass");
    let base = sep("base_usdcoin_pass");
    assert_ne!(sepolia, base, "domains must diverge");
}

#[test]
fn expiry_boundary_is_exact() {
    // validBefore == now + SETTLE_MARGIN_SECONDS passes; one second less fails
    let fixtures = load_all();
    let f = fixtures.iter().find(|f| f.name.starts_with("sepolia_usdc_pass")).expect("base fixture");
    let mut payload = payload_from(f);
    let margin = m2m_core::payment::x402v2::SETTLE_MARGIN_SECONDS;
    // well inside window first (validity 1740672000..=1740672400)
    payload.payload.authorization.valid_before = "1740672400".to_string();
    let ctx_ok = m2m_core::payment::x402v2::StructuralContext {
        expected: &f.requirement, route_url: "https://api.example.com/premium-data", now_unix: 1740672400 - margin,
    };
    assert!(m2m_core::payment::x402v2::structural_gate(&payload, &ctx_ok).is_ok(), "exactly at margin passes");
    let ctx_bad = m2m_core::payment::x402v2::StructuralContext {
        expected: &f.requirement, route_url: "https://api.example.com/premium-data", now_unix: 1740672400 - margin + 1,
    };
    assert!(m2m_core::payment::x402v2::structural_gate(&payload, &ctx_bad).is_err(), "margin-1 fails");
}

#[test]
fn required_fixture_classes_present() {
    // DeepSeek stage-2: presence of fixture CLASSES must be asserted, not
    // just count — deleting/renaming a class fixture must fail the suite.
    const REQUIRED: &[(&str, &str)] = &[
        ("sepolia_usdc_pass", "domain-binding pass"),
        ("base_usdcoin_pass", "domain divergence pass"),
        ("v_27form", "v normalization 27/28"),
        ("v_0form", "v normalization 0/1"),
        ("wrong_chain", "wrong-chain reject"),
        ("wrong_token", "wrong-token reject"),
        ("wrong_signer", "recovers-to-from reject"),
        ("high_s_malleable", "EIP-2 low-s reject"),
        ("garbage_sig", "garbage reject"),
        ("envelope_6492", "6492 pass-through"),
        ("long_non_magic", "long non-magic reject"),
        ("invalid_v", "invalid recovery id reject"),
    ];
    let names: Vec<String> = load_all().into_iter().map(|f| f.name).collect();
    for (prefix, class) in REQUIRED {
        assert!(
            names.iter().any(|n| n.starts_with(prefix)),
            "required fixture class missing: {class} ({prefix}*)"
        );
    }
}
