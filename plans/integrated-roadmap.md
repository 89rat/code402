# Integrated roadmap — merchant × crawler × monetization (2026-08-19)

Three tracks, one core, one set of invariants (plans/x402-design-logic.md).
The binding merchant plan stays `reviews/plan-rev3-2026-08-19.md`; the crawler
track is `plans/paying-crawler-plan.md`; monetization is
`plans/monetization-playbook.md`. This document is the dependency map and the
Rev 3 amendments the design-logic doc introduces.

## The critical path (unchanged)

**Stage 2 (merchant crypto conformance) gates everything.** The crawler's C1
client core is built ON m2m-core's x402v2 module — "the Rust-generates →
TS-verifies direction IS this client." Stage 2's bidirectional vectors are
simultaneously the merchant credential and the crawler's test suite. One core,
two products, zero duplicated codec work.

## Timeline (dependency-true)

```
NOW (user-owned, external lead times — file this week)
  C0 filings: Web Bot Auth keypair + verified-bot application (30+ day
  lead, real-world reports), Pay Per Crawl crawler-side signup
  Phase 0 rails: Builder Code claim (payout = payTo), Basename/Talent
  profile, crates.io name reservation, one-page audit service URL,
  discord/farcaster lurking (seeds Phase 3 list)

Stage 2  ←──── CRITICAL PATH (awaiting operator go)
  merchant crypto conformance: bidirectional TS↔Rust vectors, domain
  divergence, v-normalization, 6492/1271 classification, differential
  fuzz harness
  Phase 1 artifact #3 unlocks: OSS the x402v2 module + vectors as "the
  missing x402 v2 test kit" (Apache-2.0)

Stages 3–4 (merchant)         C1–C2 (crawler) — forks after Stage 2
  /v2 wire flow, HMAC stamps   client core + policy engine + ledger
  facilitator settle (Sepolia) signer-isolation service
  + ▸ Builder Code suffix      dry-run mode ships first (free tier)
    on settle txs (monetize)   nonce ledger, delivery scoring
  + ▸ Facilitator trait seam
  + ▸ TLA+ claim-machine model
  LEGACY REWIRED (G1)

Stage 5 (merchant ship)  ═══  Phase 2 (submit everywhere) — SAME WEEK
  KV-gated mainnet enable       day-1 x402scan + registries (conformance =
  hard-cut legacy                instant listing), ecosystem PRs, MCP
  publish paid-but-failed        registry, grant applications (cite live
  contract in openapi            deployment + OSS crate + published
                                pieces), Show HN gateway template
  Phase 1 artifacts #1–2 publish during Stages 3–4 build-in-public

C3–C4 (crawler live)
  C4a = OUR CRAWLER PAYS OUR MERCHANT on Sepolia — the mirror-principle
  e2e, highest-value test in the system (design-logic §5)
  C4b mainnet probes $10 cap → delivery-reliability dataset (observatory
  inventory starts accruing at pennies)

Phase 3 (sell, weeks 4–8): 5 outreach/week, structural-checker-first
  messages, US$7.5–15K fixed-scope reviews; trust registry free listings
  → paid tier at ≥10

C5 (package): x402-paying-client crate, OpenFang Hand (paying-researcher),
  commercial deadline-driven offers (Sept 15 does the marketing)
```

## Rev 3 amendments adopted from the design-logic doc

1. **Typestate pipeline** (design-logic §4): Stages 3–4 implement the payment
   path as `Payment<Received> → <StructurallyValid> → <Verified> → <Claimed>
   → <Settled> → Response<Served>`; illegal transitions don't compile. Crawler
   mirror: `Requirement<Parsed> → <PolicyApproved> → Payment<Signed> →
   Outcome<Receipted|Disputed>`.
2. **`Facilitator` trait** over verify/settle at Stage 4 (CDP today; the
   enterprise-facilitator product seam). **`Rail` trait** in the crawler
   (x402-native vs Pay-Per-Crawl).
3. **TLA+/model-check of the DO claim machine** — cheap Stage 4 artifact; the
   right home for formal methods (no balances exist here; EIP-3009 moves exact
   amounts).
4. **Constant-time MAC comparison** for the G6 stamp check (§11 rigor list).
5. **Parked-with-triggers register** (§10): Solana rail, self-hosted
   facilitator, channels/batching, ZK proofs, binary telemetry, m2m-exchange —
   each parked with an explicit entry trigger, not built.
6. **Integrity constraint made plan-level**: self-paid transactions labeled
   smoke tests, minimal, never Builder-Code-farmed (monetization §8).

## Consistency notes (reconciled)

- Monetization playbook "Now" items are USER-owned (base.dev claim, Basename,
  discord) — they parallel the build, cost ~2h, and start attribution
  accruing BEFORE mainnet. Nothing in them blocks or alters the build.
- Crawler decision 2 (crawler-max-price inside signed components) matches
  Cloudflare's Dec 2025 security requirement — verified, not assumed.
- The observatory/flywheel (§8) depends on C4b probes — correctly sequenced
  post-conformance so the dataset is itself spec-clean.
- One deliberate tension recorded: monetization proposes "bps pricing on the
  hosted gateway" as a 10x-volume trigger action; design-logic Law 1 says
  optimize round-trips not fees. Resolution: bps pricing is a BUSINESS-model
  trigger, not a latency optimization — no conflict.

## Operator decisions needed
1. **Stage 2 go** (critical path — everything above forks from it).
2. Phase 0 + C0 filings: user-owned this week (I can draft the verified-bot
   application fields, keypair, and UA string on request).
3. License confirmation: Apache-2.0 for the OSS x402v2 module + crate name
   (plan recommends; needs your explicit OK before publishing).
