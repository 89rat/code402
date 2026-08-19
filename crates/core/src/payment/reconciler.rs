//! RECONCILER-SPEC v1 core — the three-way stale-claim resolver as a PURE
//! function table (tests adjudicate; the cron calls it after chain reads).
//! reviews/reconciler-spec-v1.md §3.C. Terminal states are absorbing.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

/// Inputs from chain reads (multicall authorizationState + getLogs).
#[derive(Debug, Clone, PartialEq)]
pub enum ChainEvidence {
    /// authorizationState == true + AuthorizationUsed log found (tx hash).
    UsedOnChain { tx_hash: String },
    /// authorizationState == true + AuthorizationCanceled log found.
    CanceledOnChain,
    /// authorizationState == true but NEITHER event in the lookback window.
    /// Do not guess — leave stale, escalate to deep_scan next run.
    AmbiguousConsumed,
    /// authorizationState == false; authorization window has passed.
    ExpiredUnused,
    /// authorizationState == false; still inside validity (may re-drive).
    StillValid,
}

/// The resolver's verdict for one stale claim.
#[derive(Debug, Clone, PartialEq)]
pub enum Resolution {
    /// On-chain transfer confirmed → settled_reconciled, entitlement granted.
    SettledReconciled { tx_hash: String },
    /// Payer revoked → failed_canceled. Alarm-worthy (spec §5).
    FailedCanceled,
    /// Nonce dead, window passed → failed_expired. Final.
    FailedExpired,
    /// Not consumed, still valid, payload exists → re-drive /settle.
    /// ONLY resolution gated by the kill-switch (spec §3.C.3).
    ReDrive,
    /// Window wrong, truth exists deeper → leave stale, deep_scan next run.
    LeaveAmbiguous,
}

/// The three-way resolution table — PURE, total, exhaustively tested.
/// (state × event × validity) → verdict. Spec §6 test 1.
pub fn resolve(evidence: &ChainEvidence, _now_unix: u64) -> Resolution {
    match evidence {
        ChainEvidence::UsedOnChain { tx_hash } => {
            Resolution::SettledReconciled { tx_hash: tx_hash.clone() }
        }
        ChainEvidence::CanceledOnChain => Resolution::FailedCanceled,
        ChainEvidence::AmbiguousConsumed => Resolution::LeaveAmbiguous,
        ChainEvidence::ExpiredUnused => Resolution::FailedExpired,
        ChainEvidence::StillValid => Resolution::ReDrive,
    }
}

/// New terminal statuses introduced by the spec (D1 CHECK-compatible).
pub const TERMINAL_STATUSES: &[&str] = &[
    "settled",
    "settled_reconciled",
    "failed",
    "failed_canceled",
    "failed_expired",
];

/// Monotonicity law (spec §2): terminal states are absorbing. Guarded UPDATE
/// predicate — any transition out of a terminal state is a bug.
pub fn can_transition_from(current_status: &str) -> bool {
    !TERMINAL_STATUSES.contains(&current_status)
}

/// Entitlement TTL for settled_reconciled rows (spec §1 REPLAY_TTL_SECS).
pub const REPLAY_TTL_SECS: u64 = 86_400;

// ---------------------------------------------------------------------------
// Build-time selector/topic derivation (spec §1) — derived, never hand-typed.
// ---------------------------------------------------------------------------

pub fn selector_authorization_state() -> [u8; 4] {
    let h = alloy_primitives::keccak256(b"authorizationState(address,bytes32)");
    [h[0], h[1], h[2], h[3]]
}

pub fn topic_authorization_used() -> alloy_primitives::B256 {
    alloy_primitives::keccak256(b"AuthorizationUsed(address,bytes32)")
}

pub fn topic_authorization_canceled() -> alloy_primitives::B256 {
    alloy_primitives::keccak256(b"AuthorizationCanceled(address,bytes32)")
}

pub fn pad32_address(addr: &str) -> String {
    let clean = addr.strip_prefix("0x").unwrap_or(addr).to_lowercase();
    format!("0x{}{}", "0".repeat(64 - clean.len()), clean)
}

// ---------------------------------------------------------------------------
// Chain-read plumbing (spec §3.B/§3.C) — PURE encode/decode/classify helpers.
// The edge sweep supplies the raw RPC responses; these turn them into
// ChainEvidence for the resolution table above.
// ---------------------------------------------------------------------------

