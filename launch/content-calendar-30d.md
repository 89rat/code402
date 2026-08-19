# 30-Day Content Calendar — daily LinkedIn + X (evidence-first)

Format per day: **X post** (final copy — post as-is) + **LinkedIn angle** (bullets the
daily automation expands into long-form with live stats). Hashtags/groups at bottom.
Source column = the repo evidence backing every claim. Never post a claim without its URL.

## Week 1 — Launch + the phantom story
| day | X post (final) | LinkedIn angle | source |
|---|---|---|---|
| 1 | We run payments for AI agents (x402, USDC on Base, non-custodial). Stress-tested with 1,000 real settles. It "lost" 122 of them — money moved on-chain, our record timed out. That number is why this thread exists. 🧵 (then pin thread from launch/x-thread.md) | Company launch post (launch/linkedin-post.md) | /proof |
| 2 | The most expensive lie in payments: "it probably settled." Ours reconciles hourly against chain state — authorizationState + Used/Canceled logs. settled / canceled / expired. Never a guess. code402.dev/proof | Why reconciliation-not-hope is a design principle; the three-way table; tx-hash evidence | reviews/reconciler-spec-v1.md |
| 3 | Non-custodial means we CAN'T refund mistakes. So "paid but not served" became a cryptographic entitlement: chain proves you paid → next identical request runs free, bound to the original input. Different input → 400. | Entitlement-not-refund as the honest answer to the impossible position; the G2c binding | reviews/reconciler-e2e-report.md |
| 4 | Every paid call returns an XDR-1 receipt: RFC 8785 canonical, domain-separated, payment nonce signed in. Verify it offline — no call to us. The spec vector reproduces byte-for-byte in our tests. | What an offline-verifiable delivery receipt changes for auditors/agents; spec CC0 | specs + tests in repo |
| 5 | Cloudflare KV is eventually consistent — fine for config, fatal for payment nonces. Two edges, one signature, one window = double-spend. We use single-writer Durable Objects keyed by hash(from‖nonce). | The KV-vs-DO engineering post (the corpus one) — include the 132-phantom numbers | reviews/ |
| 6 | Live testing caught 3 bugs our green tests missed: the facilitator's real already-used shape matched nothing we'd mocked; the entitlement was unreachable after 5 min; the entitlement served ANY input. All public. | "Your model of reality is the bug" — the e2e-over-units doctrine, with the 3 stories | reviews/reconciler-gate.md |
| 7 | Our ops dashboard is a public endpoint: code402.dev/v1/ops/stats — reconciler runs, backlog, breaker state. If we claim it, you can curl it. | Radical telemetry transparency as trust strategy (and SEO for agents) | live endpoint |

## Week 2 — Principles
| day | X post | LinkedIn angle | source |
|---|---|---|---|
| 8 | Fail-closed on money, fail-open on metadata. Every ambiguous facilitator response → receipt-pending until the chain speaks. The taxonomy is code, not runbook. | The ambiguity-class doctrine; the 3 live CDP shapes | core classifier + matrix test |
| 9 | Exactly-once per (from, nonce), not per nonce: EIP-3009 uniqueness is per-authorizer. UNIQUE(nonce) alone lets an attacker front-insert a victim's nonce and deny them service. | The idempotency-key design note (G3) | plans/x402-design-logic.md |
| 10 | A 402 challenge is a contract, so we HMAC-stamp ours: the echoed requirement must be byte-ours, route-bound, time-boxed. Price changes never kill in-flight payments. | G6 stamp design; tamper rejection demo | reviews |
| 11 | Race day: 10 parallel requests, one payment authorization. Exactly one settle. The other nine get the byte-identical stored 200. Replay determinism: 100%. | The race/replay campaign results | /proof |
| 12 | 1,000 settles in 310 seconds: 717 confirmed, 283 retryable, zero anything-else. The bimodal curve IS the facilitator's burst queue — knowing that is the difference between paging and shrugging. | Stress-II readout with the wave chart | /proof |
| 13 | HTTP 402 was reserved in the 90s and never used. Now it's the machine-commerce handshake. The wire is boring on purpose: PAYMENT-REQUIRED → PAYMENT-SIGNATURE → PAYMENT-RESPONSE. | A short history + why boring protocols win; our conformance gate | specs/x402 |
| 14 | Receipts without canonicalization are vibes. JCS (RFC 8785) is why ours verify in any language: sorted keys, escaping rules, no floats. Our canonicalizer floats-fail-closed. | The receipts-are-engineering post | crates/core jcs.rs |

