# Wide-Angle Review — code402 full repo (Kimi, evening pass)

**Date:** 2026-08-19 (second pass — first pass ran without `crates/`; this pass
read crates/core, crates/edge, migrations, plans, and the reconciler in full).
**Rules:** findings only; severity; citation; failure scenario; concrete better
alternative. Overlaps with today's cross-verify are marked [C1–C4/known].

## Blockers

### B1 — Paid-but-tool-failed is terminally lost: no entitlement, no replay, invisible to the reconciler
`crates/edge/src/x402v2_route.rs:554-563` — on post-settle `execute_tool`
failure the claim is marked `failed` (terminal per
`crates/core/src/payment/settlement.rs:157`), D1 is marked `failed`, and the
payer gets a 500. But the money already moved. The reconciler sweeps only
`NON_TERMINAL` statuses (`crates/edge/src/reconciler.rs:35`, stale-select at
`:60-68`), so a `failed` row with a consumed on-chain nonce is never revisited.
Result: chain says paid, ledger says failed, payer has nothing — violating I1's
guarantee and G2c (bounded free re-execution), which the schema already
anticipates (`migrations/0002_settlements.sql:52-53` `reexec_count`,
`reexec_window_until`) but no code writes from the request path. The in-code
comment defers this to "Stage 4 hardening"; as written, it is the single worst
money-state the system can produce, and it is silent.
**Better:** post-settle tool failure must never produce plain terminal
`failed`. Either (a) settle + store the structured error as the response body
(identical-200 replay of the failure, G2b semantics), or (b) transition to
`settled_reconciled`-style entitlement with `reexec_window_until` set, letting
the payer re-execute free ≤3/24h. Option (b) already has every machine part.

## Majors

### M1 — 6492 PassThrough is an unguarded facilitator-quota drain (Rev 3 G4 rate limit not implemented)
`crates/core/src/payment/x402v2_verify.rs:141-145` passes any >65-byte
magic-suffixed envelope through to the facilitator; the route
(`crates/edge/src/x402v2_route.rs:267-276`) then spends a `/verify` or
`/settle` call. No per-IP/per-payer rate limit exists anywhere in the v2 route
(grep-confirmed; `RATE_LIMITED` exists only in the v1 taxonomy,
`crates/edge/src/lib.rs:16`). An attacker mints unlimited well-formed 6492
envelopes at zero cost, burns the CDP free tier (stress-2 burned 1,000 settles
in 310s), the breaker trips (I5), and honest payments stop — a quota DoS that
converts directly into payment-path downtime. Rev 3's G4 requires a
per-IP/payer rate limit before any facilitator call; the code has the
structural gate but not the limiter.
**Better:** token bucket per payer + per cf-connecting-ip before the
facilitator seam (D1 counter or KV — post-C1 a D1 counter update is atomic);
consider a 6492 factory-address allowlist as a cheap static prefilter.

### M2 — DO idempotency concession matched on error strings across an HTTP boundary
`crates/edge/src/reconciler.rs:312` and `:323` treat
`e.to_string().contains("reconcile_settled from SettledReconciled")` as
success. The string originates in
`crates/core/src/payment/settlement.rs:401,425` (`format!("... from {:?}")`),
crosses HTTP via `Response::error` (`settlement_do.rs:141`), and is re-wrapped
at `x402v2_route.rs:384`. A variant rename or wording change in core silently
breaks the concession and wedges rows forever — the exact failure the comment
says it prevents.
**Better:** typed error body from the DO (`{"error":"ILLEGAL_TRANSITION",
"from":"SettledReconciled"}`), matched structurally. Dies for free under R1.

### M3 — CDP JWT `uri` claim hardcodes the production host while the base URL is env-driven
`crates/edge/src/facilitator.rs:88-89`: `host` is the literal
`"api.cdp.coinbase.com"` regardless of `CDP_FACILITATOR_BASE`. Any non-default
base (staging proxy, or the design-logic §10 self-hosted/secondary facilitator
seam) mints JWTs whose `uri` claim mismatches the actual request → auth
failures indistinguishable from quota/transport errors on the money path.
**Better:** parse host+path from `self.base` when building the uri claim.

