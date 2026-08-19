//! Stage 3 — x402 v2 wire flow on the edge worker: POST /v2/tools/{tool}/call
//! 402 + PAYMENT-REQUIRED (HMAC-stamped, G6) -> PAYMENT-SIGNATURE retry ->
//! stamp verify -> structural gate -> EOA prefilter -> execute -> 200.
//! Settlement (facilitator /settle) is Stage 4; until then this route is
//! DARK in production (KV `ops:x402v2_enabled` absent => false, kill-switch
//! design reviews/kill-switch-design.md) and exercised only in dev/e2e.
//!
//! MAC rule (launch-checklist #9): stamps are computed over OUR canonical
//! serialization (serde struct order), never raw header bytes.

use crate::{err, execute_tool, validate_only, with_schema_header, append_event, sign_commitment, hex_decode, hex_encode, Receipt};
use crate::facilitator::{CdpFacilitator, Facilitator, MockFacilitator, MockSettle};
use m2m_core::payment::settlement::SettlementClaimMachine;
use m2m_core::payment::x402v2::FacilitatorRequest;
use m2m_core::payment::x402v2::{
    self, decode_payment_payload, encode_payment_required, ExtensionData, PaymentPayload,
    PaymentRequired, PaymentRequirements, ResourceInfo, StructuralContext, X402Error,
};
use m2m_core::payment::x402v2_errors::{map_error, Taxonomy};
use m2m_core::payment::x402v2_verify::{prefilter, VerifyOutcome};
use m2m_core::receipt::hash_json;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::BTreeMap;
use serde::Deserialize;
use worker::*;

pub const STAMP_EXT_ID: &str = "code402.stamp";
const EXPOSE: &str = "PAYMENT-REQUIRED, PAYMENT-SIGNATURE, PAYMENT-RESPONSE";

type HmacSha256 = Hmac<Sha256>;

fn v2_err(t: Taxonomy, status: u16, msg: &str) -> Result<Response> {
    let retryable = status >= 500;
    let mut r = Response::from_json(&serde_json::json!({
        "error": {"code": t.as_str(), "message": msg, "retryable": retryable}
    }))?;
    r = r.with_status(status);
    let mut r = with_schema_header(r)?;
    let h = r.headers_mut();
    h.set("Access-Control-Expose-Headers", EXPOSE)?;
    h.set("Access-Control-Allow-Origin", "*")?;
    Ok(r)
}

/// 402-class error WITH a freshly-issued PAYMENT-REQUIRED so conforming
/// clients can retry-with-payment (http.md recovery; Kimi S3 major #3).
async fn v2_payment_error(env: &Env, route_url: &str, tool: &str, amount: u64, t: Taxonomy, msg: &str) -> Result<Response> {
    let mut ch = challenge(env, route_url, tool, amount).await?;
    let body = serde_json::json!({
        "error": {"code": t.as_str(), "message": msg, "retryable": false}
    });
    let mut r = Response::from_json(&body)?.with_status(402);
    let pr = ch.headers_mut().get("PAYMENT-REQUIRED")?;
    let mut r = with_schema_header(r)?;
    if let Some(pr) = pr {
        r.headers_mut().set("PAYMENT-REQUIRED", &pr)?;
    }
    r.headers_mut().set("Access-Control-Expose-Headers", EXPOSE)?;
    r.headers_mut().set("Access-Control-Allow-Origin", "*")?;
    Ok(r)
}

fn cors(r: Response) -> Result<Response> {
    let mut r = r;
    r.headers_mut().set("Access-Control-Expose-Headers", EXPOSE)?;
    r.headers_mut().set("Access-Control-Allow-Origin", "*")?;
    Ok(r)
}

/// Constant-time equality (design-logic §11: MAC comparison is constant-time).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut acc = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        acc |= x ^ y;
    }
    acc == 0
}

fn canonical_requirement(req: &PaymentRequirements) -> Result<String> {
    serde_json::to_string(req).map_err(|_| Error::RustError("canonical serialize".into()))
}

