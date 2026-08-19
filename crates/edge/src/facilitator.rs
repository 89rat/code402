//! Stage 4 facilitator client (edge): CDP `/v2/x402/verify` + `/settle` with
//! the Facilitator seam (design-logic §10 — CDP today, self-hosted or
//! secondary tomorrow). The exact CDP auth header is confirmed at key
//! provisioning (launch-checklist #4); it is env-driven here so wiring the
//! key is a secret change, not a code change.
//!
//! G8 economics: verify is always free; settle quota is the scarce resource
//! — the breaker (KV ops:facilitator_breaker) fails CLOSED.

use m2m_core::payment::x402v2::{
    FacilitatorRequest, SettleResponse, VerifyResponse,
};
use worker::*;

pub trait Facilitator {
    fn verify<'a>(&'a self, req: &'a FacilitatorRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<VerifyResponse, Error>> + 'a>>;
    fn settle<'a>(&'a self, req: &'a FacilitatorRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<SettleResponse, Error>> + 'a>>;
}

pub struct CdpFacilitator {
    base: String,
    keys: Vec<(String, String)>,
    next: std::sync::atomic::AtomicUsize,
}

impl CdpFacilitator {
    /// CDP_API_KEY secret format: "<key-id>:<secret-base64>".
    pub fn from_env(env: &Env) -> Result<Self> {
        let base = match env.var("CDP_FACILITATOR_BASE") {
            Ok(b) if !b.to_string().is_empty() => b.to_string(),
            _ => "https://api.cdp.coinbase.com/platform".to_string(),
        };
        let raw = env.secret("CDP_API_KEY")?.to_string();
        let keys: Vec<(String, String)> = raw
            .split(',')
            .filter_map(|p| p.split_once(':'))
            .map(|(i, s)| (i.trim().to_string(), s.trim().to_string()))
            .collect();
        if keys.is_empty() {
            return Err(Error::RustError("CDP_API_KEY must be id:secret[,id:secret...]".into()));
        }
        Ok(Self { base, keys, next: std::sync::atomic::AtomicUsize::new(0) })
    }

    /// Mint a 120s EdDSA JWT (docs.cdp.coinbase.com/api-reference/v2/
    /// authentication.md — verified live against /supported 2026-08-19).
    fn mint_jwt(&self, uri: &str) -> Result<String> {
        use base64::Engine;
        use ed25519_dalek::Signer;
        let (key_id, secret_b64) = {
            let i = self.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % self.keys.len();
            &self.keys[i]
        };
        let b64u = |b: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b);
        let kp_raw = base64::engine::general_purpose::STANDARD
            .decode(secret_b64)
            .map_err(|e| Error::RustError(format!("cdp secret b64: {e}")))?;
        if kp_raw.len() != 64 {
            return Err(Error::RustError("cdp secret must be 64 bytes (keypair)".into()));
        }
        let mut kp = [0u8; 64];
        kp.copy_from_slice(&kp_raw);
        let sk = ed25519_dalek::SigningKey::from_keypair_bytes(&kp)
            .map_err(|e| Error::RustError(format!("cdp key: {e}")))?;
        let now = Date::now().as_millis() / 1000;
        // nonce: uniqueness only (the docs do not constrain its form)
        let nonce = format!("n{now}-{}", &secret_b64[secret_b64.len().saturating_sub(8)..]);
        let header = serde_json::json!({"alg":"EdDSA","typ":"JWT","kid":key_id,"nonce":nonce});
        let claims = serde_json::json!({
            "sub": key_id, "iss": "cdp", "aud": ["cdp_service"],
            "nbf": now, "exp": now + 120, "uri": uri,
        });
        let signing = format!(
            "{}.{}",
            b64u(header.to_string().as_bytes()),
            b64u(claims.to_string().as_bytes())
        );
        let sig = sk.sign(signing.as_bytes());
        Ok(format!("{signing}.{}", b64u(&sig.to_bytes())))
    }

    async fn call<T: for<'de> serde::Deserialize<'de> + RejectionShape>(
        &self,
        path: &str,
        req: &FacilitatorRequest,
    ) -> std::result::Result<T, worker::Error> {
        let url = format!("{}{}", self.base.trim_end_matches('/'), path);
        let host = "api.cdp.coinbase.com";
        let uri_claim = format!("POST {host}/platform{path}");
        let jwt = self.mint_jwt(&uri_claim)?;
        let body = serde_json::to_string(req)
            .map_err(|e| Error::RustError(format!("facilitator encode: {e}")))?;
        let mut headers = Headers::new();
        headers.set("content-type", "application/json")?;
        headers.set("Authorization", &format!("Bearer {jwt}"))?;
        let mut init = RequestInit::new();
        init.with_method(Method::Post);
        init.with_headers(headers);
        init.with_body(Some(body.into()));
        let r = Request::new_with_init(&url, &init)?;
        let mut resp = Fetch::Request(r).send().await?;
        if resp.status_code() != 200 {
            let body = resp.text().await.unwrap_or_default();
            // Deterministic facilitator rejections (4xx + JSON errorType) are
            // OUTCOMES, not transport failures — a broke payer is a clean
            // insufficient_funds, never an ambiguous timeout. (Found by the
            // claims volley, C6.) Parse into the typed response; only
            // 5xx/undecodable bodies remain Err.
            if resp.status_code() >= 400 && resp.status_code() < 500 {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(et) = v.get("errorType").and_then(|x| x.as_str()) {
                        return self.reject_as_outcome(et, &v);
                    }
                    if let (Some(false), Some(_reason)) = (
                        v.get("success").and_then(|x| x.as_bool()),
                        v.get("errorReason"),
                    ) {
                        // already a settle-response shape; failures legally omit
                        // 'transaction' (empty = nothing broadcast, spec 5.3.2)
                        let mut vv = v.clone();
                        vv.as_object_mut().map(|o| o.entry("transaction").or_insert(serde_json::json!("")));
                        return serde_json::from_value(vv)
                            .map_err(|e| Error::RustError(format!("facilitator decode: {e}")));
                    }
                }
            }
            return Err(Error::RustError(format!(
                "facilitator {}: {}",
                resp.status_code(),
                body.chars().take(300).collect::<String>()
            )));
        }
        resp.json().await.map_err(|e| Error::RustError(format!("facilitator decode: {e}")))
    }
}