### M4 — JWT nonce collides within the same second
`crates/edge/src/facilitator.rs:66-67`: nonce = `n{unix_seconds}-{secret_tail}`.
Two settles in the same second under one key produce identical JWTs' nonces;
if CDP enforces nonce uniqueness (replay protection — the header exists for
that), the second request fails as an auth error mid-settle: a self-inflicted
ambiguous-money generator. (The secret tail encodes public-key bytes, so this
is not a seed leak, but secret-derived nonces are still bad hygiene.)
**Better:** millisecond + random component; `getrandom` is already in the
dependency tree.

### M5 — Entitlement TTL is double-bookkept across D1 and the DO; the two can disagree against the payer
The stamp-age bypass reads D1 `replay_eligible_until`
(`x402v2_route.rs:656-667`); the claim authority holds
`reconciled_eligible_until` in the DO (`settlement.rs:51-53`). Written by
different paths (`d1_resolve` vs DO write-back), one can land without the
other. DO-first law (`reconciler.rs:291-296`) means a D1 hiccup after a
successful DO write leaves D1 unentitled — the late retry then 400s at the
stamp-age gate instead of executing free, exactly the case
`reconciler.rs:405-410` (Kimi M1 fix) patched for the re-drive path only.
**Better:** the entitlement check should ask the claim authority, not the
projection. Post-R1 there is exactly one clock and this class disappears.

### M6 — Reconciler silently defaults to Sepolia on a bad CHAIN_ID; the route fails closed, the janitor doesn't
`crates/edge/src/reconciler.rs:55`: `.parse().unwrap_or(84532)`. The route
correctly fails closed on an invalid CHAIN_ID (red-team Break 4,
`x402v2_route.rs:282-286`). A malformed prod var (real chain 8453) makes the
reconciler read Sepolia state: `authorizationState` false for everything →
re-drive loop until the 48h cap, then `failed_expired` for live mainnet
claims — the reconciler actively corrupting the ledger it exists to protect.
**Better:** fail closed, same as the route; a reconciler that cannot name its
chain must not run.

## Minors

- **m1** — Unauthenticated idempotency pre-check (`x402v2_route.rs:163-178`)
  is a key-existence oracle and an unauthenticated D1 read on the hot path.
  Scope it behind a presented payment or return the stored replay only.
- **m2** — `X-Schema-Version` is not in `Access-Control-Expose-Headers`
  (`x402v2_route.rs:29`); browser agents cannot read the version your own
  versioning doctrine relies on. One line.
- **m3** — `payment_ref: auth.nonce.parse().unwrap_or_default()`
  (`x402v2_route.rs:578`, `:805`): a parse failure binds the receipt to
  0x00…00 silently. The gate makes it unreachable today; the panic-deny
  doctrine argues for a checked conversion.
- **m4** — `tool_version: "1.0.0"` is hard-coded at both receipt sites
  (`x402v2_route.rs:572`, `:801`); the observability spec (EXCELLENCE-SPEC §6)
  records tool_version as a first-class field. Wire it to a real version.

## What survived this pass

Settle-before-serve ordering; MAC route-binding with constant-time compare;
fail-closed KV/secret reads on the money path; guarded absorbing UPDATEs as
the executable form of the absorbing law; ambiguous-money classification
(fail-closed receipt_pending, never a guessed terminal); claim-time D1 bridge;
the 48h re-drive age cap; deep-scan escalation logging. The mirror principle
holds in code, not just in the doc.

---

# Redesign / reimagine (discussion draft, not gate artifacts)

