1. **Blocker — Production mainnet settlement path has no OFAC/sanctions screen, despite explicit go-live requirement.**  
`OPERATING-CONTRACT.md` Immediate queue #3 says “Add OFAC denylist screen to the worker before first real-money settlement”; `EXCELLENCE-SPEC.md §9` lists sanctions screening as priority 1; `wrangler.toml` `[env.production.vars]` sets `CHAIN_ID = "8453"` and routes `code402.dev` to production. Once a payer is challenged and the facilitator settles, the merchant wallet receives USDC without a payer screen. That creates strict-liability exposure if a sanctioned address can pay, and USDC can freeze the wallet.  
**Better alternative:** enforce a denylist/SDN screen before `/settle` and before serving; reject with a `PAYER_BLOCKED` error and do not broadcast settlement. Add the screen at both the route layer and reconciler write-back layer so chain-proved payments from newly listed addresses are also blocked.

2. **Major — Durable Object claim objects are never deleted; terminal claims leak DO storage forever.**  
`crates/edge/src/settlement_do.rs` has terminal transitions (`settled`, `failed`, `receipt_pending`, `settled_reconciled`) but the `fetch` command handler never calls `delete_all()` or `storage().delete(...)` after a terminal save; `migrations/0002_settlements.sql` says the full `payment_payload` is persisted in D1 at claim time. Every successful or failed payment keyed by `(payer, nonce)` therefore leaves a Durable Object instance with stored state. At a public endpoint, an attacker or normal volume can generate unbounded DO storage and object count.  
**Better alternative:** after a terminal transition, delete DO storage and use D1 `settlements.response_body`/`payment_response` for replay. Keep DO only for in-flight mutual exclusion, not as the permanent record.

3. **Major — The Durable Object claim machine is an unnecessary second state store and can diverge from D1.**  
`migrations/0002_settlements.sql` already has `UNIQUE(payer, nonce)` and persists the full payment payload; `plans/x402-design-logic.md §6` still places a DO claim in the path. D1 can perform the same atomic claim with an `INSERT ... ON CONFLICT(payer,nonce) DO NOTHING RETURNING status`, and lease recovery can use `UPDATE ... WHERE status = 'claimed' RETURNING`. The current design needs reconciliation between DO state and D1 row if a crash happens between the two writes.  
**Better alternative:** remove the per-payment DO. Use D1 as the single source of truth with `lease_owner`, `lease_expires_at`, `terminated_at`, and atomic `UPDATE ... WHERE` transitions. This removes an entire class of divergence, plus the storage leak in finding 2.

4. **Major — Hourly cron reconciliation leaves paid-but-unserved ambiguity open for up to an hour.**  
`wrangler.toml` `[triggers] crons = ["0 * * * *", "0 2 * * *"]`; `plans/x402-design-logic.md §9` says ambiguous outcomes are closed by reconciliation. A payer whose settle succeeded but whose response was lost can retry into a `receipt_pending` or `settlement_pending` row and may be refused for up to an hour. For autonomous agents, an hour is forever and breaks the “retry the same signed payment” UX.  
**Better alternative:** subscribe to chain logs or use a CDP/Alchemy webhook to push `AuthorizationUsed`/`AuthorizationCanceled` events into a Cloudflare Queue for near-real-time status updates. Keep the hourly sweep only as a backstop and deep-scan escalator.

5. **Major — The claimed “D1 append-only ledger” is not append-only for settlements.**  
`README.md` says the repo uses a “D1 append-only ledger”; `migrations/0002_settlements.sql` creates a mutable `settlements.status` column, and `migrations/0004_reconciler_statuses.sql` rebuilds the table to extend statuses. Reconciler and route updates overwrite the current status in place. Audit cannot reconstruct whether a row went `claimed → settling → settled`, or straight to `failed_expired`.  
**Better alternative:** add an append-only `settlement_events` table with `aggregate_id`, `from_status`, `to_status`, `reason`, `tx_hash`, `created_at`; make `settlements` a current-state projection. All state changes are state-transition events.

6. **Major — Two separate reconciler run tables exist and will drift.**  
`migrations/0002_settlements.sql` creates `reconciliation_runs`; `migrations/0003_reconciler.sql` creates `reconciler_runs_v2` with different columns. Dashboard, code, and future consumers can read one while the reconciler writes the other.  
**Better alternative:** one `reconciler_runs` table with a schema version column, or a single migration that drops the old table and renames `reconciler_runs_v2` consistently.

7. **Major — The Web Bot Auth public key directory is missing.**  
`plans/paying-crawler-plan.md` requires `/.well-known/http-message-signatures-directory` for crawler identity; the repo inventory under `site/public/.well-known/` contains only `mcp.json`, `openapi.yaml`, `security.txt`, `x402.json`. A verifier or Cloudflare Pay Per Crawl discovery would 404.  
**Better alternative:** add the RFC 9421 key-directory file under `site/public/.well-known/http-message-signatures-directory` or render it from the Worker, and include it in the crawler C0 checklist as a machine-discoverable endpoint.

8. **Major — Every challenge writes a D1 `payment_events` row, enabling unauthenticated write amplification.**  
`PROFIT.md` says every 402 challenge writes a `CHALLENGED` event to D1; `migrations/0001_init.sql` has `payment_events`. Any bot can repeatedly hit the public tool endpoints, generating unlimited D1 writes without paying. This contaminates conversion analytics and can exhaust free or paid D1 budgets at scale.  
**Better alternative:** do not persist challenges. Persist only settlement attempts and terminal state, or aggregate challenge counts in memory/edge logs and flush sampled events with TTL.