fn stamp_mac(key: &[u8], canonical: &str, iat: u64) -> Result<Vec<u8>> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|_| Error::RustError("V2_STAMP_KEY bad".into()))?;
    mac.update(canonical.as_bytes());
    mac.update(b"|");
    mac.update(iat.to_string().as_bytes());
    Ok(mac.finalize().into_bytes().to_vec())
}

async fn v2_enabled(env: &Env) -> bool {
    // Fail closed on any KV read failure (kill-switch design).
    match env.kv("PRICING") {
        Ok(kv) => matches!(kv.get("ops:x402v2_enabled").text().await, Ok(Some(v)) if v == "true"),
        Err(_) => false,
    }
}

#[derive(Deserialize)]
struct CallRequest {
    input: serde_json::Value,
    idempotency_key: Option<String>,
}

pub async fn handle(req: &mut Request, env: &Env, tool: &str) -> Result<Response> {
    // CORS preflight FIRST (G10: Allow-Headers not just Expose)
    if req.method() == Method::Options {
        let mut r = Response::empty()?.with_status(204);
        let h = r.headers_mut();
        h.set("Access-Control-Allow-Origin", "*")?;
        h.set("Access-Control-Allow-Methods", "POST, OPTIONS")?;
        h.set("Access-Control-Allow-Headers", "PAYMENT-SIGNATURE, Content-Type")?;
        h.set("Access-Control-Max-Age", "86400")?;
        return Ok(r);
    }
    if req.method() != Method::Post && req.method() != Method::Options {
        return v2_err(Taxonomy::InvalidPayload, 400, "route must be POST /v2/tools/{tool}/call");
    }
    if !v2_enabled(env).await {
        // dark in production until Stage 4 + flip; indistinguishable from absent
        return err("NOT_FOUND", 404, "unknown route");
    }
    if !crate::tool_known(tool) {
        return v2_err(Taxonomy::InvalidPayload, 400, "unknown tool");
    }

    // duplicate/case-variant payment headers -> 400 (G10). The Workers
    // runtime MERGES duplicate non-set-cookie headers with ", " (getAll is
    // Set-Cookie-only); canonical base64 never contains a comma, so a comma
    // join is proof of duplication (or of a hostile value that fails decode
    // a line later — either way 4xx, never a settle).
    let sig_merged = req.headers().get("payment-signature")?;
    if let Some(v) = &sig_merged {
        if v.contains(", ") {
            return v2_err(Taxonomy::InvalidPayload, 400, "duplicate PAYMENT-SIGNATURE header");
        }
    }

    let request_id = req
        .headers()
        .get("cf-ray")?
        .unwrap_or_else(|| format!("req-{}", Date::now().as_millis()));
    let route_url = req.url()?.to_string();
    let body: CallRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return v2_err(Taxonomy::InvalidPayload, 400, "body must be JSON {input, idempotency_key?}"),
    };
    if let Err(m) = validate_only(tool, &body.input) {
        return v2_err(Taxonomy::InvalidPayload, 400, m);
    }
    // idempotency pre-check BEFORE execution (parity with /v1; the only
    // client-side mitigation for the accepted replay-within-grace window)
    if let Some(key) = &body.idempotency_key {
        if let Ok(db) = env.d1("LEDGER") {
            if let Ok(Some(row)) = db
                .prepare("SELECT response_ref FROM idempotency WHERE idem_key = ?1")
                .bind(&[key.clone().into()])?
                .first::<serde_json::Value>(None)
                .await
            {
                let mut r = Response::from_json(&serde_json::json!({
                    "idempotent_replay": true, "receipt_ref": row["response_ref"],
                }))?;
                r.headers_mut().set("X-Schema-Version", "2.0")?;
                return Ok(cors(r)?);
            }
        }
    }

    // pricing from KV (route-derived truth; echo is never trusted)
    let pricing = env.kv("PRICING")?;
    let price_raw = pricing.get(tool).text().await?;
    let price_raw = match price_raw {
        Some(p) => p,
        None => return v2_err(Taxonomy::InvalidPaymentRequirements, 500, "pricing missing"),
    };
    let price: serde_json::Value = serde_json::from_str(&price_raw)
        .map_err(|_| Error::RustError("pricing entry malformed".into()))?;
    // I5 fail-closed (red team Break 1): malformed/missing money config
    // must NEVER default — no challenge is issued on bad pricing.
    let amount = price
        .get("amount_minor")
        .and_then(|v| v.as_u64())
        .filter(|v| *v > 0)
        .ok_or_else(|| Error::RustError("pricing amount_minor missing/invalid".into()))?;

    let sig = match sig_merged {
        None => return challenge(env, &route_url, tool, amount).await,
        Some(s) => s,
    };

    // ---- inbound payment: decode -> stamp verify -> gate -> prefilter ----
    let payload: PaymentPayload = match decode_payment_payload(&sig) {
        Ok(p) => p,
        Err(e) => return v2_err(map_error(&e), 400, "PAYMENT-SIGNATURE decode failed"),
    };

    // G6 stamp check: the echoed extensions must carry our stamp; MAC is
    // recomputed over the ECHOED requirement's canonical form (any field
    // tamper breaks it), within the requirement's own grace window.
    let bad_stamp = || v2_err(Taxonomy::InvalidPaymentRequirements, 400, "missing or malformed requirement stamp");
    let stamp = match payload.extensions.as_ref().and_then(|e| e.get(STAMP_EXT_ID)) {
        Some(s) => s.clone(),
        None => return bad_stamp(),
    };
    let info = stamp.info;
    let mac_hex = match info.get("mac").and_then(|v| v.as_str()) {
        Some(m) => m.to_string(),
        None => return bad_stamp(),
    };
    let iat = match info.get("iat").and_then(|v| v.as_u64()) {
        Some(i) => i,
        None => return bad_stamp(),
    };
    let key_hex = env.secret("V2_STAMP_KEY")?.to_string();
    let key_bytes = crate::hex_decode(&key_hex)
        .map_err(|_| Error::RustError("V2_STAMP_KEY not hex".into()))?;
    let now = Date::now().as_millis() / 1000;
    let grace = payload.accepted.max_timeout_seconds;
    if iat > now + 60 || iat + grace < now {
        return v2_err(Taxonomy::InvalidPaymentRequirements, 400, "stamped requirement outside grace window");
    }
    let canonical = canonical_requirement(&payload.accepted)?;
    let route_bound = format!("{canonical}|{route_url}");
    let expect_mac = stamp_mac(&key_bytes, &route_bound, iat)?;
    let got_mac = match crate::hex_decode(&mac_hex) {
        Ok(m) => m,
        Err(_) => return bad_stamp(),
    };
    if !ct_eq(&expect_mac, &got_mac) {
        return v2_err(Taxonomy::InvalidPaymentRequirements, 400, "requirement stamp mismatch (G6)");
    }

    // MAC proves the echoed requirement is byte-ours -> it IS the expected
    // requirement (G6: verify against the stamped copy, not live config).
    let expected = payload.accepted.clone();
    let ctx = StructuralContext { expected: &expected, route_url: &route_url, now_unix: now };
    if let Err(e) = x402v2::structural_gate(&payload, &ctx) {
        let t = map_error(&e);
        // vendored http.md: no 401 exists; invalid payment => 400, timing
        // failures => 402 WITH fresh PAYMENT-REQUIRED (recovery path)
        if matches!(t, Taxonomy::InvalidValidAfter | Taxonomy::InvalidValidBefore) {
            return v2_payment_error(env, &route_url, tool, amount, t, "structural gate: timing").await;
        }
        return v2_err(t, 400, "structural gate");
    }

    match prefilter(&payload, &expected) {
        VerifyOutcome::LocalReject(e) => {
            v2_err(map_error(&e), 400, "local verification failed")
        }
        VerifyOutcome::PassThrough | VerifyOutcome::LocalPass { .. } => {
            settle_and_serve(env, &request_id, tool, &body, amount, &payload, &expected).await
        }
    }
}

