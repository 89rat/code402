use alloy_primitives::{keccak256, B256};
use k256::ecdsa::SigningKey;
use m2m_core::receipt::{hash_json, Receipt};

fn main() {
    let r = Receipt {
        request_id: "req-1".into(),
        tool: "uk-entity-validator".into(),
        tool_version: "1.0.0".into(),
        input_hash: hash_json(&serde_json::json!({"company_number": "12345678"})),
        output_hash: hash_json(&serde_json::json!({"valid": true})),
        timestamp_unix: 1_700_000_000,
    };
    // v0.2: domain-tagged commitment = keccak256("XDR-1" || 0x00 || payload)
    let mut payload = Vec::with_capacity(128);
    for s in [&r.request_id, &r.tool, &r.tool_version] {
        payload.extend_from_slice(&(s.len() as u32).to_be_bytes());
        payload.extend_from_slice(s.as_bytes());
    }
    payload.extend_from_slice(r.input_hash.as_slice());
    payload.extend_from_slice(r.output_hash.as_slice());
    payload.extend_from_slice(&r.timestamp_unix.to_be_bytes());
    // v0.2: payment_ref (32 raw bytes) + spec string (uint8 len-prefixed)
    let mut payment_ref = [0u8; 32]; payment_ref[31] = 1;
    payload.extend_from_slice(&payment_ref);
    let spec = b"xdr-1/0.2";
    payload.push(spec.len() as u8);
    payload.extend_from_slice(spec);
    let mut tagged = b"XDR-1\x00".to_vec();
    tagged.extend_from_slice(&payload);
    let commitment = keccak256(&tagged);

    println!("input_hash  = {:?}", r.input_hash);
    println!("output_hash = {:?}", r.output_hash);
    println!("commitment  = {:?}", commitment);

    // signature vector: test key = 0x00...01
    let mut k = [0u8; 32]; k[31] = 1;
    let sk = SigningKey::from_slice(&k).unwrap();
    let (sig, rid) = sk.sign_prehash_recoverable(commitment.as_slice()).unwrap();
    let mut s65 = sig.to_bytes().to_vec();
    s65.push(rid.to_byte());
    println!("signature   = 0x{}", hex::encode(&s65));
    let vk = sk.verifying_key();
    let pk = vk.to_encoded_point(false);
    let addr = &keccak256(&pk.as_bytes()[1..])[12..];
    println!("signer_addr = 0x{}", hex::encode(addr));
}
