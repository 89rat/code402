//! Stage 3 e2e generator — the C1 client as a binary.
//! Input: a file containing the PAYMENT-REQUIRED header value (b64) plus a
//! payer key (hex, or PAYER_SECRET env). Output: the PAYMENT-SIGNATURE
//! header value on stdout. Uses the real client pipeline: parse -> select
//! (policy: staging network/asset, ceiling 1M) -> construct (client nonce)
//! -> sign (Eoa) -> extensions echoed verbatim (§5.1.2).
//!
//! Usage: payv2 <402-header-file> [payer-key-hex]

use m2m_core::payment::x402v2_client::{
    build_authorization, parse_v2_payment_required, random_nonce, select_helper, sign_payment,
    AuthorizationParams, SelectionPolicy, Signer, SignedPayment,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: payv2 <402-header-file> [payer-key-hex]");
        std::process::exit(2);
    }
    let header = std::fs::read_to_string(&args[1])
        .unwrap_or_else(|e| panic!("read {}: {e}", args[1]))
        .trim()
        .to_string();
    let key_hex = args
        .get(2)
        .cloned()
        .or_else(|| std::env::var("PAYER_SECRET").ok())
        .expect("payer key (arg or PAYER_SECRET)");
    let key_bytes = hex::decode(key_hex.trim_start_matches("0x")).expect("payer key hex");

    let pr = parse_v2_payment_required(&header).expect("parse 402");
    let policy = SelectionPolicy {
        allowed_networks: vec!["eip155:84532".into(), "eip155:8453".into()],
        allowed_assets: vec![
            "0x036CbD53842c5426634e7929541eC2318f3dCF7e".into(), // sepolia usdc
            "0x833589fCD6eDb6E08f4c7C32D4F71b54bdA02913".into(), // base usdc
        ],
        max_amount: alloy_primitives::U256::from(1_000_000u64),
    };
    let req = select_helper(&pr, &policy).expect("select under policy");

    let sk = k256::ecdsa::SigningKey::from_slice(&key_bytes).expect("valid key");
    let vk = k256::ecdsa::VerifyingKey::from(&sk);
    let p = vk.to_encoded_point(false);
    let h = alloy_primitives::keccak256(&p.as_bytes()[1..]);
    let payer = alloy_primitives::Address::from_slice(&h[12..]);
    eprintln!("payer: {payer:?}");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let auth = build_authorization(&AuthorizationParams {
        payer,
        pay_to: req.pay_to_addr().expect("payTo"),
        value: req.amount_u256().expect("amount"),
        nonce: random_nonce(),
        valid_after_unix: now.saturating_sub(60),
        valid_before_unix: now + 3600,
    });

    match sign_payment(req, &auth, &Signer::Eoa(sk), pr.extensions.clone())
        .expect("sign")
    {
        SignedPayment::Signed { b64, .. } => {
            println!("{b64}");
        }
        SignedPayment::WouldPay { .. } => unreachable!("Eoa signer signs"),
    }
}
