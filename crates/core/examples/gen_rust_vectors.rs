//! Stage 2 reverse direction: Rust-generated signatures for the TS verifier
//! (tests/vectors/gen/verify-rust-vectors.mjs, viem verifyTypedData).
//! Run: cargo run -p m2m-core --example gen_rust_vectors -- <outdir>
//! Writes fixtures in the same schema as the TS generator, plus `rustDigest`
//! for cross-checking the digest derivation itself.

use alloy_primitives::{keccak256, Address, B256, U256};
use k256::ecdsa::{SigningKey, VerifyingKey};
use m2m_core::payment::eip712;
use m2m_core::payment::erc3009::{struct_hash, TransferWithAuthorization};
use m2m_core::payment::x402v2::PaymentRequirements;

fn addr(sk: &SigningKey) -> Address {
    let vk = VerifyingKey::from(sk);
    let p = vk.to_encoded_point(false);
    let h = keccak256(&p.as_bytes()[1..]);
    Address::from_slice(&h[12..])
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn requirement(network: &str, asset: &str, name: &str, pay_to: &str) -> PaymentRequirements {
    serde_json::from_value(serde_json::json!({
        "scheme": "exact",
        "network": network,
        "amount": "10000",
        "asset": asset,
        "payTo": pay_to,
        "maxTimeoutSeconds": 60,
        "extra": {
            "name": name, "version": "2",
            "assetTransferMethod": "eip3009", "paymentFlow": "upfront"
        }
    }))
    .expect("requirement")
}

fn main() {
    let out_dir = std::env::args().nth(1).expect("usage: gen_rust_vectors <outdir>");
    std::fs::create_dir_all(&out_dir).expect("mkdir");

    // two fixed test keys (same PAYER_A as the TS generator + anvil #1)
    let key_a: [u8; 32] = {
        let mut k = [0u8; 32];
        let hex_a = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318";
        for (i, c) in (0..hex_a.len()).step_by(2).enumerate() {
            k[i] = u8::from_str_radix(&hex_a[c..c + 2], 16).expect("hex");
        }
        k
    };
    let sk = SigningKey::from_slice(&key_a).expect("key");
    let from = addr(&sk);
    let to: Address = "0x3bca128282a1de2f74efc16fa44a32a6f88a72ff".parse().expect("to");

    let cases: &[(&str, &str, &str, &str, &str)] = &[
        // name, network, asset, domain name, note
        ("rust_sepolia_usdc", "eip155:84532", "0x036CbD53842c5426634e7929541eC2318f3dCF7e", "USDC", "Rust-signed Sepolia USDC"),
        ("rust_base_usdcoin", "eip155:8453", "0x833589fCD6eDb6E08f4c7C32D4F71b54bdA02913", "USD Coin", "Rust-signed Base USD Coin (mainnet domain)"),
    ];

    for (name, network, asset, domain_name, note) in cases {
        let req = requirement(network, asset, domain_name, &format!("{to:?}"));
        let chain: u64 = network.strip_prefix("eip155:").unwrap_or("0").parse().expect("chain");
        let ds = eip712::domain_separator(domain_name, "2", chain, req.asset_addr().expect("asset"));
        let twa = TransferWithAuthorization {
            from,
            to,
            value: U256::from(10000u64),
            valid_after: 1740672000,
            valid_before: 1740672400,
            nonce: B256::from(
            hex::decode("f3746613c2d920b5fdabc0856f2aeb2d4f88ee6037b8cc5d04a71a4462f13480").map(|v| {
                    let mut b = [0u8; 32];
                    for (i, x) in v.iter().enumerate() { b[i] = *x; }
                    b
                }).unwrap_or([0u8; 32])),
        };
        let sh = struct_hash(&twa);
        let digest = eip712::signing_digest(&ds, &sh);
        let (sig, rid) = sk.sign_prehash_recoverable(digest.as_slice()).expect("sign");
        let mut s65 = Vec::with_capacity(65);
        s65.extend_from_slice(&sig.to_bytes());
        s65.push(rid.to_byte());
        let sig_hex = format!("0x{}", hex(&s65));

        let fixture = serde_json::json!({
            "name": name,
            "note": note,
            "domain": { "name": domain_name, "version": "2", "chainId": chain, "verifyingContract": asset },
            "requirement": req,
            "authorization": {
                "from": format!("{from:?}"),
                "to": format!("{to:?}"),
                "value": "10000",
                "validAfter": "1740672000",
                "validBefore": "1740672400",
                "nonce": "0xf3746613c2d920b5fdabc0856f2aeb2d4f88ee6037b8cc5d04a71a4462f13480"
            },
            "signature": sig_hex,
            "expected": "local_pass",
            "rustDigest": format!("0x{}", hex(digest.as_slice())),
            "payerKey": format!("0x{}", hex(&key_a)),
        });
        let path = std::path::Path::new(&out_dir).join(format!("{name}.json"));
        std::fs::write(path, serde_json::to_string_pretty(&fixture).expect("json")).expect("write");
        println!("wrote {name}");
    }
}
