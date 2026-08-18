# KV kill-switch & mainnet gate design (plan-rev3 G8)

## Mechanism

Two flags live in the `PRICING` KV namespace (already bound, already used for
ops keys like `ops:pending_settlement`):

- `ops:x402v2_enabled` — `true` | `false` (absent ⇒ `false`). Gates acceptance
  of `PAYMENT-SIGNATURE` on `/v2/*` AND the v2 settlement pipeline on the
  rewired legacy route. Kill = put `false`; retreat is instant-off without
  redeploy.
- `ops:x402v2_network` — `eip155:84532` (default, staging) | `eip155:8453`
  (mainnet). The mainnet flip is a KV write, not a `[vars]` change. `[vars]`
  keep deploy-time defaults only (Sepolia), so a fresh deploy can never
  accidentally run mainnet without the KV gate being explicitly set.

Read path: checked once per paid request before the structural gate (cache in
isolate memory for the request lifetime; do NOT re-read mid-settlement — a flag
flip mid-flight must not abort an in-progress settle).

## Propagation caveat (documented per G8)

KV is eventually consistent: **up to ~60s** until all isolates observe a
write. Acceptable for: kill-switch (seconds-level blast-radius reduction),
mainnet enable (staged, human-triggered). NOT acceptable for per-request
security decisions that need instant global effect — if that is ever required,
the NonceGuard DO can double-check the flag (DOs are strongly consistent).
Runbook records this.

## Failure semantics

- KV read failure (binding error): **fail closed** — treated as `false`
  (v2 path refuses; legacy codec path unaffected until Stage 4 rewire, after
  which legacy shares the gate and a KV outage fails closed for paid calls
  entirely — free/manifest/trust routes unaffected).
- Breaker (G4/G7) is a separate KV flag `ops:facilitator_breaker`
  (`open`|`closed`) managed by the cron health probe + failure counters.
  Breaker open ⇒ fail closed with `503 RETRYABLE` + spec taxonomy reason.

## Interaction with `[vars]` and secrets

- `[vars]`: deploy-time defaults (staging network, token addresses) — no v2
  on/off authority.
- Secrets: `CDP_API_KEY` (new, Stage 4), `COMPANY_WALLET`, `RECEIPT_SIGNING_KEY`,
  `RPC_*` unchanged. KV never holds secrets.
