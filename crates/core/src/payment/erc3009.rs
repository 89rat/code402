use alloy_primitives::{keccak256, Address, B256, U256};
use serde::{Deserialize, Serialize};
use super::eip712; use crate::PaymentError;

#[derive(Debug,Clone,Serialize,Deserialize)] pub struct TransferWithAuthorization{pub from:Address,pub to:Address,pub value:U256,pub valid_after:u64,pub valid_before:u64,pub nonce:B256}
#[derive(Debug,Clone,Serialize,Deserialize)] pub struct PaymentVoucher{pub auth:TransferWithAuthorization,pub signature:Vec<u8>}
pub struct VerifyContext{pub token_name:String,pub token_version:String,pub chain_id:u64,pub token_address:Address,pub expected_recipient:Address,pub required_amount:U256,pub now_unix:u64}
const T:&str="TransferWithAuthorization(address from,address to,uint256 value,uint256 validAfter,uint256 validBefore,bytes32 nonce)";
pub fn struct_hash(a:&TransferWithAuthorization)->B256{let th=keccak256(T.as_bytes());let mut b=Vec::with_capacity(32*7);
 b.extend_from_slice(th.as_slice());b.extend_from_slice(a.from.into_word().as_slice());b.extend_from_slice(a.to.into_word().as_slice());
 b.extend_from_slice(&a.value.to_be_bytes::<32>());b.extend_from_slice(&U256::from(a.valid_after).to_be_bytes::<32>());
 b.extend_from_slice(&U256::from(a.valid_before).to_be_bytes::<32>());b.extend_from_slice(a.nonce.as_slice());keccak256(&b)}
pub fn verify(v:&PaymentVoucher,c:&VerifyContext)->Result<Address,PaymentError>{let a=&v.auth;
 if a.to!=c.expected_recipient{return Err(PaymentError::InvalidRecipient);}
 if a.value<c.required_amount{return Err(PaymentError::InsufficientAmount);}
 if !(a.valid_after<=c.now_unix&&c.now_unix<=a.valid_before){return Err(PaymentError::OutsideValidityWindow);}
 let d=eip712::domain_separator(&c.token_name,&c.token_version,c.chain_id,c.token_address);
 let dg=eip712::signing_digest(&d,&struct_hash(a));
 let s=eip712::recover_address(&dg,&v.signature)?;
 if s!=a.from{return Err(PaymentError::SignerMismatch);} Ok(s)}

#[cfg(test)]
mod tests {
 use super::*;
 use k256::ecdsa::SigningKey;

 // Deterministic test keys (NOT real wallets). "11"*32 / "22"*32 mirror the relay's
 // verify.test.ts vectors.
 fn payer_key()->SigningKey{SigningKey::from_slice(&[0x11u8;32]).unwrap()}
 fn other_key()->SigningKey{SigningKey::from_slice(&[0x22u8;32]).unwrap()}
 fn addr_of(sk:&SigningKey)->Address{
  use k256::ecdsa::VerifyingKey;
  let vk=VerifyingKey::from(sk);let p=vk.to_encoded_point(false);
  Address::from_slice(&keccak256(&p.as_bytes()[1..])[12..])}

 const NOW:u64=1_700_000_000;

 fn ctx()->VerifyContext{VerifyContext{
  token_name:"USD Coin".into(),token_version:"2".into(),chain_id:8453,
  token_address:Address::from([0xcc;20]),
  expected_recipient:Address::from([0xa1;20]),
  required_amount:U256::from(5000u64),now_unix:NOW}}

 fn auth(sk:&SigningKey,to:Address,value:u64,nonce_byte:u8)->TransferWithAuthorization{
  TransferWithAuthorization{from:addr_of(sk),to,value:U256::from(value),
   valid_after:0,valid_before:NOW+300,nonce:B256::from([nonce_byte;32])}}

 fn sign(sk:&SigningKey,a:&TransferWithAuthorization,c:&VerifyContext)->Vec<u8>{
  let d=eip712::domain_separator(&c.token_name,&c.token_version,c.chain_id,c.token_address);
  let dg=eip712::signing_digest(&d,&struct_hash(a));
  let (s,r)=sk.sign_prehash_recoverable(dg.as_slice()).unwrap();
  let mut s65=Vec::with_capacity(65);s65.extend_from_slice(&s.to_bytes());s65.push(27+r.to_byte());s65}

