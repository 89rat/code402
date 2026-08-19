//! C1 — x402 v2 paying-client core (crawler track, plans/paying-crawler-plan.md).
//! Pure, worker-free, reusable by: the keygen `payv2` e2e generator, the
//! future crawler, and C3's Web Bot Auth integration.
//!
//! Pipeline: detect 402 -> parse PaymentRequired (v2 header + real-v1 body)
//! -> select a requirement under policy (I2: deny by default) -> construct
//! EIP-3009 authorization -> sign via an injected Signer (DryRun ships first:
//! full pipeline, signs NOTHING) -> assemble PaymentPayload (extensions
//! echoed verbatim per spec §5.1.2) -> encode PAYMENT-SIGNATURE -> parse
//! PAYMENT-RESPONSE receipt (SettleResponse).
//!
//! I6 (content never touches payment decisions): nothing here reads response
//! bodies beyond the protocol envelopes.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::payment::x402v2::{
    decode_b64_json, Authorization, ExactEvmPayload, PaymentPayload, PaymentRequired,
    PaymentRequirements, SettleResponse, X402Error,
};
use alloy_primitives::{Address, B256, U256};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// 402 parsing — both wire generations
// ---------------------------------------------------------------------------

/// Parse the v2 form: Base64 JSON in the PAYMENT-REQUIRED header.
pub fn parse_v2_payment_required(header_value: &str) -> Result<PaymentRequired, X402Error> {
    let pr: PaymentRequired = decode_b64_json(header_value)?;
    pr.validate()?;
    Ok(pr)
}

/// Parse the REAL x402 v1 form (Kimi S3 major #2): JSON body with
/// `x402Version: 1`, `accepts[]` carrying v1 field names — `maxAmountRequired`
/// (not `amount`), non-CAIP-2 network names ("base", "base-sepolia") — and
/// no §6.1 reserved keys. Mapped into the v2 shape for one downstream
/// pipeline. Unknown network names map to nothing (denied at selection).
pub fn parse_v1_payment_required(body: &str) -> Result<PaymentRequired, X402Error> {
    #[derive(serde::Deserialize)]
    struct V1Requirement {
        scheme: String,
        network: String,
        #[serde(rename = "maxAmountRequired")]
        max_amount_required: String,
        asset: String,
        #[serde(rename = "payTo")]
        pay_to: String,
        #[serde(rename = "maxTimeoutSeconds", default)]
        max_timeout_seconds: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        extra: Option<serde_json::Value>,
    }
    #[derive(serde::Deserialize)]
    struct V1Body {
        #[serde(rename = "x402Version")]
        x402_version: u64,
        #[serde(default)]
        error: Option<String>,
        accepts: Vec<V1Requirement>,
    }
    let v1: V1Body =
        serde_json::from_str(body).map_err(|e| X402Error::InvalidJson(e.to_string()))?;
    if v1.x402_version != 1 {
        return Err(X402Error::WrongVersion(v1.x402_version));
    }
    if v1.accepts.is_empty() {
        return Err(X402Error::MissingField("accepts"));
    }
    let map_network = |n: &str| -> Option<String> {
        match n {
            "base" => Some("eip155:8453".to_string()),
            "base-sepolia" | "base-sepolia-testnet" => Some("eip155:84532".to_string()),
            other if other.starts_with("eip155:") => Some(other.to_string()),
            _ => None, // unknown v1 network name: preserved as-is, denied at selection
        }
    };
    let accepts = v1
        .accepts
        .into_iter()
        .map(|a| PaymentRequirements {
            scheme: a.scheme,
            network: map_network(&a.network).unwrap_or(a.network),
            amount: a.max_amount_required, // v1 name -> v2 semantic
            asset: a.asset,
            pay_to: a.pay_to,
            max_timeout_seconds: a.max_timeout_seconds.unwrap_or(60),
            extra: a.extra,
        })
        .collect();
    let pr = PaymentRequired {
        x402_version: 2,
        error: v1.error,
        resource: crate::payment::x402v2::ResourceInfo {
            url: String::new(),
            description: None,
            mime_type: None,
            service_name: None,
            tags: None,
            icon_url: None,
        },
        accepts,
        extensions: None,
    };
    // LENIENT validation: v1 network names we do not recognize are kept
    // verbatim and denied later at selection (parse = shape; policy =
    // selection). Only structural money fields must parse.
    for a in &pr.accepts {
        a.amount_u256()?;
        if a.asset.is_empty() || a.pay_to.is_empty() {
            return Err(crate::payment::x402v2::X402Error::BadAddress(a.asset.clone()));
        }
    }
    Ok(pr)
}

