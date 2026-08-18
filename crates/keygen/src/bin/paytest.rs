//! Staging paid-call tester (code402 acceptance gate b/c).
//! Generates a payer wallet (or reuses PAYER_SECRET env), signs an
//! EIP-3009 TransferWithAuthorization voucher for Base Sepolia USDC,
//! and prints the X-PAYMENT header JSON.
//!
//! STAGING ONLY. Uses the same m2m-core types as the edge verifier, so the
//! JSON shape is guaranteed compatible.

use alloy_primitives::{Address, B256, U256};
use k256::ecdsa::SigningKey;
use m2m_core::payment::{eip712, erc3009::TransferWithAuthorization, erc3009::PaymentVoucher};

fn addr(sk: &SigningKey) -> Address {
    let vk = sk.verifying_key();
    let p = vk.to_encoded_point(false);
    let h = alloy_primitives::keccak256(&p.as_bytes()[1..]);
    Address::from_slice(&h[12..])
}

fn main() {
    // Persist the key to a local gitignored file so the funded test wallet
    // is never stranded. Secret goes to the file, never to stdout.
    let secret_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.staging/payer-secret.txt");
    let payer = match std::env::var("PAYER_SECRET") {
        Ok(hex_key) => {
            SigningKey::from_slice(&hex::decode(hex_key).expect("PAYER_SECRET hex")).unwrap()
        }
        Err(_) => {
            let sk = SigningKey::random(&mut k256::elliptic_curve::rand_core::OsRng);
            if let Some(dir) = secret_path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(&secret_path, hex::encode(sk.to_bytes()));
            sk
        }
    };
    let company: Address = "0x3bca128282a1de2f74efc16fa44a32a6f88a72ff".parse().unwrap();
    let token: Address = "0x036CbD53842c5426634e7929541eC2318f3dCF7e".parse().unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut nonce = [0u8; 32];
    use k256::elliptic_curve::rand_core::RngCore;
    k256::elliptic_curve::rand_core::OsRng.fill_bytes(&mut nonce);

    let auth = TransferWithAuthorization {
        from: addr(&payer),
        to: company,
        value: U256::from(5000u64), // 0.005 USDC
        valid_after: 0,
        valid_before: now
            + std::env::var("VALID_SECONDS")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(86400),
        nonce: B256::from(nonce),
    };

    // Mirror the verifier's exact digest construction
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

    // Base Sepolia USDC uses on-chain name "USDC" (mainnet is "USD Coin").
    let ds = eip712::domain_separator("USDC", "2", 84532, token);
    let digest = eip712::signing_digest(&ds, &struct_hash);

    let (sig, rid) = payer.sign_prehash_recoverable(digest.as_slice()).unwrap();
    let mut s65 = sig.to_bytes().to_vec();
    s65.push(rid.to_byte());

    let voucher = PaymentVoucher { auth, signature: s65 };
    println!("PAYER_ADDRESS={}", addr(&payer));
    println!("VOUCHER_JSON={}", serde_json::to_string(&voucher).unwrap());
}
