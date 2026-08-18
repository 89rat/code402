//! PRODUCTION on-chain settler (Base mainnet).
//!
//! Reads .staging/prod-voucher.json (from payprod), builds and signs an
//! EIP-1559 transaction calling USDC.transferWithAuthorization(...), and writes
//! the raw tx hex to .staging/prod-rawtx.txt. Broadcast is done externally
//! (Python orchestrator) — this binary only signs.
//!
//! Params via env: PAYER_NONCE, MAX_FEE_WEI, PRIO_FEE_WEI, GAS_LIMIT.
//! transferWithAuthorization is a meta-tx: the payer signs, any gas-holder may
//! submit; here the payer submits (it holds the gas top-up).

use alloy_primitives::{keccak256, Address, B256, U256};
use k256::ecdsa::SigningKey;

const USDC_MAINNET: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
const CHAIN_ID: u64 = 8453;

// ---------- minimal RLP ----------
fn rlp_bytes(data: &[u8]) -> Vec<u8> {
    if data.len() == 1 && data[0] < 0x80 {
        return data.to_vec();
    }
    let mut out = Vec::new();
    if data.len() <= 55 {
        out.push(0x80 + data.len() as u8);
    } else {
        let lb = (data.len() as u64).to_be_bytes();
        let lb = &lb[lb.iter().position(|&b| b != 0).unwrap_or(7)..];
        out.push(0xb7 + lb.len() as u8);
        out.extend_from_slice(lb);
    }
    out.extend_from_slice(data);
    out
}

fn rlp_uint_be32(v: &[u8; 32]) -> Vec<u8> {
    let start = v.iter().position(|&b| b != 0).unwrap_or(32);
    rlp_bytes(&v[start..]) // zero -> empty -> 0x80
}

fn u128_be32(v: u128) -> [u8; 32] {
    let mut b = [0u8; 32];
    b[16..].copy_from_slice(&v.to_be_bytes());
    b
}

fn rlp_list(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    if payload.len() <= 55 {
        out.push(0xc0 + payload.len() as u8);
    } else {
        let lb = (payload.len() as u64).to_be_bytes();
        let lb = &lb[lb.iter().position(|&b| b != 0).unwrap_or(7)..];
        out.push(0xf7 + lb.len() as u8);
        out.extend_from_slice(lb);
    }
    out.extend_from_slice(payload);
    out
}

fn read_field(path: &std::path::Path, field: &str) -> String {
    let txt = std::fs::read_to_string(path).expect("credentials file readable");
    for line in txt.lines() {
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == field {
                return v.trim().to_string();
            }
        }
    }
    panic!("field {field} not found");
}

fn pad32(b: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[32 - b.len()..].copy_from_slice(b);
    out
}

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.staging");
    let key_hex = read_field(&dir.join("prod-payer.txt"), "COMPANY_WALLET_SECRET");
    let payer = SigningKey::from_slice(&hex::decode(key_hex).expect("payer key hex")).unwrap();

    let v: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("prod-voucher.json")).expect("voucher file"),
    )
    .unwrap();
    let auth = &v["auth"];
    let from: Address = auth["from"].as_str().unwrap().parse().unwrap();
    let to: Address = auth["to"].as_str().unwrap().parse().unwrap();
    let value = U256::from_str_radix(auth["value"].as_str().unwrap().trim_start_matches("0x"), 16).unwrap();
    let valid_after = U256::from(auth["valid_after"].as_u64().unwrap());
    let valid_before = U256::from(auth["valid_before"].as_u64().unwrap());
    let nonce32: B256 = auth["nonce"].as_str().unwrap().parse().unwrap();
    let sig: Vec<u8> = serde_json::from_value(v["signature"].clone()).unwrap();
    assert_eq!(sig.len(), 65);
    let mut v_byte = sig[64];
    if v_byte < 27 {
        v_byte += 27;
    }
    let r = &sig[0..32];
    let s = &sig[32..64];

    // calldata: transferWithAuthorization(address,address,uint256,uint256,uint256,bytes32,uint8,bytes32,bytes32)
    let sel = &keccak256(
        b"transferWithAuthorization(address,address,uint256,uint256,uint256,bytes32,uint8,bytes32,bytes32)",
    )[0..4];
    let mut data = Vec::with_capacity(4 + 32 * 9);
    data.extend_from_slice(sel);
    data.extend_from_slice(&pad32(from.as_slice()));
    data.extend_from_slice(&pad32(to.as_slice()));
    data.extend_from_slice(&value.to_be_bytes::<32>());
    data.extend_from_slice(&valid_after.to_be_bytes::<32>());
    data.extend_from_slice(&valid_before.to_be_bytes::<32>());
    data.extend_from_slice(nonce32.as_slice());
    data.extend_from_slice(&pad32(&[v_byte]));
    data.extend_from_slice(&pad32(r));
    data.extend_from_slice(&pad32(s));

    let payer_nonce: u64 = std::env::var("PAYER_NONCE").unwrap().parse().unwrap();
    let max_fee: u128 = std::env::var("MAX_FEE_WEI").unwrap().parse().unwrap();
    let prio_fee: u128 = std::env::var("PRIO_FEE_WEI").unwrap().parse().unwrap();
    let gas_limit: u64 = std::env::var("GAS_LIMIT").unwrap().parse().unwrap();
    let usdc: Address = USDC_MAINNET.parse().unwrap();

    // EIP-1559 signing payload
    let mut payload = Vec::new();
    payload.extend(rlp_uint_be32(&u128_be32(CHAIN_ID as u128)));
    payload.extend(rlp_uint_be32(&u128_be32(payer_nonce as u128)));
    payload.extend(rlp_uint_be32(&u128_be32(prio_fee)));
    payload.extend(rlp_uint_be32(&u128_be32(max_fee)));
    payload.extend(rlp_uint_be32(&u128_be32(gas_limit as u128)));
    payload.extend(rlp_bytes(usdc.as_slice()));
    payload.extend(rlp_uint_be32(&[0u8; 32])); // value = 0
    payload.extend(rlp_bytes(&data));
    payload.push(0xc0); // empty access list
    let mut preimage = vec![0x02u8];
    preimage.extend(rlp_list(&payload));
    let sighash = keccak256(&preimage);

    let (tsig, rid) = payer.sign_prehash_recoverable(sighash.as_slice()).unwrap();
    let sb = tsig.to_bytes();
    let y_parity = rid.to_byte(); // 0/1 for EIP-1559

    let mut final_payload = payload.clone();
    // replace nothing — append v,r,s: rebuild (payload already ends with 0xc0 access list)
    final_payload.extend(rlp_uint_be32(&u128_be32(y_parity as u128)));
    final_payload.extend(rlp_uint_be32(&pad32(&sb[0..32])));
    final_payload.extend(rlp_uint_be32(&pad32(&sb[32..64])));
    let mut raw = vec![0x02u8];
    raw.extend(rlp_list(&final_payload));

    let raw_hex = format!("0x{}", hex::encode(&raw));
    std::fs::write(dir.join("prod-rawtx.txt"), &raw_hex).unwrap();
    println!("TX_SIGNED bytes={} keccak=0x{}", raw.len(), hex::encode(keccak256(&raw)));
}
