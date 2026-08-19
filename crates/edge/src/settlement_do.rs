//! SettlementClaim Durable Object — single-threaded mutual exclusion per
//! (payer, nonce), running the pure core state machine (payment::settlement)
//! over DO storage. JSON command API keeps the route layer thin; every
//! command returns the serialized ClaimTransition.

use m2m_core::payment::settlement::{ClaimInput, ClaimRow, ClaimStatus, ClaimTransition, SettlementClaimMachine};
use worker::*;

/// Async DO storage around the PURE core step functions: load -> step -> save.
struct ClaimIo<'a> { state: &'a State }

impl ClaimIo<'_> {
    async fn load(&self, key: &str) -> std::result::Result<Option<ClaimRow>, String> {
        let v = self.state.storage().get::<serde_json::Value>(&format!("claim:{key}"))
            .await.map_err(|e| format!("do get: {e}"));
        match v {
            Ok(v) => serde_json::from_value::<ClaimRow>(v).map(Some).map_err(|e| e.to_string()),
            Err(e) if e.to_string().contains("NotFoundError") || e.to_string().contains("not found") || e.to_string().contains("No such") => Ok(None),
            Err(e) => Err(format!("do get: {e}")),
        }
    }
    async fn save(&self, key: &str, row: &ClaimRow) -> std::result::Result<(), String> {
        let json = serde_json::to_value(row).map_err(|e| e.to_string())?;
        self.state.storage().put(&format!("claim:{key}"), json)
            .await.map_err(|e| format!("do put: {e}"))
    }
}

fn transition_json(t: ClaimTransition) -> serde_json::Value {
    match t {
        ClaimTransition::Claimed => serde_json::json!({"kind": "claimed"}),
        ClaimTransition::InProgress => serde_json::json!({"kind": "in_progress"}),
        ClaimTransition::LeaseExpired => serde_json::json!({"kind": "lease_expired"}),
        ClaimTransition::Replay { response_body, payment_response_b64, tx_hash } => {
            use base64::Engine;
            let rb = base64::engine::general_purpose::STANDARD.encode(response_body);
            serde_json::json!({
                "kind": "replay",
                "response_body_b64": rb,
                "payment_response": payment_response_b64,
                "tx_hash": tx_hash,
            })
        }
        ClaimTransition::ReceiptPending => serde_json::json!({"kind": "receipt_pending"}),
        ClaimTransition::Failed => serde_json::json!({"kind": "failed"}),
        ClaimTransition::Terminal => serde_json::json!({"kind": "terminal"}),
        ClaimTransition::Settling => serde_json::json!({"kind": "settling"}),
        ClaimTransition::Settled => serde_json::json!({"kind": "settled"}),
        // RECONCILER-SPEC v1: chain-proved settlement with an owed execution
        ClaimTransition::Entitled { tx_hash, network } => serde_json::json!({
            "kind": "entitled", "tx_hash": tx_hash, "network": network,
        }),
        ClaimTransition::SettledReconciled => serde_json::json!({"kind": "settled_reconciled"}),
    }
}

#[durable_object]
pub struct SettlementClaim { state: State, _env: Env }

#[durable_object]
impl DurableObject for SettlementClaim {
    fn new(state: State, env: Env) -> Self { Self { state, _env: env } }

    async fn fetch(&mut self, req: Request) -> Result<Response> {
        let url = req.url()?;
        if url.path() != "/cmd" {
            return Response::error("not found", 404);
        }
        let mut req = req;
        let body: serde_json::Value = match req.json().await {
            Ok(b) => b,
            Err(e) => return Response::error(format!("bad cmd: {e}"), 400),
        };
        let io = ClaimIo { state: &self.state };
        let cmd = body.get("cmd").and_then(|c| c.as_str()).unwrap_or_default().to_string();
        let key = body.get("key").and_then(|k| k.as_str()).unwrap_or_default().to_string();
        let out: std::result::Result<ClaimTransition, String> = match cmd.as_str() {
            "claim" => {
                let i = parse_input(&body);
                let existing = io.load(&key).await?;
                let (row, t) = SettlementClaimMachine::claim_step(existing, &i);
                if let Some(r) = row { io.save(&key, &r).await?; }
                Ok(t)
            }
            "begin_settle" => {
                let now = body.get("now").and_then(|n| n.as_u64()).unwrap_or_default();
                let existing = io.load(&key).await?;
                let (row, t) = SettlementClaimMachine::begin_settle_step(existing, now)?;
                if let Some(r) = row { io.save(&key, &r).await?; }
                Ok(t)
            }
            "settled" => {
                let tx = body.get("tx").and_then(|t| t.as_str()).unwrap_or_default();
                let net = body.get("network").and_then(|n| n.as_str()).unwrap_or_default();
                let rb64 = body.get("response_body_b64").and_then(|b| b.as_str()).unwrap_or_default();
                let pr = body.get("payment_response").and_then(|p| p.as_str()).unwrap_or_default();
                use base64::Engine;
                let rb = base64::engine::general_purpose::STANDARD
                    .decode(rb64).map_err(|e| format!("response b64: {e}"))?;
                let existing = io.load(&key).await?;
                let (row, t) = SettlementClaimMachine::settled_step(existing, tx, net, &rb, pr)?;
                if let Some(r) = row { io.save(&key, &r).await?; }
                Ok(t)
            }
            "failed" => {
                let reason = body.get("reason").and_then(|r| r.as_str()).unwrap_or("unknown");
                let existing = io.load(&key).await?;
                let (row, t) = SettlementClaimMachine::terminal_step(existing, ClaimStatus::Failed, Some(reason))?;
                if let Some(r) = row { io.save(&key, &r).await?; }
                Ok(t)
            }
            "receipt_pending" => {
                let existing = io.load(&key).await?;
                let (row, t) = SettlementClaimMachine::terminal_step(existing, ClaimStatus::ReceiptPending, None)?;
                if let Some(r) = row { io.save(&key, &r).await?; }
                Ok(t)
            }
            // RECONCILER-SPEC v1 §3 (cron write-back): the chain resolved the
            // claim; the pure core steps decide legality (absorbing law).
            "reconcile_settled" => {
                let tx = body.get("tx").and_then(|t| t.as_str()).unwrap_or_default();
                let net = body.get("network").and_then(|n| n.as_str()).unwrap_or_default();
                let until = body.get("eligible_until").and_then(|u| u.as_u64()).unwrap_or_default();
                let existing = io.load(&key).await?;
                let (row, t) = SettlementClaimMachine::reconcile_settled_step(existing, tx, net, until)?;
                if let Some(r) = row { io.save(&key, &r).await?; }
                Ok(t)
            }
            "reconcile_failed" => {
                let reason = body.get("reason").and_then(|r| r.as_str()).unwrap_or("reconciled");
                let existing = io.load(&key).await?;
                let (row, t) = SettlementClaimMachine::reconcile_failed_step(existing, reason)?;
                if let Some(r) = row { io.save(&key, &r).await?; }
                Ok(t)
            }
            other => Err(format!("unknown cmd {other:?}")),
        };
        match out {
            Ok(t) => Response::from_json(&transition_json(t)),
            Err(e) => Response::error(e, 500),
        }
    }
}

fn parse_input(body: &serde_json::Value) -> ClaimInput {
    let i = body.get("input").cloned().unwrap_or_default();
    ClaimInput {
        payer: i.get("payer").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        nonce: i.get("nonce").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        request_id: i.get("request_id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        tool: i.get("tool").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        input_hash: i.get("input_hash").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        now_unix: i.get("now_unix").and_then(|v| v.as_u64()).unwrap_or_default(),
    }
}
