# Claims verification battery — 2026-08-19 (two CDP keys, volleyed)

Operator provided two CDP keys to volley and test "all the claims we are
proposing." Every testable claim in reviews/cdp-findings.md is now a
measurement. Harness: tests/claims/volley.mjs (keys via args, never stored
in-repo).

## Results
- **C1 verify-free: VERIFIED** — 80 verify calls (25 key A, 25 key B, 30
  volleyed A/B alternating): zero 429s, zero quota errors, all served
  (~350ms mean). Free at this scale, as claimed.
- **C2/C3 rate limits: PARTIAL** — no throttling observed at 80 calls /
  2 keys. True ceilings remain unknown (only meaningful at production
  volume; breaker thresholds stay conservative).
- **C4 JWT exp enforcement: VERIFIED** — JWT with exp 5min in the past →
  401.
- **C5 latency (Law 1, block-bound): VERIFIED** — 8 live paid settles:
  min 1.368s · median ~1.42s · max 2.750s. Quantized around Base block
  boundaries as predicted; Option A's one-RTT saving visible in the
  median. 0.001s remains impossible under I1 (chain physics).
- **C6 insufficient_funds taxonomy: VERIFIED AT SOURCE + 2 BUGS FIXED** —
  CDP settles a broke payer as HTTP 400 with a SettleResponse-shaped body:
  {"success":false,"errorReason":"insufficient_funds",...} — the §9 string
  matches EXACTLY. Bugs the volley exposed (both fixed):
  1. facilitator client treated ALL non-200 as transport errors → broke
     payers were misclassified as ambiguous timeouts (503 receipt_pending)
     instead of deterministic 400 — violations of the §9 failure classes.
     Fix: 4xx JSON bodies parse as typed outcomes (errorType OR
     settle-shape); only 5xx/undecodable remain Err.
  2. CDP omits `transaction` on failed settles; our decoder required it.
     Fix: default "" (spec §5.3.2: empty = nothing broadcast).
  End state verified live: HTTP 400 {"code":"insufficient_funds",
  "retryable":false} — correct taxonomy, status, retryability.
- **C7 /supported: VERIFIED + RICHER THAN RECORDED** — v2 kinds beyond
  Base: Arbitrum (84532/42161... exact/upto/batch-settlement) and World
  (480/4801, all three schemes); Solana v1 exact; extensions include
  builder-code (monetization attribution confirmed live), bazaar,
  eip2612GasSponsoring.

## Kaizen
Two defects reached the live path before the battery caught them (the
503-for-broke-payer class): catcher added to the failure-matrix fixture
list — "deterministic facilitator rejection must NEVER map to
receipt_pending" is now a standing fixture requirement (Stage-4 item).
