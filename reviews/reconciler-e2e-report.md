# RECONCILER-SPEC v1 — Live Sepolia E2E Report (spec §6 test 6)

**Date:** 2026-08-19 · **Worker:** local wrangler dev (build incl. dc09b12 + same-day fixes) ·
**Chain:** Base Sepolia (USDC `0x036CbD…dCF7e`, real funds) · **Facilitator:** live CDP
**Driver:** `tests/vectors/gen/reconciler-e2e.mjs` (modes `oob | cancel | wedge-short | wedge-long | repost`)

All four spec §6.6 scenarios executed against the real chain and the real facilitator.
Every resolution below was produced by the hourly sweep (`/__scheduled` trigger), not by hand.

## (a) Out-of-band settle → entitlement → replay — PASS

| step | evidence |
|---|---|
| OOB `transferWithAuthorization` (payer pays company directly) | tx `0x99b42102d42cea54a8e1e879f002e72c5d620d9e20f4c29f86490da344e4eebb`, status `success` |
| Route POST with the now-consumed payment | HTTP **503** `settlement_pending`; D1 row `receipt_pending` (the 122-phantom shape) |
| Sweep (after 300s staleness) | `SweepStats { scanned: 1, consumed: 1, resolved_used: 1, errors: 0 }` |
| D1 after sweep | `status='settled_reconciled'`, `resolution='reconciled_used'`, `resolution_tx=0x99b42102…`, `replay_eligible_until` set |
| Payer retries the SAME payment | HTTP **200**, output served, `settlement.transaction = 0x99b42102…` — executed **FREE** (no facilitator call; stamp AGE gate bypassed by the live entitlement, MAC still enforced), `reexec_count → 1` |
| Retry again | HTTP **200** identical stored replay, same tx |

## (b) On-chain cancel → failed_canceled — PASS

| step | evidence |
|---|---|
| OOB `cancelAuthorization` | tx `0x225e02cd4c1ea7813295d11173a4a4a472b0ce8275af9f1229628e1628`, `success` |
| Route POST (getLogs must distinguish Canceled from Used) | 503 `settlement_pending` → wedge |
| Sweep | `resolved_canceled: 1`; D1 `failed_canceled` / `reconciled_canceled` |
| Retry | HTTP **400** terminal — no entitlement for a revoked (never-paid) authorization |

## (c) Expiry → failed_expired — PASS

- Wedged via dead-facilitator instance (`--var CDP_FACILITATOR_BASE:http://127.0.0.1:9`) → route 503, D1 `settling` (bridge + marks)
- First sweep ran INSIDE validity+skew: correctly untouched (spec: expiry branch owns it next run)
- Second sweep after window: `resolved_expired: 1`; D1 `failed_expired` / `reconciled_expired`

## (d) Re-drive → settled by cron → entitled retry — PASS

- Far-validity payment wedged the same way (our settle never reached CDP)
- Healthy-instance sweep: `redriven: 1, redrive_settled: 1` — the cron re-submitted the IDENTICAL stored payload; **real CDP settle** tx `0x3bf1df1b0c91dfd69e61d5bdb18568947da80d2fe6ec3196236403702ea4508f`
- D1 `settled` / `facilitator`, entitlement granted
- Payer retries: HTTP **200 FREE**, `settlement.transaction = 0x3bf1df1b…`

Money check: payer USDC 1.66 → 1.62 (2 OOB + 1 re-drive + 1 crashed-run settle @ 0.005) — consistent.

## Defects found and fixed by this e2e (the point of the exercise)

1. **MAJOR (G2d): CDP already-used responses landed terminal `failed`.** Live shapes:
   `4xx {errorReason:"invalid_payload", transaction:<doomed replay tx>, errorMessage:"authorization nonce already submitted…"}`,
   `4xx {errorType:"settle_exact_failed_onchain"}` — neither matched the volley-era
   `invalid_exact_evm_payload_signature`+empty-transaction signature. Phantom-shaped payments
   were terminally failed instead of receipt_pending. **Fix:** ambiguous-money class = any of the
   three reasons at settle-time (our structural gate already passed → fail CLOSED on money) →
   receipt_pending in route and re-drive.
2. **Entitlement unreachable (design hole):** stamp grace (300s) < entitlement TTL (24h), so aged
   entitled payments were rejected before the claim could return `Entitled`. **Fix:** the AGE gate
   is waived for (payer, nonce) holding a live entitlement; the MAC check is never waived.
3. `d1_resolve` now carries resolution evidence into `tx_hash` (COALESCE) so re-drive settles are
   queryable by tx like route settles.
4. Driver/ops hardening: CDP faucet script (`faucet.mjs`, `token:'eth'`), raw settle probe
   (`cdp-settle-raw.mjs`), OP-stack nonce semantics (use `latest` + explicit nonce).

## Suites

core 81/81 (incl. 8 new reconciler/entitlement tests) · wire 7/7 · official-client e2e PASS
(real settle through the bridge: `resolution='facilitator'`, response persisted 748B).

Outstanding (next): 122-phantom regression corpus as standing fixtures; Kimi + DeepSeek panel gate
on the wiring (red-team focus: forged-log spoofing is defended by RPC-origin logs keyed on indexed
(from, nonce) — never client-supplied — per spec §7).
