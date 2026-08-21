# Cross-Verify: Whole-Machine Adversarial Review of code402

**Date:** 2026-08-19 · **Prompt:** identical adversarial architecture review (find better ideas for the whole machine, citations mandatory)
**Panel:** Kimi CLI 0.36.1 (full staged repo read) · DeepSeek v4-pro (API, ~114K chars of inlined docs + sources) · ZCode (independent self-review + live web verification)
**Dropouts:** Codex (usage limit until Sep 18) · Claude Code (not logged in — needs `claude /login`) · Gemini (no credentials on this machine — see §7)

Scope note: Kimi's run started before `crates/` and root docs landed in its workdir; its payment-path findings are drawn from plans/reviews/site/SDK (which cite the code by file:line). DeepSeek received all key sources inlined. ZCode read the full repo directly.

---

## 1. Convergent findings (both external agents + ZCode agree — highest confidence)

### C1. The per-payment Durable Object claim machine should be deleted; D1 is the sole claim authority
- Kimi M5: G3 keys a DO per payment forever, then needed a D1 claim-time bridge because DO state wasn't queryable — the DO/D1 divergence bug class dominated the reconciler gate (`reviews/reconciler-gate.md:15-17`). D1 single-writer SQLite gives `INSERT … UNIQUE(payer,nonce)` as the claim and guarded `UPDATE … WHERE status IN (...)` as the transition law.
- DeepSeek #3 + #2: same conclusion, plus a second defect Kimi missed — **terminal DO instances are never deleted** (`crates/edge/src/settlement_do.rs` never calls `delete_all()`), so every payment leaks DO storage forever; an attacker can mint unbounded DOs.
- ZCode concurs: after the claim-time D1 bridge (RECONCILER-SPEC amendment), the DO adds coordination cost without unique authority. One authority, no divergence class, no storage leak.
- **Better design:** D1-only claim machine — `INSERT … ON CONFLICT(payer,nonce) DO NOTHING RETURNING status`, lease columns (`lease_owner`, `lease_expires_at`), guarded absorbing transitions; keep the DO pattern parked for genuinely long-lived coordination.

### C2. Hourly-only reconciliation strands paid-but-unserved agents for up to ~85 minutes; make it push/inline
- Kimi M6: the disambiguator is one `eth_getLogs` on indexed `(authorizer, nonce)` (`reviews/reconciler-spec-v1.md:56`) — it can run inline on the ambiguous path during the retry the client is already making.
- DeepSeek #4: same; suggests chain-event webhooks (CDP/Alchemy/QuickNode) into a Queue for near-real-time resolution.
- **Better design:** on-demand targeted resolution inline on 503 `settlement_pending` (fail-soft to cron), hourly sweep as backstop, webhook push when volume justifies it. Reconciliation becomes push-assisted, not calendar-bound.

### C3. Pricing is underwater against the measured facilitator fee; the batching trigger already fired
- Kimi M4: `cdp-findings.md:22` measured **$0.001/settle past 1,000/month free** vs list prices $0.002–0.005 — a 20–50% take rate; `stress-2.md:8` burned the monthly free tier in a single 310-second test; CDP `batch-settlement` is documented in the same file and parked anyway.
- DeepSeek #9/#11: facilitator gas is an unmanaged cost assumption; dust payments buy unbounded state for negligible revenue.
- **Better design:** price floor ≥10× marginal settle fee; adopt `batch-settlement` (or the parked `upto`/session flow) for sub-cent tools; per-payer rate limits / cost-based minimums against dust griefing. Design-logic §10's parking trigger ("only if per-settle fees return") has already fired — the docs should say so.

### C4. Machine-discovery artifacts must be generated from one source of truth
- Kimi B1: `site/public/.well-known/x402.json:18` publishes `"recipient": "RUNTIME_ENV:COMPANY_WALLET"` — a literal unexpanded placeholder; `x402.json:3` says `"staging"` while `llms.txt:11` says production real USDC; m7: prices disagree across four files.
- DeepSeek #10/#13: static `x402.json` in `site/public` can shadow the per-environment worker-rendered manifest; no agent-e2e self-test exists.
- **Better design:** one route-config source generates `x402.json`, `llms.txt`, `openapi.yaml`, and the Pricing page at deploy time; CI asserts the published recipient equals the stamped `payTo`; smoke test that an autonomous agent can navigate discover → 402 → pay → verify from manifests alone.

## 2. Unique catches (one agent only)

