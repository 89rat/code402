//! m2m-edge — Cloudflare bindings only (workers-rs).
//!
//! Implements the §5 payment state machine (invariants preserved, order fixed):
//!   1. input schema validation FIRST
//!   2. no credential -> 402 challenge
//!   3. static checks -> EIP-712 recover -> signer == payer
//!   4. nonce claim via NonceGuard DO (409 on replay)
//!   5. deterministic tool execution (m2m-core)
//!   6. D1 append + receipt sign + R2 store + queue settlement msg
//!   7. 200 {output, receipt}
//!
//! Error taxonomy (do not change):
//!   PAYMENT_REQUIRED 402 · INVALID_SIGNATURE 401 · REPLAYED_NONCE 409
//!   INSUFFICIENT_PAYMENT 402 · EXPIRED_PAYMENT 402 · UNSUPPORTED_CHAIN 400
//!   UNSUPPORTED_TOKEN 400 · INVALID_RECIPIENT 400 · INPUT_SCHEMA_INVALID 400
//!   RATE_LIMITED 429 · TOOL_INTERNAL_ERROR 500

use alloy_primitives::{keccak256, Address, B256, U256};
use k256::ecdsa::SigningKey;
use m2m_core::payment::erc3009::{verify, PaymentVoucher, VerifyContext};
use m2m_core::receipt::{hash_json, Receipt};
use m2m_core::PaymentError;
use serde::{Deserialize, Serialize};
use worker::*;

mod facilitator;
mod reconciler;
mod settlement_do;
mod x402v2_route;

// ---------- deterministic hex helpers (no extra deps) ----------
pub(crate) fn hex_decode(s: &str) -> std::result::Result<Vec<u8>, ()> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    if s.len() % 2 != 0 { return Err(()); }
    (0..s.len()).step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
        .collect()
}
fn hex_encode(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b { s.push_str(&format!("{x:02x}")); }
    s
}

// ---------- error taxonomy ----------
/// Schema-stability contract (absorbed from demand-engineering doctrine):
/// agents cache response schemas and PERMANENTLY drop APIs that change them
/// unpredictably. Every response carries X-Schema-Version; breaking changes
/// require a version bump, never a silent mutation.
pub const SCHEMA_VERSION: &str = "1.0";

pub(crate) fn with_schema_header(r: Response) -> Result<Response> {
    let mut r = r;
    r.headers_mut().set("X-Schema-Version", SCHEMA_VERSION)?;
    Ok(r)
}

pub(crate) fn err(code: &str, status: u16, msg: &str) -> Result<Response> {
    // Error taxonomy: client faults (4xx) are not retryable; server faults are.
    let retryable = status >= 500 || status == 429;
    Response::from_json(&serde_json::json!({
        "error": {"code": code, "message": msg, "retryable": retryable}
    }))
        .map(|r| r.with_status(status))
        .and_then(with_schema_header)
}
fn map_payment_error(e: &PaymentError) -> Result<Response> {
    use PaymentError::*;
    match e {
        InvalidSignatureLength | InvalidRecoveryId(_) | RecoveryFailed | SignerMismatch =>
            err("INVALID_SIGNATURE", 401, &e.to_string()),
        InvalidRecipient => err("INVALID_RECIPIENT", 400, &e.to_string()),
        InsufficientAmount => err("INSUFFICIENT_PAYMENT", 402, &e.to_string()),
        OutsideValidityWindow => err("EXPIRED_PAYMENT", 402, &e.to_string()),
        UnsupportedToken => err("UNSUPPORTED_TOKEN", 400, &e.to_string()),
        UnsupportedChain => err("UNSUPPORTED_CHAIN", 400, &e.to_string()),
    }
}

