//! Stage 3 — x402 v2 wire flow on the edge worker: POST /v2/tools/{tool}/call
//! 402 + PAYMENT-REQUIRED (HMAC-stamped, G6) -> PAYMENT-SIGNATURE retry ->
//! stamp verify -> structural gate -> EOA prefilter -> execute -> 200.
//! Settlement (facilitator /settle) is Stage 4; until then this route is
//! DARK in production (KV `ops:x402v2_enabled` absent => false, kill-switch
//! design reviews/kill-switch-design.md) and exercised only in dev/e2e.
//!
//! MAC rule (launch-checklist #9): stamps are computed over OUR canonical
//! serialization (serde struct order), never raw header bytes.

use crate::{err, execute_tool, validate_only, with_schema_header, append_event, sign_commitment, Receipt};
use m2m_core::payment::x402v2::{
    self, decode_payment_payload, encode_payment_required, ExtensionData, PaymentPayload,
    PaymentRequired, PaymentRequirements, ResourceInfo, StructuralContext, X402Error,
};
use m2m_core::payment::x402v2_errors::{map_error, Taxonomy};
use m2m_core::payment::x402v2_verify::{prefilter, VerifyOutcome};
use m2m_core::payment::x402v2_client::encode_payment_signature;
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
    r.headers_mut().set("Access-Control-Expose-Headers", EXPOSE)?;
    Ok(r)
}

fn cors(r: Response) -> Result<Response> {
    let mut r = r;
    r.headers_mut().set("Access-Control-Expose-Headers", EXPOSE)?;
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

fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
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
    if req.method() != Method::Post {
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

    // pricing from KV (route-derived truth; echo is never trusted)
    let pricing = env.kv("PRICING")?;
    let price_raw = pricing.get(tool).text().await?;
    let price_raw = match price_raw {
        Some(p) => p,
        None => return v2_err(Taxonomy::InvalidPaymentRequirements, 500, "pricing missing"),
    };
    let price: serde_json::Value = serde_json::from_str(&price_raw)
        .map_err(|_| Error::RustError("pricing entry malformed".into()))?;
    let amount = price["amount_minor"].as_u64().unwrap_or(5000);

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
    let stamp = payload
        .extensions
        .as_ref()
        .and_then(|e| e.get(STAMP_EXT_ID))
        .cloned()
        .ok_or_else(|| Error::RustError("stamp".into()))?;
    let info = stamp.info;
    let mac_hex = info
        .get("mac")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::RustError("stamp.mac".into()))?;
    let iat = info
        .get("iat")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| Error::RustError("stamp.iat".into()))?;
    let key_hex = env.secret("V2_STAMP_KEY")?.to_string();
    let key_bytes = crate::hex_decode(&key_hex)
        .map_err(|_| Error::RustError("V2_STAMP_KEY not hex".into()))?;
    let now = Date::now().as_millis() / 1000;
    let grace = payload.accepted.max_timeout_seconds;
    if iat > now + 60 || iat + grace < now {
        return v2_err(Taxonomy::InvalidPaymentRequirements, 400, "stamped requirement outside grace window");
    }
    let canonical = canonical_requirement(&payload.accepted)?;
    let expect_mac = stamp_mac(&key_bytes, &canonical, iat)?;
    let got_mac = crate::hex_decode(mac_hex)
        .map_err(|_| Error::RustError("stamp.mac not hex".into()))?;
    if !ct_eq(&expect_mac, &got_mac) {
        return v2_err(Taxonomy::InvalidPaymentRequirements, 400, "requirement stamp mismatch (G6)");
    }

    // MAC proves the echoed requirement is byte-ours -> it IS the expected
    // requirement (G6: verify against the stamped copy, not live config).
    let expected = payload.accepted.clone();
    let ctx = StructuralContext { expected: &expected, route_url: &route_url, now_unix: now };
    if let Err(e) = x402v2::structural_gate(&payload, &ctx) {
        let t = map_error(&e);
        let status = match t {
            Taxonomy::InvalidSignature => 401,
            Taxonomy::InvalidValidAfter | Taxonomy::InvalidValidBefore => 402,
            _ => 400,
        };
        return v2_err(t, status, "structural gate");
    }

    match prefilter(&payload, &expected) {
        VerifyOutcome::LocalReject(e) => {
            let t = map_error(&e);
            let status = if t == Taxonomy::InvalidSignature { 401 } else { 400 };
            v2_err(t, status, "local verification failed")
        }
        VerifyOutcome::PassThrough => v2_err(
            Taxonomy::UnexpectedVerifyError,
            503,
            "EIP-6492/1271 verification requires the facilitator (Stage 4); retryable",
        ),
        VerifyOutcome::LocalPass { .. } => serve(env, &request_id, tool, &body).await,
    }
}

async fn challenge(env: &Env, route_url: &str, tool: &str, amount_minor: u64) -> Result<Response> {
    let chain_id: u64 = env.var("CHAIN_ID")?.to_string().parse().unwrap_or(8453);
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
    let mac = stamp_mac(&key_bytes, &canonical, now)?;
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

async fn serve(env: &Env, request_id: &str, tool: &str, body: &CallRequest) -> Result<Response> {
    // Stage 3: verified-serve. Stage 4 inserts facilitator /verify + /settle
    // BEFORE this point (settle-before-serve, I1). The route is KV-gated
    // dark in production until then.
    let output = match execute_tool(tool, &body.input) {
        Ok(o) => o,
        Err(m) => return v2_err(Taxonomy::UnexpectedVerifyError, 500, m),
    };
    let amount = 5000u64; // stamped requirement governs; recorded for telemetry
    let _ = append_event(env, request_id, tool, None, amount, "V2_VERIFIED_SETTLE_PENDING", None).await;

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
