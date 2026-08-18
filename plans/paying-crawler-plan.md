# Production Paying Crawler — Build Plan (buyer side)

> Provenance: parallel Claude Cowork session, adopted by operator directive
> 2026-08-19. External claims verified — see plans/PROVENANCE.md (Sept 15 2026
> Cloudflare deadline, Pay Per Crawl signed-component requirement, Web Bot Auth
> key-directory path and registration lead times). Integrated dependencies in
> plans/integrated-roadmap.md.

Companion to `x402-v2-plan-rev3.md`. Same voice, same rhythm: staged, panel-gated, tests gate every stage. The client shares Rev 3 Stages 1–2 (types, codec, crypto, vectors) — one core, two products. The two builds then test each other live: our crawler pays our merchant on staging before either touches a stranger's money.

Why now: Cloudflare blocks unverified/mixed AI crawlers Sept 15, 2026; Pay Per Crawl is evolving into Pay Per Use; the Monetization Gateway settles in USDC over x402. Every AI company with a crawler needs exactly this and almost none have it. We build it for ourselves, then sell it.

## Architecture decisions (proposed locks)

1. **Shared core.** The crawler client is built on m2m-core's `x402v2` module. Rev 3 Stages 1–2 are prerequisites and proceed unchanged; the crawler track forks after Stage 2. No second codec, no second vector suite.
2. **Two rails, one identity.** The crawler's identity is a Web Bot Auth keypair (RFC 9421 HTTP Message Signatures, Ed25519, hosted key directory). Rail A: native x402/USDC — Monetization Gateway endpoints and any x402 merchant, no signup, wallet pays. Rail B: Pay Per Crawl — Cloudflare as merchant of record, fiat billing, `crawler-max-price` carried **inside the signed components** per their spec, Discovery API for payable domains. Registration for verified-bot status and the crawler-side beta is filed at C0 because approval lead time is outside our control and the deadline isn't.
3. **Signer isolation.** Crawl workers never hold keys. A separate signer service (its own process/DO) holds the wallet key and the Web Bot Auth key, enforces policy *before* signing, and returns either a signed payment or a refusal. A compromised crawl worker can request payments within policy; it cannot exfiltrate keys or exceed grants.
4. **Budget-policy-first, deny by default.** Every payment passes the policy engine: canonical-USDC-only asset allowlist per CAIP-2 network (a malicious server naming a fake token or wrong chain is the buyer-side equivalent of our merchant-side forged requirement); per-content-class price ceilings; per-domain, per-hour, per-day caps; global KV kill-switch. Cloudflare shipping Wallets with spend caps in August confirms this is table stakes — ours is the differentiated, auditable version.
5. **Etiquette is strategy.** robots.txt respected, honest stable UA, conservative rates. Paying does not exempt manners; verified-bot status is a business asset that unlocks Rail B and the Discovery API, and reputation is what we sell on the merchant side.

## Buyer-side threat model (the mirror of Rev 3's)