## R1 — Collapse the claim authority: D1-only, one round trip
Accelerates cross-verify C1. D1 (single-writer SQLite) gives
`INSERT … ON CONFLICT(payer,nonce) DO NOTHING RETURNING status` as the claim
and guarded `UPDATE … WHERE status IN (…)` as the transition law — atomic, one
authority, one clock. Deletes: `settlement_do.rs`, the JSON command layer,
3 DO round-trips per paid call, the DO storage leak (DeepSeek #3), the DO/D1
divergence class, M2's string matching, M5's double clock. Keep the DO pattern
parked for genuinely long-lived coordination. The pure step functions in
`settlement.rs` survive unchanged as the transition law — only the store
changes. **Roughly 40% of today's open findings die in this migration.**

## R2 — Client-driven reconciliation (extends C2)
The retry the client is already making becomes the reconciler. On any
`settlement_pending` retry, run the targeted `eth_getLogs((authorizer,
nonce))` inline (one indexed read — the evidence functions
`evidence_from_reads`/`resolve` are already pure), then replay, entitle, or
settle accordingly. Cron drops to backstop. Worst-case ambiguity window:
~85 minutes → one client round-trip. `Retry-After` on the 503 makes it
self-scheduling.

## R3 — Own the facilitator (reframe the parked item)
The parking trigger in design-logic §10 ("state channels/batching only if
per-settle fees return") has already fired: C3 measured $0.001/settle against
$0.002–0.005 list prices, quota is the scarce resource, and M1/M3/M4 are all
CDP-seam defects. A minimal self-facilitator is code that already exists in
this repo: verify = the local structural gate + ecrecover prefilter promoted
to authority; settle = broadcast `transferWithAuthorization` via your own RPC
with a managed sender key; confirmation = your reconciler. CDP stays as
fallback behind the same `Facilitator` trait. This removes the quota ceiling,
the per-settle fee, and the largest external dependency — and makes "the
chain is the root of truth" literal instead of proxied. New costs: gas
(fractions of a cent on Base), sender-key nonce management, and a policy
decision about competing with facilitators your index ranks.

## R4 — Entitlement → signed credit note (the reimagine)
Today: paid-but-unserved → one free re-execution bound to the original input.
Reimagine: the failure mints an **XDR-1 credit note** — same JCS machinery,
same offline verifiability — denominated in amount_minor, redeemable against
any tool, 24h expiry, non-transferable. "We never refund; we issue
chain-reconciled bearer credit." The worst UX moment becomes a live
demonstration of the receipt product, and credit notes become a primitive in
the receipt economy (B1's fix falls out of this: post-settle tool failure =
auto-minted credit note). Guards: amount-denominated (no compute-oracle
arbitrage), redemption is a ledger row under the same exactly-once law.
Open question: counsel check against the zero-custody position — a bearer
credit note must stay clearly non-custodial and non-monetary (your §1960
control-test reasoning should cover amount-capped, expiring, non-transferable
service credit, but it deserves the same written validation).

## R5 — Conformance as the first paid product (business reimagine)
The prober, the vectors, and the failure taxonomy already exist as exhaust.
Flip the cost center: sellers pay (in x402, dogfooded) for conformance
certification and a badge API; the price index stays free as the wedge.
Revenue arrives before DIS-1 registry and take-rate escrow exist.

## R6 — Typestate as code, not doc
Design-logic §4 promises `Payment<Received> → … → Payment<Settled>`; the code
is runtime enums plus stringly JSON commands. After R1, implement the
typestate in core — illegal transitions stop compiling, and the TLA+ model's
job shrinks to lease/replay interleavings only.

## R7 — One manifest generator + agent self-test (adopts C4)
Route config generates `x402.json`, `llms.txt`, `openapi.yaml`, `mcp.json`,
and the pricing page at deploy; CI asserts published recipient == stamped
`payTo`; e2e proves an autonomous agent can navigate discover → 402 → pay →
verify from manifests alone.

## R8 — Economics rebase
Price floor ≥10× marginal settle cost; publish quoted-vs-settled baselines as
the free wedge; `upto`/session scheme for sub-cent tools once R3 removes the
facilitator floor.
