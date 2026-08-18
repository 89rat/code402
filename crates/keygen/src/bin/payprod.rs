//! PRODUCTION paid-call voucher signer (Base mainnet, real USDC).
//!
//! Reads the payer key from .staging/prod-payer.txt (field COMPANY_WALLET_SECRET —
//! historical mislabel; that file backs up the funded payer wallet 0xD654…4729).
//! GUARD: aborts unless the derived address equals the expected funded payer.
//! Writes the voucher to .staging/prod-voucher.json. Never prints secrets.

use alloy_primitives::{Address, B256, U256};
use k256::ecdsa::SigningKey;
use m2m_core::payment::{eip712, erc3009::PaymentVoucher, erc3009::TransferWithAuthorization};

const EXPECTED_PAYER: &str = "0xd654cd6e272571e1be074c5499cb20fe855a4729";
const COMPANY: &str = "0xdcd0fe977640add2dbe62ca0fb30c63f2fd9fdcf";
const USDC_MAINNET: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
const VALUE_MINOR: u64 = 5000; // $0.005 — cheapest live quote
const CHAIN_ID: u64 = 8453;

fn addr(sk: &SigningKey) -> Address {
    let vk = sk.verifying_key();
    let p = vk.to_encoded_point(false);
    let h = alloy_primitives::keccak256(&p.as_bytes()[1..]);
    Address::from_slice(&h[12..])
}

/// Load a `KEY=value` file and return one field. Never logs the value.
fn read_field(path: &str, field: &str) -> String {
    let txt = std::fs::read_to_string(path).expect("credentials file readable");
    for line in txt.lines() {
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == field {
                return v.trim().to_string();
            }
        }
    }
    panic!("field {field} not found in {path}");
}

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.staging");
    let key_hex = read_field(dir.join("prod-payer.txt").to_str().unwrap(), "COMPANY_WALLET_SECRET");
    let payer = SigningKey::from_slice(&hex::decode(key_hex).expect("payer key hex")).unwrap();

    let payer_addr = addr(&payer);
    assert_eq!(
        format!("{payer_addr:?}").to_lowercase(),
        EXPECTED_PAYER,
        "KEY GUARD: prod-payer.txt key does not derive to the funded payer wallet — aborting"
    );

    let company: Address = COMPANY.parse().unwrap();
    let token: Address = USDC_MAINNET.parse().unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut nonce = [0u8; 32];
    use k256::elliptic_curve::rand_core::RngCore;
    k256::elliptic_curve::rand_core::OsRng.fill_bytes(&mut nonce);

    let auth = TransferWithAuthorization {
        from: payer_addr,
        to: company,
        value: U256::from(VALUE_MINOR),
        valid_after: 0,
        valid_before: now + 3600,
        nonce: B256::from(nonce),
    };

    let th = alloy_primitives::keccak256(
        b"TransferWithAuthorization(address from,address to,uint256 value,uint256 validAfter,uint256 validBefore,bytes32 nonce)",
    );
    let mut sh = Vec::with_capacity(32 * 7);
    sh.extend_from_slice(th.as_slice());
    sh.extend_from_slice(auth.from.into_word().as_slice());
    sh.extend_from_slice(auth.to.into_word().as_slice());
    sh.extend_from_slice(&auth.value.to_be_bytes::<32>());
    sh.extend_from_slice(&U256::from(auth.valid_after).to_be_bytes::<32>());
    sh.extend_from_slice(&U256::from(auth.valid_before).to_be_bytes::<32>());
    sh.extend_from_slice(auth.nonce.as_slice());
    let struct_hash = alloy_primitives::keccak256(&sh);

    // Base MAINNET USDC: EIP-712 domain name "USD Coin", version "2"
    let ds = eip712::domain_separator("USD Coin", "2", CHAIN_ID, token);
    let digest = eip712::signing_digest(&ds, &struct_hash);

    let (sig, rid) = payer.sign_prehash_recoverable(digest.as_slice()).unwrap();
    let mut s65 = sig.to_bytes().to_vec();
    s65.push(rid.to_byte());

    let voucher = PaymentVoucher { auth, signature: s65 };
    let out = dir.join("prod-voucher.json");
    std::fs::write(&out, serde_json::to_string(&voucher).unwrap()).unwrap();
    println!("PAYER_ADDRESS={payer_addr:?}");
    println!("VOUCHER_WRITTEN={}", out.display());
}
