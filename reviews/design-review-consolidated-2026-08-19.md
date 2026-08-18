# Design Review: x402 v2 Upgrade Plan — Panel Consolidation

**Date:** 2026-08-19 · **Reviewers:** Kimi (kimi-k2, full repo read) · ZCode (self-review + consolidation) · Claude Code (pending — CLI auth; critique to be appended when available)

## Verdict on original plan
Staging and vendored-spec + cross-implementation-vector spine endorsed. Two architecture decisions were being dodged and are now forced into Stage 0 as explicit gates. Permit2 cut.

## Adopted changes (from Kimi review, code-cited, plus ZCode self-review)

### Forced decisions (Stage 0 gates, panel-recommended defaults)
1. **Settle-before-serve** (canonical facilitator flow: verify → settle → execute → PAYMENT-RESPONSE). Current code serves output before settlement confirms (`lib.rs:382-428`) — a payment hole under facilitator settlement. Latency cost (block time on paid calls) acknowledged; optimistic serving only as written risk-acceptance, not default.
2. **Parallel `/v2/` route + dated sunset** for the bespoke X-PAYMENT dialect — not a config-flag fork in one handler. The "legacy" format is private (only our own SDK speaks it), not x402 v1.
3. **Facilitator-authoritative verification** — local EIP-712 recovery kept for telemetry only, because `recover_address` handles only 65-byte ECDSA and EIP-6492 smart-wallet payers would be wrongly 401'd. CDP `/verify` is the judge.
4. **Permit2 cut** from scope — doubles crypto surface, zero demand, USDC has native EIP-3009 on both chains.

### Stage-by-stage deltas
- **Stage 0:** SPEC-VERSION file (vendored commit + SDK version + facilitator API version, read by CI so drift fails loudly); snapshot e2e of current 402/X-PAYMENT behavior BEFORE refactoring (zero HTTP-level tests exist today); conformance checklist as executable fixtures, not markdown; D1 migration for settlement table; CDP secret provisioning; KV kill-switch for instant v2 disable.
- **Stage 1:** `amount` as decimal string → U256-native (current `amount_minor: u64` truncates); Base64 codec with size caps (Cloudflare ~32KB header ceiling; oversized → 4xx, never panic); byte-exact golden vectors committed under `crates/core/tests/vectors/`.
- **Stage 2:** bidirectional vectors (TS→Rust AND Rust→TS — one-directional can't catch a shared misreading); domain-name divergence (Sepolia "USDC" vs mainnet "USD Coin") as an explicit vector; v-normalization (0/1 vs 27/28); expiry boundaries; error taxonomy mapping to spec strings (no internal names leaked).
- **Stage 3:** client-owned nonce (server stops minting challenges — spec clients pick their own 32-byte nonce; NonceGuard demoted to legacy path only); field-by-field re-verification of the client-echoed `accepted` requirement (classic exact-scheme cheat: client swaps amount in echo); exact means exact (`value == amount`, current `>=` is divergent); CAIP-2 everywhere (`eip155:8453`, not `"base"`); CORS `Access-Control-Expose-Headers` for the three payment headers; update `/.well-known/x402.json`, `llms.txt`, `openapi.yaml`, `mcp.json` (all currently advertise the bespoke format); header normative / body decorative.
- **Stage 4:** sequence verify → DO-keyed idempotency claim → settle → execute → respond; DO (single-threaded) for mutual exclusion keyed by payment nonce + D1 record with UNIQUE constraint (not D1-as-lock); at-least-once retry trap: "authorization already used" on settle-retry maps to success, not failure; facilitator failure matrix as fixtures (timeout/5xx/4xx/verify-ok-settle-fail/settle-ok-timeout); circuit-breaker flag in KV → fail closed; health check on existing hourly cron.
- **Stage 5:** SDK helpers mirror the spec flow, old SDK importable unchanged; v2/network selection as pure `[vars]` (one-var mainnet flip); update AGENTS.md/DEPLOY.md/sdk README; idempotency replay path extended to carry settlement tx reference (client must recover tx after a 5xx).

### Added workstream items
Monthly spec-drift watch (cron/intel pipeline); rollback runbook (KV kill-switch, no redeploy); concurrency test (two simultaneous same-nonce requests → exactly one settle) against the DO.

### Testing suite (new, blocks Stage 1)
HTTP-level e2e (wrangler dev/miniflare) snapshotting legacy; concurrency; duplicate delivery (queue + client retry); facilitator failure matrix; malformed-input fuzz (non-base64/oversized/huge amounts/wrong version/v1-shaped payloads — no panics); bidirectional golden vectors; expiry/clock skew; CORS assertions; regression on non-payment surfaces (trust registry, badge, manifests).

## Artifacts
- Kimi full review: `/tmp/xv-design/verdict-kimi.md` (to be committed under `code402/reviews/` at Stage 0)
- Claude critique: pending CLI login