/// ABI calldata for `authorizationState(address,bytes32)` — static args only
/// (selector + padded address + 32-byte nonce), derived, never hand-typed.
pub fn encode_authorization_state_call(from: &str, nonce_hex: &str) -> Result<String, String> {
    let nonce = nonce_hex.strip_prefix("0x").unwrap_or(nonce_hex).to_lowercase();
    if nonce.len() != 64 || !nonce.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("nonce not 0x+64hex: {nonce_hex}"));
    }
    Ok(format!(
        "0x{}{}{}",
        alloy_primitives::hex::encode(selector_authorization_state()),
        pad32_address(from).trim_start_matches("0x"),
        nonce
    ))
}

/// Decode the single 32-byte ABI word returned by `authorizationState`.
/// Nonzero => consumed (Used OR Canceled — the state is only the gate; the
/// event log disambiguates, spec §0). Malformed => None (ambiguous this run).
pub fn decode_consumed_word(ret: &str) -> Option<bool> {
    let s = ret.strip_prefix("0x").unwrap_or(ret);
    if s.len() != 64 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(s.bytes().any(|b| b != b'0'))
}

/// Which consuming event a log's topic0 announces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumingLog {
    Used,
    Canceled,
}

pub fn classify_consuming_log(topic0_hex: &str) -> Option<ConsumingLog> {
    let t = topic0_hex
        .strip_prefix("0x")
        .or_else(|| topic0_hex.strip_prefix("0X"))
        .unwrap_or(topic0_hex)
        .to_lowercase();
    let used = alloy_primitives::hex::encode(topic_authorization_used());
    let canceled = alloy_primitives::hex::encode(topic_authorization_canceled());
    if t == used {
        Some(ConsumingLog::Used)
    } else if t == canceled {
        Some(ConsumingLog::Canceled)
    } else {
        None
    }
}

