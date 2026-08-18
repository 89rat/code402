//! Stage 2 crypto conformance — EIP-712/EIP-3009 verification for x402 v2
//! payloads, layered between the Stage-1 structural gate and the Stage-4
//! facilitator call (G4: local verification is a QUOTA PREFILTER, never the
//! authority — the facilitator's /verify is the judge).
//!
//! Domain binding: the EIP-712 domain comes from the REQUIREMENT (which we
//! issued and MAC-verified upstream — launch-checklist #1/#2), not from the
//! client echo: name/version from `extra`, chainId derived from the CAIP-2
//! `network`, verifyingContract = `asset`. A signature made for a different
//! token/chain/domain therefore fails recovery-to-`from` — wrong-chain and
//! wrong-token forgeries die here without spending facilitator quota.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::payment::x402v2::{Authorization, PaymentPayload, PaymentRequirements, X402Error};
use crate::payment::eip712;
use crate::payment::erc3009::{self, TransferWithAuthorization};
use alloy_primitives::{Address, B256};

/// 65 bytes = plain EOA ECDSA case; longer = EIP-6492 envelope.
pub const EOA_SIG_LEN: usize = 65;
const EOA_SIG_HEX: usize = EOA_SIG_LEN * 2;

/// Outcome of the local prefilter. Facilitator remains authoritative in ALL
/// cases — `LocalPass` is a quota-guard decision, not final judgment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyOutcome {
    /// Exactly-65-byte signature, recovered address == authorization.from.
    /// Almost certainly an EOA that signed this exact domain+struct.
    LocalPass { recovered: Address },
    /// Definite local garbage (shape/recovery/signer failures). Rejected
    /// without a facilitator call.
    LocalReject(X402Error),
    /// EIP-6492 envelope (>65 bytes) or otherwise not locally verifiable.
    /// Forwarded to the facilitator, which verifies 6492/1271 properly.
    ///
    /// NOTE (recorded rule, Kimi stage-1 + DeepSeek): a plain-65-byte
    /// signature that does NOT recover to `from` is treated as garbage and
    /// locally rejected. A legitimate ERC-1271 contract wallet that chose to
    /// send a non-recovering dummy 65-byte blob instead of a 6492 envelope
    /// WOULD be falsely rejected here. Accepted trade: 6492 exists precisely
    /// to signal this case and the spec's client guidance wraps smart-wallet
    /// signatures. If production ever shows a false reject, forward
    /// recovery-failing 65-byte sigs too (quota cost) — and the incident
    /// becomes a conformance vector (kaizen rule).
    PassThrough,
}

/// Extract chainId from a CAIP-2 `eip155:<id>` network. Other namespaces
/// (solana:…) cannot carry this EVM scheme — reject (the facilitator would).
pub fn chain_id_from_network(network: &str) -> Result<u64, X402Error> {
    let rest = network
        .strip_prefix("eip155:")
        .ok_or_else(|| X402Error::BadNetwork(network.to_string()))?;
    rest.parse::<u64>()
        .map_err(|_| X402Error::BadNetwork(network.to_string()))
}

fn extra_str<'a>(req: &'a PaymentRequirements, key: &str) -> Result<&'a str, X402Error> {
    req.extra
        .as_ref()
        .and_then(|e| e.get(key))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or(X402Error::ReservedExtraKey("extra.name/version"))
}

/// Build the EIP-712 domain separator from OUR issued requirement.
pub fn domain_separator_from_requirement(req: &PaymentRequirements) -> Result<B256, X402Error> {
    let name = extra_str(req, "name")?;
    let version = extra_str(req, "version")?;
    let chain_id = chain_id_from_network(&req.network)?;
    let token = req.asset_addr()?;
    Ok(eip712::domain_separator(name, version, chain_id, token))
}

fn to_twa(auth: &Authorization) -> Result<TransferWithAuthorization, X402Error> {
    Ok(TransferWithAuthorization {
        from: auth.from_addr()?,
        to: auth.to_addr()?,
        value: auth.value_u256()?,
        valid_after: auth.valid_after_unix()?,
        valid_before: auth.valid_before_unix()?,
        // nonce_bytes() already enforces the 32-byte shape
        nonce: B256::from(auth.nonce_bytes()?),
    })
}

fn hex_decode(s: &str, expect_bytes: usize) -> Result<Vec<u8>, X402Error> {
    let body = s.strip_prefix("0x").ok_or(X402Error::BadSignature(0))?;
    if body.len() != expect_bytes * 2 || !body.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(X402Error::BadSignature(s.len()));
    }
    (0..body.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&body[i..i + 2], 16).map_err(|_| X402Error::BadSignature(s.len()))
        })
        .collect()
}

/// Local EOA-ecrecover prefilter (G4). Exactly-65-byte signatures are
/// verified against the domain-bound digest; longer well-formed signatures
/// pass through to the facilitator (6492); local failures reject without
/// quota spend.
pub fn prefilter(payload: &PaymentPayload, req: &PaymentRequirements) -> VerifyOutcome {
    match run(payload, req) {
        Ok(o) => o,
        Err(e) => VerifyOutcome::LocalReject(e),
    }
}

fn run(payload: &PaymentPayload, req: &PaymentRequirements) -> Result<VerifyOutcome, X402Error> {
    let sig_str = &payload.payload.signature;
    let body = sig_str.strip_prefix("0x").ok_or(X402Error::BadSignature(0))?;
    if !body.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(X402Error::BadSignature(sig_str.len()));
    }
    // 6492 envelope classification: strictly longer than the EOA form
    if body.len() > EOA_SIG_HEX {
        return Ok(VerifyOutcome::PassThrough);
    }
    if body.len() != EOA_SIG_HEX {
        return Err(X402Error::BadSignature(sig_str.len()));
    }

    let auth = &payload.payload.authorization;
    let ds = domain_separator_from_requirement(req)?;
    let twa = to_twa(auth)?;
    let sh = erc3009::struct_hash(&twa);
    let digest = eip712::signing_digest(&ds, &sh);
    let sig = hex_decode(sig_str, EOA_SIG_LEN)?;
    // EIP-2: high-s signatures are non-canonical/malleable and rejected by
    // conforming verifiers (incl. the facilitator). normalize_s() returning
    // Some means the signature was high-s.
    let ecsig = k256::ecdsa::Signature::from_slice(&sig[..64])
        .map_err(|_| X402Error::BadSignature(EOA_SIG_LEN))?;
    if ecsig.normalize_s().is_some() {
        return Err(X402Error::BadSignature(EOA_SIG_LEN));
    }
    match eip712::recover_address(&digest, &sig) {
        Ok(recovered) => {
            if recovered == twa.from {
                Ok(VerifyOutcome::LocalPass { recovered })
            } else {
                // recovered to a different address: wrong key, wrong domain
                // (chain/token/name), or forged `from` — all die here.
                Err(X402Error::BadAddress(format!(
                    "signer mismatch: recovered {recovered:?} != declared {:?}",
                    twa.from
                )))
            }
        }
        Err(_) => Err(X402Error::BadSignature(EOA_SIG_LEN)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caip2_chain_extraction() {
        assert_eq!(chain_id_from_network("eip155:84532"), Ok(84532));
        assert_eq!(chain_id_from_network("eip155:8453"), Ok(8453));
        assert!(chain_id_from_network("solana:5eykt4").is_err());
        assert!(chain_id_from_network("base").is_err());
    }
}