// ---------- tool registry (deterministic; executed by m2m-core) ----------
pub(crate) fn execute_tool(tool: &str, input: &serde_json::Value) -> std::result::Result<serde_json::Value, &'static str> {
    match tool {
        "vat-mod97-check" => {
            let raw = input.get("vat_number").and_then(|v| v.as_str()).ok_or("input.vat_number must be a string")?;
            match m2m_core::validate::vat::canonicalise_vat(raw) {
                Ok(body) => Ok(serde_json::json!({"valid": true, "canonical": body})),
                Err(e) => Ok(serde_json::json!({"valid": false, "reason": e.to_string()})),
            }
        }
        "company-number-format" => {
            let raw = input.get("company_number").and_then(|v| v.as_str()).ok_or("input.company_number must be a string")?;
            Ok(serde_json::json!({"valid": m2m_core::validate::validate_company_number_format(raw)}))
        }
        "context-distill" => {
            let html = input.get("html").and_then(|v| v.as_str()).ok_or("input.html must be a string")?;
            let max_bytes = input.get("max_bytes").and_then(|v| v.as_u64()).unwrap_or(4000).min(16000) as usize;
            let r = m2m_core::validate::distill::distill(html, max_bytes);
            Ok(serde_json::json!({
                "clean_text": r.clean_text,
                "original_bytes": r.original_bytes,
                "output_bytes": r.output_bytes,
                "estimated_tokens_saved": r.estimated_tokens_saved,
            }))
        }
        "iban-check" => {
            let raw = input.get("iban").and_then(|v| v.as_str()).ok_or("input.iban must be a string")?;
            Ok(serde_json::json!({"valid": m2m_core::validate::identifiers::iban_ok(raw)}))
        }
        "lei-check" => {
            let raw = input.get("lei").and_then(|v| v.as_str()).ok_or("input.lei must be a string")?;
            Ok(serde_json::json!({"valid": m2m_core::validate::identifiers::lei_ok(raw)}))
        }
        "isin-check" => {
            let raw = input.get("isin").and_then(|v| v.as_str()).ok_or("input.isin must be a string")?;
            Ok(serde_json::json!({"valid": m2m_core::validate::identifiers::isin_ok(raw)}))
        }
        "luhn-check" => {
            let raw = input.get("number").and_then(|v| v.as_str()).ok_or("input.number must be a string")?;
            Ok(serde_json::json!({"valid": m2m_core::validate::identifiers::luhn_ok(raw)}))
        }
        "swift-bic-check" => {
            let raw = input.get("bic").and_then(|v| v.as_str()).ok_or("input.bic must be a string")?;
            Ok(serde_json::json!({"valid": m2m_core::validate::identifiers::bic_ok(raw)}))
        }
        "ean13-check" => {
            let raw = input.get("ean").and_then(|v| v.as_str()).ok_or("input.ean must be a string")?;
            Ok(serde_json::json!({"valid": m2m_core::validate::identifiers::ean13_ok(raw)}))
        }
        "gstin-check" => {
            let raw = input.get("gstin").and_then(|v| v.as_str()).ok_or("input.gstin must be a string")?;
            match m2m_core::validate::identifiers::canonicalise_gstin(raw) {
                Ok(body) => Ok(serde_json::json!({"valid": true, "canonical": body})),
                Err(e) => Ok(serde_json::json!({"valid": false, "reason": e.to_string()})),
            }
        }
        _ => Err("unknown tool"),
    }
}
fn tool_known(tool: &str) -> bool {
    matches!(tool,
        "vat-mod97-check" | "company-number-format" | "context-distill"
        | "iban-check" | "lei-check" | "isin-check" | "luhn-check"
        | "swift-bic-check" | "ean13-check" | "gstin-check")
}

// ---------- trust badge SVG (shields-style, deterministic) ----------
fn render_badge(record: Option<&str>) -> String {
    let (label_right, color) = match record.and_then(|r| serde_json::from_str::<serde_json::Value>(r).ok()) {
        Some(v) => {
            let lvl = v.get("level").and_then(|x| x.as_str()).unwrap_or("unrated");
            let fid = v.get("fidelity_pct").and_then(|x| x.as_f64()).unwrap_or(0.0);
            let c = match lvl {
                "verified-gold" => "#d4af37",
                "verified" => "#2ea44f",
                "flagged" => "#cf222e",
                _ => "#6e7781",
            };
            (format!("{lvl} · {fid:.1}%"), c)
        }
        None => ("unrated".to_string(), "#6e7781"),
    };
    let left = "code402 trust";
    let lw = (left.len() as f64 * 6.2 + 12.0) as u32;
    let rw = (label_right.len() as f64 * 6.2 + 12.0) as u32;
    let total = lw + rw;
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{total}\" height=\"20\" role=\"img\" aria-label=\"{left}: {label_right}\">\
<rect width=\"{lw}\" height=\"20\" fill=\"#24292f\"/>\
<rect x=\"{lw}\" width=\"{rw}\" height=\"20\" fill=\"{color}\"/>\
<text x=\"{lx}\" y=\"14\" fill=\"#ffffff\" font-family=\"monospace\" font-size=\"11\">{left}</text>\
<text x=\"{rx}\" y=\"14\" fill=\"#ffffff\" font-family=\"monospace\" font-size=\"11\">{label_right}</text>\
</svg>",
        total = total, lw = lw, rw = rw, color = color,
        lx = 6, rx = lw + 6, left = left, label_right = label_right
    )
}

#[derive(Deserialize)]
struct CallRequest { input: serde_json::Value, idempotency_key: Option<String> }

#[derive(Serialize, Deserialize)]
pub struct SettlementMsg { request_id: String, tx_hash: Option<String>, payer: String, amount_minor: String }

// ---------- fetch ----------
#[event(fetch)]
pub async fn fetch(req: Request, env: Env, ctx: Context) -> Result<Response> {
    let req_id = req.headers().get("cf-ray")?.unwrap_or_else(|| "unknown".into());
    let mut resp = fetch_inner(req, env, ctx).await?;
    // Required gateway header: every response carries the request id.
    resp.headers_mut().set("X-Request-Id", &req_id)?;
    Ok(resp)
}