/// Assemble ChainEvidence from the raw reads (spec §3.C, the sweep's step B):
/// `consumed` from authorizationState, `log`/`tx_hash` from getLogs
/// disambiguation, `valid_before_unix` parsed off the stored payload.
pub fn evidence_from_reads(
    consumed: bool,
    log: Option<ConsumingLog>,
    tx_hash: Option<String>,
    valid_before_unix: Option<u64>,
    now_unix: u64,
    clock_skew_secs: u64,
) -> ChainEvidence {
    if consumed {
        match log {
            Some(ConsumingLog::Used) => {
                ChainEvidence::UsedOnChain { tx_hash: tx_hash.unwrap_or_default() }
            }
            Some(ConsumingLog::Canceled) => ChainEvidence::CanceledOnChain,
            None => ChainEvidence::AmbiguousConsumed,
        }
    } else {
        match valid_before_unix {
            Some(vb) if now_unix > vb.saturating_add(clock_skew_secs) => {
                ChainEvidence::ExpiredUnused
            }
            // unknown validity (unparseable stored payload): never guess expiry
            _ => ChainEvidence::StillValid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec §6 test 1: the full resolution table, all six combinations.
    #[test]
    fn resolution_table_is_total_and_correct() {
        let now = 1_740_672_100u64;
        // (state=true, event=Used) → settled_reconciled with tx
        assert!(matches!(
            resolve(&ChainEvidence::UsedOnChain { tx_hash: "0xtx".into() }, now),
            Resolution::SettledReconciled { tx_hash } if tx_hash == "0xtx"
        ));
        // (state=true, event=Canceled) → failed_canceled
        assert_eq!(resolve(&ChainEvidence::CanceledOnChain, now), Resolution::FailedCanceled);
        // (state=true, neither event) → leave ambiguous (deep_scan next run)
        assert_eq!(resolve(&ChainEvidence::AmbiguousConsumed, now), Resolution::LeaveAmbiguous);
        // (state=false, expired) → failed_expired
        assert_eq!(resolve(&ChainEvidence::ExpiredUnused, now), Resolution::FailedExpired);
        // (state=false, still valid) → re-drive (kill-switch-gated at the caller)
        assert_eq!(resolve(&ChainEvidence::StillValid, now), Resolution::ReDrive);
    }

    /// Spec §6 test 3: monotonicity — terminal states are absorbing.
    #[test]
    fn terminal_states_absorb() {
        for t in TERMINAL_STATUSES {
            assert!(!can_transition_from(t), "{t} must be absorbing");
        }
        for alive in ["claimed", "settling", "receipt_pending"] {
            assert!(can_transition_from(alive), "{alive} must be transitionable");
        }
    }

    /// Spec §6 test 5: derived selectors/topics match known constants.
    #[test]
    fn selectors_derive_correctly() {
        use alloy_primitives::hex;
        // authorizationState(address,bytes32) — known selector
        assert_eq!(hex::encode(selector_authorization_state()), "e94a0102",
            "authorizationState selector must derive, not be typed");
        let used = hex::encode(topic_authorization_used());
        assert_eq!(used.len(), 64, "AuthorizationUsed topic is 32 bytes");
        assert_ne!(used, "0".repeat(64), "AuthorizationUsed topic nonzero");
        assert_ne!(used, hex::encode(topic_authorization_canceled()));
        // padding
        assert_eq!(pad32_address("0xabc"), format!("0x{}abc", "0".repeat(61)));
    }

    /// Calldata shape: derived selector + zero-padded from + nonce.
    #[test]
    fn authorization_state_calldata_encodes() {
        let from = "0x857b06519e91e3a54538791bDbb0E22373e36b66";
        let nonce = format!("0x{}", "ab".repeat(32));
        let cd = encode_authorization_state_call(from, &nonce).unwrap();
        assert!(cd.starts_with("0xe94a0102"), "derived selector leads");
        assert_eq!(cd.len(), 2 + 8 + 64 + 64, "selector + two 32-byte args");
        assert!(cd.ends_with(&"ab".repeat(32)));
        assert!(cd.contains(&format!("{}{}", "0".repeat(24), from[2..].to_lowercase())));
        // malformed nonce never reaches the wire
        assert!(encode_authorization_state_call(from, "0x1234").is_err());
    }

    /// The state word: zero => unconsumed; any nonzero byte => consumed;
    /// anything that is not exactly one 32-byte word => ambiguous (None).
    #[test]
    fn consumed_word_decodes() {
        assert_eq!(decode_consumed_word(&format!("0x{}", "0".repeat(64))), Some(false));
        assert_eq!(
            decode_consumed_word(&format!("0x{}1", "0".repeat(63))),
            Some(true)
        );
        // enum-style return (Unused=0, Used=1, Cancelled=2) decodes the same way
        assert_eq!(
            decode_consumed_word(&format!("0x{}2", "0".repeat(63))),
            Some(true)
        );
        assert_eq!(decode_consumed_word("0x01"), None);
        assert_eq!(decode_consumed_word(""), None);
        assert_eq!(decode_consumed_word("zz"), None);
    }

    /// Topic classification against the DERIVED topics (never hand-typed).
    #[test]
    fn consuming_logs_classify() {
        let used = format!("0x{}", alloy_primitives::hex::encode(topic_authorization_used()));
        let canceled = format!("0x{}", alloy_primitives::hex::encode(topic_authorization_canceled()));
        assert_eq!(classify_consuming_log(&used), Some(ConsumingLog::Used));
        assert_eq!(classify_consuming_log(&canceled), Some(ConsumingLog::Canceled));
        // case-insensitive, prefix-tolerant, unknown => None
        assert_eq!(classify_consuming_log(&used.to_uppercase()[..]), Some(ConsumingLog::Used));
        assert_eq!(classify_consuming_log(&canceled[2..]), Some(ConsumingLog::Canceled));
        assert_eq!(
            classify_consuming_log(&format!("0x{}", "de".repeat(32))),
            None
        );
    }

    /// The sweep's evidence assembly: every (state x event x validity) input
    /// maps to the spec'd ChainEvidence, including the skew boundary.
    #[test]
    fn evidence_assembly_is_spec_conformant() {
        use ConsumingLog::{Canceled, Used};
        let now = 1_000u64;
        let skew = 30u64;
        // consumed + Used log -> UsedOnChain with the log's tx
        assert!(matches!(
            evidence_from_reads(true, Some(Used), Some("0xtx".into()), None, now, skew),
            ChainEvidence::UsedOnChain { tx_hash } if tx_hash == "0xtx"
        ));
        // consumed + Canceled log -> CanceledOnChain
        assert_eq!(
            evidence_from_reads(true, Some(Canceled), None, None, now, skew),
            ChainEvidence::CanceledOnChain
        );
        // consumed + neither event in window -> ambiguous, never guessed
        assert_eq!(
            evidence_from_reads(true, None, None, None, now, skew),
            ChainEvidence::AmbiguousConsumed
        );
        // unconsumed, past validBefore+skew -> expired; at the boundary: valid
        assert_eq!(
            evidence_from_reads(false, None, None, Some(now - skew), now, skew),
            ChainEvidence::StillValid
        );
        assert_eq!(
            evidence_from_reads(false, None, None, Some(now - skew - 1), now, skew),
            ChainEvidence::ExpiredUnused
        );
        // unconsumed, validity unknowable -> still valid (re-drive guards it)
        assert_eq!(
            evidence_from_reads(false, None, None, None, now, skew),
            ChainEvidence::StillValid
        );
    }
}
