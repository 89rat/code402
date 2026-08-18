//! One-shot staging key generator for code402.
//! Generates two secp256k1 keypairs:
//!   1. COMPANY_WALLET (Base Sepolia staging receiving address)
//!   2. RECEIPT_SIGNING_KEY (receipt signature key)
//! STAGING ONLY — testnet funds have no value. Production keys are
//! generated separately and never touch this machine's disk in plaintext.

use k256::ecdsa::SigningKey;
use sha3::{Digest, Keccak256};

fn evm_address(key: &SigningKey) -> String {
    let vk = key.verifying_key();
    let uncompressed = vk.to_encoded_point(false);
    let pubkey = &uncompressed.as_bytes()[1..]; // strip 0x04 prefix
    let mut hasher = Keccak256::new();
    hasher.update(pubkey);
    let hash = hasher.finalize();
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..]);
    format!("0x{}", hex::encode(addr))
}

fn main() {
    // Two independent random keys
    let company = SigningKey::random(&mut k256::elliptic_curve::rand_core::OsRng);
    let receipt = SigningKey::random(&mut k256::elliptic_curve::rand_core::OsRng);

    println!("COMPANY_WALLET_ADDRESS={}", evm_address(&company));
    println!("COMPANY_WALLET_SECRET={}", hex::encode(company.to_bytes()));
    println!("RECEIPT_SIGNING_ADDRESS={}", evm_address(&receipt));
    println!("RECEIPT_SIGNING_KEY={}", hex::encode(receipt.to_bytes()));
}