async fn fetch_inner(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let url = req.url()?;
    let path = url.path().to_string();
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // Agent-discovery manifests — the machine-readable storefront. GET only.
    // Static files embedded at build time from site/dist; x402.json is rendered
    // per-environment from runtime vars (chain, token, domain, signers differ).
    if req.method() == Method::Get {
        if path == "/.well-known/x402.json" {
            let chain_id: u64 = env.var("CHAIN_ID")?.to_string().parse().unwrap_or(8453);
            let token = env.var("USDC_BASE")?.to_string();
            let recipient = env.secret("COMPANY_WALLET")?.to_string();
            let manifest = serde_json::json!({
                "x402_version": 1,
                "environment": if chain_id == 8453 { "production" } else { "staging" },
                "asset": "USDC",
                "network": {"chain_id": chain_id, "name": if chain_id == 8453 { "base" } else { "base-sepolia" }},
                "token_address": token,
                "authorization_type": "TransferWithAuthorization (EIP-3009)",
                "signature_scheme": "EIP-712",
                "eip712_domain": {
                    "name": env.var("TOKEN_NAME")?.to_string(),
                    "version": env.var("TOKEN_VERSION")?.to_string(),
                    "chainId": chain_id,
                    "verifyingContract": token,
                },
                "recipient": recipient,
                "x402_v2": {
                    "route": "/v2/tools/{tool}/call",
                    "network": format!("eip155:{chain_id}"),
                    "scheme": "exact",
                    "assetTransferMethod": "eip3009",
                    "paymentFlow": "upfront",
                    "headers": ["PAYMENT-REQUIRED", "PAYMENT-SIGNATURE", "PAYMENT-RESPONSE"],
                    "enabled": "kv:ops:x402v2_enabled",
                    "notes": "v2 wire per x402 specification v2; settlement lands with the facilitator stage"
                },
                "receipt_signing_address": env.var("RECEIPT_SIGNER_ADDRESS")?.to_string(),
                "decimals": 6,
                "default_price": { "amount": "5000", "per": "call" },
                "challenge_ttl_seconds": 300,
                "notes": "The 402 challenge body is always authoritative for recipient, amount, chain, and nonce. Do not hardcode."
            });
            let mut resp = Response::from_json(&manifest)?;
            resp.headers_mut().set("Cache-Control", "public, max-age=300")?;
            return Ok(resp);
        }
        let served: Option<(&'static str, &'static str)> = match path.as_str() {
            "/llms.txt" => Some((include_str!("../../../site/dist/llms.txt"), "text/plain; charset=utf-8")),
            "/.well-known/mcp.json" => Some((include_str!("../../../site/dist/.well-known/mcp.json"), "application/json")),
            "/.well-known/openapi.yaml" => Some((include_str!("../../../site/dist/.well-known/openapi.yaml"), "application/yaml")),
            "/.well-known/security.txt" => Some((include_str!("../../../site/dist/.well-known/security.txt"), "text/plain; charset=utf-8")),
            _ => None,
        };
        if let Some((body, ctype)) = served {
            let mut resp = Response::ok(body)?;
            resp.headers_mut().set("Content-Type", ctype)?;
            resp.headers_mut().set("Cache-Control", "public, max-age=300")?;
            return Ok(resp);
        }
    }

    // ---------- GET /v1/ops/stats — public-safe operational telemetry ----------
    // KV-backed writers: cron (reconciler/pending counts), route (breaker,
    // settle_pending). No secrets, no row data; 30s edge cache.
    if req.method() == Method::Get && path == "/v1/ops/stats" {
        let kv = env.kv("PRICING")?;
        let val = |k: &'static str| async { kv.get(k).text().await.ok().flatten() };
        let body = serde_json::json!({
            "schema": "1",
            "x402v2_enabled": val("ops:x402v2_enabled").await.as_deref() == Some("true"),
            "facilitator_breaker": val("ops:facilitator_breaker").await,
            "settle_pending_count": val("ops:settle_pending_count").await,
            "pending_settlement_events": val("ops:pending_settlement").await,
            "reconciler": {
                "last_success_ms": val("ops:reconciler_last_success").await,
                "stale_backlog": val("ops:stale_backlog").await,
                "oldest_stale_age_s": val("ops:oldest_stale_age").await,
                "canceled_last_run": val("ops:canceled_last_run").await,
            },
        });
        let mut resp = Response::from_json(&body)?;
        resp.headers_mut().set("Cache-Control", "public, max-age=30")?;
        return Ok(resp);
    }

    // ---------- code402 Verified trust registry (Phase 1) ----------
    // GET /v1/trust/{domain}            -> trust record JSON (from KV trust:{domain})
    // GET /v1/trust/{domain}/badge.svg  -> shields-style badge rendered from the record
    // Records are computed OFF-WORKER by the append-only crawler pipeline and
    // ingested via POST /v1/trust-ingest (Bearer TRUST_INGEST_KEY). The worker
    // never computes trust — it only serves what the evidence supports.
    if req.method() == Method::Get && segs.len() >= 3 && segs[0] == "v1" && segs[1] == "trust" {
        let domain = segs[2];
        let kv = env.kv("PRICING")?;
        let rec = kv.get(&format!("trust:{domain}")).text().await?;
        if segs.len() == 4 && segs[3] == "badge.svg" {
            let svg = render_badge(rec.as_deref());
            let mut resp = Response::ok(svg)?;
            resp.headers_mut().set("Content-Type", "image/svg+xml")?;
            resp.headers_mut().set("Cache-Control", "public, max-age=3600")?;
            return Ok(resp);
        }
        return match rec {
            Some(j) => {
                let mut resp = Response::ok(j)?;
                resp.headers_mut().set("Content-Type", "application/json")?;
                resp.headers_mut().set("Cache-Control", "public, max-age=300")?;
                Ok(resp)
            }
            None => err("NOT_FOUND", 404, "no trust record for this domain"),
        };
    }
    if req.method() == Method::Post && path == "/v1/trust-ingest" {
        let want = env.secret("TRUST_INGEST_KEY")?.to_string();
        let got = req.headers().get("authorization")?.unwrap_or_default();
        if got != format!("Bearer {want}") {
            return err("INVALID_SIGNATURE", 401, "bad ingest credential");
        }
        let mut req = req;
        let body: serde_json::Value = match req.json().await {
            Ok(b) => b,
            Err(_) => return err("INPUT_SCHEMA_INVALID", 400, "body must be JSON {domain, record}"),
        };
        let domain = body.get("domain").and_then(|d| d.as_str()).unwrap_or("");
        let record = body.get("record").cloned().unwrap_or(serde_json::Value::Null);
        if domain.is_empty() || !record.is_object() {
            return err("INPUT_SCHEMA_INVALID", 400, "need {domain: string, record: object}");
        }
        let kv = env.kv("PRICING")?;
        kv.put(&format!("trust:{domain}"), record.to_string())?.execute().await?;
        return Response::from_json(&serde_json::json!({"ok": true, "domain": domain}));
    }

    // Route: POST /v2/tools/{tool}/call (x402 v2 wire flow; KV-gated)
    if segs.len() == 4 && segs[0] == "v2" && segs[1] == "tools" && segs[3] == "call" {
        let tool = segs[2].to_string();
        let mut req = req;
        return x402v2_route::handle(&mut req, &env, &tool).await;
    }

    // Route: POST /v1/tools/{tool}/call (legacy dialect; hard-cut at Stage 5)
    if req.method() != Method::Post || segs.len() != 4 || segs[0] != "v1" || segs[1] != "tools" || segs[3] != "call" {
        return err("INPUT_SCHEMA_INVALID", 400, "route must be POST /v1/tools/{tool}/call or /v2/...");
    }
    let tool = segs[2];
    if !tool_known(tool) {
        return err("INPUT_SCHEMA_INVALID", 400, "unknown tool");
    }

    // request id: cf-ray is globally unique per request; fallback to timestamp
    let request_id = req.headers().get("cf-ray")?
        .unwrap_or_else(|| format!("req-{}", Date::now().as_millis()));

    // STEP 1 — validate input schema FIRST (reject before cost)
    let mut req = req;
    let body: CallRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return err("INPUT_SCHEMA_INVALID", 400, "body must be JSON {input, idempotency_key?}"),
    };
    if let Err(msg) = validate_only(tool, &body.input) {
        return err("INPUT_SCHEMA_INVALID", 400, msg);
    }

    // idempotency: a previously completed key returns its stored receipt ref
    if let Some(key) = &body.idempotency_key {
        let db = env.d1("LEDGER")?;
        let hit = db.prepare("SELECT response_ref FROM idempotency WHERE idem_key = ?1")
            .bind(&[key.clone().into()])?
            .first::<serde_json::Value>(None).await?;
        if let Some(row) = hit {
            return Response::from_json(&serde_json::json!({
                "idempotent_replay": true,
                "receipt_ref": row["response_ref"],
            }));
        }
    }

    // pricing from KV
    let pricing = env.kv("PRICING")?;
    let price_raw = pricing.get(tool).text().await?
        .ok_or_else(|| Error::RustError(format!("pricing missing for {tool}")))?;
    let price: serde_json::Value = serde_json::from_str(&price_raw)
        .map_err(|_| Error::RustError("pricing entry malformed".into()))?;
    let amount_minor = price["amount_minor"].as_u64().unwrap_or(5000); // 0.005 USDC @ 6dp

    // STEP 2 — no credential -> 402 challenge
    let payment_header = req.headers().get("x-payment")?;
    let voucher: PaymentVoucher = match payment_header {
        None => return challenge(&env, &request_id, tool, amount_minor).await,
        Some(h) => match serde_json::from_str(&h) {
            Ok(v) => v,
            Err(_) => return err("INVALID_SIGNATURE", 401, "X-PAYMENT must be a JSON PaymentVoucher"),
        },
    };

    // STEP 3 — static checks then EIP-712 recover, signer == payer
    let chain_id: u64 = env.var("CHAIN_ID")?.to_string().parse()
        .map_err(|_| Error::RustError("CHAIN_ID var not u64".into()))?;
    if chain_id != 8453 && chain_id != 84532 {
        return err("UNSUPPORTED_CHAIN", 400, "chain not allowlisted");
    }
    let token: Address = env.var("USDC_BASE")?.to_string().parse()
        .map_err(|_| Error::RustError("USDC_BASE var not an address".into()))?;
    let recipient: Address = env.secret("COMPANY_WALLET")?.to_string().parse()
        .map_err(|_| Error::RustError("COMPANY_WALLET secret not an address".into()))?;
    let ctx = VerifyContext {
        token_name: env.var("TOKEN_NAME")?.to_string(),
        token_version: env.var("TOKEN_VERSION")?.to_string(),
        chain_id,
        token_address: token,
        expected_recipient: recipient,
        required_amount: U256::from(amount_minor),
        now_unix: Date::now().as_millis() / 1000,
    };
    let payer = match verify(&voucher, &ctx) {
        Ok(p) => p,
        Err(e) => return map_payment_error(&e),
    };

    // STEP 4 — claim nonce via NonceGuard DO (409 on replay)
    let nonce_key = hex_encode(voucher.auth.nonce.as_slice());
    let ns = env.durable_object("NONCE_GUARD")?;
    let stub = ns.id_from_name("nonce-guard-v1")?.get_stub()?;
    let do_resp = stub.fetch_with_str(&format!("https://do/claim?key={nonce_key}")).await?;
    if do_resp.status_code() == 409 {
        return err("REPLAYED_NONCE", 409, "payment nonce already claimed");
    }
    if do_resp.status_code() != 200 {
        return err("TOOL_INTERNAL_ERROR", 500, "nonce guard failure");
    }

    // STEP 5 — execute deterministic tool
    let output = match execute_tool(tool, &body.input) {
        Ok(o) => o,
        Err(msg) => {
            let _ = append_event(&env, &request_id, tool, None, amount_minor, "ERROR", Some("TOOL_INTERNAL_ERROR")).await;
            return err("TOOL_INTERNAL_ERROR", 500, msg);
        }
    };

    // STEP 6 — append D1 event, sign receipt, store in R2, enqueue settlement
    let tx_hash = req.headers().get("x-settlement-tx")?; // facilitator-submitted tx, if any
    append_event(&env, &request_id, tool, tx_hash.as_deref(), amount_minor, "PENDING_SETTLEMENT", None).await?;

    let receipt = Receipt {
        request_id: request_id.clone(),
        tool: tool.to_string(),
        tool_version: "1.0.0".into(),
        input_hash: hash_json(&body.input),
        output_hash: hash_json(&output),
        timestamp_unix: Date::now().as_millis() / 1000,
        payment_ref: alloy_primitives::B256::from_slice(voucher.auth.nonce.as_slice()), // XDR-1 v0.2
    };
    let commitment = receipt.commitment();
    let sig_hex = sign_commitment(&env, &commitment)?;
    let receipt_doc = serde_json::json!({
        "receipt": receipt, "spec": m2m_core::receipt::SPEC, "commitment": hex_encode(commitment.as_slice()), "signature": sig_hex,
    });

    let bucket = env.bucket("RECEIPTS")?;
    let r2_key = format!("receipts/{request_id}.json");
    bucket.put(&r2_key, receipt_doc.to_string()).execute().await?;

    if let Some(key) = &body.idempotency_key {
        env.d1("LEDGER")?
            .prepare("INSERT OR IGNORE INTO idempotency(idem_key, request_id, response_ref, created_at) VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ','now'))")
            .bind(&[key.clone().into(), request_id.clone().into(), r2_key.clone().into()])?
            .run().await?;
    }

    env.queue("SETTLEMENT_CONFIRM")?.send(SettlementMsg {
        request_id: request_id.clone(),
        tx_hash,
        payer: format!("{payer:?}"),
        amount_minor: amount_minor.to_string(),
    }).await?;

    // STEP 7 — 200 {output, receipt} — paid responses are never cacheable
    Response::from_json(&serde_json::json!({"output": output, "receipt": receipt_doc}))
        .and_then(with_schema_header)
        .and_then(|mut r| {
            r.headers_mut().set("Cache-Control", "private, no-store")?;
            Ok(r)
        })
}