## Week 3 — Ecosystem + Sept 15 (crawler-side)
| day | X post | LinkedIn angle | source |
|---|---|---|---|
| 15 | Sept 15 changes the web: unverified bots get blocked by default; verified, paying crawlers get through. One cryptographic identity across both rails. Are you filed? | The two-rail one-identity thesis (operator checklist angle) | plans/paying-crawler-plan.md |
| 16 | Our crawler's wallet can't be drained by a hostile server: deny-by-default asset allowlist, per-domain caps, and page content provably can't influence payment decisions — tested. | The inverted security model (merchant defends payment; buyer defends wallet) | same |
| 17 | Atlas index says: 209+ machine-payable endpoints probed hourly, 1486 alive last count. The agent economy already has a map. code402.dev indexes it; agents search it. | The meta-layer: selling the market's own map | atlas stats |
| 18 | Price intelligence from the index: what agents actually pay per call across the living endpoints. (Post 2–3 real datapoints.) | Atlas pricing data as the published benchmark | atlas |
| 19 | Pay Per Crawl bills through Cloudflare as merchant of record; native x402 settles in USDC directly. Same crawler, both rails, one identity, receipts either way. | Multi-rail buyer architecture | plans |
| 20 | The drain-proof policy engine: ACCEPT / REJECT / ESCALATE before any signature. Deterministic code decides — the LLM proposes, the policy disposes. | C2 policy engine + the I6 invariant | gtm/openfang-gtm-design.md |
| 21 | We gave an agent a $5 wallet and a research task across paid endpoints. Full transaction trace, cost breakdown, receipts attached. Here's what it cost and what came back. | The $5-agent case study (run it this week if not yet run) | NEW — needs the run |

## Week 4 — Process + the factory
| day | X post | LinkedIn angle | source |
|---|---|---|---|
| 22 | Every payment-path change ships through an AI review panel: independent red-team + wide-angle attack the diff; I must prove each finding wrong or concede. Verdicts public. | PANEL.md as the engineering constitution | PANEL.md |
| 23 | The panel caught 3 real bugs I'd shipped green — and filed 4 wrong findings, which I had to disprove. Adjudication, not consensus. That's the whole trick. | The adjudication doctrine (this is also your services marketing) | reviews/reconciler-gate.md |
| 24 | Every defect becomes a test vector. The 132 lost payments? Exported, anonymized, standing regression corpus. The facilitator shape bug? A classifier matrix. Grief becomes armor. | Kaizen loops + corpus-as-asset | tests/fixtures |
| 25 | One operator + an AI panel took a payments system spec→production→mainnet-proven in weeks, at ~$50/mo infra. The factory is the product. (Careful, honest tone.) | The factory post — capability marketing for services | this session |
| 26 | 402 challenge latency: 137ms. That's the free tier of the protocol. Settle p50: 3.5s. The numbers nobody publishes because nobody measures: code402.dev/proof | Metrics-first post; invite others to publish theirs | /proof |
| 27 | What we refuse: custody, escrow, "agent wallet" branding, dynamic pricing games. Each refusal is an architecture decision with a reason. | The strategy-hygiene post (builds enterprise trust) | gtm design §6 |
| 28 | Teaser: the same settlement guarantees, now on the buyer side — a crawler runtime with policy-gated spend and receipts for every payment. OpenFang. More this month. | OpenFang positioning post (segments 1+2) | gtm design |
| 29 | We're looking for 5 crawler operators / agent teams to run the stack against real paywalled targets in September. Free, receipts included, findings public. DMs open. | Design-partner recruitment (the conversion post) | — |
| 30 | Month one in public: what shipped, what broke, what the numbers say. (The automation drafts this from the intel log.) | Retrospective + month-two commitments | gtm/intel-log.md |

## Targets (post into, reply in, DM from)

**LinkedIn groups:** AI & Machine Learning Engineers · FinTech Network · API World ·
AI Agents & Autonomous Systems · Cloudflare Users (unofficial) · Base Builders ·
Payments & Cards professionals · DevOps India circles (your timezone advantage).

**X engagement list (reply daily, 10 min):** @x402_foundation, @CloudflareDev,
@coinbasedev, @base, agent-framework authors (LangChain/CrewAI), indie-hackers building
agent products. Hashtags: #x402 #AgenticPayments #AIagents #BuildOnBase #HTTP402.

**Rule:** every post carries one verifiable link. No link = don't post it.
