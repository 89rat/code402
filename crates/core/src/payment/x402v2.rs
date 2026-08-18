//! x402 v2 wire types, codec, and structural validation (plan-rev3 Stage 1, G9).
//!
//! SINGLE SOURCE OF TRUTH: the vendored spec at `specs/x402/` (pinned commit in
//! `specs/x402/SPEC-VERSION`). Field names, requiredness, and types mirror
//! §5.1 (PaymentRequired), §5.2 (PaymentPayload), §5.3 (SettleResponse),
//! §5.4 (VerifyResponse), §6.1 (reserved `extra` keys). Where this file and
//! the vendored spec disagree, the spec wins and this file is wrong.
//!
//! Conventions:
//! - Wire strings stay strings (addresses, amount, timestamps, nonce) so
//!   serialization is byte-stable; typed accessors parse on demand.
//! - `Option` fields use `skip_serializing_if` to match spec examples exactly
//!   (optional fields are omitted, not null).
//! - Amounts are decimal strings parsed to U256 — never floats, never u64
//!   (G10 / integer-money rule).
//! - No panics on the payment path: `#![deny(unwrap_used, expect_used, panic)]`.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use alloy_primitives::{Address, U256};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Cloudflare rejects requests with very large headers (~32KB total); our cap
/// on a single payment header sits below that with room for peers' headers.
pub const MAX_HEADER_B64_BYTES: usize = 24_000;
/// Settle-time safety margin: validBefore must exceed now by at least this
/// much when checked pre-settle (G5) — Base settles ~2s; retries can stretch.
pub const SETTLE_MARGIN_SECONDS: u64 = 30;
pub const X402_VERSION: u64 = 2;