pub(crate) fn validate_only<'a>(tool: &str, input: &serde_json::Value) -> std::result::Result<(), &'a str> {
    let field = match tool {
        "vat-mod97-check" => "vat_number",
        "company-number-format" => "company_number",
        "context-distill" => "html",
        "iban-check" => "iban",
        "lei-check" => "lei",
        "isin-check" => "isin",
        "luhn-check" => "number",
        "swift-bic-check" => "bic",
        "ean13-check" => "ean",
        "gstin-check" => "gstin",
        _ => return Err("unknown tool"),
    };
    if input.get(field).and_then(|v| v.as_str()).is_none() {
        return Err("input field missing or not a string");
    }
    Ok(())
}

async fn challenge(env: &Env, request_id: &str, tool: &str, amount_minor: u64) -> Result<Response> {
    let chain_id: u64 = env.var("CHAIN_ID")?.to_string().parse().unwrap_or(8453);
    let recipient = env.secret("COMPANY_WALLET")?.to_string();
    let token = env.var("USDC_BASE")?.to_string();
    let now = Date::now().as_millis() / 1000;
    let nonce = keccak256(format!("{request_id}:{now}").as_bytes());
    let token_name = env.var("TOKEN_NAME")?.to_string();
    let token_version = env.var("TOKEN_VERSION")?.to_string();
    // RFC 3339 UTC expiry alongside the unix field (normative schema P4).
    let secs = (now + 300) % 60;
    let mins = ((now + 300) / 60) % 60;
    let hours = ((now + 300) / 3600) % 24;
    let days = (now + 300) / 86400;
    let (y, m, d) = civil_from_days(days as i64);
    let expires_rfc3339 = format!("{y:04}-{m:02}-{d:02}T{hours:02}:{mins:02}:{secs:02}Z");
    let body = serde_json::json!({
        "x402_version": 1,
        "request_id": request_id,
        "payment_intent_id": request_id,   // 1 intent == 1 request in v1 (G2)
        "service_id": tool,
        "tool": tool,
        "price": {"amount": amount_minor.to_string(), "decimals": 6, "asset": "USDC", "token_address": token},
        "network": {"chain_id": chain_id, "name": if chain_id == 8453 { "base" } else { "base-sepolia" }},
        // EIP-712 domain is part of the challenge: Sepolia USDC uses name
        // "USDC", mainnet uses "USD Coin". Signers must not hardcode.
        "eip712": {"name": token_name, "version": token_version},
        "settlement_mode": "facilitated_direct",
        "proof": {"type": "eip3009_voucher", "header": "X-PAYMENT"},
        "recipient": recipient,
        "nonce": format!("0x{}", hex_encode(nonce.as_slice())),
        "expires_at": now + 300,
        "expires_at_rfc3339": expires_rfc3339,
        "status_url": format!("/v1/requests/{request_id}"),
    });
    // Funnel top: every 402 is a lead. Best-effort — never fail the challenge.
    let _ = append_event_s(env, request_id, "-ch", tool, None, amount_minor, "CHALLENGED", None).await;
    Response::from_json(&body).map(|r| r.with_status(402)).and_then(with_schema_header)
        .and_then(|mut r| {
            r.headers_mut().set("Cache-Control", "private, no-store")?;
            Ok(r)
        })
}