**Kimi:**
- **B2 (blocker)** — Public discovery teaches the X-PAYMENT dialect the roadmap hard-cuts with zero sunset; a conformant `@x402/fetch` agent cannot pay production today, and after the flip, agents trained on llms.txt hit a dead route with no `Sunset` header. Keep `/v1/*` as a shim emitting `Sunset` + v2-compatible 402; ship v2 discovery docs *before* the flip.
- **M1 (major)** — `Trust.tsx:96` markets a mainnet settlement that `cdp-findings.md:52` records as 0 SETTLED / 1 PENDING never confirmed in the prod ledger. Evidence-first brand can't cite txs that aren't terminal rows with `resolution_tx`.
- **M2 (major)** — Site sells XDR-1/JCS offline-verifiable receipts; the shipped SDK implements a bespoke keccak commitment with no JCS and recovers over a raw 32-byte hash with no EIP-191/712 domain separation (`sdk/src/index.js:47-71`).
- **M3 (major)** — SDK has no payee/asset policy gate (`to: ch.recipient` signed blindly); the crawler client fixed this exact hole (stage-3 red-team Break 2) and the fix never propagated. Pin `payTo`/`asset` from the manifest; deny-by-default policy hook.
- **M7 (major)** — `specs/model/claim.tla` models 5 states; migration 0003 added 3 terminal states the same day; `receipt_pending` marked absorbing though the reconciler exists to leave it; only `Absorbing` is formalized — INV-A never actually checked. Regenerate the model from the D1 state machine (post-C1 there is exactly one) and gate CI on model-states == migration enum.
- **M8 (major)** — `mcp.json` is not an MCP manifest; no MCP server exists; the playbook schedules MCP registry submissions that will be rejected. A Workers MCP adapter over existing tool routes makes "monetizable MCP tools" literally true.
- Minors: alarms surface on a public page and page nobody (m1); single trusted RPC can mint/hide entitlements — two-provider compare is ~20 lines (m2); cancel-probing pages on existence not rate (m3); replay path withholds the paid output (m4); SDK returns fabricated `PENDING_SETTLEMENT` literal (m5); `/verify` answered two opposite ways in binding docs (m6); $10 probe cap contradicts the observatory dataset claim (m8); PANEL.md/OPERATING-CONTRACT.md absent from repo (m9 — actually present in repo root; Kimi's workdir lacked them at start — artifact of staging); dead `new_home.tsx` (m10).

**DeepSeek:**
- **#1 (blocker)** — No OFAC/payer screen on the live mainnet path, though it's in the operating contract's own immediate queue; strict-liability exposure + USDC freeze risk. Screen before `/settle` and at reconciler write-back.
- **#5 (major)** — "Append-only ledger" isn't: `settlements.status` mutates in place; audit cannot reconstruct transitions. Add append-only `settlement_events` (from_status → to_status, reason, tx) with `settlements` as a projection.
- **#6 (major)** — Two reconciler run tables (`reconciliation_runs` vs `reconciler_runs_v2`) will drift.
- **#7 (major)** — Web Bot Auth key directory (`/.well-known/http-message-signatures-directory`) missing — Cloudflare verified-bot discovery would 404.
- **#8 (major)** — Every 402 challenge writes a D1 row: unauthenticated write amplification and analytics contamination. Persist settlement attempts, sample challenges.
- **#12 (major)** — Body-only `idempotency_key` deviates from the header idempotency model generic x402 clients use; accept the header now.

**ZCode (self-review + web verification):**
- **Z1 (major, web-verified)** — The crawler plan's **Rail B targets a retired program**: Cloudflare retired Pay Per Crawl on July 1, 2026 and replaced it with **Pay Per Use** (publishers paid when content shapes an answer; first partners Ceramic.ai and You.com). `paying-crawler-plan.md` decision 2 (`crawler-max-price` in signed components, PPC crawler-side signup) needs re-verification against the current program before C0 filings are submitted. The Sept 15, 2026 default-blocking deadline is real but **scoped**: new domains, new sites, free-tier, ad-bearing pages; paying customers keep dashboard overrides — "every AI company needs this by Sept 15" overstates the forcing function. [Sources: developers.cloudflare.com/bots/block-ai-bots; Cloudflare July 1, 2026 announcement coverage]
- **Z2 (minor)** — `ops:settle_pending_count` breaker counter is a KV read-modify-write across isolates (`x402v2_route.rs:634-646`): racy and eventually consistent. Fine as a heuristic; document it as such or move to the DO (which already exists per-key) — but post-C1, a D1 counter update is atomic.
- **Z3 (minor)** — Both crons fire at minute 0 (`wrangler.toml:56,94`) — top-of-hour congestion across tenants; pick off-peak minutes (e.g. `:11`, `:47`). Trivial.

## 3. Divergences

- **DO claim machine:** Kimi and DeepSeek both say delete it; the repo's design-logic §6 argues the DO's single-threaded serialization is the clean exactly-once story and was model-checked. Panel position (2 external + ZCode): the D1 UNIQUE + guarded-UPDATE pattern achieves the same law with one authority, and the reconciler already treats D1 as the bridge of record. The design-logic's argument predates the claim-time bridge that made the divergence concrete. Not a unanimous-in-principle verdict against DOs generally — scoped to *per-payment* DOs.
- **`/verify` before `/settle`:** binding docs disagree with each other (plan-rev3 G9 note vs launch-checklist Option A); Kimi m6 says pick one and update both. UNRESOLVED — operator decision; spec §7.1 supports Option A (upfront omits `/verify`).

## 4. What survived adversarial review

- Settle-before-serve (I1), the claim→settle→execute→persist ordering, and the entitlement design (paid-but-unserved → free re-execution bound to original input) — no agent found a break; Kimi's entitlement-store-before-respond fix (M3.2, already in code at `x402v2_route.rs:816-833`) is correct.
- The G6 HMAC stamp over canonical requirement + route binding, constant-time comparison, iat grace — clean.
- Fail-closed kill-switch/breaker semantics (I5) — consistently implemented.
- The mirror principle (merchant/crawler as one threat model) and one-core-two-products — validated as the right skeleton by both external reviewers' "rebuild from zero" answers, which keep the same payment path shape.
- Sept 15 2026 deadline, Web Bot Auth/RFC 9421, Ed25519 identity — externally verified real.

## 5. Ratings

| Agent | Depth 1–10 | Note |
|---|---|---|
| Kimi | 9 | Widest cross-file net (site↔manifest↔ledger↔SDK contradictions); ran without crates/ — payment-path findings inferred via reviews |
| DeepSeek | 8 | Sharpest on data-model and ops-hygiene (DO leak, event sourcing, duplicate tables, write amplification, OFAC) |
| ZCode | — | Web-checked the two market-timing claims; confirmed skeleton, found Rail B program retirement |

## 6. Consolidated top actions (merged, deduped, ranked)

1. **Fix the public truth layer first** (C4 + Kimi B1/M1): deploy-generate all machine-facing files from one config; unpublish the placeholder recipient; reconcile or unpublish the marketed mainnet tx.
2. **Delete the per-payment DO; make D1 the single claim authority** with atomic INSERT/UPDATE + lease columns (C1) — also fixes the DO storage leak (DeepSeek #2) and makes the TLA+ model regenerable from one state machine (Kimi M7).
3. **Inline/push reconciliation** on the ambiguous path; cron as janitor (C2).
4. **Re-price against the $0.001/settle reality** and adopt batch-settlement for sub-cent tools (C3).
5. **OFAC payer screen before mainnet volume** — already in the operating contract queue; promote to a Stage-5 hard gate (DeepSeek #1).
6. **V1 sunset shim + v2 discovery docs before the flip** (Kimi B2); SDK: manifest-pinned payTo/asset + real receipt verification (Kimi M2/M3).
7. **Re-verify Rail B** against the post-July-1 Pay Per Use program before C0 filings (Z1).
8. **Stand up a real MCP server** or delete `mcp.json` and the registry line item (Kimi M8).
9. **Append-only `settlement_events`**; merge the two reconciler-run tables; stop persisting per-challenge rows (DeepSeek #5/#6/#8).
10. **Ship the Web Bot Auth key directory** with C0 (DeepSeek #7).

## 7. Panel participation

| Agent | Status |
|---|---|
| Kimi CLI 0.36.1 | ✅ full verdict (~18K chars) |
| DeepSeek v4-pro (API) | ✅ full verdict (~11K chars) |
| ZCode | ✅ self-review + web verification |
| Codex 0.145.0-alpha | ❌ usage limit until 2026-09-18 |
| Claude Code | ❌ not logged in (`claude /login` needed) |
| Gemini | ❌ no credentials: no `GEMINI_API_KEY`, no `~/.gemini` OAuth creds, no npx on PATH. To include Gemini next run: install Node standalone + `npm i -g @google/gemini-cli`, then either `setx GEMINI_API_KEY <key>` or one interactive `gemini` login. |

Verdict files: `verdict-kimi.md`, `verdict-deepseek.md` beside this report; scratch dir `~/.zcode/tmp/cross-verify-20260819-1630/`.
