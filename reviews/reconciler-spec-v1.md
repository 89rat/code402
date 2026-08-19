# RECONCILER-SPEC v1 — On-Chain Settlement Reconciliation (G7)

Implementable spec for the orchestrator. Resolves every stale claim to a terminal state using the chain as root of truth (invariant I4). Runs even when the payment kill-switch is on — the reconciler is the janitor, not a payer. Written against: D1 settlements table (Rev 3 migration 0002), claim states `claimed → settling → settled | failed`, hourly cron, Workers/WASM runtime.

## 0. Definitions

- **Stale claim**: row with `status IN ('claimed','settling')` AND `updated_at < now − LEASE_SECS`.
- **Chain truth**: `authorizationState(authorizer, nonce) → bool` view on the USDC contract (EIP-3009). `true` means the nonce was consumed — by **either** `transferWithAuthorization` (we may have been paid) **or** `cancelAuthorization` (payer revoked; we were NOT paid). `true` is therefore *necessary but not sufficient* for "paid"; the disambiguator is the event log (§3 step B).
- **Events** (both indexed on `(authorizer, nonce)`):
  - `AuthorizationUsed(address indexed authorizer, bytes32 indexed nonce)` → transfer happened.
  - `AuthorizationCanceled(address indexed authorizer, bytes32 indexed nonce)` → revoked, no transfer.

## 1. Configuration (all in a single `ReconcilerConfig`, values via vars/KV)

| Key | Default | Notes |
|---|---|---|
| `LEASE_SECS` | 300 | must exceed max settle path latency |
| `CLOCK_SKEW_SECS` | 30 | applied to validBefore comparisons |
| `SETTLE_MARGIN_SECS` | 30 | min remaining validity to re-drive a settle (G5 margin) |
| `MAX_CLAIMS_PER_RUN` | 500 | oldest-first |
| `MULTICALL_CHUNK` | 100 | authorizationState calls per aggregate3 |
| `LOGS_BATCH` | 20 | eth_getLogs per JSON-RPC batch |
| `MAX_LOOKBACK_BLOCKS` | 50_000 | ≈27h at 2s blocks; deep-scan mode may page beyond for the initial backlog |
| `REPLAY_TTL_SECS` | 86_400 | entitlement window after `settled_reconciled` |
| `RPC_URL` | secret | Base / Base Sepolia endpoint |
| USDC per network | config | `eip155:84532 → 0x036CbD53842c5426634e7929541eC2318f3dCF7e`; `eip155:8453 → 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` — verify both against SPEC-VERSION at implementation |
| Multicall3 | `0xcA11bde05977b3631167028862bE2a173976CA11` | canonical; verify deployed on both networks |

Selectors/topics are **derived at build time**, never hand-typed: `selector = keccak256("authorizationState(address,bytes32)")[0..4]`; `topic_used = keccak256("AuthorizationUsed(address,bytes32)")`; `topic_canceled = keccak256("AuthorizationCanceled(address,bytes32)")`. Add a build-time assertion test that derives each and byte-compares against a checked-in constant, and a Sepolia fixture that proves the derived selector returns `true` for a known-consumed authorization.

## 2. Schema delta (migration 0003)

```sql
ALTER TABLE settlements ADD COLUMN resolution TEXT;            -- 'facilitator' | 'reconciled_used' | 'reconciled_canceled' | 'reconciled_expired'
ALTER TABLE settlements ADD COLUMN resolution_tx TEXT;         -- nullable; backfilled from AuthorizationUsed log
ALTER TABLE settlements ADD COLUMN resolved_at INTEGER;        -- unix seconds
ALTER TABLE settlements ADD COLUMN replay_eligible_until INTEGER; -- nullable; set on settled_reconciled
CREATE INDEX idx_settlements_stale ON settlements (status, updated_at);
CREATE TABLE reconciler_runs (
  id INTEGER PRIMARY KEY, started_at INTEGER, finished_at INTEGER,
  scanned INTEGER, resolved_used INTEGER, resolved_canceled INTEGER,
  resolved_expired INTEGER, redriven INTEGER, left_ambiguous INTEGER, error TEXT
);
```

New terminal statuses: `settled_reconciled`, `failed_expired`, `failed_canceled`. State-machine law: **terminal states are absorbing** — any transition out of `settled*`/`failed*` is a bug; enforce with a D1 trigger or a guarded UPDATE (`WHERE status IN ('claimed','settling')`) and a property test.

## 3. Algorithm (per hourly cron; resumable by construction — resolved rows leave the stale set)

**A. Select.** `SELECT ... WHERE status IN ('claimed','settling') AND updated_at < ?now − LEASE ORDER BY updated_at ASC LIMIT MAX_CLAIMS_PER_RUN`.

**B. Chain read, chunked.** For each chunk of `MULTICALL_CHUNK` claims: one `eth_call` to Multicall3 `aggregate3` wrapping `authorizationState(from, nonce)` per claim (`allowFailure=true`; a failed inner call → treat claim as ambiguous this run, count it, continue).

**C. Three-way resolve per claim:**

