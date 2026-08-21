//! RECONCILER-SPEC v1 §3 wiring (reviews/reconciler-spec-v1.md): the hourly
//! stale-claim sweep. The chain is root of truth (I4) — batched
//! `authorizationState` reads (JSON-RPC batch transport; one HTTP POST per
//! chunk of claims — same RPC budget as a Multicall3 aggregate, without
//! hand-rolling dynamic ABI encoding), `eth_getLogs` disambiguation of
//! Used-vs-Canceled, the PURE core resolver decides, the DO write-back makes
//! the replay-entitlement live, D1 records with guarded (absorbing) UPDATEs.
//!
//! Runs even when the payment kill-switch is on — the reconciler is the
//! janitor, not a payer. The re-drive action alone is gated by the
//! kill-switch AND the facilitator breaker (spec §3.C.3).

use crate::facilitator::select_for_ops;
use crate::x402v2_route::{breaker_open, claim_do, v2_enabled};
use m2m_core::payment::reconciler::{
    classify_consuming_log, classify_settle_failure, decode_consumed_word,
    encode_authorization_state_call, evidence_from_reads, pad32_address, resolve, ConsumingLog,
    Resolution, SettleFailureClass, REPLAY_TTL_SECS,
};
use m2m_core::payment::x402v2::{FacilitatorRequest, PaymentPayload};
use worker::*;

/// Staleness threshold (spec §1): must exceed the DO lease (120s) plus settle
/// latency so the sweep never races a live request-path attempt.
pub const RECONCILER_LEASE_SECS: i64 = 300;
const CLOCK_SKEW_SECS: u64 = 30;
const SETTLE_MARGIN_SECS: u64 = 30;
const MAX_CLAIMS_PER_RUN: usize = 500;
const CHUNK: usize = 20; // JSON-RPC batch size (also LOGS_BATCH)
const LOGS_WINDOW_BLOCKS: u64 = 10_000; // provider-safe getLogs range
const BASE_WINDOWS: u64 = 5; // 50k blocks ≈ 27h at 2s (spec MAX_LOOKBACK_BLOCKS)
const DEEP_WINDOWS: u64 = 16; // age > 24h: deep_scan ≈ 3.7 days of blocks
const RUN_BUDGET_MS: u64 = 45_000; // unfinished rows simply wait for next run
/// Absorbing-state law: only these statuses may receive a terminal write.
const NON_TERMINAL: &str = "('claimed','settling','receipt_pending','settlement_pending')";

#[derive(Default, Debug, serde::Serialize)]
pub struct SweepStats {
    pub scanned: u64,
    pub consumed: u64,
    pub resolved_used: u64,
    pub resolved_canceled: u64,
    pub resolved_expired: u64,
    pub redriven: u64,
    pub redrive_settled: u64,
    pub left_ambiguous: u64,
    pub errors: u64,
}

