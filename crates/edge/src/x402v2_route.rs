//! Stage 3 — x402 v2 wire flow on the edge worker: POST /v2/tools/{tool}/call
//! 402 + PAYMENT-REQUIRED (HMAC-stamped, G6) -> PAYMENT-SIGNATURE retry ->
//! stamp verify -> structural gate -> EOA prefilter -> execute -> 200.
//! Settlement (facilitator /settle) is Stage 4; until then this route is
//! DARK in production (KV `ops:x402v2_enabled` absent => false, kill-switch
//! design reviews/kill-switch-design.md) and exercised only in dev/e2e.
//!
//! MAC rule (launch-checklist #9): stamps are computed over OUR canonical
//! serialization (serde struct order), never raw header bytes.

use crate::{err, execute_tool, validate_only, with_schema_header, append_event, sign_commitment, hex_encode, Receipt};
use crate::facilitator::{CdpFacilitator, Facilitator, MockFacilitator, MockSettle};
use m2m_core::payment::settlement::SettlementClaimMachine;
use m2m_core::payment::x402v2::FacilitatorRequest;
use m2m_core::payment::x402v2::{
    self, decode_payment_payload, encode_payment_required, ExtensionData, PaymentPayload,
    PaymentRequired, PaymentRequirements, ResourceInfo, StructuralContext,
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
const EXPOSE: &str = "PAYMENT-REQUIRED, PAYMENT-SIGNATURE, PAYMENT-RESPONSE, X-Schema-Version";

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
    // cache hygiene: payment-negotiated responses must never be stored by
    // intermediaries (stamps are iat-fresh per issuance)
    h.set("Cache-Control", "private, no-store")?;
    Ok(r)
}

