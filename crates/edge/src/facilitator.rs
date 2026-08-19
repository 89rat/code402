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
    api_key: String,
    auth_header: String, // e.g. "Authorization" or "x-api-key" — set from env
}

impl CdpFacilitator {
    pub fn from_env(env: &Env) -> Result<Self> {
        Ok(Self {
            base: env.var("CDP_FACILITATOR_BASE")?.to_string(),
            api_key: env.secret("CDP_API_KEY")?.to_string(),
            auth_header: env.var("CDP_AUTH_HEADER")?.to_string(),
        })
    }

    async fn call<T: for<'de> serde::Deserialize<'de>>(
        &self,
        path: &str,
        req: &FacilitatorRequest,
    ) -> std::result::Result<T, worker::Error> {
        let url = format!("{}{}", self.base.trim_end_matches('/'), path);
        let body = serde_json::to_string(req)
            .map_err(|e| Error::RustError(format!("facilitator encode: {e}")))?;
        let mut headers = Headers::new();
        headers.set("content-type", "application/json")?;
        headers.set(&self.auth_header, &self.api_key)?;
        let mut init = RequestInit::new();
        init.with_method(Method::Post);
        init.with_headers(headers);
        init.with_body(Some(body.into()));
        let r = Request::new_with_init(&url, &init)?;
        let mut resp = Fetch::Request(r).send().await?;
        if resp.status_code() >= 500 {
            return Err(Error::RustError(format!("facilitator 5xx: {}", resp.status_code())));
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