// days since epoch -> (year, month, day), Howard Hinnant's algorithm
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub(crate) async fn append_event(env: &Env, request_id: &str, tool: &str, tx_hash: Option<&str>,
                      amount_minor: u64, status: &str, error_code: Option<&str>) -> Result<()> {
    append_event_s(env, request_id, "", tool, tx_hash, amount_minor, status, error_code).await
}

// suffix disambiguates lifecycle events sharing a request_id (e.g. "-ch" challenge)
async fn append_event_s(env: &Env, request_id: &str, suffix: &str, tool: &str, tx_hash: Option<&str>,
                        amount_minor: u64, status: &str, error_code: Option<&str>) -> Result<()> {
    use wasm_bindgen::JsValue;
    let db = env.d1("LEDGER")?;
    // NOTE: Option<String>.into() yields JS `undefined`, which D1 rejects.
    // Explicit JsValue::NULL for absent values.
    db.prepare("INSERT INTO payment_events(event_id, request_id, tool_id, tool_version, tx_hash, amount_minor, status, error_code) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)")
        .bind(&[
            JsValue::from_str(&format!("evt-{request_id}{suffix}")),
            JsValue::from_str(request_id),
            JsValue::from_str(tool),
            JsValue::from_str("1.0.0"),
            tx_hash.map(JsValue::from_str).unwrap_or(JsValue::NULL),
            JsValue::from_f64(amount_minor as f64),
            JsValue::from_str(status),
            error_code.map(JsValue::from_str).unwrap_or(JsValue::NULL),
        ])?
        .run().await?;
    Ok(())
}

