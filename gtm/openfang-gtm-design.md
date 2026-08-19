# OpenFang GTM — design + intel (2026-08-19)

OpenFang = the agent execution OS; Code402 = the settlement layer it trusts; Atlas = the
index it buys from. One flywheel, three products. This document is the GTM for OpenFang
specifically, sequenced off the Code402 plan (30/100/300 days).

## 1. Positioning (one line each audience)

- **To agent builders:** "The runtime where your agent can spend money without draining
  your budget — policy-gated, drain-proof, receipts attached."
- **To crawler operators:** "A crawler that pays its own way through the post-Sept-15
  web — verified-bot identity, per-domain budgets, proof of every payment."
- **To enterprises:** "Agent execution with treasury controls your CFO will actually
  sign: caps, allowlists, audit-grade receipts, zero custody."

**Core differentiator (only-we-can-say-it):** the runtime whose payments reconcile to
the chain. Every other agent framework treats paying as fire-and-forget; ours is the one
where an ambiguous payment becomes an entitlement, not a support ticket.

## 2. ICP + segments (in attack order)

| # | segment | who exactly | pain | wedge |
|---|---|---|---|---|
| 1 | **Paying-crawler operators** | SEO/AI-data shops hitting post-Sept-15 paywalls | Blocked by Cloudflare, or bleeding budget uncontrolled | C0 filings + policy engine + toll-free verified-bot identity |
| 2 | **Agent product teams** (seed/Series-A) | builders shipping agent products with tool-spend | Wallet drains, no spend governance, no receipts for finance | The C2 policy engine as an embedded crate |
| 3 | **Enterprise agent fleets** | platform teams evaluating agent runtimes | Procurement needs audit + caps; custody is a non-starter | Non-custodial + XDR-1 receipts + GAAP-ready ledger |

## 3. Competitive intel (live, refresh weekly via automation)

| player | what they have | what they lack | our counter |
|---|---|---|---|
| Cloudflare Agents SDK + Pay Per Crawl | Distribution, edge, verified-bot rail | Seller-side focus; no buyer-side spend governance; no receipts | We complement: their rail, our policy + receipts. Never fight the edge. |
| x402 official SDKs (@x402/fetch etc.) | Standard client, free | No execution OS, no policy engine, no reconciliation | We're conformant + verified against them (e2e PASS) — cite it |
| LangChain/CrewAI ecosystems | Mindshare | Payment = bolt-on, custody-prone, no audit trail | Drain-proof determinism + receipts; their agents can still call our tools |
| Hosted "agent wallet" startups | VC backing, polish | Custodial by design — the exact thing enterprises reject | Non-custodial architecture as the dealbreaker question: "who holds the keys?" |

**Market timing note:** real x402 commerce ≈ $28K/day today; exploration demand (what we
monetize irrespective) is large and marketing-driven by the giants. Sept 15 = the forcing
event for segment 1.

## 4. Channel plan (shared engine with Code402)

1. **Daily evidence content** (LinkedIn + X, the automated calendar in `launch/`)
   — 70% Code402/settlement evidence, 30% OpenFang agent-side stories
2. **Ecosystem presence**: x402 Foundation threads, Cloudflare dev community, Base
   builders — reply-level engineering value, never launch-speak
3. **The corpus posts** (KV-vs-DO, phantoms, reconciler) — each one is OpenFang GTM too:
   the runtime that produced that evidence is the product
4. **Founder-led DMs**: 5/week to crawler-operator and agent-team leads (from Atlas
   seller/buyer relationships) — the only stream that converts in <90 days
5. **Hackathon/bounty presence** (ETHGlobal-style): "best paying agent on OpenFang+Code402"

## 5. Sequencing

- **Day 1–30:** C0 filings (operator), policy engine v1 ships, 2 corpus posts, 10 DMs/wk.
  Metric: crawler runs against ≥5 real paywalled endpoints with receipts.
- **Day 31–100:** v2 mainnet flip (Code402 gate), paying-crawler case study published
  with real spend data, first 2 design-partner conversations from DMs.
  Metric: first external agent pays through the stack.
- **Day 100–300:** embedded policy crate (segment 2), enterprise pilot (segment 3),
  Atlas-sourced pricing intel as product. Metric: 1 signed rev-share or design partner.

## 6. What we will NOT do (strategy hygiene)

No custody, no "agent wallet" branding (it invites the custodial comparison), no
growth-hack posts detached from evidence, no competing with Cloudflare at the edge.