// ---------------------------------------------------------------------------
// Errors — taxonomy maps to spec §9 strings in a later layer; these are the
// parse/structural causes. Never leak internal names on the wire verbatim.
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum X402Error {
    #[error("header exceeds size cap ({0} > {MAX_HEADER_B64_BYTES} b64 chars)")]
    HeaderTooLarge(usize),
    #[error("header is not canonical base64 (RFC 4648)")]
    NotCanonicalBase64,
    #[error("payload is not valid JSON: {0}")]
    InvalidJson(String),
    #[error("x402Version must be 2, got {0}")]
    WrongVersion(u64),
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    #[error("amount must be a decimal digit string, got {0:?}")]
    AmountNotDecimal(String),
    #[error("amount exceeds 256-bit range: {0}")]
    AmountOverflow(String),
    #[error("timestamp must be a digit string (unix seconds), got {0:?}")]
    TimestampNotString(String),
    #[error("address is not valid hex: {0}")]
    BadAddress(String),
    #[error("nonce must be 0x + 64 hex chars (32 bytes), got len {0}")]
    BadNonce(usize),
    #[error("signature must be 0x + 130 hex chars (65 bytes), got len {0}")]
    BadSignature(usize),
    #[error("network must be a CAIP-2 string (e.g. eip155:84532), got {0:?}")]
    BadNetwork(String),
    #[error("scheme not supported: {0:?} (only exact)")]
    BadScheme(String),
    #[error("reserved extra key {0:?} missing or invalid")]
    ReservedExtraKey(&'static str),
    #[error("validBefore {0} is not at least now+{SETTLE_MARGIN_SECONDS}s ({1})")]
    ValidBeforeMargin(u64, u64),
    #[error("authorization value {0} != required amount {1} (exact scheme)")]
    ExactAmountMismatch(String, String),
    #[error("payload resource url {0:?} does not match the called route")]
    ResourceUrlMismatch(String),
    #[error("serviceName must be printable ASCII, max 32 chars")]
    BadServiceName,
    #[error("tags: max 5 entries, each printable ASCII max 32 chars")]
    BadTags,
    #[error("iconUrl must be http(s) and max 2048 chars")]
    BadIconUrl,
}

fn json_err(e: serde_json::Error) -> X402Error {
    X402Error::InvalidJson(e.to_string())
}

// ---------------------------------------------------------------------------
// §5.1 PaymentRequired (402 response, PAYMENT-REQUIRED header value)
// ---------------------------------------------------------------------------

/// ResourceInfo — §5.1.2. `url` required; optional fields validated on issue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceInfo {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(rename = "serviceName", skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(rename = "iconUrl", skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

impl ResourceInfo {
    pub fn validate(&self) -> Result<(), X402Error> {
        if let Some(name) = &self.service_name {
            if name.len() > 32 || !name.chars().all(|c| (0x20u32..=0x7e).contains(&(c as u32))) {
                return Err(X402Error::BadServiceName);
            }
        }
        if let Some(tags) = &self.tags {
            if tags.len() > 5
                || tags
                    .iter()
                    .any(|t| t.len() > 32 || !t.chars().all(|c| (0x20u32..=0x7e).contains(&(c as u32))))
            {
                return Err(X402Error::BadTags);
            }
        }
        if let Some(icon) = &self.icon_url {
            if icon.len() > 2048
                || !(icon.starts_with("https://") || icon.starts_with("http://"))
            {
                return Err(X402Error::BadIconUrl);
            }
        }
        Ok(())
    }
}

/// Extensions map — §5.1.2: extension id → {info, schema}. BTreeMap for
/// deterministic serialization.
pub type Extensions = BTreeMap<String, ExtensionData>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtensionData {
    pub info: serde_json::Value,
    pub schema: serde_json::Value,
}

impl ExtensionData {
    /// §5.1.2: both `info` and `schema` are wire type `object`.
    pub fn validate(&self) -> Result<(), X402Error> {
        if !self.info.is_object() || !self.schema.is_object() {
            return Err(X402Error::ReservedExtraKey("extensions {info,schema} objects"));
        }
        Ok(())
    }
}

/// PaymentRequirements — one entry of `accepts`. §5.1.2.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentRequirements {
    pub scheme: String,
    pub network: String,
    /// Decimal string in atomic units (parse with [`Self::amount_u256`]).
    pub amount: String,
    pub asset: String,
    #[serde(rename = "payTo")]
    pub pay_to: String,
    #[serde(rename = "maxTimeoutSeconds")]
    pub max_timeout_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

impl PaymentRequirements {
    /// Parse the wire amount string to U256. Digits only — no sign, no
    /// exponent, no decimal point (integer money; G10).
    pub fn amount_u256(&self) -> Result<U256, X402Error> {
        parse_amount(&self.amount)
    }

    pub fn asset_addr(&self) -> Result<Address, X402Error> {
        self.asset
            .parse()
            .map_err(|_| X402Error::BadAddress(self.asset.clone()))
    }

    pub fn pay_to_addr(&self) -> Result<Address, X402Error> {
        self.pay_to
            .parse()
            .map_err(|_| X402Error::BadAddress(self.pay_to.clone()))
    }

    /// Spec-level (mechanism-agnostic) validation: §5.1.2 allows ISO 4217
    /// `asset` codes and role-constant `payTo` — no EVM address parsing here.
    /// EVM/exact specifics live in [`Self::validate_issued`].
    pub fn validate_spec(&self) -> Result<(), X402Error> {
        if self.scheme.is_empty() {
            return Err(X402Error::BadScheme(self.scheme.clone()));
        }
        if !is_caip2(&self.network) {
            return Err(X402Error::BadNetwork(self.network.clone()));
        }
        self.amount_u256()?;
        if self.asset.is_empty() {
            return Err(X402Error::BadAddress(self.asset.clone()));
        }
        if self.pay_to.is_empty() {
            return Err(X402Error::BadAddress(self.pay_to.clone()));
        }
        // §5.1.2: extra is wire type `object` — reject scalar/array values.
        if let Some(x) = &self.extra {
            if !x.is_object() {
                return Err(X402Error::ReservedExtraKey("extra (must be object)"));
            }
        }
        Ok(())
    }

    /// OUR issuance rules (exact/EVM + §6.1 reserved keys + scheme-required
    /// domain parameters declared explicitly).
    pub fn validate_issued(&self) -> Result<(), X402Error> {
        self.validate_spec()?;
        if self.scheme != "exact" {
            return Err(X402Error::BadScheme(self.scheme.clone()));
        }
        self.asset_addr()?;
        self.pay_to_addr()?;
        // scheme_exact_evm.md:71-73 — for eip3009, extra.name/version are
        // REQUIRED (EIP-712 domain of the token contract; a client cannot
        // construct a valid signature without them). Blocks Stage 2 if absent.
        // §6.1 reserved keys — we always declare both (G9 amendment).
        let extra = self.extra.as_ref().ok_or(X402Error::ReservedExtraKey("extra"))?;
        let get = |k: &'static str| -> Result<&str, X402Error> {
            extra
                .get(k)
                .and_then(|v| v.as_str())
                .ok_or(X402Error::ReservedExtraKey(k))
        };
        if get("assetTransferMethod")? != "eip3009" {
            return Err(X402Error::ReservedExtraKey("assetTransferMethod"));
        }
        if get("paymentFlow")? != "upfront" {
            return Err(X402Error::ReservedExtraKey("paymentFlow"));
        }
        if get("name")?.is_empty() || get("version")?.is_empty() {
            return Err(X402Error::ReservedExtraKey("name/version (EIP-712 domain)"));
        }
        Ok(())
    }
}

