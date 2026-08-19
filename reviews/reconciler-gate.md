# RECONCILER GATE — panel consolidation (dc09b12 + 882e3d8 + kaizen b0e3cac/9430053)

**Date:** 2026-08-19 · **Panel:** Kimi (wide-angle, full-file read) + DeepSeek (red team) + ZCode (builder/consolidator)
**Inputs:** `reviews/reconciler-spec-v1.md` (+Amendments), `reviews/reconciler-e2e-report.md`, both commit diffs, full source.

## Verdicts in

- **Kimi: FIX-FIRST** — 3 MAJOR (M1 re-drive entitlement unreachable after grace; M2 no self-repair on failed D1 write; M3 entitlement unbound-to-input + not consumed on serve), 11 MINOR.
- **DeepSeek red team:** 7 BREAKS, 6 HOLDS. Notable holds: forged-log spoofing (RPC-origin, indexed-keyed — spec §7 defense intact), MAC never waived, kill-switch+breaker gating, chain-level exactly-once makes double-settle impossible on-chain.

## Adjudication & disposition (builder, per PANEL.md disagreement rule)

| finding | adjudication | disposition |
|---|---|---|
| Kimi M2 / DS#1+#2 — DO/D1 divergence wedge (one failed D1 write = permanent error loop, entitlement unavailable) | REAL — convergence bug | **FIXED** (9430053): "DO already terminal" write-backs are idempotent success; D1 converges next run |
| Kimi M3.1 / DS#3 — entitled execution serves ANY input (compute oracle, G2c/I1) | REAL | **FIXED + LIVE-PROVEN**: entitlement carries the original input_hash; diff input → 400, same input → 200 free (oob3 cycle) |
| Kimi M1 — re-drive writes D1 'settled' but DO SettledReconciled → d1_entitled misses after 300s grace; e2e (d) passed only inside grace | REAL — production-only miss | **FIXED**: re-drive writes D1 'settled_reconciled' (matches DO) |
| Kimi M3.2 — served entitlement survives failed store; concurrent entitled retries both execute | REAL (half fixed) | **FIXED (store half)**: settled-store mandatory before respond; failure → retryable 503, entitlement preserved. Lease-on-entitled-claim tracked as follow-up F-4 |
| DS#4 / Kimi m7 — re-drive forever on gate-passing-but-rejected garbage | REAL (bounded DoS) | **FIXED**: 48h row-age cap (~48 attempts max/row) |
| Kimi m1 — pad32 underflow panics sweep on corrupt payer | REAL | **FIXED**: saturating + payer shape validation |
| Kimi m9 — d1_entitled fail-open → wrongful non-retryable 400 on D1 brownout | REAL (availability) | **FIXED**: unknown → retryable 503 |
| Kimi m8 / DS duplicated classification | REAL (drift risk — the exact G2d bug pattern) | **FIXED**: core `classify_settle_failure`, matrix-tested, both call sites |
| Kimi m5 — cancel alarm missing | REAL | **FIXED**: alarm + `ops:canceled_last_run` |
| Kimi m2 — Multicall3 vs JSON-RPC batch undocumented | REAL (gradeability) | **FIXED**: spec Amendment 2 |
| Kimi m3 — deep scan capped 3.7d, keyed on updated_at, silent | PARTIAL | **FIXED (visibility)**: exhaustion escalates. Full to-creation paging = follow-up F-1 |
| Kimi m4 — per-transition audit lines; error column type confusion | REAL (spec §3.D) | follow-up F-2 |
| Kimi m6 — unparseable-payload rows never resolve | REAL | follow-up F-3 (needs operator queue decision) |
| Kimi m10 — served entitlement rows invisible to status='settled' queries | REAL (observability) | follow-up F-5 |
| Kimi m11 — §6 test 2/4/7 gates | test 7 **DONE** (b0e3cac, 132 phantoms); test 2 discharged by M2 fix + two live no-op re-sweeps; test 4 superseded by Amendment 2 math (26 posts/501) | documented here |
| DS#5 — SettledReconciled→Settled DO transition "absorbing violation" | HOLD **by design** — documented maturation; D1 stays absorbing; core rejects all other exits | no change |
| DS#6 — re-drive vs live request race double-settles | HOLD — EIP-3009 nonce = exactly-once at contract level; worst case a doomed CDP submission (quota) | accepted with rationale |
| DS#7 — NOCASE payer collision splits DO keys | HOLD — the signature pins `from` byte-exact; case variants can't produce two valid claims | accepted; NOCASE is the uniqueness backstop |

## Post-fix verification (all live, this session)

- Full entitlement cycle re-run with all fixes: OOB → 503 wedge → sweep `resolved_used:1` → same-input **200 free** → identical replay → D1 `settled_reconciled/reconciled_used` (oob4, tx `0xfd5901ae…`)
- Input-binding: different input on a live entitlement → **400** (G2c enforced)
- Suites: core **86/86**, wire 7/7, official-client e2e PASS
- Remote staging D1 schema current (0002–0004 applied — deploy-preflight gap found & closed)

## Consolidated verdict

Kimi's FIX-FIRST conditions (M1/M2/M3) are **discharged with live evidence**; remaining items are tracked follow-ups (F-1..F-5), none money-side. Per PANEL.md the builder does not self-merge: **conditional SHIP, pending Kimi re-review of 9430053** at the next panel convening. DeepSeek's spoof/hold analysis requires no action.

## Follow-ups register

F-1 deep-scan paging to creation date · F-2 per-transition audit lines + run-row column hygiene · F-3 terminal exit for unresolvable (unparseable-payload) rows · F-4 lease on entitled claim (concurrent entitled retries) · F-5 served-marker for settled_reconciled rows in stats queries · F-6 pure chunk-math test (Amendment 2 math)