9. **Major — Facilitator gas is an unmanaged, unpriced cost assumption for sub-cent sales.**  
`PROFIT.md §1` says the public facilitator “currently sponsors gas on Base” and labels mainnet terms an assumption; `plans/x402-design-logic.md §10` parks batching until “per-settle fees return.” At prices of 0.002–0.010 USDC, if facilitator gas is charged at real Base rates, `0.002 USDC` sells lose money on every call.  
**Better alternative:** price a gas-aware floor at challenge time from a live gas oracle, or implement batch settlement/HybridBatch before mainnet, or make the payer include facilitator gas in the amount. Do not treat zero facilitator gas as a permanent cost basis.

10. **Major — Static discovery files can shadow per-environment x402 data.**  
`wrangler.toml` comments that `x402.json` must never be shadowed by `site/dist`; `site/public/.well-known/x402.json` exists. If asset routing ordering changes or a deployment ships stale static files, an agent can read the wrong chain, token, or recipient and construct an invalid payment.  
**Better alternative:** remove the static `x402.json` from `site/public` and render it exclusively from the Worker; add CI or smoke test that fetches `/.well-known/x402.json` and asserts `CHAIN_ID`, `USDC_BASE`, and recipient match the current environment.

11. **Major — Dust payments can exhaust state and are unprofitable if facilitator costs shift.**  
`PROFIT.md` lists prices as low as 0.002 USDC; `migrations/0002_settlements.sql` creates a full settlements row per payment and `crates/edge/src/settlement_do.rs` creates a DO per nonce. An attacker can send many valid dust payments and force D1 writes, DO creation, R2 receipts, queue work, and facilitator settle capacity for negligible revenue.  
**Better alternative:** enforce a cost-based minimum payment amount, per-payer rate limits, or a global daily cap for low-value tools. Batched settlement also reduces per-payment state cost.

12. **Major — SDK/body-only idempotency key deviates from the natural x402 retry model.**  
`EXCELLENCE-SPEC.md §15.2` says `idempotency_key` travels in the body and headers are planned later. A generic x402 client will use the standard idempotency header; code402 will not honor it, so retry semantics and `PAYMENT-SIGNATURE` replay behavior differ by client.  
**Better alternative:** accept the standard idempotency header on v2 now, map it to the body field for backward compatibility, and document the header as canonical. Do not wait for schema v1.1, because current agents already use the header.

13. **Minor — Missing `http-message-signatures-directory` is one symptom of a larger discovery weakness: agent-facing files are static and split between site and worker, with no self-test that an autonomous agent can navigate the full pay-and-call loop from manifests alone.**  
`site/public/.well-known/` has `mcp.json`, `openapi.yaml`, `x402.json`, but no crawler identity directory; `wrangler.toml` only routes `/.well-known/*` to the worker, and static files still exist. A real agent may discover the service but then hit 404 for crawler identity or read stale payment metadata.  
**Better alternative:** make a single `/.well-known/x402.json` + `llms.txt` + `openapi.yaml` + `http-message-signatures-directory` set generated by the Worker from runtime environment, with an end-to-end agent simulation test that performs discovery, reads the 402 challenge, constructs a payment, and verifies the receipt.

### TOP 5 HIGHEST-LEVERAGE IMPROVEMENTS  
1. Replace the per-payment Durable Object claim machine with atomic D1 `INSERT`/`UPDATE RETURNING` plus lease columns, eliminating second-state divergence and unbounded DO storage (`migrations/0002_settlements.sql`, `crates/edge/src/settlement_do.rs`).  
2. Add push-based chain-event reconciliation into a Queue/Webhook so ambiguous claims resolve in seconds, not hourly (`wrangler.toml [triggers]`, `plans/x402-design-logic.md §9`).  
3. Implement OFAC/payer screening before facilitator settlement and reconciler write-back, because production mainnet is configured but the screen is not (`OPERATING-CONTRACT.md`, `EXCELLENCE-SPEC.md §9`).  
4. Make the ledger genuinely append-only with settlement state-transition events rather than mutable status updates (`README.md`, `migrations/0002_settlements.sql`).  
5. Deploy the Web Bot Auth `http-message-signatures-directory` and generate all machine-discovery manifests dynamically, then verify them with an agent end-to-end test (`plans/paying-crawler-plan.md`, `site/public/.well-known/`).

### IF I WERE REBUILDING FROM ZERO  
I would build a single event-sourced payment ledger in D1 as the system of record, with atomic claims via unique `(payer, nonce)` inserts and lease columns instead of Durable Objects. The payment path would be `challenge → verify → atomic claim → facilitator settle → execute → append settlement event → signed receipt`, with all state changes appended, never overwritten. I would subscribe to chain events through a relayer/webhook into a queue for real-time reconciliation, with the hourly cron retained only as a backstop. Before any mainnet settlement, I would enforce payer sanctions screening, gas-aware pricing floors, and per-payer spend caps. Machine discovery would be fully worker-rendered from runtime config, and the crawler identity directory would ship with the first commit. Finally, I would price and batch settlement from day one as if facilitator gas were never free, so the business survives the transition from sponsored Sepolia to real Base mainnet costs.