async fn challenge(env: &Env, route_url: &str, tool: &str, amount_minor: u64) -> Result<Response> {
    // I5 fail-closed (red team Break 4): an invalid CHAIN_ID must never
    // silently default to Base MAINNET.
    let chain_id: u64 = env
        .var("CHAIN_ID")?
        .to_string()
        .parse()
        .map_err(|_| Error::RustError("CHAIN_ID var invalid".into()))?;
    let asset = env.var("USDC_BASE")?.to_string();
    let pay_to = env.secret("COMPANY_WALLET")?.to_string();
    let name = env.var("TOKEN_NAME")?.to_string();
    let version = env.var("TOKEN_VERSION")?.to_string();
    let req = PaymentRequirements {
        scheme: "exact".into(),
        network: format!("eip155:{chain_id}"),
        amount: amount_minor.to_string(),
        asset,
        pay_to,
        max_timeout_seconds: 300,
        extra: Some(serde_json::json!({
            "name": name, "version": version,
            "assetTransferMethod": "eip3009", "paymentFlow": "upfront",
        })),
    };
    let now = Date::now().as_millis() / 1000;
    let canonical = canonical_requirement(&req)?;
    let key_hex = env.secret("V2_STAMP_KEY")?.to_string();
    let key_bytes = crate::hex_decode(&key_hex)
        .map_err(|_| Error::RustError("V2_STAMP_KEY not hex".into()))?;
    let route_bound = format!("{canonical}|{route_url}");
    let mac = stamp_mac(&key_bytes, &route_bound, now)?;
    let mut exts: BTreeMap<String, ExtensionData> = BTreeMap::new();
    exts.insert(
        STAMP_EXT_ID.into(),
        ExtensionData {
            info: serde_json::json!({
                "mac": format!("0x{}", hex_encode(&mac)),
                "iat": now,
            }),
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "mac": {"type": "string", "pattern": "^0x[0-9a-f]{64}$"},
                    "iat": {"type": "integer"}
                },
                "required": ["mac", "iat"]
            }),
        },
    );
    let pr = PaymentRequired {
        x402_version: 2,
        error: Some("PAYMENT-SIGNATURE header is required".into()),
        resource: ResourceInfo {
            url: route_url.to_string(),
            description: Some(format!("code402 paid tool: {tool}")),
            mime_type: Some("application/json".into()),
            service_name: Some("code402".into()),
            tags: None,
            icon_url: None,
        },
        accepts: vec![req],
        extensions: Some(exts),
    };
    let b64 = encode_payment_required(&pr)
        .map_err(|e| Error::RustError(format!("encode 402: {e:?}")))?;
    let mut r = Response::from_json(&serde_json::json!({
        "error": {"code": "PAYMENT_REQUIRED", "message": "see PAYMENT-REQUIRED header", "retryable": false}
    }))?;
    r = r.with_status(402);
    let mut r = with_schema_header(r)?;
    {
        let h = r.headers_mut();
        h.set("PAYMENT-REQUIRED", &b64)?;
        h.set("Access-Control-Expose-Headers", EXPOSE)?;
    }
    let _ = append_event(env, &format!("req-{now}-v2ch"), tool, None, amount_minor, "V2_CHALLENGED", None).await;
    Ok(r)
}