pub async fn sweep(env: &Env) -> Result<SweepStats> {
    let started = Date::now().as_millis();
    let db = env.d1("LEDGER")?;
    let rpc = env.secret("RPC_PRIMARY")?.to_string();
    let token = env.var("USDC_BASE")?.to_string().to_lowercase();
    // M6 (wide-angle 2026-08-19): fail CLOSED like the route (red-team
    // Break 4) — a silent Sepolia default would make the reconciler mark live
    // mainnet claims expired/absent.
    let chain_id: u64 = env
        .var("CHAIN_ID")?
        .to_string()
        .parse()
        .map_err(|_| Error::RustError("CHAIN_ID var invalid — reconciler fails closed".into()))?;
    let network = format!("eip155:{chain_id}");
    let now = ((Date::now().as_millis() / 1000) as u64).max(1);
    let latest = block_number(&rpc).await?;

    let rows = db
        .prepare(
            "SELECT id, payer, nonce, payment_payload, \
                    CAST(strftime('%s', updated_at) AS INTEGER) AS updated_epoch \
             FROM settlements \
             WHERE status IN ('claimed','settling','receipt_pending','settlement_pending') \
               AND updated_at < strftime('%Y-%m-%dT%H:%M:%fZ','now', ?1) \
             ORDER BY updated_at ASC LIMIT ?2",
        )
        .bind(&[
            format!("-{RECONCILER_LEASE_SECS} seconds").into(),
            (MAX_CLAIMS_PER_RUN as f64).into(),
        ])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    let mut st = SweepStats { scanned: rows.len() as u64, ..Default::default() };
    // re-drive is the ONLY kill-switch-gated action (spec §3.C.3); the breaker
    // guards the cron exactly as it guards the request path.
    let redrive_allowed = v2_enabled(env).await && !breaker_open(env).await;

    for chunk in rows.chunks(CHUNK) {
        if Date::now().as_millis() - started > RUN_BUDGET_MS {
            break; // resumable by construction: unresolved rows stay stale
        }

        // ---- step B: batched authorizationState(authorizer, nonce) ----
        let mut consumed: Vec<Option<bool>> = vec![None; chunk.len()];
        let batch: Vec<serde_json::Value> = chunk
            .iter()
            .enumerate()
            .filter_map(|(i, r)| {
                encode_authorization_state_call(
                    r["payer"].as_str().unwrap_or_default(),
                    r["nonce"].as_str().unwrap_or_default(),
                )
                .ok()
                .map(|data| {
                    serde_json::json!({
                        "jsonrpc": "2.0", "id": i, "method": "eth_call",
                        "params": [{"to": token, "data": data}, "latest"],
                    })
                })
            })
            .collect();
        if let Ok(serde_json::Value::Array(items)) =
            rpc_post(&rpc, serde_json::to_string(&batch)?).await
        {
            for it in items {
                if let Some(id) = it["id"].as_u64() {
                    let i = id as usize;
                    if i < chunk.len() {
                        consumed[i] = it["result"].as_str().and_then(decode_consumed_word);
                    }
                }
            }
        }

        // ---- step C: three-way resolve per claim ----
        let mut to_disambiguate: Vec<usize> = Vec::new();
        for (i, r) in chunk.iter().enumerate() {
            match consumed[i] {
                // inner-call failure or malformed word: ambiguous THIS run
                None => st.left_ambiguous += 1,
                Some(false) => {
                    let vb = valid_before_of(r["payment_payload"].as_str().unwrap_or_default());
                    let ev = evidence_from_reads(false, None, None, vb, now, CLOCK_SKEW_SECS);
                    match resolve(&ev, now) {
                        Resolution::FailedExpired => {
                            if do_reconcile_failed(env, r, "reconciled_expired").await {
                                d1_resolve(&db, r, "failed_expired", "reconciled_expired", None, None).await;
                                st.resolved_expired += 1;
                            } else {
                                st.errors += 1;
                            }
                        }
                        Resolution::ReDrive if redrive_allowed => {
                            redrive(env, &db, r, now, &mut st).await
                        }
                        // ReDrive while gated: leave untouched; the expiry
                        // branch resolves it within one validity window.
                        _ => {}
                    }
                }
                Some(true) => {
                    st.consumed += 1;
                    to_disambiguate.push(i);
                }
            }
        }

        // ---- step C.1 disambiguation: window-major batched getLogs ----
        if !to_disambiguate.is_empty() {
            let oldest = chunk
                .iter()
                .filter_map(|r| r["updated_epoch"].as_i64())
                .min()
                .unwrap_or(0);
            let deep = now.saturating_sub(oldest as u64) > 86_400;
            let windows = if deep { DEEP_WINDOWS } else { BASE_WINDOWS };
            let mut found: Vec<(Option<ConsumingLog>, Option<String>)> =
                vec![(None, None); chunk.len()];
            let mut remaining: Vec<usize> = to_disambiguate.clone();
            let topic_used = format!(
                "0x{}",
                alloy_primitives::hex::encode(
                    m2m_core::payment::reconciler::topic_authorization_used()
                )
            );
            let topic_canceled = format!(
                "0x{}",
                alloy_primitives::hex::encode(
                    m2m_core::payment::reconciler::topic_authorization_canceled()
                )
            );
            for w in 0..windows {
                if remaining.is_empty()
                    || Date::now().as_millis() - started > RUN_BUDGET_MS
                {
                    break;
                }
                let to_b = latest.saturating_sub(w * LOGS_WINDOW_BLOCKS);
                let from_b = latest.saturating_sub((w + 1) * LOGS_WINDOW_BLOCKS);
                if to_b == 0 {
                    break;
                }
                let body: Vec<serde_json::Value> = remaining
                    .iter()
                    .enumerate()
                    .map(|(j, &i)| {
                        let r = &chunk[i];
                        serde_json::json!({
                            "jsonrpc": "2.0", "id": j, "method": "eth_getLogs",
                            "params": [{
                                "address": token,
                                "topics": [
                                    [topic_used.clone(), topic_canceled.clone()],
                                    pad32_address(r["payer"].as_str().unwrap_or_default()),
                                    r["nonce"].as_str().unwrap_or_default(),
                                ],
                                "fromBlock": format!("0x{from_b:x}"),
                                "toBlock": format!("0x{to_b:x}"),
                            }],
                        })
                    })
                    .collect();
                match rpc_post(&rpc, serde_json::to_string(&body)?).await {
                    Ok(serde_json::Value::Array(items)) => {
                        let mut still: Vec<usize> = Vec::new();
                        for (j, &i) in remaining.iter().enumerate() {
                            let logs = items
                                .iter()
                                .find(|it| it["id"].as_u64() == Some(j as u64))
                                .and_then(|it| it["result"].as_array().cloned())
                                .unwrap_or_default();
                            let mut hit = false;
                            for lg in &logs {
                                let t0 = lg["topics"].as_array()
                                    .and_then(|t| t.first())
                                    .and_then(|t| t.as_str())
                                    .unwrap_or_default();
                                if let Some(kind) = classify_consuming_log(t0) {
                                    found[i] = (
                                        Some(kind),
                                        lg["transactionHash"].as_str().map(String::from),
                                    );
                                    hit = true;
                                    break;
                                }
                            }
                            if !hit {
                                still.push(i);
                            }
                        }
                        remaining = still;
                    }
                    _ => {
                        st.errors += 1;
                        break;
                    }
                }
            }
            for &i in &to_disambiguate {
                let r = &chunk[i];
                let (log, tx) = &found[i];
                let ev = evidence_from_reads(true, *log, tx.clone(), None, now, CLOCK_SKEW_SECS);
                match resolve(&ev, now) {
                    Resolution::SettledReconciled { tx_hash } => {
                        let until = now + REPLAY_TTL_SECS;
                        if do_reconcile_settled(env, r, &tx_hash, &network, until).await {
                            d1_resolve(&db, r, "settled_reconciled", "reconciled_used", Some(&tx_hash), Some(until)).await;
                            st.resolved_used += 1;
                        } else {
                            st.errors += 1;
                        }
                    }
                    Resolution::FailedCanceled => {
                        if do_reconcile_failed(env, r, "reconciled_canceled").await {
                            d1_resolve(&db, r, "failed_canceled", "reconciled_canceled", None, None).await;
                            st.resolved_canceled += 1;
                        } else {
                            st.errors += 1;
                        }
                    }
                    // neither event in any window: do not guess (spec §3.C.1)
                    Resolution::LeaveAmbiguous => {
                        st.left_ambiguous += 1;
                        if deep {
                            // m3: silent forever-ambiguous is invisible — escalate
                            console_error!(
                                "RECONCILER: nonce {} consumed but NO event found even after deep scan ({} blocks) — RPC indexer lag or wrong window",
                                r["nonce"].as_str().unwrap_or_default(),
                                DEEP_WINDOWS * LOGS_WINDOW_BLOCKS
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    record_run(env, &db, &st).await;
    Ok(st)
}

// ---------------------------------------------------------------------------
// apply helpers
// ---------------------------------------------------------------------------

/// DO write-back FIRST (the DO is the claim authority — if it refuses, the
/// row stays stale and retries next run; D1 never records a terminal state
/// the DO does not hold). IDEMPOTENT: a refusal because the DO is ALREADY in
/// the target terminal state (a previous run's DO write landed but the D1
/// update failed) counts as success — the D1 resolve then converges. Without
/// this, a single D1 hiccup wedges the row forever (red team #2).
async fn do_reconcile_settled(
    env: &Env,
    row: &serde_json::Value,
    tx: &str,
    network: &str,
    until: u64,
) -> bool {
    let key = row["id"].as_str().unwrap_or_default().to_string();
    let cmd = serde_json::json!({
        "cmd": "reconcile_settled", "key": key,
        "tx": tx, "network": network, "eligible_until": until,
    });
    match claim_do(env, &key, cmd).await {
        Ok(_) => true,
        // "reconcile_settled from SettledReconciled" = already resolved
        Err(e) if e.to_string().contains("reconcile_settled from SettledReconciled") => true,
        Err(_) => false,
    }
}

async fn do_reconcile_failed(env: &Env, row: &serde_json::Value, reason: &str) -> bool {
    let key = row["id"].as_str().unwrap_or_default().to_string();
    let cmd = serde_json::json!({"cmd": "reconcile_failed", "key": key, "reason": reason});
    match claim_do(env, &key, cmd).await {
        Ok(_) => true,
        // already failed by a previous run (D1 write is the straggler)
        Err(e) if e.to_string().contains("reconcile_failed from Failed") => true,
        Err(_) => false,
    }
}

/// Guarded terminal UPDATE (absorbing law): the WHERE clause is the law's
/// executable form — a terminal status can never be written over.
async fn d1_resolve(
    db: &worker::D1Database,
    row: &serde_json::Value,
    status: &str,
    resolution: &str,
    tx: Option<&str>,
    replay_until: Option<u64>,
) {
    use worker::wasm_bindgen::JsValue;
    let payer = row["payer"].as_str().unwrap_or_default();
    let nonce = row["nonce"].as_str().unwrap_or_default();
    let sql = format!(
        "UPDATE settlements SET status=?3, resolution=?4, resolution_tx=?5, \
         tx_hash=COALESCE(?5, tx_hash), \
         resolved_at=CAST(strftime('%s','now') AS INTEGER), replay_eligible_until=?6, \
         settled_at=CASE WHEN ?3 LIKE 'settled%' THEN strftime('%Y-%m-%dT%H:%M:%fZ','now') ELSE settled_at END, \
         updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') \
         WHERE payer=?1 AND nonce=?2 AND status IN {NON_TERMINAL}"
    );
    let _ = async {
        db.prepare(&sql)
            .bind(&[
                JsValue::from_str(payer),
                JsValue::from_str(nonce),
                JsValue::from_str(status),
                JsValue::from_str(resolution),
                tx.map(JsValue::from_str).unwrap_or(JsValue::NULL),
                replay_until
                    .map(|u| JsValue::from_f64(u as f64))
                    .unwrap_or(JsValue::NULL),
            ])?
            .run()
            .await?;
        Ok::<(), Error>(())
    }
    .await;
}

/// Spec §3.C.3: our settle never landed and the authorization is still
/// consumable — resubmit the IDENTICAL stored payload. "Already used" is NOT
/// success-blindness: the row is left for next run's chain disambiguation.
/// AGE-CAPPED at 48h: a garbage-but-gate-passing payload that CDP keeps
/// rejecting would otherwise re-drive every hour until its (attacker-chosen,
/// possibly year-2100) validBefore — the cap bounds any single row to ~48
/// attempts (red team #4).
async fn redrive(env: &Env, db: &worker::D1Database, row: &serde_json::Value, now: u64, st: &mut SweepStats) {
    if let Some(updated) = row["updated_epoch"].as_i64() {
        if now.saturating_sub(updated as u64) > 48 * 3600 {
            return; // past the re-drive age cap; expiry resolves it (or it never will — bounded garbage)
        }
    }
    let payload_json = row["payment_payload"].as_str().unwrap_or_default();
    let Ok(pp) = serde_json::from_str::<PaymentPayload>(payload_json) else {
        st.left_ambiguous += 1;
        return;
    };
    let Ok(vb) = pp.payload.authorization.valid_before_unix() else {
        st.left_ambiguous += 1;
        return;
    };
    if now + SETTLE_MARGIN_SECS >= vb {
        return; // too late to settle honestly; expiry branch resolves it
    }
    let Some(fac) = select_for_ops(env).await else {
        return;
    };
    let Ok(freq) = FacilitatorRequest::new(pp.clone(), pp.accepted.clone()) else {
        st.errors += 1;
        return;
    };
    st.redriven += 1;
    match fac.settle(&freq).await {
        Ok(sr) if sr.success => {
            let until = now + REPLAY_TTL_SECS;
            if do_reconcile_settled(env, row, &sr.transaction, &sr.network, until).await {
                // D1 must MATCH the DO (Kimi M1): the DO claim is
                // SettledReconciled-with-entitlement, so D1 is
                // 'settled_reconciled' too — otherwise d1_entitled() misses
                // re-drive rows after the 300s stamp grace and the payer's
                // late retry 400s instead of executing free.
                d1_resolve(db, row, "settled_reconciled", "facilitator", Some(&sr.transaction), Some(until)).await;
                st.redrive_settled += 1;
            } else {
                st.errors += 1;
            }
        }
        Ok(sr) if !sr.success
            && matches!(
                classify_settle_failure(
                    sr.error_reason.as_deref().unwrap_or("")
                ),
                SettleFailureClass::AmbiguousMoney
            ) =>
        {
            // already-used between our read and the settle (live CDP shape:
            // invalid_payload + the doomed replay tx hash): the chain will
            // prove it next run — never blind-success (G2d)
        }
        Ok(_) => {} // other rejections: leave; expiry resolves within a window
        Err(_) => st.errors += 1,
    }
}

fn valid_before_of(payload_json: &str) -> Option<u64> {
    serde_json::from_str::<PaymentPayload>(payload_json)
        .ok()?
        .payload
        .authorization
        .valid_before_unix()
        .ok()
}

// ---------------------------------------------------------------------------
// RPC plumbing (single + batch share one POST shape)
// ---------------------------------------------------------------------------

async fn rpc_post(rpc: &str, body: String) -> Result<serde_json::Value> {
    let mut headers = Headers::new();
    headers.set("content-type", "application/json")?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post);
    init.with_headers(headers);
    init.with_body(Some(body.into()));
    let req = Request::new_with_init(rpc, &init)?;
    let mut resp = Fetch::Request(req).send().await?;
    resp.json().await
}

async fn block_number(rpc: &str) -> Result<u64> {
    let v = rpc_post(
        rpc,
        serde_json::json!({"jsonrpc":"2.0","id":0,"method":"eth_blockNumber","params":[]}).to_string(),
    )
    .await?;
    v["result"]
        .as_str()
        .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .ok_or_else(|| Error::RustError("blockNumber".into()))
}

// ---------------------------------------------------------------------------
// run bookkeeping + alarms (spec §5)
// ---------------------------------------------------------------------------

async fn record_run(env: &Env, db: &worker::D1Database, st: &SweepStats) {
    use worker::wasm_bindgen::JsValue;
    let _ = async {
        db.prepare(
            "INSERT INTO reconciler_runs_v2(started_at, finished_at, scanned, resolved_used, \
             resolved_canceled, resolved_expired, redriven, left_ambiguous, error) \
             VALUES (CAST(strftime('%s','now') AS INTEGER), CAST(strftime('%s','now') AS INTEGER), \
             ?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(&[
            JsValue::from_f64(st.scanned as f64),
            JsValue::from_f64(st.resolved_used as f64),
            JsValue::from_f64(st.resolved_canceled as f64),
            JsValue::from_f64(st.resolved_expired as f64),
            JsValue::from_f64(st.redriven as f64),
            JsValue::from_f64(st.left_ambiguous as f64),
            JsValue::from_f64(st.errors as f64),
        ])?
        .run()
        .await?;
        Ok::<(), Error>(())
    }
    .await;
    // dead-man switch + backlog alarms (spec §5), surfaced via the stats KV
    let kv = match env.kv("PRICING") {
        Ok(kv) => kv,
        Err(_) => return,
    };
    // spec §5: ANY cancel against us is anomalous (possibly adversarial
    // probing) — investigate at >= 1 (Kimi m5)
    if st.resolved_canceled > 0 {
        console_error!("RECONCILER ALARM: {}/{} claims canceled on-chain this run — adversarial probing?",
            st.resolved_canceled, st.scanned);
        if let Ok(p) = kv.put("ops:canceled_last_run", st.resolved_canceled.to_string()) {
            let _ = p.execute().await;
        }
    }
    let now_ms = Date::now().as_millis().to_string();
    if let Ok(p) = kv.put("ops:reconciler_last_success", now_ms) {
        let _ = p.execute().await;
    }
    let backlog_sql = format!(
        "SELECT COUNT(*) AS c, CAST(strftime('%s','now') AS INTEGER) \
                - MIN(CAST(strftime('%s', updated_at) AS INTEGER)) AS age \
         FROM settlements \
         WHERE status IN {NON_TERMINAL} \
           AND updated_at < strftime('%Y-%m-%dT%H:%M:%fZ','now', ?1)"
    );
    let row = async {
        let r = db
            .prepare(&backlog_sql)
            .bind(&[format!("-{RECONCILER_LEASE_SECS} seconds").into()])?
            .first::<serde_json::Value>(None)
            .await?;
        Ok::<Option<serde_json::Value>, Error>(r)
    }
    .await;
    if let Ok(Some(row)) = row {
        if let Some(c) = row["c"].as_u64() {
            if let Ok(p) = kv.put("ops:stale_backlog", c.to_string()) {
                let _ = p.execute().await;
            }
        }
        if let Some(age) = row["age"].as_i64() {
            if let Ok(p) = kv.put("ops:oldest_stale_age", age.to_string()) {
                let _ = p.execute().await;
            }
        }
    }
    console_log!("RECONCILER sweep: {st:?}");
}
