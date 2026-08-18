# x402 Monetization Playbook — placement + sequence

> Provenance: parallel Claude Cowork session, adopted by operator directive
> 2026-08-19. External claims verified — see plans/PROVENANCE.md. Grant/listing
> venues individually re-verified at application time (Phase 2).

Companion to `x402-v2-plan-rev3.md`. Build stages fund nothing by themselves; this is where each revenue stream gets placed and in what order. Rule of thumb: 70% build / 30% distribution until Stage 5 ships, then invert.

## Placement map

| Stream | Where | How | When |
|---|---|---|---|
| Builder Code (attribution → rewards) | base.dev → Settings | Claim code (ERC-8021 NFT, maps to payout wallet); append code suffix to settle txs at Stage 4 | **Now** (5 min) |
| Builder Rewards (weekly ETH) | Talent Protocol / Basename | Set up Basename + Builder Score profile; accrues automatically off shipped GitHub + onchain activity | Now (30 min) |
| Base Builder Grant (retroactive) | Gitcoin — Base Builder Grants | Apply **after** Stage 5 ships; cite live deployment, OSS crate, Builder Code analytics. 1–5 ETH, "reward shipped work" | Stage 5 + 1 week |
| x402 Foundation grants | x402.org/get-involved → Linux Foundation | Dev-grant application; pitch = "first spec-conformant Rust v2 implementation + public conformance suite" | Stage 5 |
| Endpoint discovery | x402scan.com/resources/register | Submit URL — auto-listed if it returns a valid x402 schema (your conformance = instant pass) | Stage 5 day one |
| Ecosystem listings | PR to `coinbase/x402` ecosystem page; both `awesome-x402` lists (Merit-Systems, xpaysh); x402daily.xyz | One PR each, point at live endpoint + repo | Stage 5 week |
| Bazaar / CDP discovery | CDP facilitator discovery resources | List paid endpoints once mainnet-enabled | Post-mainnet flip |
| MCP registries (broadened rail) | Official MCP registry + major directories | You already serve `mcp.json` — submit it; "monetizable MCP tools" is the today-demand framing | Stage 5 week |
| Audit clients | Direct outreach (see Phase 3) + inbound from published reviews | Fixed-scope productized review | Weeks 4–8 |
| Gateway template | GitHub "Deploy to Cloudflare" template + Show HN | Free template → hosted tier later | Stage 5 + 2 weeks |
| Certification | Your existing trust registry + badge | Free listings first; paid "verified x402 merchant" tier at ≥10 listings | Month 2–3 |

## Step-by-step

### Phase 0 — Rails (this week, ~2 hours, parallel with Stage 0)

1. Claim Builder Code at base.dev; payout wallet = your `payTo`.
2. Basename + Talent Protocol profile → Builder Rewards start accruing on their own.
3. Decide OSS license for m2m-core's x402 module (Apache-2.0 recommended — foundation-friendly), reserve crate name on crates.io.
4. Join the x402 Discord, Base Discord, Farcaster /x402 channel. Lurk, note who's shipping — this seeds the Phase 3 outreach list.
5. One-page "x402 Deployment Review" service description (fixed scope: 5 days, report mapped to the published attack taxonomy, fixed fee). A README is fine; it just needs a URL.

### Phase 1 — Build in public (weeks 1–3, during Stages 1–4)

You already have the content; it's sitting in `reviews/`. Publish three pieces, each ending with the audit CTA:

1. **"What we found designing x402 v2 properly"** — the Rev 3 gap analysis (G1–G10), sanitized. Post: X + Farcaster + HN.
2. **"The x402 attack taxonomy, mapped to defenses"** — the published attacks (grant-before-settle, replay, sig bypass, gas abuse) each paired with the concrete control from your plan. This is the audit-sales artifact.
3. **Conformance vectors release** — OSS the x402v2 module + bidirectional golden vectors when Stage 2 goes green. The vectors are the credibility; announce as "the missing x402 v2 test kit."

Embed the Builder Code in the settle path at Stage 4 so every settled payment attributes to you from day one.

### Phase 2 — Submit everywhere (Stage 5 ship week)

Day 1: x402scan register + x402daily. Day 2: ecosystem PRs (coinbase/x402 + both awesome lists). Day 3: MCP registry submissions. Day 4: grant applications — Base Builder Grant on Gitcoin + x402 Foundation form, both citing the now-live deployment, the OSS crate, and the three published pieces. Day 5: Show HN the deploy-to-Cloudflare gateway template.

One rule: every listing links the same canonical URL (your site → endpoint + repo + service page), so all discovery funnels to one place.

### Phase 3 — Sell (weeks 4–8, ~5 outreach/week)

Build a 20-name target list from: x402scan top-volume merchants, ecosystem-page services, foundation charter members' integration teams, and anyone announcing x402 funding. Message shape: one specific observation about *their* deployment (your structural checker run against their live 402 endpoint — takes minutes and is devastatingly effective), link to the taxonomy post, fixed-scope offer. Anchor US$7.5–15K per review; retainers and integration work follow naturally.

In parallel: invite every team you talk to into the trust registry free. At ≥10 listings, introduce the paid verified tier.

### Phase 4 — Instruments and triggers (ongoing, 30 min/week)

Watch: x402scan volume (market), Builder Code dashboard (your attribution), inbound rate. Act on triggers only:

- Real (non-gamified) volume 10x → ship bps pricing on the hosted gateway.
- ≥3 inbound audit leads/month → raise the price, stop outbound.
- Grant landed → fund the next build stage, don't extend scope.
- Kill signals (volume flat into 2027, no non-Coinbase facilitator, card networks route around crypto) → freeze new investment, keep endpoint + registry alive (near-zero cost), keep consulting.

## Sequencing logic

Grants pay for shipped work → so ship first, apply immediately after. Audits pay for *published expertise* → so the three posts precede outreach. The gateway and certification only compound if discovery finds you → so listings go live the same week as the deployment. Nothing here delays the Rev 3 build; distribution rides the artifacts the build already produces.

## Integrity constraint (from design-logic §8, plan-level)

No manufactured volume — self-paid transactions are labeled smoke tests, minimal, never Builder-Code-farmed.
