# Stage 0 audit consolidation — 2026-08-19

Panel: **Kimi** (full pass, 6 questions) · **Codex** (dropout — ChatGPT usage
limit hit during launch; joins next gate) · **ZCode** (verification + adoption).

## Verdict
Stage 0 artifacts sound; **7 required corrections identified, all applied**
in `d3780a1`. One genuine spec catch adopted as binding after independent
verification against the vendored text.

## The spec catch (Kimi Q6, verified by ZCode vs vendored §6.1)
`accepts[].extra.paymentFlow` is protocol-reserved; non-`authorization`
flows MUST declare it or conforming clients SKIP the requirement. Adopted
into plan G9: every issued requirement carries
`paymentFlow="upfront"`, `assetTransferMethod="eip3009"`. Deviation note
recorded (extra `/verify` is a wire-invisible G4 quota guard; offering only
`upfront` while clients prefer `authorization` is an accepted trade-off).

## Corrections applied (Kimi required list 1–7)
1. `input_hash NOT NULL` in 0002 — closes pay-once/re-execute-varied-inputs. ✔
2. `payment_payload NOT NULL` persisted at claim; DO holds the live state
   machine, D1 the durable record — header comment documents the split. ✔
3. `settlement_pending` added to the status enum (spec §9 non-terminal; tx
   known, confirmation unknown; cron reconciles). ✔
4. Plan G9 amended (paymentFlow/assetTransferMethod + deviation note). ✔
5. Fixtures 13→16: OPTIONS preflight (400 — no CORS today), `/v1/requests/{id}`
   (400 — route unimplemented today), semantic voucher (validly signed via
   paytest ephemeral key → 400 INVALID_RECIPIENT, proving the EIP-712 chain).
   **Still open:** paid-200 + idempotent_replay fixtures require a funded
   Base Sepolia wallet — operator decision pending. ✔/◐
6. Kill-switch doc: breaker read-failure ⇒ fail-closed; malformed ⇒ false;
   reconciliation cron ungated by the v2 kill; KV write authority. ✔
7. Pre-cut obligations in cdp-findings (reconcile prod's 1 PENDING row
   on-chain; re-run traffic query at Stage 5). ✔

## Nice-to-haves applied / deferred
Applied: CHECKs + COLLATE NOCASE + reconciliation_runs.finished_at.
Deferred: SPEC-VERSION per-file SHA-256s (Stage 1 adds with the conformance
test that reads it); expired-window/wrong-amount vouchers (Stage 2 vector
suite supersedes).

## Panel participation
Kimi ✓ (~9 min, repo-read, cited files) · Codex ✗ usage limit ·
Claude ✗ CLI login still pending · ZCode ✓ (5/5 spec claims verified,
paymentFlow claim verified, corrections applied, fixtures captured).

**Gate: awaiting operator go for Stage 1** (v2 types in m2m-core).
