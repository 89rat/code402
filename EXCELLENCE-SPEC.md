# code402 × Excellence Spec — Integration Map

How the normative "Excellence in Code and Design" model maps onto the deployed
code402 system. Three dispositions: **ADOPTED** (in code now), **ROADMAP**
(committed, phased), **REJECTED** (with reason). Honest-metrics doctrine
applies: no latency/volume promises that aren't measured.

## 1. Objective (their §21 / Final Rule)

```
Preferred(code402, incumbent) iff
  Price ≤ 0.90 × incumbent  AND  Reliability ≥ incumbent
  AND  IntegrationFriction < incumbent  AND  SettlementAuditability > incumbent
```

- Price initiates replacement; correctness and auditability keep the position.
- **ADOPTED** as the GTM objective. First incumbent class: per-call SaaS API
  subscriptions for deterministic validation/enrichment tools.

## 2. Payment challenge schema (their §4.4, rules P1–P5)

| Rule | Status |
|---|---|
| P1 integer-string minor units | ADOPTED — `price.amount` is a string |
| P2 no floating point for money | ADOPTED — u64 minor units everywhere |
| P3 asset/chain registry validation | ADOPTED — chain allowlist 8453/84532, token from env |
| P4 RFC 3339 expiry | ADOPTED — `expires_at_rfc3339` added (unix field kept for compat) |
| P5 explicit proof type | ADOPTED — `proof: {type: "eip3009_voucher", header: "X-PAYMENT"}` |
| payment_intent_id | ADOPTED — 1 intent == 1 request in v1 (invariant G2) |
| settlement_mode field | ADOPTED — current value `"facilitated_direct"` |

## 3. Settlement modes (their §4.5)

Current: **facilitated_direct** (EIP-3009 voucher, public facilitator settles,
queue consumer confirms on-chain). ROADMAP selection rule:

```
if RiskScore > high        → DirectEscrow     (needs escrow contract, Phase 4)
elif Amount >= high_value  → DirectEscrow
elif repeated consumer     → PaymentChannel   (Phase 5)
elif micro + good credit   → CreditLedger     (REQUIRES policy change —
                                               uncollateralized credit is
                                               currently BANNED by policy)
else                       → HybridBatch      (batch-settlement scheme,
                                               facilitator already advertises it)
```

## 4. Gateway invariants (their §14.3) — audited against live code

| Invariant | Status |
|---|---|
| G1 no execution before payment verification | ADOPTED — verify() precedes execute_tool() |
| G2 exactly one payment intent per request | ADOPTED |
| G3 same idempotency key → same logical outcome | ADOPTED — D1 idempotency table, tested live |
| G4 delivery receipt before settlement eligibility | ADOPTED — receipt written to R2, event PENDING_SETTLEMENT until queue confirms tx |
| G5 no settlement signing without delivery | N/A — facilitator model; worker never signs settlement approvals |

Required headers: `X-Request-Id` on every response ADOPTED (from cf-ray).
`X-PAYMENT` is our payment header; `idempotency_key` travels in the body
(documented deviation: header form planned in schema v1.1).

## 5. Error taxonomy (their §15.2)

ADOPTED: every error body now carries `{code, message, retryable}`.
retryable = status ≥ 500 or 429; 4xx client faults are not retryable.
Our codes remain SCREAMING_SNAKE under X-Schema-Version 1.0 (frozen);
mapping to their classes lives here:

- Payment: INSUFFICIENT_PAYMENT, EXPIRED_PAYMENT, REPLAYED_NONCE
- Validation: INPUT_SCHEMA_INVALID, INVALID_RECIPIENT, UNSUPPORTED_TOKEN/CHAIN
- Auth: INVALID_SIGNATURE
- System: TOOL_INTERNAL_ERROR (retryable)

## 6. Observability (their §15.3)

Per-request record (D1 payment_events) has: request_id, tool_id,
tool_version, tx_hash, amount_minor, status, error_code, created_at.
ROADMAP: consumer_id (payer address — already in queue msg, add to D1),
latency_ms, chain confirmation delay, reconciliation discrepancy counter.
Dashboard widgets: success rate, payment conversion (paid/challenged),
settlement success, refund rate, p50/p95/p99 overhead.

## 7. Treasury controls (their §25.3) — SECURITY CRITICAL

```
Separate: operating wallet / fee vault / reserve / upgrade authority.
Multisig for upgrades. Timelocks for governance. Daily reconciliation.
Gas reserve + refund reserve.
```

Status: production company wallet `0xdcd0fe…fdcf` key lives in
`code402/.staging/prod-company.txt` (gitignored). PLAN:
1. User backs up key to a password manager NOW.
2. Balance > $1,000 → migrate recipient to a Safe multisig (2-of-3).
3. Daily reconciliation = the D1/R2 ledger diff (script roadmap).
4. Key compromise → rotate RECEIPT_SIGNING_KEY + COMPANY_WALLET, announce
   via security.txt + manifest (incident rule adopted).

## 8. Release gates (their §17.1) — status

| Gate | Status |
|---|---|
| Unit tests (state transitions) | ✅ 16/16 core, 56/56 guard |
| Integration (end-to-end 402) | ✅ live acceptance suite, both envs |
| Observability (events traceable) | ✅ D1 + R2 + request-id header |
| Security review | ◐ internal only; external review before real volume |
| Property/fuzz/contract tests | ROADMAP (proptest on verify(), foundry on escrow) |
| Load tests | ROADMAP (no latency claims until measured) |
| Reconciliation tests | ◐ D1/R2 audit path exists; automated diff ROADMAP |

## 9. Funnels (their funnel doc) — instrumentation priority

1. **Priority 1 (revenue safety)**: Settlement trust funnel (every call →
   receipt → settlement state visible), compliance/risk funnel (sanctions
   screening on payout addresses before mainnet volume).
2. **Priority 2 (growth)**: consumer activation (time-to-first-paid-call —
   the SDK is the instrument), M2M lead qualification (track 402→paid
   conversion per tool).
3. **Priority 3 (leadership)**: provider supply funnel (third-party tool
   onboarding), winback, standards/conformance.

## 10. Acceptance criteria (their §20) — current truth

- A3 payment flow without manual intervention: ✅ staging proven
- A4 settlement idempotent + auditable: ✅ replay→409, receipts signed
- A5 deterministic refund path: ❌ not built (ROADMAP Phase 4, SET-4)
- A6 reconciliation discrepancy zero: ◐ by construction, unmeasured
- A8 lead rate 2× baseline: unmeasured (no baseline yet)
- A10 no unresolved critical security finding: ✅ known bug (Sepolia
  domain name) found and fixed 2026-08-15
- A1/A2/A7/A9/A11/A12: pending first incumbent replacement pilot

## 11. Platform blueprint (their node doc)

GOV/MKT/DIS/ACC/SET node set adopted as the target architecture. Deployed
today: ACC-1 (edge gateway), ACC-4 (proof verifier), ACC-6 (receipts),
SET-2 (settlement confirm), DIS-2 (manifests). Next build order:
DIS-1 registry → MKT-2 pricing control → SET-4 refunds → GOV-5 treasury
multisig → SET-1 contracts engine (escrow).
