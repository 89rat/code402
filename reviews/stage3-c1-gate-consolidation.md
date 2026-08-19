# Stage 3 + C1 gate — audit consolidation — 2026-08-19

Panel: **Kimi** (wide-angle per PANEL.md — findings only, cited) ·
**DeepSeek** (red team per PANEL.md — 4 invariant breaks + 4 holds) ·
**ZCode** (orchestrator + gate consolidation in Claude's absent advisor slot).
Commits under audit: `5265205` (C1), `a1afc02` (Stage 3) → corrections in
`ba76a6e`. **64 core tests + 10/10 e2e green** post-corrections.

## Verdict structure (PANEL.md advisor format)

### 1. Findings both reviewers missed
None identified. The two reviews were complementary rather than overlapping:
Kimi owned spec/transport conformance and cross-file regressions; DeepSeek
owned invariant violations. Zero contradiction between them.

### 2. Disposition of every blocker/major (all FIXED in ba76a6e)
- Kimi M1 stamp-500s → FIXED (400 taxonomy). Kimi M2 fake-v1 parser → FIXED
  (real v1: maxAmountRequired, network names, lenient-unknown). Kimi M3 401
  divergence → FIXED (http.md statuses + recovery PAYMENT-REQUIRED on 402s).
  Kimi M4 idempotency regression → FIXED (pre-execution check).
- DeepSeek B1 pricing fail-open → FIXED (explicit error). B2 missing payee
  gate → FIXED (allowed_payees; fixture added). B3 unbound signing → FIXED
  (auth bound to selected requirement; fixture added). B4 mainnet-defaulting
  CHAIN_ID → FIXED (explicit error).
- Kimi minors 5–9 → all fixed (route-bound MAC, dark preflight, ACAO
  everywhere, real telemetry amount, skip-not-abort select, dead import,
  negative e2e coverage via the corrected paths).

### 3. Invariant check (I1–I6)
- I1: HOLD by design (route KV-dark; Stage 4 completes settle-before-serve).
- I2: RESTORED — two real breaks (B2 payee gate, B3 binding) closed with
  fixtures; policy is now deny-by-default across network/asset/payee/amount
  AND signing is bound to the selected requirement.
- I3: HOLD (merchant DO claim = Stage 4; crawler nonce ledger = C2; both
  documented, both next stages).
- I4: HOLD (Stage 4/C2 reconciliation).
- I5: STRENGTHENED — two fail-open defaults (B1, B4) closed; malformed money
  config now refuses to challenge at all.
- I6: HOLDS (both reviewers confirmed; content never enters payment fields).

### 4. Gate verdict
- **Stage 3: PASS** (conditions carried to Stage 4: official @x402/fetch
  client e2e is a HARD prerequisite before any production flip — payv2 is
  our own client and no third-party implementation has spoken this wire yet;
  Kimi explicitly endorsed this condition).
- **C1: PASS** (dry-run-first shipped; policy engine shape proven; the two
  I2 breaks found and fixed are exactly what the mirror principle exists to
  catch — our red team attacked our own client before any crawler does).

## Kaizen retro additions (gate-retro.md)
- Stage 3 leak: e2e tested the happy path + garbage, never the
  malformed-VALID-JSON middle (stamp fields) — catcher: negative e2e class
  "spec-valid envelope, semantically-invalid payment" now mandatory per gate.
- C1 leak: fixtures were written from our own implementation's assumptions
  (v1-shaped-v2) — catcher: fixtures for FOREIGN formats must be derived
  from the foreign spec/docs, never from our types (standing rule).

## Deferred (recorded)
Replay-within-grace + PAYMENT-RESPONSE header + PassThrough facilitation —
all Stage 4. C2 carries the nonce ledger + reconciliation on the client.