Drain via inflated `amount`, 402-loops, or mid-session price escalation → ceilings + per-domain breakers + loop detection. Wrong asset / wrong chain → hard allowlist. Pay-no-serve (settled, then 5xx or garbage) → receipt + evidence ledger, per-domain delivery scoring, blacklist; this dataset is itself product (below). Own-retry double-pay → nonce ledger: one authorization per resource attempt, resend the *same* signed payment on ambiguous failure, never mint a fresh nonce until the prior outcome is known. Receipt forgery (PAYMENT-RESPONSE claims a settle that didn't happen) → periodic on-chain reconciliation, ledger == chain. Malicious redirect (3xx onto a different domain/payTo) → policy re-evaluated per hop, payments never follow cross-domain redirects. Key exfiltration → decision 3. Injection: **page content never influences payment decisions** — the policy engine reads protocol fields only, never response bodies; this is the payment-layer prompt-injection defense, tested explicitly.

## Stages (each ends: tests green + panel audit + your gate)

**C0 — Identity, rails, policy design.** Generate Web Bot Auth Ed25519 keypair; host key directory (`/.well-known/http-message-signatures-directory`); pick the permanent UA string. File verified-bot application and Pay Per Crawl crawler-side signup (both external lead time — file first, build while waiting). Testnet wallet + funding runbook; spend-ledger schema (D1: payments, receipts, delivery outcomes, reconciliation status); policy config format (versioned, reviewed like code); kill-switch. Panel gate on the threat model.

**C1 — Client core.** On m2m-core: 402 detection → PaymentRequired parse (v2 header form and v1/body forms — the wild has both) → requirement selection under policy (network, asset, price) → EIP-3009 authorization construction → EIP-712 signing via signer service → PAYMENT-SIGNATURE retry → PAYMENT-RESPONSE receipt parse and store. Golden vectors reused from Rev 3 Stage 2 (the "Rust generates → TS verifies" direction *is* this client). **Dry-run mode ships first**: full pipeline, logs the payment it would have made, signs nothing — this is also the free tier of the future product.

**C2 — Policy engine + ledger.** Allowlists, ceilings, caps, breakers; nonce ledger for own-payment idempotency; receipt store; on-chain reconciliation job (shares the merchant side's cron pattern); budget exhaustion fails closed; structured spend telemetry (cost per domain, per content class, success rate).

**C3 — Crawler integration.** Web Bot Auth signing on every request, `crawler-max-price` in signed components for Rail B; robots.txt + crawl-delay compliance; conditional requests (ETag / If-Modified-Since) and content-addressed cache so we **never pay twice for unchanged content** — cache hit rate is the crawler's gross margin; price-aware scheduler (value score vs price against remaining budget); Discovery API polling for payable domains.

**C4 — Live loops.** (a) Sepolia: our crawler pays our own Rev 3 staging merchant — full bidirectional e2e, both products validated against each other. (b) Mainnet probes: $10 hard cap, real x402 endpoints from the public registries; every probe writes the delivery-reliability dataset (who serves after taking money) — the observatory/reputation inventory starts accruing here at pennies of COGS. (c) Rail B sandbox the day Cloudflare approves the bot.

**C5 — Package and sell.** `x402-paying-client` crate, Apache-2.0, policy engine as the headline feature ("the paying client that can't be drained"). OpenFang Hand packaging (`paying-researcher`) — the first agent OS Hand with native, capped payments. Commercial offer for AI companies facing the deadline: integration + support, dry-run-to-production in a week. Publish the client conformance results next to the merchant vectors — one credential, both directions.

## Test suite gating C1

Drain fixtures (10⁶× price, 402-after-payment loops, escalating requirements) · wrong-asset / wrong-chain rejection · pay-no-serve evidence path + delivery scoring · retry-storm: exactly one settle per resource attempt · budget exhaustion fail-closed · signer-isolation compromise sim (worker cannot exceed grant or extract keys) · reconciliation ledger == chain · robots/UA compliance snapshot · Web Bot Auth signatures verify against the reference verifier · injection test: response-body content provably cannot alter any payment decision · cache: unchanged content never re-paid.

## Economics

Probe sweeps: ~1,000 endpoints × ~$0.01 = ~$10/sweep — the reputation dataset costs pennies and sells twice (observatory API on the merchant side, target intelligence for audits). Product: the crate is free, the policy engine hosted tier and integration support are paid, and the deadline does the marketing. Internal: the crawler is also the data engine for certification and outreach.

## Out of scope

Content resale/sublicensing (license risk — we buy access, we don't relicense) · non-USDC assets · Solana rail (types accommodate; later) · search-engine-scale crawl infrastructure · autonomous unattended mainnet spend above caps (human raises limits, never the agent).

## Rhythm

C0 admin files this week (external lead times start ticking); C1–C2 ≈ one session once Rev 3 Stages 1–2 land; C3 ≈ one session; C4 spans days on external approvals; C5 short. Panel gate per stage. Mainnet caps raise only on your word — same rule as the merchant flip.
