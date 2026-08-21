# BUILD BLUEPRINT — The Whole Build, Consolidated for Profit

**Date:** 2026-08-13 · **Status:** consolidated inventory + competitive position + execution sequence

---

## 1. What the whole build actually contains

| Asset | What it is | State | Distance to revenue |
|---|---|---|---|
| **code402/** | Cloudflare-native x402 product: Rust/WASM edge worker (KV pricing, D1 ledger, R2 receipts, NonceGuard DO, settlement queue), deterministic paid tools, signed receipts, React site, full machine manifests (mcp.json, openapi.yaml, x402.json, llms.txt) | **16/16 tests pass** (7 new). Site built. **Domain code402.dev is DOWN (not deployed)** | **NEAREST — deploy it** |
| **openfang-x402-os/** | Embedded Rust x402 guard daemon: 16 modules (trial, nonce, circuit breaker, settlement adapters, attribution, audit chain, persistence, semcache) | **33/33 tests pass** (WSL) | Medium — embedded/device play |
| **openfang-gtm-hands/** | 4 GTM hands (lead-harvester, researcher, publisher, market-intelligence w/ WASM scorer) + ops (heartbeat, kaizen) | Templates + code, not deployed | Feeds distribution |
| **openfang-x402-platform skill** | Operating knowledge: 6 references + assets, installed + packaged | Live | Force multiplier |
| **docs/** | ENTERPRISE-MOAT.md (v0.5), NECTAR-STRATEGY.md | Current | The rulebook |
| gtm-archive/, mining/, fang-gtm-import/ | Older strategy archives | Reference only | — |

**Key insight from inventory:** code402 is the crown jewel and was sitting
undeployed. Everything else (moat modules, GTM hands, skill) exists to make
code402 win. The build is ONE company: **code402 is the product, openfang is
the moat + distribution engine.**

## 2. Competitive position (live scan, 2026-08-13)

- x402 ecosystem is real and crowded at the PROTOCOL layer: Coinbase/CDP,
  Stripe (ACP), Google (AP2), Cloudflare+AWS embedding x402 natively, 20+ orgs.
- Competing on "payment infrastructure" head-on = losing game against
  Coinbase/Stripe. **The winning position is one layer up: paid deterministic
  machine-verifiable APIs** (code402's exact niche) — tools agents consume,
  not rails they pay through.
- PayMesh docs (absorbed) validate the marketplace/take-rate model long-term,
  but its Year-3 $24M projection is hype-class; our honest sequencing applies.

## 3. The profitable sequence (highest earning, legally safe)

**Runbooks:** [AUTOPILOT.md](AUTOPILOT.md) (self-running loops + human boundary)
· [code402/DEPLOY.md](code402/DEPLOY.md) (staging → mainnet acceptance gates)

```
WEEK 1:  Deploy code402 to Cloudflare staging (Base Sepolia) — code402/DEPLOY.md
         - wrangler kv/d1/r2/queue create, secrets: COMPANY_WALLET,
           RECEIPT_SIGNING_KEY, RPC_PRIMARY/FALLBACK
         - Verify ONE paid loop end-to-end with test USDC (acceptance a–f)
         - Point code402.dev DNS at the worker; site is already built
WEEK 2:  Mainnet (Base 8453) + publish manifests to MCP registries
         + GTM publisher hand starts trial-first outreach (AUTOPILOT Loop B)
WEEK 3-4: Market-data nectar via yahoo_finance (verified working):
          new code402 tool wrapping structured equity/ETF data
MONTH 2: Schema normalizer tool (same risk class as distill)
MONTH 3: Air-gapped sandbox ONLY after AUP + abuse controls
ONGOING: heartbeat + kaizen daily (Loop C); semcache on; referral from realized net (Loop D)
```

**Pricing:** code402's manifest sets 0.005 USDC/call standard. Do NOT raise it
pre-traction. Volume × margin: the pragmatic portfolio ceiling (~$485/day by
Day 300) requires distribution, not price hikes.

## 4. What makes this "best in the business"

1. **Deterministic + signed receipts** — nobody else in the x402 tool space
   ships byte-identical outputs with hash-chained verifiable receipts. That is
   THE enterprise differentiator (audit-ready machine commerce).
2. **Test vectors + machine manifests** — agents can verify before paying
   (APEX contract model). code402 already publishes mcp.json/openapi/llms.txt.
3. **The moat modules** (openfang-x402-os): trial, nonce, fraud-bounded
   referral, wash-farming controls, audit chain, semantic cache — portable to
   the code402 edge over time.
4. **Legal shield**: non-custodial settlement (not a money transmitter),
   ephemeral data (store nothing but hashes), AUP endpoint, no scraping ever,
   banned grey-zone products (anti-bot evasion, universal-traffic proxy).

## 5. Banned moves (claim-audit, enforced)

- No anti-bot evasion service. No MITM "universal gateway." No $2 attacker
  pricing (fail-tax only). No polymorphic APIs. No promised latency or volume.
- No deployment claims without deployment. code402.dev stays "down" in every
  doc until it is actually live.

## 6. Open items (honest)

- code402 edge worker tests: core passes 16/16; the edge crate compiles under
  worker-build (wasm) — not yet compile-verified here.
- DNS/hosting for code402.dev: domain resolution failing — register/verify
  domain ownership, then attach to Cloudflare.
- Facilitator conformance: verify settlement payload against live facilitator
  on Sepolia before mainnet.
- yahoo_finance redistribution terms must be checked before selling feeds.