pub(crate) fn sign_commitment(env: &Env, commitment: &B256) -> Result<String> {
    let key_hex = env.secret("RECEIPT_SIGNING_KEY")?.to_string();
    let key_bytes = hex_decode(&key_hex).map_err(|_| Error::RustError("RECEIPT_SIGNING_KEY not hex".into()))?;
    let sk = SigningKey::from_slice(&key_bytes).map_err(|_| Error::RustError("RECEIPT_SIGNING_KEY invalid".into()))?;
    let (sig, rid) = sk.sign_prehash_recoverable(commitment.as_slice())
        .map_err(|_| Error::RustError("receipt signing failed".into()))?;
    let mut s65 = sig.to_bytes().to_vec();
    s65.push(rid.to_byte());
    Ok(format!("0x{}", hex_encode(&s65)))
}

// ---------- queue consumer: async settlement confirmation ----------
#[event(queue)]
pub async fn queue(batch: MessageBatch<SettlementMsg>, env: Env, _ctx: Context) -> Result<()> {
    let rpc = env.secret("RPC_PRIMARY")?.to_string();
    let db = env.d1("LEDGER")?;
    for msg in batch.messages()? {
        let m = msg.body();
        let settled = match &m.tx_hash {
            Some(tx) => confirm_tx(&rpc, tx).await.unwrap_or(false),
            None => false, // no facilitator tx yet; retry until one is attached
        };
        if settled {
            db.prepare("UPDATE payment_events SET status = 'SETTLED' WHERE request_id = ?1")
                .bind(&[m.request_id.clone().into()])?
                .run().await?;
            msg.ack();
        } else {
            msg.retry();
        }
    }
    Ok(())
}

