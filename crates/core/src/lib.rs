pub mod payment; pub mod validate; pub mod receipt;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PaymentError {
    #[error("signature must be exactly 65 bytes (r||s||v)")] InvalidSignatureLength,
    #[error("invalid recovery id: {0}")] InvalidRecoveryId(u8),
    #[error("elliptic curve recovery failed")] RecoveryFailed,
    #[error("recovered signer does not match declared payer")] SignerMismatch,
    #[error("token address is not allowlisted")] UnsupportedToken,
    #[error("chain id is not allowlisted")] UnsupportedChain,
    #[error("recipient is not the corporate wallet")] InvalidRecipient,
    #[error("amount is below the required price")] InsufficientAmount,
    #[error("payment is outside its validity window")] OutsideValidityWindow,
}
