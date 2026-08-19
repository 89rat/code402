use alloy_primitives::{keccak256, B256};
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use m2m_core::receipt::Receipt;

fn hex_decode(s: &str) -> Vec<u8> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i+2], 16).unwrap()).collect()
}

fn main() {
    // Real receipt from R2: receipts/a2c55a11cebfd2ba.json (Base mainnet settlement)
    let r = Receipt {
        request_id: "a2c55a11cebfd2ba".into(),
        tool: "vat-mod97-check".into(),
        tool_version: "1.0.0".into(),
        input_hash: B256::from_slice(&hex_decode("0x6c82534c961b7974528381d7ab0279fd622dda98270fdbf9df97dd78f81c6287")),
        output_hash: B256::from_slice(&hex_decode("0x313cccdad4b6de7a28120d31aee6864128fc60e129d21247cdb8ecb2137aa237")),
        timestamp_unix: 1786934823,
        payment_ref: B256::ZERO, // v0 receipt (issued pre-v0.2)
    };
    let stored_commitment = "fcf1ea426d16a0713e3c29fc12259ff687f0c9741cfcc216e263cd05af76412b";
    let recomputed = r.commitment();
    println!("recomputed commitment = {}", hex::encode(recomputed.as_slice()));
    println!("stored commitment     = {}", stored_commitment);
    println!("COMMITMENT MATCH: {}", hex::encode(recomputed.as_slice()) == stored_commitment);

    let sig_bytes = hex_decode("0x22847f1c33668e3eb0212f5bcb36a0769c7cb877226a5ce0048207284ac30f53705f5e44064b34d35138df6c2bb02e84f5aac63d361c367455c452537ee7c56400");
    let sig = Signature::from_slice(&sig_bytes[..64]).unwrap();
    let v = sig_bytes[64];
    let vk = VerifyingKey::recover_from_prehash(recomputed.as_slice(), &sig, RecoveryId::new(v > 0, false)).unwrap();
    let pk = vk.to_encoded_point(false);
    let addr = &keccak256(&pk.as_bytes()[1..])[12..];
    println!("recovered signer      = 0x{}", hex::encode(addr));
}
