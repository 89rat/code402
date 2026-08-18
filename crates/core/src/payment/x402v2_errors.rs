//! §9 error taxonomy — vendored verbatim from the pinned spec
//! (specs/x402/x402-specification-v2.md §9 @ ddf98ee5). These are the ONLY
//! error strings that may appear on the wire in errorReason/invalidReason.
//! Mapped exhaustively from internal errors; internal names never leak.
//!
//! `settlement_pending` is NON-TERMINAL (§9): broadcast-but-unconfirmed;
//! SettleResponse carrying it MUST include non-empty transaction + network.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

/// Spec §9 error codes. Exhaustive match enforced by `map_error` — adding a
/// variant without mapping is a compile error; the spec gaining a code is a
/// SPEC-VERSION bump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Taxonomy {
    InsufficientFunds,
    InvalidValidAfter,
    InvalidValidBefore,
    InvalidValueMismatch,
    InvalidSignature,
    InvalidRecipientMismatch,
    InvalidNetwork,
    InvalidPayload,
    InvalidPaymentRequirements,
    InvalidScheme,
    UnsupportedScheme,
    InvalidX402Version,
    InvalidTransactionState,
    UnexpectedVerifyError,
    UnexpectedSettleError,
    SettlementPending,
}

impl Taxonomy {
    /// The exact §9 wire string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Taxonomy::InsufficientFunds => "insufficient_funds",
            Taxonomy::InvalidValidAfter => "invalid_exact_evm_payload_authorization_valid_after",
            Taxonomy::InvalidValidBefore => "invalid_exact_evm_payload_authorization_valid_before",
            Taxonomy::InvalidValueMismatch => "invalid_exact_evm_payload_authorization_value_mismatch",
            Taxonomy::InvalidSignature => "invalid_exact_evm_payload_signature",
            Taxonomy::InvalidRecipientMismatch => "invalid_exact_evm_payload_recipient_mismatch",
            Taxonomy::InvalidNetwork => "invalid_network",
            Taxonomy::InvalidPayload => "invalid_payload",
            Taxonomy::InvalidPaymentRequirements => "invalid_payment_requirements",
            Taxonomy::InvalidScheme => "invalid_scheme",
            Taxonomy::UnsupportedScheme => "unsupported_scheme",
            Taxonomy::InvalidX402Version => "invalid_x402_version",
            Taxonomy::InvalidTransactionState => "invalid_transaction_state",
            Taxonomy::UnexpectedVerifyError => "unexpected_verify_error",
            Taxonomy::UnexpectedSettleError => "unexpected_settle_error",
            Taxonomy::SettlementPending => "settlement_pending",
        }
    }
}

use crate::payment::x402v2::X402Error;

