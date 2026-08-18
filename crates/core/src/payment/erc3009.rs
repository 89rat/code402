use alloy_primitives::{keccak256, Address, B256, U256};
use serde::{Deserialize, Serialize};
use super::eip712; use crate::PaymentError;

#[derive(Debug,Clone,Serialize,Deserialize)] pub struct TransferWithAuthorization{pub from:Address,pub to:Address,pub value:U256,pub valid_after:u64,pub valid_before:u64,pub nonce:B256}
#[derive(Debug,Clone,Serialize,Deserialize)] pub struct PaymentVoucher{pub auth:TransferWithAuthorization,pub signature:Vec<u8>}
pub struct VerifyContext{pub token_name:String,pub token_version:String,pub chain_id:u64,pub token_address:Address,pub expected_recipient:Address,pub required_amount:U256,pub now_unix:u64}
const T:&str="TransferWithAuthorization(address from,address to,uint256 value,uint256 validAfter,uint256 validBefore,bytes32 nonce)";
fn struct_hash(a:&TransferWithAuthorization)->B256{let th=keccak256(T.as_bytes());let mut b=Vec::with_capacity(32*7);
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
