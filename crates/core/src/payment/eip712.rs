use alloy_primitives::{keccak256, Address, B256, U256};
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use crate::PaymentError;

pub fn domain_separator(name:&str, version:&str, chain_id:u64, verifying_contract:Address) -> B256 {
    let th = keccak256(b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)");
    let mut b = Vec::with_capacity(2+32*5);
    b.extend_from_slice(th.as_slice());
    b.extend_from_slice(keccak256(name.as_bytes()).as_slice());
    b.extend_from_slice(keccak256(version.as_bytes()).as_slice());
    b.extend_from_slice(&U256::from(chain_id).to_be_bytes::<32>());
    b.extend_from_slice(verifying_contract.into_word().as_slice());
    keccak256(&b)
}
pub fn signing_digest(ds:&B256, sh:&B256) -> B256 {
    let mut b = Vec::with_capacity(66);
    b.extend_from_slice(&[0x19,0x01]); b.extend_from_slice(ds.as_slice()); b.extend_from_slice(sh.as_slice());
    keccak256(&b)
}
pub fn recover_address(digest:&B256, signature:&[u8]) -> Result<Address, PaymentError> {
    if signature.len()!=65 { return Err(PaymentError::InvalidSignatureLength); }
    let (sb,vb)=signature.split_at(64);
    let sig=Signature::from_slice(sb).map_err(|_|PaymentError::RecoveryFailed)?;
    let rid=match vb[0]{
        r@(0|1)=>RecoveryId::try_from(r).map_err(|_|PaymentError::InvalidRecoveryId(r))?,
        r@(27|28)=>RecoveryId::try_from(r-27).map_err(|_|PaymentError::InvalidRecoveryId(r))?,
        o=>return Err(PaymentError::InvalidRecoveryId(o)),
    };
    let vk=VerifyingKey::recover_from_prehash(digest.as_slice(),&sig,rid).map_err(|_|PaymentError::RecoveryFailed)?;
    let p=vk.to_encoded_point(false); let h=keccak256(&p.as_bytes()[1..]);
    Ok(Address::from_slice(&h[12..]))
}
#[cfg(test)] mod tests { use super::*; use k256::ecdsa::SigningKey;
  fn addr(sk:&SigningKey)->Address{let vk=VerifyingKey::from(sk);let p=vk.to_encoded_point(false);let h=keccak256(&p.as_bytes()[1..]);Address::from_slice(&h[12..])}
  #[test] fn roundtrip(){let sk=SigningKey::from_slice(&hex::decode("4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318").unwrap()).unwrap();
    let d=keccak256(b"m2m finops test digest");let (s,r)=sk.sign_prehash_recoverable(d.as_slice()).unwrap();
    let mut s65=[0u8;65];s65[..64].copy_from_slice(&s.to_bytes());s65[64]=r.to_byte();
    assert_eq!(recover_address(&d,&s65).unwrap(),addr(&sk));}
  #[test] fn bad_len(){let d=keccak256(b"x");assert_eq!(recover_address(&d,&[0u8;64]).unwrap_err(),crate::PaymentError::InvalidSignatureLength);}
}