async fn confirm_tx(rpc: &str, tx_hash: &str) -> Result<bool> {
    let payload = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "eth_getTransactionReceipt",
        "params": [tx_hash],
    });
    let mut headers = Headers::new();
    headers.set("content-type", "application/json")?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post);
    init.with_headers(headers);
    init.with_body(Some(serde_json::to_string(&payload)?.into()));
    let req = Request::new_with_init(rpc, &init)?;
    let mut resp = Fetch::Request(req).send().await?;
    let body: serde_json::Value = resp.json().await?;
    Ok(body["result"]["status"].as_str() == Some("0x1"))
}

// ---------- cron: hourly sweep-policy check; 02:00 daily accounting export ----------
#[event(scheduled)]
pub async fn scheduled(event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    if let Err(e) = run_scheduled(&event, &env).await {
        console_error!("scheduled handler failed: {e}");
    }
}

async fn run_scheduled(event: &ScheduledEvent, env: &Env) -> Result<()> {
    let db = env.d1("LEDGER")?;
    match event.cron().as_str() {
        "0 * * * *" => {
            // sweep-policy check: count unsettled events; treasury sweep itself
            // is an operator action (keys never live in the Worker beyond receipts)
            let row = db.prepare("SELECT COUNT(*) AS c FROM payment_events WHERE status = 'PENDING_SETTLEMENT'")
                .first::<serde_json::Value>(None).await?;
            let count = row.and_then(|r| r["c"].as_u64()).unwrap_or(0);
            env.kv("PRICING")?.put("ops:pending_settlement", count.to_string())?.execute().await?;
            // G7: on-chain reconciliation + /supported health probe (G8 breaker).
            // The RECONCILER-SPEC v1 sweep runs FIRST (three-way resolver over
            // stale claims: authorizationState + getLogs + entitlement
            // write-back); the Transfer-scan backfill below remains for
            // row-less phantoms (crash before the claim-time bridge).
            match reconciler::sweep(&env).await {
                Ok(st) => console_log!("reconciler sweep ok: {st:?}"),
                Err(e) => console_error!("reconciler sweep failed: {e}"),
            }
            if let Err(e) = reconcile_and_probe(&env).await {
                console_error!("G7 reconciliation failed: {e}");
            }
        }
        "0 2 * * *" => {
            // daily accounting export to R2
            let rows = db.prepare("SELECT * FROM payment_events WHERE created_at >= date('now','-1 day')")
                .all().await?;
            let results = rows.results::<serde_json::Value>()?;
            let key = format!("exports/{}.json", Date::now().as_millis());
            env.bucket("RECEIPTS")?.put(&key, serde_json::to_string(&results)?).execute().await?;
        }
        _ => {}
    }
    Ok(())
}

// ---------- NonceGuard Durable Object: single-threaded replay safety ----------
#[durable_object]
pub struct NonceGuard { state: State, _env: Env }

#[durable_object]
impl DurableObject for NonceGuard {
    fn new(state: State, env: Env) -> Self { Self { state, _env: env } }

    async fn fetch(&mut self, req: Request) -> Result<Response> {
        let url = req.url()?;
        if url.path() != "/claim" {
            return Response::error("not found", 404);
        }
        let key = url.query_pairs().find(|(k, _)| k == "key")
            .map(|(_, v)| v.to_string())
            .ok_or_else(|| Error::RustError("missing key".into()))?;
        // Storage::get errors when the key is absent; present => replay.
        let mut storage = self.state.storage();
        if storage.get::<serde_json::Value>(&key).await.is_ok() {
            return Response::error("replayed", 409);
        }
        storage.put(&key, serde_json::json!({"claimed": true})).await?;
        Response::ok("claimed")
    }
}