1. **`state == true`** → nonce consumed; disambiguate via one `eth_getLogs` per claim (batched `LOGS_BATCH` per JSON-RPC batch): `address = USDC(network)`, `topics = [[topic_used, topic_canceled], pad32(from), nonce]`, `fromBlock = max(latest − MAX_LOOKBACK_BLOCKS, claim_created_block_estimate)`, `toBlock = latest` where `claim_created_block_estimate = latest − ceil((now − created_at)/2) − 1800` (safety pad).
   - `AuthorizationUsed` found → **`settled_reconciled`**: `resolution='reconciled_used'`, `resolution_tx = log.transactionHash`, `replay_eligible_until = now + REPLAY_TTL_SECS`. The client paid and never got the result: this row is now an **entitlement record** — the next request bearing this `(from, nonce)` routes to the replay path and executes **free** (G2 contract), once, until the TTL.
   - `AuthorizationCanceled` found → **`failed_canceled`**: payer revoked before our settle landed; no funds moved; claim released. Alarm at any nonzero rate (§5) — cancels against us are anomalous and possibly adversarial probing.
   - Neither event in window → do **not** guess: leave in `settling`, increment `left_ambiguous`, and if age > 24h flip the claim into `deep_scan` mode next run (paged getLogs walking back beyond `MAX_LOOKBACK_BLOCKS` to the claim's creation date). Truth exists on-chain; only the window was wrong.

2. **`state == false` AND `now > validBefore + CLOCK_SKEW`** → authorization expired unused → **`failed_expired`**, `resolution='reconciled_expired'`. Funds never moved; nonce is dead on-chain; safe and final.

3. **`state == false` AND still valid** → our settle may be in flight or was never sent. If the stored payment payload exists AND remaining validity > `SETTLE_MARGIN_SECS` AND kill-switch is **off** → **re-drive**: resubmit the identical payload to facilitator `/settle` (at-least-once trap already handled: "already used" → rerun step C.1 disambiguation, not blind success — per G2(d)). Otherwise leave untouched; the expiry branch (C.2) resolves it within one validity window. Re-drive is the *only* reconciler action gated by the kill-switch.

**D. Record.** One `reconciler_runs` row per run; one audit line per transition (claim id, old→new, resolution, evidence: tx hash or "expiry"). Every transition idempotent: re-running the resolver over an already-terminal row is a no-op by the guarded UPDATE.

## 4. RPC budget

500 claims/run worst case: 5 multicall `eth_call`s + getLogs only for the `state==true` fraction (expected minority; the 122-backlog first run may spike — acceptable) + re-drives via facilitator (not RPC). Retries: 3 attempts, jittered exponential backoff, per-run wall budget conservative (chunk loop checks elapsed time and exits cleanly; unfinished rows are simply picked up next run — no cursor needed).

## 5. Alarms and metrics (KV-backed; surfaced on the live stats endpoint from weakness item #9)

| Metric | Warn | Page |
|---|---|---|
| `stale_backlog` (post-run) | > 50 | > 200 |
| `oldest_stale_age` | > 6h | > 24h |
| `failed_canceled` per run | ≥ 1 (investigate) | ≥ 5 |
| `left_ambiguous` trend | rising 3 runs | — |
| `reconciler_last_success` | — | > 2 missed runs (dead-man) |

First production run against the 122 backlog is *expected* to page `stale_backlog` — acknowledge, don't tune thresholds around the incident.

## 6. Tests gating merge (write failing-first per PANEL.md)

1. Resolution table unit test: all six (state × event × validity) combinations land in the spec'd terminal state, including **canceled** and **neither-event-in-window**.
2. Idempotency: run the resolver twice over the same fixtures → identical end state, second run all no-ops.
3. Monotonicity property test: no sequence of resolver operations exits a terminal state.
4. Chunk math: 501 claims → 6 multicall chunks; elapsed-budget exit mid-run loses nothing.
5. Build-time selector/topic derivation assertions.
6. Sepolia e2e (extends `paytest`): (a) settle out-of-band, wedge the claim in `settling`, run reconciler → `settled_reconciled` with correct tx hash, replay path serves free exactly once, then denies; (b) `cancelAuthorization` on-chain → `failed_canceled`; (c) mock-clock expiry → `failed_expired`; (d) valid+payload → re-drive settles, "already used" path re-disambiguates.
7. Regression corpus: the real 122 phantoms (exported, anonymized) become the standing reconciliation fixture set — every defect becomes a vector.

## 7. Acceptance

Backlog 122 → 0 within ≤ 3 runs with zero manual transitions; every resolved row carries evidence (`resolution_tx` or expiry); alarms fired and cleared in a staged drill; Kimi + DeepSeek review filed in `reviews/` (red-team focus: can a forged/replayed log spoof `settled_reconciled`? — note the defense: logs are fetched by *us* from our configured RPC, keyed by indexed `(from, nonce)`, never accepted from a client); gate verdict in `reviews/reconciler-gate.md`.

Out of scope: multi-RPC quorum reads (single trusted RPC is acceptable at this stage; parked with trigger — adopt quorum when settled value/day exceeds what a lying RPC could plausibly steal), event-driven reconciliation via webhooks (cron is sufficient at current volume).
- AMENDMENT (ZCode, 2026-08-19): accepted with the D1-claim-bridge — the route writes a settlements row at CLAIM-time (status='claimed', with payment_payload) so the reconciler's stale-select works; the resolver writes back to the DO on terminal resolution so the replay-entitlement routing is live end-to-end. The DO remains the claim authority; D1 is the reconciler's working set.
- AMENDMENT 2 (ZCode, 2026-08-19, panel gate): §3.B transport is JSON-RPC batched `eth_call`s (chunk 20) instead of Multicall3 `aggregate3` (chunk 100) — identical RPC-budget goal, no hand-rolled dynamic ABI encoding (the class of bug the derived-selector rule exists to prevent). §6 test 4's chunk math becomes "501 claims → 26 batched posts". Re-drive attempts are additionally bounded by a 48h row age cap (red team #4 / Kimi m7), and sweep idempotency (§6 test 2) is enforced by treating "DO already terminal" write-backs as success so a failed D1 write self-heals next run.
