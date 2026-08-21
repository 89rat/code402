# The Rental Catalog — every sellable/rentable module (2026-08-19)

> Doctrine: plans/lego-wall.md. Every item is a standalone brick:
> adoptable in an afternoon, no hostage dependencies, composes via the wire.
> Status: ✔ built · ◐ partial · ○ new build. Rent models: OSS=free brick,
> SUB=subscription, METER=per-call x402 micropayment, FIX=fixed-fee service,
> BPS=take rate, LIC=license+support.

## Shelf 1 — For sellers (agents must be able to pay me)

| # | Brick | Buyer pain | Rent | Status |
|---|---|---|---|---|
| S1 | Gateway template ("Deploy to Cloudflare") | "I want a payable endpoint tonight" | OSS → hosted tier BPS | ◐ Stage 5+2w |
| S2 | Hosted merchant gateway | "Run my payment path for me" | SUB + BPS at volume | ◐ code402.dev |
| S3 | Conformance test kit (codec + golden vectors) | "Does my implementation match the spec?" | OSS | ◐ Stage 2 gate |
| S4 | Certification badge (monthly probed) | "Agents don't trust unknown sellers" | SUB | ○ atlas exhaust |
| S5 | Deployment Review (adversarial audit) | "What breaks when real money hits?" | FIX $7.5–15K | ✔ page written |
| S6 | Manifest generator (x402.json/llms.txt/openapi/mcp from one config) | "My discovery files contradict each other" | OSS tool; hosted SUB | ○ fixes C4 |
| S7 | Claim machine (exactly-once, D1-only) | "Double-settles / wedged nonces" | OSS crate + FIX integration | ◐ R1 |
| S8 | Reconciler module (chain-as-truth sweep) | "My ledger and the chain disagree" | OSS + FIX integration | ✔ built |
| S9 | XDR-1 receipts (JCS, offline-verifiable) | "I need to prove service happened" | CC0 spec + METER verification API | ✔ built |
| S10 | OFAC/sanctions payout screen | "Strict liability on USDC flows" | SUB (compliance brick) | ○ DeepSeek blocker — build first for ourselves |
| S11 | Credit notes (paid-but-unserved → bearer credit) | "Refunds without custody" | protocol feature; METER on mint/redeem | ○ R4, counsel check |
| S12 | Pricing optimizer (KV repricing + conversion analytics) | "I'm guessing my prices" | SUB | ◐ funnel events exist |

## Shelf 2 — For buyers (my agent spends money safely)

| # | Brick | Buyer pain | Rent | Status |
|---|---|---|---|---|
| B1 | x402-paying-client crate | "Five lines to pay any x402 endpoint" | OSS | ◐ C5 |
| B2 | Signer service (grant-bounded, zeroize-on-drop) | "I can't put a raw wallet key in an agent" | OSS self-host; hosted SUB | ◐ C2 design |
| B3 | Policy engine (deny-by-default spend rules) | "My agent might get drained" | OSS core; policy packs SUB | ◐ I2 |
| B4 | Safe-to-pay reputation API (who serves after taking money) | "Which endpoints rob agents?" | METER per lookup | ○ observatory |
| B5 | Price index API (cheapest alive endpoint for X) | "Am I overpaying?" | free web; METER API | ○ atlas |
| B6 | Delivery-reliability dataset | "Counterparty due diligence" | METER / dataset snapshots | ○ C4b probes |
| B7 | Agent budget ledger (channels/credit for repeat spend) | "Per-call fees eat my margin" | SUB | ○ parked channels, R3-dependent |

## Shelf 3 — For publishers (post-Sept-15 wall builders)

| # | Brick | Buyer pain | Rent | Status |
|---|---|---|---|---|
| P1 | x402 paywall brick (retrofit any site/CMS) | "Blocked agents were my traffic" | SUB or BPS of crawl revenue | ○ deadline-driven demand |
| P2 | Web Bot Auth identity setup (keypair, directory, verified-bot filing) | "My good bot gets blocked by default" | FIX (we did it for ourselves first) | ◐ C0 filings — OURS FIRST |
| P3 | Sept-15 readiness audit ("will you survive the flip") | "What happens to my traffic on the 15th?" | FIX, scoreboard feeds inbound | ○ instant product |
| P4 | Pay Per Use / Monetization Gateway integration | "Cloudflare's program, my stack" | FIX consulting | ○ re-aims retired Rail B |

## Shelf 4 — For enterprises & the ecosystem

| # | Brick | Buyer pain | Rent | Status |
|---|---|---|---|---|
| E1 | Self-hosted facilitator (verify+settle+confirm) | "CDP quota/fees/dependency" | LIC + support | ○ R3, LOI-gated |
| E2 | Conformance suite site license (CI integration) | "Regression-proof our x402 stack" | annual LIC | ◐ vectors exist |
| E3 | Ambiguous-money incident forensics (24h retainer) | "Money moved, state unknown — now what" | retainer FIX | ◐ reconciler is the tool |
| E4 | x402 v2 internals workshop | "Get my team up the curve" | per-seat FIX | ○ material exists in reviews/ |
| E5 | Custom vector development | "Prove MY edge case is handled" | FIX per vector pack | ○ kaizen pipeline |

## Shelf 5 — Data & voice (the flywheel that sells the shelves)

| # | Brick | Buyer | Rent | Status |
|---|---|---|---|---|
| D1 | Weekly "State of x402" report | everyone (marketing) | free | ◐ cadence defined |
| D2 | Sept 15 Scoreboard (who's ready, measured) | press, sellers, publishers | free — viral spike | ○ build before deadline |
| D3 | Historical probe dataset snapshots (R2) | funds, researchers | METER per download | ○ accrues daily |
| D4 | Quoted-vs-settled baselines | sellers, analysts | free summary; METER full | ○ atlas core |

## Cash ordering (honest)

- **Now (0–4 wks):** S5 audits, P2/P3 deadline services — services cash.
- **Next (1–3 mo):** S4 badge SUB, B4/B5 METER APIs, S10 compliance brick.
- **Later (3+ mo):** S2 hosted BPS, E1 facilitator LIC, B7 budgets.
- **Never sold, always free:** S3 test kit, D1/D2 voice — they are why the
  wall has foot traffic.