/// Exhaustive map: internal error → spec taxonomy. Every X402Error variant
/// MUST map (compile-enforced by match exhaustiveness). Where the internal
/// error is finer-grained than the taxonomy, the closest §9 code is chosen
/// and the detail stays in logs, never on the wire.
pub fn map_error(e: &X402Error) -> Taxonomy {
    match e {
        X402Error::WrongVersion(_) => Taxonomy::InvalidX402Version,
        X402Error::HeaderTooLarge(_)
        | X402Error::NotCanonicalBase64
        | X402Error::InvalidJson(_)
        | X402Error::MissingField(_) => Taxonomy::InvalidPayload,
        X402Error::AmountNotDecimal(_) | X402Error::AmountOverflow(_) => {
            Taxonomy::InvalidPaymentRequirements
        }
        X402Error::TimestampNotString(_) => Taxonomy::InvalidPayload,
        X402Error::BadAddress(_) => Taxonomy::InvalidPayload,
        X402Error::BadNonce(_) => Taxonomy::InvalidPayload,
        X402Error::BadSignature(_) => Taxonomy::InvalidSignature,
        X402Error::BadNetwork(_) => Taxonomy::InvalidNetwork,
        X402Error::BadScheme(_) => Taxonomy::InvalidScheme,
        X402Error::ReservedExtraKey(_) => Taxonomy::InvalidPaymentRequirements,
        X402Error::ValidBeforeMargin(_, _) => Taxonomy::InvalidValidBefore,
        X402Error::ValidAfterFuture(_, _) => Taxonomy::InvalidValidAfter,
        X402Error::RecipientMismatch(_) => Taxonomy::InvalidRecipientMismatch,
        X402Error::ExactAmountMismatch(_, _) => Taxonomy::InvalidValueMismatch,
        X402Error::ResourceUrlMismatch(_) => Taxonomy::InvalidPayload,
        X402Error::BadServiceName | X402Error::BadTags | X402Error::BadIconUrl => {
            Taxonomy::InvalidPaymentRequirements
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taxonomy_strings_are_verbatim_section9() {
        // pinned verbatim from vendored spec §9 @ ddf98ee5 — if the spec
        // changes these, SPEC-VERSION bump + this test changes together.
        assert_eq!(Taxonomy::SettlementPending.as_str(), "settlement_pending");
        assert_eq!(
            Taxonomy::InvalidValueMismatch.as_str(),
            "invalid_exact_evm_payload_authorization_value_mismatch"
        );
        assert_eq!(Taxonomy::InvalidSignature.as_str(), "invalid_exact_evm_payload_signature");
    }

    #[test]
    fn every_error_maps() {
        // compile-time exhaustiveness is the real test; this exercises a few.
        assert_eq!(
            map_error(&X402Error::WrongVersion(1)),
            Taxonomy::InvalidX402Version
        );
        assert_eq!(
            map_error(&X402Error::ExactAmountMismatch("1".into(), "2".into())),
            Taxonomy::InvalidValueMismatch
        );
        assert_eq!(
            map_error(&X402Error::ValidBeforeMargin(0, 0)),
            Taxonomy::InvalidValidBefore
        );
    }
}

#[cfg(test)]
mod reachability {
    use super::*;
    use std::collections::HashSet;

    /// DeepSeek stage-2: every §9 taxonomy code must be REACHABLE from some
    /// internal error, or documented facilitator-origin. Codes only a
    /// facilitator emits (insufficient_funds, invalid_transaction_state,
    /// unexpected_*, settlement_pending, unsupported_scheme) are exempt.
    #[test]
    fn every_taxonomy_code_reachable_or_facilitator_origin() {
        let facilitator_only: HashSet<&str> = [
            "insufficient_funds",
            "invalid_transaction_state",
            "unexpected_verify_error",
            "unexpected_settle_error",
            "settlement_pending",
            "unsupported_scheme",
        ].into_iter().collect();

        // one internal error per taxonomy variant that we claim reachable
        let probes: Vec<(Taxonomy, X402Error)> = vec![
            (Taxonomy::InvalidValidAfter, X402Error::ValidAfterFuture(1, 0)),
            (Taxonomy::InvalidValidBefore, X402Error::ValidBeforeMargin(0, 1)),
            (Taxonomy::InvalidValueMismatch, X402Error::ExactAmountMismatch("a".into(), "b".into())),
            (Taxonomy::InvalidSignature, X402Error::BadSignature(0)),
            (Taxonomy::InvalidRecipientMismatch, X402Error::RecipientMismatch("0x0".into())),
            (Taxonomy::InvalidNetwork, X402Error::BadNetwork("x".into())),
            (Taxonomy::InvalidPayload, X402Error::InvalidJson("x".into())),
            (Taxonomy::InvalidPaymentRequirements, X402Error::AmountNotDecimal("x".into())),
            (Taxonomy::InvalidScheme, X402Error::BadScheme("x".into())),
            (Taxonomy::InvalidX402Version, X402Error::WrongVersion(1)),
        ];
        let mut reachable: HashSet<&str> = probes.iter().map(|(t, e)| {
            assert_eq!(map_error(e), *t, "{:?} probe maps wrong", t);
            t.as_str()
        }).collect();
        reachable.extend(facilitator_only);
        let all: HashSet<&str> = [
            "insufficient_funds", "invalid_exact_evm_payload_authorization_valid_after",
            "invalid_exact_evm_payload_authorization_valid_before",
            "invalid_exact_evm_payload_authorization_value_mismatch",
            "invalid_exact_evm_payload_signature",
            "invalid_exact_evm_payload_recipient_mismatch", "invalid_network",
            "invalid_payload", "invalid_payment_requirements", "invalid_scheme",
            "unsupported_scheme", "invalid_x402_version", "invalid_transaction_state",
            "unexpected_verify_error", "unexpected_settle_error", "settlement_pending",
        ].into_iter().collect();
        let missing: Vec<_> = all.difference(&reachable).collect();
        assert!(missing.is_empty(), "unreachable+undocumented codes: {missing:?}");
    }
}