/// 402-class error WITH a freshly-issued PAYMENT-REQUIRED so conforming
/// clients can retry-with-payment (http.md recovery; Kimi S3 major #3).
async fn v2_payment_error(env: &Env, route_url: &str, tool: &str, amount: u64, t: Taxonomy, msg: &str) -> Result<Response> {
    let mut ch = challenge(env, route_url, tool, amount).await?;
    let body = serde_json::json!({
        "error": {"code": t.as_str(), "message": msg, "retryable": false}
    });
    let r = Response::from_json(&body)?.with_status(402);
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
    r.headers_mut().set("Cache-Control", "private, no-store")?;
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

pub(crate) async fn v2_enabled(env: &Env) -> bool {
    // Fail closed on any KV read failure (kill-switch design). Also the
    // reconciler's re-drive gate (RECONCILER-SPEC §3.C.3).
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
    // Entitlement TTL (24h) exceeds stamp grace (300s) BY DESIGN (G2 +
    // RECONCILER-SPEC §3.C.1): a payment whose nonce holds a live
    // reconciler entitlement skips the AGE gate only — the MAC still binds
    // the echoed requirement, so tampering stays fatal. D1 unavailability is
    // NOT a rejection (Kimi m9): unknown entitlement => retryable 503.
    let entitled = d1_entitled(env, &payload.payload.authorization.from, &payload.payload.authorization.nonce, now).await;
    if iat > now + 60 || (iat + grace < now && entitled != Some(true)) {
        if entitled.is_none() {
            return v2_err(Taxonomy::SettlementPending, 503, "entitlement check unavailable; retry");
        }
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

    let outcome = prefilter(&payload, &expected);
    match outcome {
        VerifyOutcome::LocalReject(e) => {
            v2_err(map_error(&e), 400, "local verification failed")
        }
        VerifyOutcome::PassThrough | VerifyOutcome::LocalPass { .. } => {
            let verified_locally = matches!(outcome, VerifyOutcome::LocalPass { .. });
            settle_and_serve(env, &request_id, tool, &body, amount, &payload, &expected, verified_locally).await
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

// ---------------------------------------------------------------------------
// Stage 4: settle-before-serve (I1). verify -> claim (DO) -> settle ->
// execute -> persist -> respond with PAYMENT-RESPONSE.
// ---------------------------------------------------------------------------

fn do_cmd_url(_key: &str) -> String {
    "https://do/cmd".to_string()
}

/// One JSON command round-trip to a SettlementClaim DO instance. Shared with
/// the reconciler sweep's write-back (the DO is the claim authority).
pub(crate) async fn claim_do(env: &Env, key: &str, cmd_json: serde_json::Value) -> Result<serde_json::Value> {
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

/// M1 helper: per-payer per-minute cap on facilitator-bound PassThrough calls
/// (limit 10/min). Fail CLOSED on any KV error — this guards quota, and a
/// false rejection is a retryable 503, never a money error.
async fn passthrough_rate_limited(env: &Env, payer: &str) -> bool {
    let kv = match env.kv("PRICING") {
        Ok(kv) => kv,
        Err(_) => return true,
    };
    let bucket = Date::now().as_millis() / 60_000;
    let k = format!("ops:pt:{payer}:{bucket}");
    let n: u64 = match kv.get(&k).text().await {
        Ok(Some(v)) => v.parse().unwrap_or(0),
        Ok(None) => 0,
        Err(_) => return true,
    };
    if n >= 10 {
        return true;
    }
    if let Ok(p) = kv.put(&k, (n + 1).to_string()) {
        let _ = p.expiration_ttl(120).execute().await;
    }
    false
}

pub(crate) async fn breaker_open(env: &Env) -> bool {
    match env.kv("PRICING") {
        // fail CLOSED on read failure (kill-switch design)
        Err(_) => true,
        Ok(kv) => {
            if matches!(kv.get("ops:facilitator_breaker").text().await, Ok(Some(v)) if v == "open") {
                return true;
            }
            // Stress-II finding: sustained ambiguity rate is its own failure
            // signal — the degradation window would have tripped this at
            // wave 21, not wave 40. Counter incremented per settlement_pending
            // response; hourly cron (G7) resets it.
            if let Ok(Some(n)) = kv.get("ops:settle_pending_count").text().await {
                if let Ok(n) = n.parse::<u64>() {
                    return n > 100; // >100 pending in the hour = degraded facilitator
                }
            }
            false
        }
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
    verified_locally: bool,
) -> Result<Response> {
    use m2m_core::payment::x402v2::encode_settle_response;

    // I5: fail closed on money
    if breaker_open(env).await {
        return v2_err(Taxonomy::UnexpectedVerifyError, 503, "facilitator circuit breaker open (fail closed); retryable");
    }

    // M1 (wide-angle 2026-08-19): PassThrough payments (EIP-6492/1271) are the
    // ONLY path that spends facilitator quota on unverified input — an
    // attacker can mint unlimited well-formed 6492 envelopes and burn the CDP
    // tier until the breaker trips (a quota DoS that stops honest payments).
    // Cap facilitator-bound calls per payer per minute BEFORE the facilitator
    // seam (Rev 3 G4). KV counter, documented race (Z2): errs toward
    // protection, never a money decision.
    if !verified_locally
        && passthrough_rate_limited(env, &payload.payload.authorization.from).await
    {
        return v2_err(Taxonomy::SettlementPending, 503, "facilitator-bound verification rate limited; retry shortly");
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
        // RECONCILER-SPEC §3.C.1: the chain proved this nonce paid and the
        // entitlement is live — execute FREE (no facilitator), once, bound to
        // the ORIGINAL input (G2c: no compute oracle).
        "entitled" => {
            let tx = claim.get("tx_hash").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let net = claim.get("network").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let bound_input = claim.get("input_hash").and_then(|v| v.as_str()).unwrap_or_default();
            let want_input = hex_encode(hash_json(&body.input).as_slice());
            if bound_input != want_input {
                return v2_err(Taxonomy::InvalidPayload, 400,
                    "entitlement is bound to the original request input (G2c); re-sign a new payment for different input");
            }
            return entitled_serve(env, &key, request_id, tool, body, auth, expected, amount, &tx, &net).await;
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

    // Claim-time D1 bridge (RECONCILER-SPEC amendment): the settlements row
    // exists from the moment a claim is held, carrying the full payload —
    // a crashed isolate leaves a reconcilable record, not a void.
    let _ = d1_bridge_claim(env, &key, auth, request_id, tool, &input_hash, expected, payload).await;

    let freq = FacilitatorRequest::new(payload.clone(), expected.clone())
        .map_err(|e| Error::RustError(format!("facilitator request: {e:?}")))?;
    // Option A (operator-accepted 2026-08-19): LocalPass payments SKIP
    // facilitator /verify — the local ecrecover already proved the signature
    // under our domain, and the spec's `upfront` ordering does not include
    // /verify ("validity is established by settle"). Verify runs ONLY for
    // PassThrough (6492/1271), where the facilitator is the sole verifier.
    // /settle remains authoritative for every path.
    if !verified_locally {
        let verify = match facilitator.verify(&freq).await {
            Ok(v) => v,
            Err(e) => {
                return v2_err(Taxonomy::UnexpectedVerifyError, 503, &format!("facilitator verify error: {e:?}"));
            }
        };
        if !verify.is_valid {
            let reason = verify.invalid_reason.clone().unwrap_or_else(|| "invalid_payment_requirements".into());
            return v2_err(Taxonomy::InvalidPaymentRequirements, 400, &format!("facilitator rejected: {reason}"));
        }
    }

    // begin settle (lease refresh)
    let _ = claim_do(env, &key, serde_json::json!({"cmd": "begin_settle", "key": key, "now": now})).await?;
    let _ = d1_mark(env, &auth.from, &auth.nonce, "settling", None, None).await;

    // /settle — the money moment
    let settle_out = facilitator.settle(&freq).await;
    match settle_out {
        Ok(sr) if sr.success => {
            let output = match execute_tool(tool, &body.input) {
                Ok(o) => o,
                Err(m) => {
                    // B1 (wide-angle review 2026-08-19): the money already
                    // moved — marking this claim `failed` would strand the
                    // payer FOREVER (terminal status = invisible to the
                    // reconciler's non-terminal sweep). Convert to the
                    // entitlement path instead: claim → SettledReconciled is
                    // legal from Settling; the payer's identical retry
                    // executes FREE within 24h, bound to the original input
                    // (G2c). Never a terminal loss after a successful settle.
                    let until = now + m2m_core::payment::reconciler::REPLAY_TTL_SECS;
                    let _ = claim_do(env, &key, serde_json::json!({
                        "cmd": "reconcile_settled", "key": key,
                        "tx": sr.transaction, "network": sr.network,
                        "eligible_until": until,
                    })).await;
                    d1_mark_entitled(env, &auth.from, &auth.nonce, &sr.transaction, until).await;
                    let _ = append_event(env, request_id, tool, Some(&sr.transaction), amount, "V2_SETTLED_EXEC_FAILED", None).await;
                    return v2_err(Taxonomy::SettlementPending, 503, &format!(
                        "settled (tx {}) but execution failed: {m}; an identical retry within 24h executes free — no new payment",
                        sr.transaction
                    ));
                }
            };
            let pr_b64 = encode_settle_response(&sr)
                .map_err(|e| Error::RustError(format!("encode PAYMENT-RESPONSE: {e:?}")))?;
            // build the FULL response now; the exact bytes are what replay
            // serves (G2b: identical 200s — receipt + settlement included)
            let receipt = Receipt {
                request_id: request_id.to_string(),
                tool: tool.to_string(),
                tool_version: "1.0.0".into(),
                input_hash: hash_json(&body.input),
                output_hash: hash_json(&output),
                timestamp_unix: Date::now().as_millis() / 1000,
                // XDR-1 v0.2: bind the receipt to THIS payment authorization
                // (the structural gate guarantees 0x+64hex by settle time)
                payment_ref: auth.nonce.parse().unwrap_or_default(),
            };
            let commitment = receipt.commitment();
            let sig_hex = sign_commitment(env, &commitment).await?;
            let full_body = serde_json::json!({
                "output": output,
                "receipt": {"receipt": receipt, "spec": m2m_core::receipt::SPEC, "commitment": hex_encode(commitment.as_slice()), "signature": sig_hex},
                "settlement": {"transaction": sr.transaction, "network": sr.network},
            });
            let body_bytes = serde_json::to_vec(&full_body)
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
            // durable record (0002/0003 settlements) — best effort; the DO is
            // the claim authority, D1 the reconciliation record
            let _ = d1_record_settlement(env, &key, auth, request_id, tool, &input_hash, expected, payload, &sr, &pr_b64, &body_bytes).await;
            let _ = append_event(env, request_id, tool, Some(&sr.transaction), amount, "V2_SETTLED", None).await;
            // serve the exact stored bytes
            let mut r = Response::from_bytes(body_bytes.clone())?;
            r.headers_mut().set("Content-Type", "application/json")?;
            r.headers_mut().set("X-Schema-Version", "2.0")?;
            r.headers_mut().set("PAYMENT-RESPONSE", &pr_b64)?;
            Ok(cors(r)?)
        }
        Ok(sr) => {
            let reason = sr.error_reason.clone().unwrap_or_else(|| "unexpected_settle_error".into());
            // Single source of truth (core::reconciler::classify_settle_failure):
            // ambiguous-money (already-used shapes observed live) is NEVER a
            // terminal failure — receipt_pending, the cron's chain read decides.
            use m2m_core::payment::reconciler::{classify_settle_failure, SettleFailureClass};
            match classify_settle_failure(&reason) {
                SettleFailureClass::AmbiguousMoney => {
                    let _ = claim_do(env, &key, serde_json::json!({"cmd": "receipt_pending", "key": key})).await;
                    let _ = d1_mark(env, &auth.from, &auth.nonce, "receipt_pending", None, None).await;
                    return v2_err(Taxonomy::SettlementPending, 503, "authorization already used; reconciliation pending; retryable");
                }
                SettleFailureClass::InsufficientFunds => {
                    let _ = claim_do(env, &key, serde_json::json!({"cmd": "failed", "key": key, "reason": &reason})).await;
                    let _ = d1_mark(env, &auth.from, &auth.nonce, "failed", None, Some(&reason)).await;
                    return v2_err(Taxonomy::InsufficientFunds, 400, &format!("settlement failed: {reason}"));
                }
                SettleFailureClass::CleanReject => {}
            }
            // clean reject: nonce definitely unconsumed — terminal failed
            let _ = claim_do(env, &key, serde_json::json!({"cmd": "failed", "key": key, "reason": &reason})).await;
            let _ = d1_mark(env, &auth.from, &auth.nonce, "failed", None, Some(&reason)).await;
            v2_err(Taxonomy::UnexpectedSettleError, 400, &format!("settlement failed: {reason}"))
        }
        Err(_) => {
            {
                if let Ok(kv) = env.kv("PRICING") {
                    if let Ok(Some(cur)) = kv.get("ops:settle_pending_count").text().await {
                        let n: u64 = cur.parse().unwrap_or(0);
                        if let Ok(p) = kv.put("ops:settle_pending_count", (n + 1).to_string()) {
                            let _ = p.execute().await;
                        }
                    } else if let Ok(kv2) = env.kv("PRICING") {
                        if let Ok(p) = kv2.put("ops:settle_pending_count", "1") {
                            let _ = p.execute().await;
                        }
                    }
                }
                v2_err(Taxonomy::SettlementPending, 503, "settle outcome unknown (timeout); lease recovers the claim; retryable")
            }
        }
    }
}

/// Live reconciler entitlement? (status settled_reconciled within TTL) —
/// gates only the stamp AGE bypass above; the claim DO remains the authority.
/// None = D1 unavailable (caller must treat as retryable-unknown, Kimi m9).
async fn d1_entitled(env: &Env, payer: &str, nonce: &str, now: u64) -> Option<bool> {
    let db = env.d1("LEDGER").ok()?;
    let stmt = db.prepare(
        "SELECT 1 FROM settlements WHERE payer=?1 AND nonce=?2 \
         AND status='settled_reconciled' AND replay_eligible_until > ?3 LIMIT 1",
    );
    let bound = stmt.bind(&[payer.into(), nonce.into(), (now as f64).into()]).ok()?;
    match bound.first::<serde_json::Value>(None).await {
        Ok(row) => Some(row.is_some()),
        Err(_) => None,
    }
}

/// Claim-time bridge (RECONCILER-SPEC amendment): the settlements row exists
/// from claim, status 'claimed', full payload persisted — the reconciler's
/// stale-select and the re-drive both work off this row. Idempotent.
#[allow(clippy::too_many_arguments)]
async fn d1_bridge_claim(
    env: &Env,
    key: &str,
    auth: &m2m_core::payment::x402v2::Authorization,
    request_id: &str,
    tool: &str,
    input_hash: &str,
    expected: &PaymentRequirements,
    payload: &m2m_core::payment::x402v2::PaymentPayload,
) -> Result<()> {
    use worker::wasm_bindgen::JsValue;
    let payload_json = serde_json::to_string(payload)
        .map_err(|e| Error::RustError(format!("payload: {e}")))?;
    let db = env.d1("LEDGER")?;
    db.prepare(
        "INSERT OR IGNORE INTO settlements(id, payer, nonce, request_id, tool, input_hash, status, scheme, network, asset, amount, pay_to, payment_payload) VALUES (?1,?2,?3,?4,?5,?6,'claimed','exact',?7,?8,?9,?10,?11)",
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
    ])?.run().await?;
    Ok(())
}

/// Non-terminal status mark (settling / receipt_pending / failed) — guarded
/// by the same absorbing predicate as the reconciler: a terminal row is
/// never overwritten by the request path either.
async fn d1_mark(env: &Env, payer: &str, nonce: &str, status: &str, tx: Option<&str>, failure: Option<&str>) -> Result<()> {
    use worker::wasm_bindgen::JsValue;
    let db = env.d1("LEDGER")?;
    db.prepare(
        "UPDATE settlements SET status=?3, tx_hash=COALESCE(?4, tx_hash), failure_reason=?5, updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE payer=?1 AND nonce=?2 AND status IN ('claimed','settling','receipt_pending','settlement_pending')",
    ).bind(&[
        JsValue::from_str(payer),
        JsValue::from_str(nonce),
        JsValue::from_str(status),
        tx.map(JsValue::from_str).unwrap_or(JsValue::NULL),
        failure.map(JsValue::from_str).unwrap_or(JsValue::NULL),
    ])?.run().await?;
    Ok(())
}

/// B1: post-settle execution failure — record the entitlement
/// (settled_reconciled, 24h window) so the claim machine's Entitled path and
/// the d1_entitled() stamp-age bypass both see it. Guarded by the same
/// absorbing predicate: a terminal row is never overwritten.
async fn d1_mark_entitled(env: &Env, payer: &str, nonce: &str, tx: &str, until: u64) {
    if let Ok(db) = env.d1("LEDGER") {
        let _ = async {
            use worker::wasm_bindgen::JsValue;
            db.prepare(
                "UPDATE settlements SET status='settled_reconciled', tx_hash=?3, resolution='facilitator', resolution_tx=?3, resolved_at=CAST(strftime('%s','now') AS INTEGER), replay_eligible_until=?4, settled_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'), updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE payer=?1 AND nonce=?2 AND status IN ('claimed','settling','receipt_pending','settlement_pending')",
            )
            .bind(&[
                JsValue::from_str(payer),
                JsValue::from_str(nonce),
                JsValue::from_str(tx),
                JsValue::from_f64(until as f64),
            ])?
            .run()
            .await?;
            Ok::<(), Error>(())
        }
        .await;
    }
}

/// Settle-success record: bridge-ensure (safety net if the claim-time write
/// failed), then the guarded terminal UPDATE. Response bytes persist for D1
/// readers (the DO remains the replay authority), mirroring the DO's 256KB
/// non-replayable gate (G10).
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
    body_bytes: &[u8],
) -> Result<()> {
    use worker::wasm_bindgen::JsValue;
    let _ = d1_bridge_claim(env, key, auth, request_id, tool, input_hash, expected, payload).await;
    let replayable_body = if body_bytes.len() <= 256 * 1024 {
        std::str::from_utf8(body_bytes).map(JsValue::from_str).unwrap_or(JsValue::NULL)
    } else {
        JsValue::NULL
    };
    let db = env.d1("LEDGER")?;
    db.prepare(
        "UPDATE settlements SET status='settled', tx_hash=?3, settle_network=?4, payment_response=?5, response_body=?6, settled_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'), updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'), resolution='facilitator', resolution_tx=?3, resolved_at=CAST(strftime('%s','now') AS INTEGER) WHERE payer=?1 AND nonce=?2 AND status IN ('claimed','settling','receipt_pending','settlement_pending')",
    ).bind(&[
        JsValue::from_str(&auth.from),
        JsValue::from_str(&auth.nonce),
        JsValue::from_str(&sr.transaction),
        JsValue::from_str(&sr.network),
        JsValue::from_str(pr_b64),
        replayable_body,
    ])?.run().await?;
    Ok(())
}

/// The entitlement execution (RECONCILER-SPEC §3.C.1): the chain already
/// moved this money; the payer retries the identical payment and receives
/// the tool's output FREE — once; the DO stores the response so later
/// retries replay identically.
#[allow(clippy::too_many_arguments)]
async fn entitled_serve(
    env: &Env,
    key: &str,
    request_id: &str,
    tool: &str,
    body: &CallRequest,
    auth: &m2m_core::payment::x402v2::Authorization,
    expected: &PaymentRequirements,
    amount: u64,
    tx: &str,
    network: &str,
) -> Result<Response> {
    use m2m_core::payment::x402v2::{encode_settle_response, SettleResponse};

    let output = match execute_tool(tool, &body.input) {
        Ok(o) => o,
        // entitlement SURVIVES a tool failure until the TTL — retry executes free again
        Err(m) => return v2_err(Taxonomy::UnexpectedVerifyError, 500, m),
    };
    // synthesize the settlement proof header from the chain evidence
    let sr = SettleResponse {
        success: true,
        error_reason: None,
        payer: Some(auth.from.clone()),
        transaction: tx.to_string(),
        network: network.to_string(),
        amount: Some(expected.amount.clone()),
        extensions: None,
    };
    let pr_b64 = encode_settle_response(&sr)
        .map_err(|e| Error::RustError(format!("encode PAYMENT-RESPONSE: {e:?}")))?;
    let receipt = Receipt {
        request_id: request_id.to_string(),
        tool: tool.to_string(),
        tool_version: "1.0.0".into(),
        input_hash: hash_json(&body.input),
        output_hash: hash_json(&output),
        timestamp_unix: Date::now().as_millis() / 1000,
        payment_ref: auth.nonce.parse().unwrap_or_default(), // XDR-1 v0.2
    };
    let commitment = receipt.commitment();
    let sig_hex = sign_commitment(env, &commitment).await?;
    let full_body = serde_json::json!({
        "output": output,
        "receipt": {"receipt": receipt, "spec": m2m_core::receipt::SPEC, "commitment": hex_encode(commitment.as_slice()), "signature": sig_hex},
        "settlement": {"transaction": tx, "network": network},
    });
    let body_bytes = serde_json::to_vec(&full_body)
        .map_err(|e| Error::RustError(format!("body: {e}")))?;
    // store the executed response (DO: SettledReconciled -> Settled) —
    // MANDATORY before responding (Kimi M3.2): if this write fails the payer
    // gets a retryable 503 and the entitlement survives; a served response
    // with a failed store would leave the entitlement live = free executions.
    let body_b64 = {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(&body_bytes)
    };
    let stored = claim_do(env, key, serde_json::json!({
        "cmd": "settled", "key": key,
        "tx": tx, "network": network,
        "response_body_b64": body_b64, "payment_response": pr_b64,
    })).await;
    if stored.is_err() {
        let _ = append_event(env, request_id, tool, Some(tx), amount, "V2_ENTITLED_STORE_FAILED", None).await;
        return v2_err(Taxonomy::SettlementPending, 503,
            "entitlement store failed; retry executes free again (TTL bounded)");
    }
    // D1: status stays 'settled_reconciled' (absorbing); only the response
    // columns and the bounded re-execution counter move.
    if let Ok(db) = env.d1("LEDGER") {
        let _ = async {
            use worker::wasm_bindgen::JsValue;
            let replayable = if body_bytes.len() <= 256 * 1024 {
                std::str::from_utf8(&body_bytes).map(JsValue::from_str).unwrap_or(JsValue::NULL)
            } else {
                JsValue::NULL
            };
            db.prepare(
                "UPDATE settlements SET response_body=?3, payment_response=?4, reexec_count=MIN(reexec_count+1,3), updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE payer=?1 AND nonce=?2 AND status='settled_reconciled'",
            )
            .bind(&[
                JsValue::from_str(&auth.from),
                JsValue::from_str(&auth.nonce),
                replayable,
                JsValue::from_str(&pr_b64),
            ])?
            .run()
            .await?;
            Ok::<(), Error>(())
        }
        .await;
    }
    let _ = append_event(env, request_id, tool, Some(tx), amount, "V2_ENTITLED_SERVE", None).await;
    let mut r = Response::from_bytes(body_bytes)?;
    r.headers_mut().set("Content-Type", "application/json")?;
    r.headers_mut().set("X-Schema-Version", "2.0")?;
    r.headers_mut().set("PAYMENT-RESPONSE", &pr_b64)?;
    Ok(cors(r)?)
}