/// PaymentRequired — the 402 body envelope; Base64-JSON in the
/// `PAYMENT-REQUIRED` header. §5.1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentRequired {
    #[serde(rename = "x402Version")]
    pub x402_version: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub resource: ResourceInfo,
    pub accepts: Vec<PaymentRequirements>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Extensions>,
}

impl PaymentRequired {
    /// Spec-level structural validation (§5.1): version, required fields.
    /// Accepts any spec-conformant object — including spec examples whose
    /// `extra` omits the §6.1 reserved keys (they resolve to mechanism
    /// defaults). Decode/roundtrip uses ONLY this.
    pub fn validate(&self) -> Result<(), X402Error> {
        if self.x402_version != X402_VERSION {
            return Err(X402Error::WrongVersion(self.x402_version));
        }
        self.resource.validate()?;
        if self.accepts.is_empty() {
            return Err(X402Error::MissingField("accepts"));
        }
        for a in &self.accepts {
            a.validate_spec()?;
        }
        if let Some(exts) = &self.extensions {
            for d in exts.values() {
                d.validate()?;
            }
        }
        Ok(())
    }

    /// OUR issuance rules — stricter than spec: exactly one requirement, both
    /// §6.1 reserved keys declared explicitly (G9 amendment; code402 never
    /// relies on mechanism defaults).
    pub fn validate_for_issue(&self) -> Result<(), X402Error> {
        self.validate()?;
        if self.accepts.len() != 1 {
            return Err(X402Error::MissingField("accepts (single)"));
        }
        self.accepts[0].validate_issued()?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// §5.2 PaymentPayload (client retry, PAYMENT-SIGNATURE header value)
// ---------------------------------------------------------------------------

/// EIP-3009 authorization — §5.2.2. All wire strings; typed accessors below.
/// validAfter/validBefore are STRINGS on the wire ("1740672089").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Authorization {
    pub from: String,
    pub to: String,
    /// Decimal string, atomic units.
    pub value: String,
    #[serde(rename = "validAfter")]
    pub valid_after: String,
    #[serde(rename = "validBefore")]
    pub valid_before: String,
    /// 0x + 64 hex chars.
    pub nonce: String,
}

impl Authorization {
    pub fn value_u256(&self) -> Result<U256, X402Error> {
        parse_amount(&self.value)
    }
    pub fn valid_after_unix(&self) -> Result<u64, X402Error> {
        parse_timestamp(&self.valid_after)
    }
    pub fn valid_before_unix(&self) -> Result<u64, X402Error> {
        parse_timestamp(&self.valid_before)
    }
    pub fn from_addr(&self) -> Result<Address, X402Error> {
        self.from
            .parse()
            .map_err(|_| X402Error::BadAddress(self.from.clone()))
    }
    pub fn to_addr(&self) -> Result<Address, X402Error> {
        self.to
            .parse()
            .map_err(|_| X402Error::BadAddress(self.to.clone()))
    }
    pub fn nonce_bytes(&self) -> Result<[u8; 32], X402Error> {
        let s = self.nonce.strip_prefix("0x").unwrap_or(&self.nonce);
        if s.len() != 64 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(X402Error::BadNonce(self.nonce.len()));
        }
        let mut out = [0u8; 32];
        for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
            let hi = (chunk[0] as char).to_digit(16).ok_or(X402Error::BadNonce(64))?;
            let lo = (chunk[1] as char).to_digit(16).ok_or(X402Error::BadNonce(64))?;
            out[i] = ((hi << 4) | lo) as u8;
        }
        Ok(out)
    }
}

/// Scheme-specific payload for exact/EVM: signature + authorization. §5.2.2.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExactEvmPayload {
    /// 0x + 130 hex chars (65 bytes, r||s||v).
    pub signature: String,
    pub authorization: Authorization,
}

/// PaymentPayload — Base64-JSON in the `PAYMENT-SIGNATURE` header. §5.2.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentPayload {
    #[serde(rename = "x402Version")]
    pub x402_version: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<ResourceInfo>,
    pub accepted: PaymentRequirements,
    pub payload: ExactEvmPayload,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Extensions>,
}

// ---------------------------------------------------------------------------
// §7 Facilitator request envelope — POST /v2/x402/verify and /settle body.
// The resource server forwards OUR issued requirement (never the client's
// echo — launch-checklist dependency, see reviews/launch-checklist.md).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FacilitatorRequest {
    #[serde(rename = "x402Version")]
    pub x402_version: u64,
    pub payment_payload: PaymentPayload,
    pub payment_requirements: PaymentRequirements,
}

