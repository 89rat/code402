//! Verify a code402 receipt signature recovers to the receipt signing address.
//! Usage: verifyreceipt <commitment_hex> <signature_hex_0x>

use alloy_primitives::{Address, B256};
use m2m_core::payment::eip712;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let commitment = hex::decode(&args[1]).expect("commitment hex");
    let sig = hex::decode(args[2].trim_start_matches("0x")).expect("signature hex");
    let digest = B256::from_slice(&commitment);
    let recovered = eip712::recover_address(&digest, &sig).expect("recover");
    let expected: Address = "0x7b885c42b47671a91b3d81694ce18d38e25e7149".parse().unwrap();
    println!("recovered={:?}", recovered);
    println!("expected ={:?}", expected);
    if recovered == expected {
        println!("RECEIPT_SIGNATURE_VALID");
    } else {
        println!("RECEIPT_SIGNATURE_MISMATCH");
        std::process::exit(1);
    }
}
