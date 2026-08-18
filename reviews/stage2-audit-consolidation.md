# Stage 2 gate — audit consolidation — 2026-08-19

Panel: **Kimi** (wide-angle, full repo; re-ran the entire suite AND the
differential harness live before verdicting) · **DeepSeek** (red-team pass on
composite inline) · **ZCode** (orchestrator; verification + adoption).
Claude ✗ (CLI auth pending — advisor role covered by this consolidation;
appends retroactively per PANEL.md status note). Codex ✗ (quota).

## Commits under audit
`ac22567` (stage-2 body) → `6306ceb` (audit corrections + PANEL.md).
**54 tests green** (31 unit + 4 crypto + 19 stage-1). Differential 300/300.

## Convergence — both panels independently found the same three defects
1. **Quota leak via long non-magic hex** (Kimi Q2 = DeepSeek #3): blanket
   `>65-byte → PassThrough` spent facilitator quota on garbage. FIXED: 32-byte
   EIP-6492 magic check in prefilter AND gate; 2048-hex ceiling.
2. **validAfter mis-mapped** (Kimi Bug A = DeepSeek #4): future-valid auths
   emitted the validBefore §9 code. FIXED: `ValidAfterFuture` variant.
3. **Recipient/signer mismatch collapse** (Kimi Bug B = DeepSeek #4):
   both fell into `BadAddress → invalid_payload`. FIXED: dedicated
   `RecipientMismatch` → recipient code; signer mismatch → signature code;
   reachability test proves every §9 code reachable or facilitator-origin.

## Unique findings applied
- DeepSeek: `assetTransferMethod` guard on the prefilter (only eip3009 is
  locally hashed; anything else passes through) — defensive, applied.
- Kimi: CI/pin enforcement (nightly workflow + exact SDK pin + SPEC-VERSION
  drift check), MAC-over-canonical-serialization note (launch-checklist #9),
  fixture-class manifest, invalid-v fixture.
- Kimi verified my differential finding independently: byte divergence is
  `extra` key order ONLY (BTreeMap vs SDK insertion), top-level byte-identical,
  G6 MAC unaffected.

## Findings assessed and NOT applied (with rationale)
- DeepSeek "explicit v validation before recover": already enforced —
  `eip712.rs:24-28` rejects v ∉ {0,1,27,28} via `InvalidRecoveryId`; the
  inlined materials didn't include eip712.rs. Confirmed by the new
  `invalid_v` fixture. No change needed.
- DeepSeek "SettleResponse settlement_pending invariant": already enforced +
  tested since Stage 1 (`settle_response_pending_requires_tx`); Kimi (repo
  access) confirmed. No change needed.

## Gate verdict (per PANEL.md advisor structure)
1. Findings both reviewers missed: none identified at blocker/major; the
   validAfter fixture-class location question was settled by citation (§9
   assigns it a taxonomy code; the check lives in the gate where the
   timestamp compare happens — accepted by both).
2. Disposition: every blocker/major from both verdicts is FIXED in `6306ceb`;
   none refuted-open; nice-to-haves deferred to the record (below).
3. Invariants: I1/I3/I5 untouched by this stage (no serving/settling yet);
   the prefilter hardens the G4 quota guard that I5's fail-closed breaker
   depends on; I6 unaffected (protocol fields only).
4. **Verdict: PASS.** Stage 2 closes. Crawler track C1 unblocks per
   plans/integrated-roadmap.md.

## Deferred nice-to-haves (recorded, non-blocking)
Malformed-input differential corpus; PaymentPayload/SettleResponse codec
fuzz; facilitator errorReason validation against §9 strings in
SettleResponse::validate; larger nightly corpus fields variance (extensions
maps, multi-accept).

## Kaizen retro (gate-retro.md line)
Stage-1 leak: taxonomy-code reachability was never proven — Stage-1's
exhaustive `map_error` satisfied the compiler but left two §9 codes dead.
Catcher: the new reachability test class (every code reachable or documented
facilitator-origin) — now a standing pattern for every enum↔enum mapping.