impl FacilitatorRequest {
    pub fn new(
        payload: PaymentPayload,
        requirements: PaymentRequirements,
    ) -> Result<Self, X402Error> {
        requirements.validate_spec()?;
        Ok(Self { x402_version: X402_VERSION as u64, payment_payload: payload, payment_requirements: requirements })
    }
}

// ---------------------------------------------------------------------------
// §5.3 SettleResponse / §5.4 VerifyResponse (facilitator + PAYMENT-RESPONSE)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SettleResponse {
    pub success: bool,
    #[serde(rename = "errorReason", skip_serializing_if = "Option::is_none")]
    pub error_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payer: Option<String>,
    /// Empty string if no tx broadcast; MUST be non-empty when errorReason is
    /// `settlement_pending` (§5.3.2) — enforced by [`Self::validate`].
    pub transaction: String,
    pub network: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Extensions>,
}

impl SettleResponse {
    pub fn validate(&self) -> Result<(), X402Error> {
        if !is_caip2(&self.network) {
            return Err(X402Error::BadNetwork(self.network.clone()));
        }
        if self.success {
            if self.transaction.is_empty() {
                return Err(X402Error::MissingField("transaction"));
            }
            // errorReason is "omitted if successful" (§5.3.2)
            if self.error_reason.is_some() {
                return Err(X402Error::MissingField("errorReason-on-success"));
            }
        } else if self.error_reason.as_deref() == Some("settlement_pending")
            && self.transaction.is_empty()
        {
            return Err(X402Error::MissingField("transaction (settlement_pending)"));
        }
        if let Some(payer) = &self.payer {
            let _ = payer
                .parse::<Address>()
                .map_err(|_| X402Error::BadAddress(payer.clone()))?;
        }
        if let Some(amt) = &self.amount {
            parse_amount(amt)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerifyResponse {
    #[serde(rename = "isValid")]
    pub is_valid: bool,
    #[serde(rename = "invalidReason", skip_serializing_if = "Option::is_none")]
    pub invalid_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Codec — Base64(JSON) for the three payment headers. Size-capped; canonical
// base64 enforced on decode (re-encode must round-trip byte-identically).
// ---------------------------------------------------------------------------

pub fn encode_b64_json<T: Serialize>(v: &T) -> Result<String, X402Error> {
    let json = serde_json::to_string(v).map_err(json_err)?;
    let b64 = B64.encode(json.as_bytes());
    if b64.len() > MAX_HEADER_B64_BYTES {
        return Err(X402Error::HeaderTooLarge(b64.len()));
    }
    Ok(b64)
}

pub fn decode_b64_json<T: for<'de> Deserialize<'de>>(s: &str) -> Result<T, X402Error> {
    if s.len() > MAX_HEADER_B64_BYTES {
        return Err(X402Error::HeaderTooLarge(s.len()));
    }
    let raw = B64.decode(s).map_err(|_| X402Error::NotCanonicalBase64)?;
    // canonical check: re-encoding must be identical (rejects trailing bits,
    // alternate alphabets, embedded whitespace)
    if B64.encode(&raw) != s {
        return Err(X402Error::NotCanonicalBase64);
    }
    serde_json::from_slice(&raw).map_err(json_err)
}

pub fn encode_payment_required(v: &PaymentRequired) -> Result<String, X402Error> {
    // Encoding is for OUR issuance — apply the stricter issuance rules.
    v.validate_for_issue()?;
    encode_b64_json(v)
}
pub fn encode_settle_response(v: &SettleResponse) -> Result<String, X402Error> {
    v.validate()?;
    encode_b64_json(v)
}
pub fn decode_payment_payload(s: &str) -> Result<PaymentPayload, X402Error> {
    let p: PaymentPayload = decode_b64_json(s)?;
    if p.x402_version != X402_VERSION {
        return Err(X402Error::WrongVersion(p.x402_version));
    }
    Ok(p)
}

// ---------------------------------------------------------------------------
// G4 structural gate — everything cheap and local that must hold BEFORE any
// facilitator call. Crypto (ecrecover prefilter) layers on top in Stage 2/4.
// ---------------------------------------------------------------------------

/// Expected-context for the structural gate: what WE issued and when we're
/// checking. The echoed `accepted` is compared against `expected` field-by-
/// field (never trusted from the client).
pub struct StructuralContext<'a> {
    /// The requirement we stamped in PAYMENT-REQUIRED (MAC-verified upstream).
    pub expected: &'a PaymentRequirements,
    /// Absolute URL of the called route — REQUIRED (fail-closed: the route is
    /// always known to the resource server). Checked against payload.resource.
    pub route_url: &'a str,
    pub now_unix: u64,
}

pub fn structural_gate(p: &PaymentPayload, ctx: &StructuralContext) -> Result<(), X402Error> {
    if p.x402_version != X402_VERSION {
        return Err(X402Error::WrongVersion(p.x402_version));
    }
    // exact field match of the echo against what we issued (G4/G6).
    // maxTimeoutSeconds is compared too — a divergent echo is not what we
    // offered. `extra` is NOT compared here: it carries the G6 HMAC stamp,
    // verified upstream against the issued copy.
    let a = &p.accepted;
    let e = ctx.expected;
    if a.scheme != e.scheme || a.network != e.network || a.amount != e.amount
        || a.asset != e.asset || a.pay_to != e.pay_to
        || a.max_timeout_seconds != e.max_timeout_seconds
    {
        return Err(X402Error::ExactAmountMismatch(
            a.amount.clone(),
            e.amount.clone(),
        ));
    }
    let auth = &p.payload.authorization;
    // payer address must parse (invalid `from` never reaches the facilitator)
    auth.from_addr()?;
    // value must parse as integer money
    auth.value_u256()?;
    // nonce: 0x-prefixed, exactly 32 bytes (G10)
    if !auth.nonce.starts_with("0x") {
        return Err(X402Error::BadNonce(auth.nonce.len()));
    }
    auth.nonce_bytes()?;
    // signature shape: hex, AT LEAST 65 bytes. Exactly-65 is the plain EOA
    // case (eligible for the Stage-2 local ecrecover prefilter); LONGER
    // signatures are EIP-6492 smart-account envelopes — they PASS here and
    // skip straight to the facilitator, which verifies them (G4). Only
    // too-short, odd-length, or non-hex fails. 0x prefix is mandatory
    // (spec examples and every reference client emit it).
    let sig = match p.payload.signature.strip_prefix("0x") {
        Some(s) => s,
        None => return Err(X402Error::BadSignature(0)),
    };
    if sig.len() < 130 || sig.len() % 2 != 0 || !sig.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(X402Error::BadSignature(p.payload.signature.len()));
    }
    // G5: settle margin — validBefore ≥ now + margin
    let vb = auth.valid_before_unix()?;
    if vb < ctx.now_unix.saturating_add(SETTLE_MARGIN_SECONDS) {
        return Err(X402Error::ValidBeforeMargin(vb, ctx.now_unix));
    }
    // not-yet-valid authorizations must not burn a facilitator call
    let va = auth.valid_after_unix()?;
    if va > ctx.now_unix {
        return Err(X402Error::ValidBeforeMargin(va, ctx.now_unix));
    }
    // payload resource (when present) must itself be valid
    if let Some(res) = &p.resource {
        res.validate()?;
    }
    // exact means exact: value == amount (spec §exact; fixes legacy >=)
    if auth.value != e.amount {
        return Err(X402Error::ExactAmountMismatch(auth.value.clone(), e.amount.clone()));
    }
    // recipient binding
    if auth.to != e.pay_to {
        return Err(X402Error::BadAddress(auth.to.clone()));
    }
    // G9: payload.resource.url must match the called route when present
    if let Some(res) = &p.resource {
        if res.url != ctx.route_url {
            return Err(X402Error::ResourceUrlMismatch(res.url.clone()));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn parse_amount(s: &str) -> Result<U256, X402Error> {
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
        return Err(X402Error::AmountNotDecimal(s.to_string()));
    }
    s.parse::<U256>()
        .map_err(|_| X402Error::AmountOverflow(s.to_string()))
}

fn parse_timestamp(s: &str) -> Result<u64, X402Error> {
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
        return Err(X402Error::TimestampNotString(s.to_string()));
    }
    s.parse::<u64>()
        .map_err(|_| X402Error::TimestampNotString(s.to_string()))
}

/// CAIP-2 sanity: `namespace:reference`, both non-empty, no spaces.
/// (Full CAIP-2 validation is out of scope; this rejects the legacy
/// `{"chain_id":…, "name":"base"}` shape and obvious garbage.)
fn is_caip2(s: &str) -> bool {
    match s.split_once(':') {
        Some((ns, ref_)) => {
            !ns.is_empty()
                && !ref_.is_empty()
                && ns
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                && !s.contains(' ')
        }
        None => false,
    }
}