async fn serve(env: &Env, request_id: &str, tool: &str, body: &CallRequest, stamped_amount: u64) -> Result<Response> {
    // Stage 3: verified-serve. Stage 4 inserts facilitator /verify + /settle
    // BEFORE this point (settle-before-serve, I1). The route is KV-gated
    // dark in production until then.
    let output = match execute_tool(tool, &body.input) {
        Ok(o) => o,
        Err(m) => return v2_err(Taxonomy::UnexpectedVerifyError, 500, m),
    };
    let _ = append_event(env, request_id, tool, None, stamped_amount, "V2_VERIFIED_SETTLE_PENDING", None).await;

    let receipt = Receipt {
        request_id: request_id.to_string(),
        tool: tool.to_string(),
        tool_version: "1.0.0".into(),
        input_hash: hash_json(&body.input),
        output_hash: hash_json(&output),
        timestamp_unix: Date::now().as_millis() / 1000,
    };
    let commitment = receipt.commitment();
    let sig_hex = sign_commitment(env, &commitment)?;
    let receipt_doc = serde_json::json!({
        "receipt": receipt, "commitment": hex_encode(commitment.as_slice()), "signature": sig_hex,
    });
    let bucket = env.bucket("RECEIPTS")?;
    let r2_key = format!("receipts/{request_id}.json");
    bucket.put(&r2_key, receipt_doc.to_string()).execute().await?;
    if let Some(key) = &body.idempotency_key {
        let _ = env
            .d1("LEDGER")?
            .prepare("INSERT OR IGNORE INTO idempotency(idem_key, request_id, response_ref, created_at) VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ','now'))")
            .bind(&[key.clone().into(), request_id.to_string().into(), r2_key.into()])?
            .run()
            .await;
    }
    let mut r = Response::from_json(&serde_json::json!({"output": output, "receipt": receipt_doc}))?;
    r.headers_mut().set("X-Schema-Version", "2.0")?;
    // PAYMENT-RESPONSE lands with Stage 4 settlement (SettleResponse carries
    // the required transaction); verified-serve is Stage-3 semantics.
    Ok(cors(r)?)
}