 // Port of kaizen-relay verify.test.ts: "verifyAuth recovers the true payer"
 #[test] fn valid_voucher_verifies_to_payer(){
  let sk=payer_key();let c=ctx();let a=auth(&sk,c.expected_recipient,5000,0x01);
  let sig=sign(&sk,&a,&c);let v=PaymentVoucher{auth:a,signature:sig};
  assert_eq!(verify(&v,&c).unwrap(),addr_of(&sk));}

 // "signer mismatch rejected": signed by a different key, from: still payer
 #[test] fn signer_mismatch_rejected(){
  let payer=payer_key();let c=ctx();
  let mut a=auth(&other_key(),c.expected_recipient,5000,0x02);
  a.from=addr_of(&payer); // claim payer, signed by other
  let sig=sign(&other_key(),&a,&c);
  let v=PaymentVoucher{auth:a,signature:sig};
  assert_eq!(verify(&v,&c).unwrap_err(),PaymentError::SignerMismatch);}

 // "expired rejected"
 #[test] fn expired_rejected(){
  let sk=payer_key();let mut c=ctx();c.now_unix=NOW+400; // past valid_before
  let a=auth(&sk,c.expected_recipient,5000,0x03);
  let sig=sign(&sk,&a,&c);let v=PaymentVoucher{auth:a,signature:sig};
  assert_eq!(verify(&v,&c).unwrap_err(),PaymentError::OutsideValidityWindow);}

 #[test] fn not_yet_valid_rejected(){
  let sk=payer_key();let c=ctx();
  let mut a=auth(&sk,c.expected_recipient,5000,0x04);a.valid_after=NOW+100;
  let sig=sign(&sk,&a,&c);let v=PaymentVoucher{auth:a,signature:sig};
  assert_eq!(verify(&v,&c).unwrap_err(),PaymentError::OutsideValidityWindow);}

 #[test] fn wrong_recipient_rejected(){
  let sk=payer_key();let c=ctx();
  let a=auth(&sk,Address::from([0x99;20]),5000,0x05);
  let sig=sign(&sk,&a,&c);let v=PaymentVoucher{auth:a,signature:sig};
  assert_eq!(verify(&v,&c).unwrap_err(),PaymentError::InvalidRecipient);}

 #[test] fn insufficient_amount_rejected(){
  let sk=payer_key();let c=ctx();
  let a=auth(&sk,c.expected_recipient,4999,0x06);
  let sig=sign(&sk,&a,&c);let v=PaymentVoucher{auth:a,signature:sig};
  assert_eq!(verify(&v,&c).unwrap_err(),PaymentError::InsufficientAmount);}

 // "high-s malleation rejected": s' = n - s, v flipped — the chain would reject it, so must we.
 #[test] fn high_s_malleation_rejected(){
  let sk=payer_key();let c=ctx();let a=auth(&sk,c.expected_recipient,5000,0x07);
  let mut sig=sign(&sk,&a,&c);
  // secp256k1 group order n
  const N:[u8;32]=[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xfe,
   0xba,0xae,0xdc,0xe6,0xaf,0x48,0xa0,0x3b,0xbf,0xd2,0x5e,0x8c,0xd0,0x36,0x41,0x41];
  let s=&sig[32..64];let mut s_hi=[0u8;32];let mut borrow=0i16;
  for i in (0..32).rev(){let d=N[i] as i16-s[i] as i16-borrow;
   if d<0{s_hi[i]=(d+256) as u8;borrow=1;}else{s_hi[i]=d as u8;borrow=0;}}
  sig[32..64].copy_from_slice(&s_hi);
  sig[64]=if sig[64]==27{28}else{27}; // recid flips when s is negated
  let v=PaymentVoucher{auth:a,signature:sig};
  assert_eq!(verify(&v,&c).unwrap_err(),PaymentError::HighSSignature);}
}
