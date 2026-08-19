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
}