// ---------------------------------------------------------------------------
// Stage 4: settle-before-serve (I1). verify -> claim (DO) -> settle ->
// execute -> persist -> respond with PAYMENT-RESPONSE.
// ---------------------------------------------------------------------------

fn do_cmd_url(key: &str) -> String {
    "https://do/cmd".to_string()
}

async fn claim_do(env: &Env, key: &str, cmd_json: serde_json::Value) -> Result<serde_json::Value> {
    let ns = env.durable_object("SETTLEMENT_CLAIM")?;
    // instance id derived from the claim key (hash(from||nonce))
    let id = ns.id_from_name(key)?;
    let stub = id.get_stub()?;
    let mut headers = Headers::new();
    headers.set("content-type", "application/json")?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post);
    init.with_headers(headers);
    init.with_body(Some(cmd_json.to_string().into()));
    let req = Request::new_with_init(&do_cmd_url(key), &init)?;
    let mut resp = stub.fetch_with_request(req).await?;
    if resp.status_code() != 200 {
        let e = resp.text().await.unwrap_or_default();
        return Err(Error::RustError(format!("claim DO: {e}")));
    }
    resp.json().await
}

async fn breaker_open(env: &Env) -> bool {
    match env.kv("PRICING") {
        // fail CLOSED on read failure (kill-switch design)
        Err(_) => true,
        Ok(kv) => matches!(kv.get("ops:facilitator_breaker").text().await, Ok(Some(v)) if v == "open"),
    }
}

fn facilitator_from_env(env: &Env) -> Option<Box<dyn Facilitator>> {
    // production requires the explicit base URL; the mock is dev-only and
    // requires BOTH no base AND the explicit KV opt-in
    if let Ok(f) = CdpFacilitator::from_env(env) {
        if !f_is_empty(&f) {
            return Some(Box::new(f));
        }
    }
    None
}

fn f_is_empty(_f: &CdpFacilitator) -> bool { false }

async fn mock_facilitator_allowed(env: &Env) -> bool {
    match env.kv("PRICING") {
        Ok(kv) => matches!(kv.get("ops:mock_facilitator").text().await, Ok(Some(v)) if v == "true"),
        Err(_) => false,
    }
}