// ---------- G7: on-chain reconciliation + facilitator health probe ----------
// Scans AuthorizationUsed events from the USDC contract (EIP-3009 burns the
// nonce on-chain — the root of truth). Every event whose (payer, nonce) has
// no settled row in D1 is a phantom (settle completed server-side after our
// timeout) — backfilled here. Also probes GET /supported and opens the
// breaker if our scheme/network pair disappears.
async fn reconcile_and_probe(env: &Env) -> Result<()> {
    let chain: u64 = env.var("CHAIN_ID")?.to_string().parse().unwrap_or(84532);
    let rpc = env.secret("RPC_PRIMARY")?.to_string();
    let token = env.var("USDC_BASE")?.to_string().to_lowercase();

    // health probe first (cheap, fail-open on meta per I5 — logged, not fatal)
    let _ = probe_supported(env).await;

    // OUR money only: Transfer(from, to, value) with to = COMPANY_WALLET.
    // (AuthorizationUsed watches the whole world's testnet USDC; watching
    // transfers TO US is the precise reconciliation scope.)
    let company = env.secret("COMPANY_WALLET")?.to_string().to_lowercase();
    let addr_pad = format!("0x{}", format!("{}{}", "0".repeat(64 - (company.len() - 2)), &company[2..]));
    let topic0 = format!("0x{}", {
        let h = keccak256(b"Transfer(address,address,uint256)");
        h.iter().map(|x| format!("{x:02x}")).collect::<String>()
    });
    // last ~4h of blocks (~7200 at 2s)
    let latest: u64 = serde_json::from_str::<serde_json::Value>(
        &rpc_call(&rpc, "eth_blockNumber", serde_json::json!([])).await?,
    )?["result"].as_str().and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .ok_or_else(|| Error::RustError("blockNumber".into()))?;
    let from = latest.saturating_sub(7200);
    let logs_body = serde_json::json!([{
        "address": token,
        "topics": [topic0, null, addr_pad],  // Transfer -> US
        "fromBlock": format!("0x{from:x}"),
        "toBlock": "latest",
    }]);
    let logs_raw = rpc_call(&rpc, "eth_getLogs", logs_body).await?;
    let logs: Vec<serde_json::Value> = serde_json::from_str::<serde_json::Value>(&logs_raw)?["result"]
        .as_array().cloned().unwrap_or_default();
    let db = env.d1("LEDGER")?;
    let mut checked = 0usize; let mut backfilled = 0usize;
    for log in &logs {
        let topics = log["topics"].as_array().cloned().unwrap_or_default();
        if topics.len() < 3 { continue; }
        // topic1 = from (payer); nonce unknown from Transfer — use tx-bound id
        let payer_full = topics[1].as_str().unwrap_or_default().to_lowercase();
        let payer = format!("0x{}", &payer_full[payer_full.len().saturating_sub(40)..]);
        let tx = log["transactionHash"].as_str().unwrap_or_default().to_string();
        let nonce = format!("0x{}", &tx[2..66]); // tx-derived unique id for reconciled rows
        checked += 1;
        let exists = db.prepare("SELECT id FROM settlements WHERE (payer = ?1 AND nonce = ?2) OR tx_hash = ?3")
            .bind(&[payer.clone().into(), nonce.clone().into(), tx.clone().into()])?
            .first::<serde_json::Value>(None).await?;
        if exists.is_none() {
            db.prepare("INSERT OR IGNORE INTO settlements(id, payer, nonce, request_id, tool, input_hash, status, scheme, network, asset, amount, pay_to, payment_payload, tx_hash, settle_network, failure_reason) VALUES (?1,?2,?3,'reconciled','unknown','rh','settled','exact',?4,?5,'0','0','{}',?6,?4,'reconciled_from_chain')")
                .bind(&[
                    nonce.clone().into(), payer.into(), nonce.clone().into(),
                    format!("eip155:{chain}").into(), token.clone().into(), tx.into(),
                ])?
                .run().await?;
            backfilled += 1;
        }
    }
    db.prepare("INSERT INTO reconciliation_runs(run_id, started_at, finished_at, checked, backfilled, divergent, notes) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ','now'), strftime('%Y-%m-%dT%H:%M:%fZ','now'), ?2, ?3, 0, ?4)")
        .bind(&[format!("run-{}", Date::now().as_millis()).into(), (checked as f64).into(), (backfilled as f64).into(), format!("chain events in window").into()])?
        .run().await?;
    // reset the hourly settlement_pending counter (breaker window)
    let _ = env.kv("PRICING")?.delete("ops:settle_pending_count").await;
    console_log!("G7 reconcile: {checked} chain events, {backfilled} phantoms backfilled");
    Ok(())
}

async fn probe_supported(env: &Env) -> Result<()> {
    // G7/G8: GET /supported — assert our scheme+network is still offered;
    // open the breaker if not (I5: fail closed on money).
    let chain: u64 = env.var("CHAIN_ID")?.to_string().parse().unwrap_or(84532);
    let want = format!("eip155:{chain}");
    let mut init = RequestInit::new();
    init.with_method(Method::Get);
    let req = Request::new_with_init("https://api.cdp.coinbase.com/platform/v2/x402/supported", &init)?;
    let mut resp = Fetch::Request(req).send().await?;
    let body: serde_json::Value = resp.json().await?;
    let ok = body["kinds"].as_array().map(|k| {
        k.iter().any(|x| x["network"].as_str() == Some(want.as_str()) && x["scheme"].as_str() == Some("exact"))
    }).unwrap_or(false);
    let kv = env.kv("PRICING")?;
    if ok {
        let _ = kv.delete("ops:facilitator_breaker").await; // only the health probe lifts it
    } else {
        kv.put("ops:facilitator_breaker", "open")?.execute().await?;
        console_error!("G7: /supported no longer lists {want}:exact — breaker OPEN");
    }
    Ok(())
}

async fn rpc_call(rpc: &str, method: &str, params: serde_json::Value) -> Result<String> {
    let payload = serde_json::json!({"jsonrpc":"2.0","id":1,"method":method,"params":params});
    let mut headers = Headers::new();
    headers.set("content-type", "application/json")?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post);
    init.with_headers(headers);
    init.with_body(Some(payload.to_string().into()));
    let req = Request::new_with_init(rpc, &init)?;
    let mut resp = Fetch::Request(req).send().await?;
    resp.text().await
}