// ---------------------------------------------------------------------------
// Requirement selection under policy (I2 — deny by default)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SelectionPolicy {
    /// CAIP-2 networks we are willing to pay on (e.g. ["eip155:84532"]).
    pub allowed_networks: Vec<String>,
    /// Token contract addresses we are willing to pay with.
    pub allowed_assets: Vec<String>,
    /// APPROVED PAYEES (red team Break 2): a malicious 402 naming an
    /// attacker wallet must be denied even with valid network/asset/amount.
    pub allowed_payees: Vec<String>,
    /// Price ceiling in atomic units (per content class at C2; global here).
    pub max_amount: U256,
}

impl SelectionPolicy {
    /// First requirement matching every predicate; deny-by-default. A
    /// spec-INVALID matching requirement is SKIPPED, not fatal (Kimi S3
    /// minor #8) — a later accept may be perfectly good.
    pub fn select<'a>(
        &self,
        pr: &'a PaymentRequired,
    ) -> Result<&'a PaymentRequirements, X402Error> {
        for a in &pr.accepts {
            if !self.allowed_networks.contains(&a.network) {
                continue;
            }
            if !self.allowed_assets.iter().any(|x| x.eq_ignore_ascii_case(&a.asset)) {
                continue;
            }
            if !self.allowed_payees.iter().any(|x| x.eq_ignore_ascii_case(&a.pay_to)) {
                continue; // unapproved payee: deny (I2)
            }
            if a.amount_u256().unwrap_or(alloy_primitives::U256::MAX) > self.max_amount {
                continue;
            }
            if a.validate_spec().is_err() {
                continue; // skip invalid, try next accept
            }
            return Ok(a);
        }
        Err(X402Error::PolicyDenied("no acceptable requirement under policy".into()))
    }
}

// ---------------------------------------------------------------------------
// Authorization construction + signing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AuthorizationParams {
    pub payer: Address,
    pub pay_to: Address,
    pub value: U256,
    /// Client-chosen 32-byte random nonce (spec: the CLIENT picks it).
    pub nonce: [u8; 32],
    pub valid_after_unix: u64,
    pub valid_before_unix: u64,
}

pub fn build_authorization(p: &AuthorizationParams) -> Authorization {
    let hexs = |b: &[u8]| -> String {
        b.iter().map(|x| format!("{x:02x}")).collect::<String>()
    };
    Authorization {
        from: format!("{:?}", p.payer),
        to: format!("{:?}", p.pay_to),
        value: p.value.to_string(),
        valid_after: p.valid_after_unix.to_string(),
        valid_before: p.valid_before_unix.to_string(),
        nonce: format!("0x{}", hexs(&p.nonce)),
    }
}

/// I2: signatures exist only through policy. DryRun signs NOTHING.
pub enum Signer {
    /// Real EOA signer (k256). Test/e2e + future crawler signer service.
    Eoa(k256::ecdsa::SigningKey),
    /// Full pipeline, refuses to sign: returns the would-be payment for
    /// logging. This is also the free tier of the future product.
    DryRun,
}

pub enum SignedPayment {
    Signed { payload: PaymentPayload, b64: String },
    /// Dry-run: nothing signed; the payment the pipeline WOULD have made.
    WouldPay { requirement: PaymentRequirements, authorization: Authorization },
}

