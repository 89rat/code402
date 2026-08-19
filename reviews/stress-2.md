# Stress campaign 2 — 1,000 real settles (2026-08-19)

Driver: tests/vectors/gen/thousand.mjs — 40 waves × 25 parallel, one payer,
real CDP verify+settle throughout, real on-chain USDC. Telemetry:
/tmp/thousand.jsonl (1,000 records). Wallet truth via RPC pre/post.

## Headline numbers
- 1,000 attempts in 310s: **717×200 (71.7%) + 283×503 settlement_pending + 0 anything else**
- Settle latency: p50 3.5s · p90 3.9s · p99 7.1s · max 9.7s (single-shot is 1.4s;
  sustained 25-wide load pushes to ~2 blocks)
- 402 challenge: p50 137ms (the free tier of the system is fast)
- Throughput: **~6.2 settles/s in clean windows** (25 per 4s wave), 2.3/s
  sustained across degradation, zero crashes, zero 5xx, zero wrong answers

## The bimodal wave structure (the real discovery)
Waves were either ~4s/25-ok or ~10s/many-pending — CDP's settle queue has a
burst capacity; exceed it and settles queue server-side past our subrequest
timeout (~10s, workerd cap), then RECOVER fully (waves 37-40: 25/25 clean).
The system degraded exactly per I5: fail-closed, retryable, lease-protected.

## THE PHANTOM SETTLES (~122) — G2d is not theoretical
On-chain moved 4.19 USDC; confirmed-200s account for 3.585. Delta =
**~122 settles that OUR side recorded as timeout but CDP completed anyway**
after our fetch died. Those nonces are burned on-chain; our claims sit in
`settling` until lease expiry; D1 has no settled row. This is precisely the
`receipt_pending` class: **the reconciliation cron (G7) has ~122 real work
items waiting** — and ~161 genuinely-failed claims the lease will free.
Client retries of phantoms will hit already-used → our receipt_pending path
→ cron backfills from AuthorizationUsed events. The design anticipated
this; the stress test manufactured the backlog to prove it.

## Faucet rate limits (bonus data)
Burst window: ~5 rapid drips per wallet then 429 (on top of the documented
10/day). Two keys did NOT raise the per-address cap.

## Actions
1. Reconciliation cron (G7) is now URGENT, not just planned — it has real
   work items.
2. Client settle timeout should exceed workerd's ~10s subrequest cap is
   impossible — instead: treat 10s-timeouts as ALWAYS-ambiguous (correct
   today), and the breaker should count settlement_pending rate (open at
   sustained >30%? — tuned at Stage 4 hardening).
3. Throughput planning: 6/s burst per facilitator key; horizontal scaling
   = more keys (the Facilitator trait takes a pool trivially).
