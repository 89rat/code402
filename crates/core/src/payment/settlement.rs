//! Stage 4 claim machine (plan-rev3 G3): pure state machine + storage trait.
//! `claimed -> settling -> settled | failed` with an alarm-style lease so a
//! crashed isolate mid-settle never wedges a nonce; `receipt_pending` for
//! money-moved-without-our-record (G2d, cron backfills). The Durable Object
//! in the edge worker wraps this exact logic with real storage; the TLA+
//! artifact (specs/model/claim.tla) mirrors it for review, and the
//! exhaustive interleaving test is the runnable model-check.
//!
//! The stored response (G2b) is what makes retries, race losers, and
//! duplicate deliveries receive IDENTICAL 200s.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use alloy_primitives::keccak256;

/// Lease: a claim in `claimed`/`settling` older than this (unix secs) with no
/// outcome is retriable — the isolate that held it is presumed dead.
pub const LEASE_SECS: u64 = 120;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ClaimStatus {
    Claimed,
    Settling,
    Settled,
    Failed,
    ReceiptPending,
    /// RECONCILER-SPEC v1: the chain proved the nonce was consumed by a
    /// transfer, but we hold no stored response — the payer is owed ONE free
    /// execution until `reconciled_eligible_until` (G2 entitlement). Transitions
    /// to `Settled` when the entitled execution stores its response.
    SettledReconciled,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClaimRow {
    pub status: ClaimStatus,
    pub payer: String,
    pub nonce: String,
    pub request_id: String,
    pub tool: String,
    pub input_hash: String,
    /// set at claim; lease base for claimed/settling
    pub claimed_at: u64,
    /// settle outcome (Settled only)
    pub tx_hash: Option<String>,
    pub network: Option<String>,
    /// persisted tool response for replay (G2b); None until Settled
    pub response_body: Option<Vec<u8>>,
    pub payment_response_b64: Option<String>,
    pub failure_reason: Option<String>,
    /// entitlement deadline (unix) while status == SettledReconciled (spec §3.C.1)
    #[serde(default)]
    pub reconciled_eligible_until: Option<u64>,
}

/// Storage seam: the DO (production), D1 (record), or MemStore (tests).
pub trait ClaimStore {
    fn load(&self, key: &str) -> Result<Option<ClaimRow>, String>;
    fn save(&mut self, key: &str, row: ClaimRow) -> Result<(), String>;
}

pub struct ClaimInput {
    pub payer: String,
    pub nonce: String,
    pub request_id: String,
    pub tool: String,
    pub input_hash: String,
    pub now_unix: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClaimTransition {
    /// This caller won the claim race; proceed to settle.
    Claimed,
    /// Another holder is mid-flight; the caller must wait briefly and retry
    /// (the loser of the race — they will receive the stored replay).
    InProgress,
    /// Prior lease expired (holder presumed crashed); this caller re-claimed.
    LeaseExpired,
    /// A settled record exists: replay the STORED response (identical 200).
    Replay { response_body: Vec<u8>, payment_response_b64: String, tx_hash: String },
    /// Money moved without our record — reconcile via cron before trusting.
    ReceiptPending,
    /// Terminal failure for this authorization.
    Failed,
    /// Terminal state reached; this authorization is spent/dead.
    Terminal,
    Settling,
    Settled,
    /// Chain-proved settlement with no stored response: the caller executes
    /// the tool FREE, once, before the entitlement deadline (reconciler G2).
    /// input_hash binds the free execution to the ORIGINAL request (G2c) —
    /// the entitlement is not a compute oracle.
    Entitled { tx_hash: String, network: String, input_hash: String },
    /// Reconciler audit transition: claim moved to settled_reconciled.
    SettledReconciled,
}

#[derive(Clone, Copy)]
pub struct SettlementClaimMachine;

impl SettlementClaimMachine {
    /// DO id semantics (G3): hash(from ‖ nonce).
    pub fn key_for(payer: &str, nonce: &str) -> String {
        let mut b = Vec::with_capacity(payer.len() + nonce.len());
        b.extend_from_slice(payer.as_bytes());
        b.extend_from_slice(nonce.as_bytes());
        format!("0x{}", {
            let h = keccak256(&b);
            h.iter().map(|x| format!("{x:02x}")).collect::<String>()
        })
    }

    fn fresh_row(i: &ClaimInput) -> ClaimRow {
        ClaimRow {
            status: ClaimStatus::Claimed,
            payer: i.payer.clone(),
            nonce: i.nonce.clone(),
            request_id: i.request_id.clone(),
            tool: i.tool.clone(),
            input_hash: i.input_hash.clone(),
            claimed_at: i.now_unix,
            tx_hash: None,
            network: None,
            response_body: None,
            payment_response_b64: None,
            failure_reason: None,
            reconciled_eligible_until: None,
        }
    }

    /// Attempt to claim (from, nonce). Idempotent replay of a SETTLED claim
    /// returns the stored response immediately (G2b).
    pub fn claim<S: ClaimStore>(&self, s: &mut S, i: &ClaimInput) -> Result<ClaimTransition, String> {
        let key = Self::key_for(&i.payer, &i.nonce);
        match s.load(&key)? {
            None => {
                s.save(&key, Self::fresh_row(i))?;
                Ok(ClaimTransition::Claimed)
            }
            Some(row) => match row.status {
                ClaimStatus::Settled => {
                    let body = row.response_body.clone();
                    let b64 = row.payment_response_b64.clone();
                    let tx = row.tx_hash.clone();
                    match (body, b64, tx) {
                        (Some(b), Some(h), Some(t)) => Ok(ClaimTransition::Replay {
                            response_body: b,
                            payment_response_b64: h,
                            tx_hash: t,
                        }),
                        // settled without a persisted response (non_replayable
                        // class): the caller serves fresh, never re-settles
                        _ => Ok(ClaimTransition::Terminal),
                    }
                }
                ClaimStatus::Failed | ClaimStatus::ReceiptPending => Ok(ClaimTransition::Terminal),
                ClaimStatus::SettledReconciled => {
                    // reconciler entitlement: paid on chain, never served —
                    // ONE free execution until the deadline, then dead.
                    let until = row.reconciled_eligible_until.unwrap_or(0);
                    if i.now_unix <= until {
                        Ok(ClaimTransition::Entitled {
                            tx_hash: row.tx_hash.clone().unwrap_or_default(),
                            network: row.network.clone().unwrap_or_default(),
                            input_hash: row.input_hash.clone(),
                        })
                    } else {
                        Ok(ClaimTransition::Terminal)
                    }
                }
                ClaimStatus::Claimed | ClaimStatus::Settling => {
                    if i.now_unix.saturating_sub(row.claimed_at) > LEASE_SECS {
                        // holder presumed dead: free the nonce, re-claim
                        s.save(&key, Self::fresh_row(i))?;
                        Ok(ClaimTransition::LeaseExpired)
                    } else {
                        Ok(ClaimTransition::InProgress)
                    }
                }
            },
        }
    }

    pub fn begin_settle<S: ClaimStore>(
        &self,
        s: &mut S,
        key: &str,
        now_unix: u64,
    ) -> Result<ClaimTransition, String> {
        match s.load(key)? {
            Some(mut row) if row.status == ClaimStatus::Claimed => {
                row.status = ClaimStatus::Settling;
                row.claimed_at = now_unix; // lease refreshes for the settle leg
                s.save(key, row)?;
                Ok(ClaimTransition::Settling)
            }
            Some(row) => Err(format!("begin_settle from {:?}", row.status)),
            None => Err("no such claim".into()),
        }
    }

    /// Settle succeeded: persist the response + PAYMENT-RESPONSE for replay.
    pub fn settled<S: ClaimStore>(
        &self,
        s: &mut S,
        key: &str,
        _i: &ClaimInput,
        tx_hash: &str,
        network: &str,
        response_body: &[u8],
        payment_response_b64: &str,
    ) -> Result<ClaimTransition, String> {
        match s.load(key)? {
            Some(mut row) if matches!(row.status, ClaimStatus::Settling | ClaimStatus::Claimed | ClaimStatus::SettledReconciled) => {
                row.status = ClaimStatus::Settled;
                row.tx_hash = Some(tx_hash.to_string());
                row.network = Some(network.to_string());
                // >256KB responses are non-replayable (G10): keep the flag,
                // drop the body (callers serve fresh, never re-settle)
                if response_body.len() <= 256 * 1024 {
                    row.response_body = Some(response_body.to_vec());
                    row.payment_response_b64 = Some(payment_response_b64.to_string());
                }
                s.save(key, row)?;
                Ok(ClaimTransition::Settled)
            }
            Some(row) => Err(format!("settled from {:?}", row.status)),
            None => Err("no such claim".into()),
        }
    }

    /// G2d: facilitator reports authorization-already-used and we hold no
    /// settled record — record receipt_pending; cron reconciles on-chain.
    pub fn receipt_pending<S: ClaimStore>(&self, s: &mut S, key: &str) -> Result<ClaimTransition, String> {
        match s.load(key)? {
            Some(mut row) if matches!(row.status, ClaimStatus::Settling | ClaimStatus::Claimed) => {
                row.status = ClaimStatus::ReceiptPending;
                s.save(key, row)?;
                Ok(ClaimTransition::ReceiptPending)
            }
            Some(row) => Err(format!("receipt_pending from {:?}", row.status)),
            None => Err("no such claim".into()),
        }
    }

    pub fn failed<S: ClaimStore>(
        &self,
        s: &mut S,
        key: &str,
        reason: &str,
    ) -> Result<ClaimTransition, String> {
        match s.load(key)? {
            Some(mut row) if matches!(row.status, ClaimStatus::Settling | ClaimStatus::Claimed) => {
                row.status = ClaimStatus::Failed;
                row.failure_reason = Some(reason.to_string());
                s.save(key, row)?;
                Ok(ClaimTransition::Failed)
            }
            Some(row) => Err(format!("failed from {:?}", row.status)),
            None => Err("no such claim".into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Pure step functions — the async-free core of each transition. The Durable
// Object (edge) does: async load -> step -> async save. The machine methods
// above compose these with ClaimStore; tests exercise both paths.
// ---------------------------------------------------------------------------

impl SettlementClaimMachine {
    pub fn claim_step(
        existing: Option<ClaimRow>,
        i: &ClaimInput,
    ) -> (Option<ClaimRow>, ClaimTransition) {
        match existing {
            None => (Some(Self::fresh_row(i)), ClaimTransition::Claimed),
            Some(row) => match row.status {
                ClaimStatus::Settled => {
                    let body = row.response_body.clone();
                    let b64 = row.payment_response_b64.clone();
                    let tx = row.tx_hash.clone();
                    match (body, b64, tx) {
                        (Some(b), Some(h), Some(t)) => {
                            (None, ClaimTransition::Replay { response_body: b, payment_response_b64: h, tx_hash: t })
                        }
                        _ => (None, ClaimTransition::Terminal),
                    }
                }
                ClaimStatus::Failed | ClaimStatus::ReceiptPending => (None, ClaimTransition::Terminal),
                ClaimStatus::SettledReconciled => {
                    let until = row.reconciled_eligible_until.unwrap_or(0);
                    if i.now_unix <= until {
                        (
                            None,
                            ClaimTransition::Entitled {
                                tx_hash: row.tx_hash.clone().unwrap_or_default(),
                                network: row.network.clone().unwrap_or_default(),
                                input_hash: row.input_hash.clone(),
                            },
                        )
                    } else {
                        (None, ClaimTransition::Terminal)
                    }
                }
                ClaimStatus::Claimed | ClaimStatus::Settling => {
                    if i.now_unix.saturating_sub(row.claimed_at) > LEASE_SECS {
                        (Some(Self::fresh_row(i)), ClaimTransition::LeaseExpired)
                    } else {
                        (None, ClaimTransition::InProgress)
                    }
                }
            },
        }
    }

    pub fn begin_settle_step(
        existing: Option<ClaimRow>,
        now_unix: u64,
    ) -> Result<(Option<ClaimRow>, ClaimTransition), String> {
        match existing {
            Some(mut row) if row.status == ClaimStatus::Claimed => {
                row.status = ClaimStatus::Settling;
                row.claimed_at = now_unix;
                Ok((Some(row), ClaimTransition::Settling))
            }
            Some(row) => Err(format!("begin_settle from {:?}", row.status)),
            None => Err("no such claim".into()),
        }
    }

    pub fn settled_step(
        existing: Option<ClaimRow>,
        tx_hash: &str,
        network: &str,
        response_body: &[u8],
        payment_response_b64: &str,
    ) -> Result<(Option<ClaimRow>, ClaimTransition), String> {
        match existing {
            Some(mut row) if matches!(row.status, ClaimStatus::Settling | ClaimStatus::Claimed | ClaimStatus::SettledReconciled) => {
                row.status = ClaimStatus::Settled;
                row.tx_hash = Some(tx_hash.to_string());
                row.network = Some(network.to_string());
                if response_body.len() <= 256 * 1024 {
                    row.response_body = Some(response_body.to_vec());
                    row.payment_response_b64 = Some(payment_response_b64.to_string());
                }
                Ok((Some(row), ClaimTransition::Settled))
            }
            Some(row) => Err(format!("settled from {:?}", row.status)),
            None => Err("no such claim".into()),
        }
    }

    pub fn terminal_step(
        existing: Option<ClaimRow>,
        to: ClaimStatus,
        reason: Option<&str>,
    ) -> Result<(Option<ClaimRow>, ClaimTransition), String> {
        let t = match to {
            ClaimStatus::Failed => ClaimTransition::Failed,
            ClaimStatus::ReceiptPending => ClaimTransition::ReceiptPending,
            _ => return Err("terminal_step to non-terminal".into()),
        };
        match existing {
            Some(mut row) if matches!(row.status, ClaimStatus::Settling | ClaimStatus::Claimed) => {
                row.status = to;
                row.failure_reason = reason.map(|r| r.to_string());
                Ok((Some(row), t))
            }
            Some(row) => Err(format!("terminal_step from {:?}", row.status)),
            None => Err("no such claim".into()),
        }
    }

    /// RECONCILER-SPEC §3.C.1: the chain says AuthorizationUsed — record the
    /// on-chain settle (tx from the log) and grant the replay entitlement.
    /// ReceiptPending is a legal source: the chain just resolved the "maybe
    /// used" hypothesis in favour of "used".
    pub fn reconcile_settled_step(
        existing: Option<ClaimRow>,
        tx_hash: &str,
        network: &str,
        eligible_until: u64,
    ) -> Result<(Option<ClaimRow>, ClaimTransition), String> {
        match existing {
            Some(mut row)
                if matches!(
                    row.status,
                    ClaimStatus::Claimed | ClaimStatus::Settling | ClaimStatus::ReceiptPending
                ) =>
            {
                row.status = ClaimStatus::SettledReconciled;
                row.tx_hash = Some(tx_hash.to_string());
                row.network = Some(network.to_string());
                row.reconciled_eligible_until = Some(eligible_until);
                row.failure_reason = None;
                Ok((Some(row), ClaimTransition::SettledReconciled))
            }
            Some(row) => Err(format!("reconcile_settled from {:?}", row.status)),
            None => Err("no such claim".into()),
        }
    }

    /// RECONCILER-SPEC §3.C.1/C.2: the chain says Canceled (or the window
    /// expired unused) — terminal failure with the chain-truth reason. Never
    /// reachable from a terminal state (absorbing law).
    pub fn reconcile_failed_step(
        existing: Option<ClaimRow>,
        reason: &str,
    ) -> Result<(Option<ClaimRow>, ClaimTransition), String> {
        match existing {
            Some(mut row)
                if matches!(
                    row.status,
                    ClaimStatus::Claimed | ClaimStatus::Settling | ClaimStatus::ReceiptPending
                ) =>
            {
                row.status = ClaimStatus::Failed;
                row.failure_reason = Some(reason.to_string());
                Ok((Some(row), ClaimTransition::Failed))
            }
            Some(row) => Err(format!("reconcile_failed from {:?}", row.status)),
            None => Err("no such claim".into()),
        }
    }
}

impl SettlementClaimMachine {
    /// Store-composed wrappers for the reconciler steps (the DO calls the
    /// step functions directly; these serve tests and future callers).
    pub fn reconcile_settled<S: ClaimStore>(
        &self,
        s: &mut S,
        key: &str,
        tx_hash: &str,
        network: &str,
        eligible_until: u64,
    ) -> Result<ClaimTransition, String> {
        let (row, t) = Self::reconcile_settled_step(s.load(key)?, tx_hash, network, eligible_until)?;
        if let Some(r) = row {
            s.save(key, r)?;
        }
        Ok(t)
    }

    pub fn reconcile_failed<S: ClaimStore>(
        &self,
        s: &mut S,
        key: &str,
        reason: &str,
    ) -> Result<ClaimTransition, String> {
        let (row, t) = Self::reconcile_failed_step(s.load(key)?, reason)?;
        if let Some(r) = row {
            s.save(key, r)?;
        }
        Ok(t)
    }
}