/// Assemble + (maybe) sign. `stamp_extension` is the server's extensions map
/// echoed VERBATIM (§5.1.2: must include at least the info received).
pub fn sign_payment(
    requirement: &PaymentRequirements,
    auth: &Authorization,
    signer: &Signer,
    stamp_extension: Option<BTreeMap<String, crate::payment::x402v2::ExtensionData>>,
) -> Result<SignedPayment, X402Error> {
    match signer {
        Signer::DryRun => Ok(SignedPayment::WouldPay {
            requirement: requirement.clone(),
            authorization: auth.clone(),
        }),
        Signer::Eoa(sk) => {
            // I2 (red team Break 3): the signed authorization is BOUND to the
            // policy-selected requirement — value and payee must match exactly
            // or nothing is signed.
            if auth.value_u256()? != requirement.amount_u256()? {
                return Err(X402Error::ExactAmountMismatch(auth.value.clone(), requirement.amount.clone()));
            }
            if auth.to_addr()? != requirement.pay_to_addr()? {
                return Err(X402Error::RecipientMismatch(auth.to.clone()));
            }
            let ds = crate::payment::x402v2_verify::domain_separator_from_requirement(requirement)?;
            let twa = crate::payment::x402v2_verify::authorization_to_twa(auth)?;
            let sh = crate::payment::erc3009::struct_hash(&twa);
            let digest = crate::payment::eip712::signing_digest(&ds, &sh);
            let (sig, rid) = sk
                .sign_prehash_recoverable(digest.as_slice())
                .map_err(|_| X402Error::BadSignature(0))?;
            let mut s65 = Vec::with_capacity(65);
            s65.extend_from_slice(&sig.to_bytes());
            s65.push(rid.to_byte());
            let hexs = |b: &[u8]| -> String {
                b.iter().map(|x| format!("{x:02x}")).collect::<String>()
            };
            let payload = PaymentPayload {
                x402_version: 2,
                resource: None,
                accepted: requirement.clone(),
                payload: ExactEvmPayload {
                    signature: format!("0x{}", hexs(&s65)),
                    authorization: auth.clone(),
                },
                extensions: stamp_extension,
            };
            let b64 = encode_payment_signature(&payload)?;
            Ok(SignedPayment::Signed { payload, b64 })
        }
    }
}

pub fn encode_payment_signature(p: &PaymentPayload) -> Result<String, X402Error> {
    if p.x402_version != 2 {
        return Err(X402Error::WrongVersion(p.x402_version));
    }
    crate::payment::x402v2::encode_b64_json(p)
}

/// Parse the PAYMENT-RESPONSE header into a validated SettleResponse (I4:
/// receipts are only trusted after on-chain reconciliation — C2 adds that).
pub fn parse_settle_response(header_value: &str) -> Result<SettleResponse, X402Error> {
    let sr: SettleResponse = decode_b64_json(header_value)?;
    sr.validate()?;
    Ok(sr)
}

/// Random 32-byte nonce (client-owned per spec; the CLIENT picks it).
pub fn random_nonce() -> [u8; 32] {
    use k256::elliptic_curve::rand_core::{OsRng, RngCore};
    let mut n = [0u8; 32];
    OsRng.fill_bytes(&mut n);
    n
}

/// Helper for C2's nonce ledger: authorization digest preimage binding is
/// (from ‖ nonce); expose the pair deterministically.
pub fn nonce_key(auth: &Authorization) -> Result<B256, X402Error> {
    let mut b = Vec::with_capacity(52);
    b.extend_from_slice(auth.from_addr()?.as_slice());
    b.extend_from_slice(&auth.nonce_bytes()?);
    Ok(alloy_primitives::keccak256(&b))
}

/// Helper mirroring SelectionPolicy::select for non-async callers (bin tools).
pub fn select_helper<'a>(
    pr: &'a crate::payment::x402v2::PaymentRequired,
    policy: &SelectionPolicy,
) -> Result<&'a crate::payment::x402v2::PaymentRequirements, X402Error> {
    policy.select(pr)
}