impl Facilitator for CdpFacilitator {
    fn verify<'a>(&'a self, req: &'a FacilitatorRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<VerifyResponse, Error>> + 'a>> {
        Box::pin(async move { self.call("/v2/x402/verify", req).await })
    }
    fn settle<'a>(&'a self, req: &'a FacilitatorRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<SettleResponse, Error>> + 'a>> {
        Box::pin(async move { self.call("/v2/x402/settle", req).await })
    }
}

/// Mock facilitator for dev/e2e and the failure-matrix fixtures. NEVER used
/// unless CDP_FACILITATOR_BASE is unset AND ops:mock_facilitator == "true"
/// (dev-only guard; production requires the explicit base URL).
pub struct MockFacilitator {
    pub verify_valid: bool,
    pub settle: MockSettle,
}

pub enum MockSettle {
    Success,
    AlreadyUsed,
    Reject(&'static str),
    Timeout5xx,
}

impl Facilitator for MockFacilitator {
    fn verify<'a>(&'a self, _req: &'a FacilitatorRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<VerifyResponse, Error>> + 'a>> {
        Box::pin(async move {
            Ok(VerifyResponse {
                is_valid: self.verify_valid,
                invalid_reason: if self.verify_valid { None } else { Some("insufficient_funds".into()) },
                payer: None,
                extra: None,
            })
        })
    }
    fn settle<'a>(&'a self, req: &'a FacilitatorRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<SettleResponse, Error>> + 'a>> {
        Box::pin(async move {
            match &self.settle {
                MockSettle::Success => Ok(SettleResponse {
                    success: true,
                    error_reason: None,
                    payer: Some(req.payment_payload.payload.authorization.from.clone()),
                    transaction: format!("0x{}", "c1".repeat(32)),
                    network: req.payment_requirements.network.clone(),
                    amount: Some(req.payment_requirements.amount.clone()),
                    extensions: None,
                }),
                MockSettle::AlreadyUsed => Ok(SettleResponse {
                    success: false,
                    error_reason: Some("invalid_exact_evm_payload_signature".into()),
                    payer: Some(req.payment_payload.payload.authorization.from.clone()),
                    transaction: String::new(),
                    network: req.payment_requirements.network.clone(),
                    amount: None,
                    extensions: None,
                }),
                MockSettle::Reject(r) => Ok(SettleResponse {
                    success: false,
                    error_reason: Some((*r).into()),
                    payer: None,
                    transaction: String::new(),
                    network: req.payment_requirements.network.clone(),
                    amount: None,
                    extensions: None,
                }),
                MockSettle::Timeout5xx => Err(Error::RustError("facilitator 5xx: simulated".into())),
            }
        })
    }
}

impl CdpFacilitator {
    /// Shape a facilitator 4xx JSON error into the typed outcome. The two
    /// call sites differ only in response type; we detect via the generic.
    fn reject_as_outcome<T: RejectionShape>(
        &self,
        error_type: &str,
        body: &serde_json::Value,
    ) -> std::result::Result<T, Error> {
        T::from_error_type(error_type, body)
    }
}

/// Ops/cron facilitator selection (RECONCILER-SPEC §3.C.3 re-drive): CDP when
/// configured; the dev mock ONLY behind the explicit KV opt-in (same guard as
/// the route). None => re-drive is skipped, never guessed.
pub async fn select_for_ops(env: &Env) -> Option<Box<dyn Facilitator>> {
    if let Ok(f) = CdpFacilitator::from_env(env) {
        return Some(Box::new(f));
    }
    if let Ok(kv) = env.kv("PRICING") {
        if matches!(kv.get("ops:mock_facilitator").text().await, Ok(Some(v)) if v == "true") {
            return Some(Box::new(MockFacilitator {
                verify_valid: true,
                settle: MockSettle::Success,
            }));
        }
    }
    None
}

/// Types that can be built from a facilitator errorType rejection.
pub trait RejectionShape: Sized {
    fn from_error_type(et: &str, body: &serde_json::Value) -> std::result::Result<Self, Error>;
}

impl RejectionShape for VerifyResponse {
    fn from_error_type(et: &str, _b: &serde_json::Value) -> std::result::Result<Self, Error> {
        Ok(VerifyResponse { is_valid: false, invalid_reason: Some(et.to_string()), payer: None, extra: None })
    }
}

impl RejectionShape for SettleResponse {
    fn from_error_type(et: &str, _b: &serde_json::Value) -> std::result::Result<Self, Error> {
        Ok(SettleResponse {
            success: false,
            error_reason: Some(et.to_string()),
            payer: None,
            transaction: String::new(),
            network: String::new(),
            amount: None,
            extensions: None,
        })
    }
}