#[allow(clippy::too_many_arguments)]
async fn settle_and_serve(
    env: &Env,
    request_id: &str,
    tool: &str,
    body: &CallRequest,
    amount: u64,
    payload: &m2m_core::payment::x402v2::PaymentPayload,
    expected: &PaymentRequirements,
) -> Result<Response> {
    use m2m_core::payment::x402v2::{encode_settle_response, SettleResponse};

    // I5: fail closed on money
    if breaker_open(env).await {
        return v2_err(Taxonomy::UnexpectedVerifyError, 503, "facilitator circuit breaker open (fail closed); retryable");
    }

    // facilitator selection (trait seam)
    let facilitator: Box<dyn Facilitator> = match facilitator_from_env(env) {
        Some(f) => f,
        None => {
            if mock_facilitator_allowed(env).await {
                Box::new(MockFacilitator { verify_valid: true, settle: MockSettle::Success })
            } else {
                return v2_err(Taxonomy::UnexpectedVerifyError, 503, "no facilitator configured (dev: set ops:mock_facilitator=true); retryable");
            }
        }
    };

    // G2: FULL request validation happened before the 402 was issued; the
    // body was re-validated on entry — nothing rejectable reaches a settle.

    // claim FIRST (idempotent replay short-circuits before any facilitator call)
    let auth = &payload.payload.authorization;
    let key = SettlementClaimMachine::key_for(&auth.from, &auth.nonce);
    let now = Date::now().as_millis() / 1000;
    let input_hash = {
        let h = hash_json(&body.input);
        hex_encode(h.as_slice())
    };
    let claim = claim_do(env, &key, serde_json::json!({
        "cmd": "claim", "key": key,
        "input": {
            "payer": auth.from, "nonce": auth.nonce,
            "request_id": request_id, "tool": tool,
            "input_hash": input_hash, "now_unix": now,
        }
    })).await?;
    match claim.get("kind").and_then(|k| k.as_str()).unwrap_or_default() {
        "replay" => {
            let b64body = claim.get("response_body_b64").and_then(|b| b.as_str()).unwrap_or_default();
            let pr = claim.get("payment_response").and_then(|p| p.as_str()).unwrap_or_default();
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(b64body)
                .map_err(|_| Error::RustError("replay body b64".into()))?;
            let mut r = Response::from_bytes(bytes)?;
            r.headers_mut().set("Content-Type", "application/json")?;
            r.headers_mut().set("X-Schema-Version", "2.0")?;
            if !pr.is_empty() {
                r.headers_mut().set("PAYMENT-RESPONSE", pr)?;
            }
            return Ok(cors(r)?);
        }
        "in_progress" => {
            return v2_err(Taxonomy::UnexpectedVerifyError, 503, "settlement in progress by concurrent request; retry shortly");
        }
        "terminal" => {
            return v2_err(Taxonomy::InvalidPayload, 400, "authorization already consumed");
        }
        "claimed" | "lease_expired" => {} // we hold the claim
        other => return v2_err(Taxonomy::UnexpectedVerifyError, 500, &format!("claim kind {other:?}")),
    }

    // facilitator /verify (always free per CDP economics)
    let freq = FacilitatorRequest::new(payload.clone(), expected.clone())
        .map_err(|e| Error::RustError(format!("facilitator request: {e:?}")))?;
    let verify = match facilitator.verify(&freq).await {
        Ok(v) => v,
        Err(_) => {
            return v2_err(Taxonomy::UnexpectedVerifyError, 503, "facilitator verify unavailable; retryable");
        }
    };
    if !verify.is_valid {
        let reason = verify.invalid_reason.clone().unwrap_or_else(|| "invalid_payment_requirements".into());
        return v2_err(Taxonomy::InvalidPaymentRequirements, 400, &format!("facilitator rejected: {reason}"));
    }

    // begin settle (lease refresh)
    let _ = claim_do(env, &key, serde_json::json!({"cmd": "begin_settle", "key": key, "now": now})).await?;

    // /settle — the money moment
    let settle_out = facilitator.settle(&freq).await;
    match settle_out {
        Ok(sr) if sr.success => {
            let output = match execute_tool(tool, &body.input) {
                Ok(o) => o,
                Err(m) => {
                    // money taken, tool failed: G2c — bounded free re-execution
                    // window is handled by replay (stored failure) in Stage 4
                    // hardening; record + report honestly
                    let _ = claim_do(env, &key, serde_json::json!({"cmd": "failed", "key": key, "reason": "TOOL_INTERNAL_ERROR"})).await;
                    return v2_err(Taxonomy::UnexpectedVerifyError, 500, m);
                }
            };
            let pr_b64 = encode_settle_response(&sr)
                .map_err(|e| Error::RustError(format!("encode PAYMENT-RESPONSE: {e:?}")))?;
            let body_bytes = serde_json::to_vec(&serde_json::json!({"output": output}))
                .map_err(|e| Error::RustError(format!("body: {e}")))?;
            let body_b64 = {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(&body_bytes)
            };
            let _ = claim_do(env, &key, serde_json::json!({
                "cmd": "settled", "key": key,
                "tx": sr.transaction, "network": sr.network,
                "response_body_b64": body_b64, "payment_response": pr_b64,
            })).await?;
            // durable record (0002 settlements) — best effort; the DO is the
            // claim authority, D1 the reconciliation record
            let _ = d1_record_settlement(env, &key, &auth, request_id, tool, &input_hash, expected, payload, &sr, &pr_b64).await;
            let _ = append_event(env, request_id, tool, Some(&sr.transaction), amount, "V2_SETTLED", None).await;
            let receipt = Receipt {
                request_id: request_id.to_string(),
                tool: tool.to_string(),
                tool_version: "1.0.0".into(),
                input_hash: hash_json(&body.input),
                output_hash: hash_json(&output),
                timestamp_unix: Date::now().as_millis() / 1000,
            };
            let commitment = receipt.commitment();
            let sig_hex = sign_commitment(env, &commitment)?;
            let mut r = Response::from_json(&serde_json::json!({
                "output": output,
                "receipt": {"receipt": receipt, "commitment": hex_encode(commitment.as_slice()), "signature": sig_hex},
                "settlement": {"transaction": sr.transaction, "network": sr.network},
            }))?;
            r.headers_mut().set("X-Schema-Version", "2.0")?;
            r.headers_mut().set("PAYMENT-RESPONSE", &pr_b64)?;
            Ok(cors(r)?)
        }
        Ok(sr) => {
            let reason = sr.error_reason.clone().unwrap_or_else(|| "unexpected_settle_error".into());
            if reason == "invalid_exact_evm_payload_signature" && sr.transaction.is_empty() {
                let _ = claim_do(env, &key, serde_json::json!({"cmd": "receipt_pending", "key": key})).await;
                return v2_err(Taxonomy::SettlementPending, 503, "authorization already used; reconciliation pending; retryable");
            }
            let _ = claim_do(env, &key, serde_json::json!({"cmd": "failed", "key": key, "reason": &reason})).await;
            let t = if reason == "insufficient_funds" { Taxonomy::InsufficientFunds } else { Taxonomy::UnexpectedSettleError };
            v2_err(t, 400, &format!("settlement failed: {reason}"))
        }
        Err(_) => {
            v2_err(Taxonomy::SettlementPending, 503, "settle outcome unknown (timeout); lease recovers the claim; retryable")
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn d1_record_settlement(
    env: &Env,
    key: &str,
    auth: &m2m_core::payment::x402v2::Authorization,
    request_id: &str,
    tool: &str,
    input_hash: &str,
    expected: &PaymentRequirements,
    payload: &m2m_core::payment::x402v2::PaymentPayload,
    sr: &m2m_core::payment::x402v2::SettleResponse,
    pr_b64: &str,
) -> Result<()> {
    use worker::wasm_bindgen::JsValue;
    let payload_json = serde_json::to_string(payload)
        .map_err(|e| Error::RustError(format!("payload: {e}")))?;
    let db = env.d1("LEDGER")?;
    db.prepare(
        "INSERT OR IGNORE INTO settlements(id, payer, nonce, request_id, tool, input_hash, status, scheme, network, asset, amount, pay_to, payment_payload, tx_hash, settle_network, payment_response) VALUES (?1,?2,?3,?4,?5,?6,'settled','exact',?7,?8,?9,?10,?11,?12,?13,?14)"
    ).bind(&[
        JsValue::from_str(key),
        JsValue::from_str(&auth.from),
        JsValue::from_str(&auth.nonce),
        JsValue::from_str(request_id),
        JsValue::from_str(tool),
        JsValue::from_str(input_hash),
        JsValue::from_str(&expected.network),
        JsValue::from_str(&expected.asset),
        JsValue::from_str(&expected.amount),
        JsValue::from_str(&expected.pay_to),
        JsValue::from_str(&payload_json),
        JsValue::from_str(&sr.transaction),
        JsValue::from_str(&sr.network),
        JsValue::from_str(pr_b64),
    ])?.run().await?;
    Ok(())
}
