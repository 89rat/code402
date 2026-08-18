# code402 Launch Post — Drift Wall Edition (DRAFT v1 — INTERNAL until promoter approves)

Doctrine: confident, not hype. Every number links to evidence. No unverifiable claims.
Self-trades disclosed. Nothing here we can't defend line-by-line in public.

---

## LONG FORM (blog / dev.to / Medium / Mirror)

**Title: We measured 129 x402 endpoints. 7.8% of their catalog prices were wrong.**

Every AI agent that pays for an API call today trusts a number it didn't verify.

The x402 ecosystem has a public catalog — the canonical list of payable endpoints and
their prices. Wallets read it. Agents read it. We read it too. Then we did something
apparently nobody else does: **we probed the actual endpoints and compared.**

129 comparable endpoints. 10 disagreements. A 7.8% drift rate between what the catalog
says and what the endpoint actually charges.

Some of what we found:

- **stableupload.dev** — catalog says $0.005. The live endpoint demands **$2.00**.
  A 400× difference. An agent paying catalog prices would be rejected — or worse,
  an agent auto-approving "cheap" quotes would overpay.
- **blockrun.ai** — six endpoints return amounts like `"0.0110"` — a decimal *string*
  where the spec requires integer minor units. Naive payers will misread or choke.
- **exa.ai** — one endpoint now quotes `0`. Free? Broken? The catalog doesn't know.

This isn't a hit piece. Catalogs drift; that's what catalogs do. The point is that
**an economy where machines pay machines cannot run on unverified prices** — and
until now, nobody was measuring the gap.

So we built the measurement.

**What we run, daily, unattended:**

1. A crawler that ingests the public catalog (1,000+ endpoints indexed so far) and
   re-probes every endpoint's live 402 challenge. A 402 is a public price quote;
   we never pay for these probes.
2. A drift detector that flags catalog-vs-live mismatches and tracks
   *time-to-correction* — how long the ecosystem stays wrong.
3. A trust registry that computes seller levels from append-only, hash-chained
   evidence: `verified` (≥7 days, ≥99% fidelity), `verified-gold`, `flagged`,
   and the honest default — `unrated`.

**We're seller #1 in our own registry — and we're currently `unrated`.**

Our own badge won't flip to `verified` until we've survived 7 consecutive days of
our own measurement. We published the rules and live under them before asking
anyone else to. That's not marketing; it's the product.

**For sellers:** if you run an x402 endpoint, you're very likely already in our
dataset as `unrated`. Claim your domain, get measured, carry the badge — it updates
itself daily from evidence. It's free. Trust is not for sale; that's the point.

**For wallets and agent builders:** the drift feed exists. Ask us about it.

The wall updates every morning at 06:47 IST. Watch a seller fix their price and
slide off the wall in (usually) a few days — or watch them stay on it.

→ Trust record & methodology: https://code402.dev/trust
→ The badge: https://code402.dev/v1/trust/code402.dev/badge.svg
→ First mainnet settlement, facilitator-free, receipts included:
  https://basescan.org/tx/0xc6478aea46f82fb9bde295e052c5c26e42e4c80ceb6f44db35a4896cd2c7672d

*code402 — the trust layer for agent payments. Measured, not claimed.*

---

## SHORT FORM (X/Twitter thread, 6 posts)

1/ We probed 129 x402 endpoints and compared their live prices against the public
catalog. **7.8% were wrong.** One was wrong by 400×. Thread on what agent-payment
pricing actually looks like when someone measures it. 🧵

2/ stableupload.dev: catalog $0.005 → live $2.00. blockrun.ai: quoting "0.0110" as
a *string* where the spec demands integers. exa.ai: quoting $0. Agents paying
catalog prices are flying blind.

3/ This isn't a hit piece — catalogs drift. The problem: an economy where machines
pay machines can't run on unverified prices. Nobody was measuring the gap. So we
built the measurement.

4/ We crawl the catalog + re-probe every endpoint daily, publish the Drift Wall,
and compute seller trust levels from append-only, hash-chained evidence. Levels are
computed by code. Nobody is rated by hand. Including us.

5/ We're seller #1 in our own registry and currently rated **unrated** — our own
rules require 7 measured days. We live under them before asking anyone else to.
Badge is free. Trust is not for sale. That's the point.

6/ The Drift Wall updates every morning. Watch sellers slide off it — or stay on it.
https://code402.dev/trust

---

## STICKINESS MECHANICS (why this spreads)

- **A repeatable number:** "7.8% of x402 catalog prices are wrong" — citeable,
  repeat-able, unfalsifiable-because-true. This is the hook people quote.
- **A named wall:** the Drift Wall is a place, not a chart. People return to places.
- **Self-inflicted unrated:** us flunking our own bar is the most shared detail —
  it inverts every launch-post cliché.
- **The badge loop:** every seller who embeds it broadcasts us; it self-updates,
  so it stays on their site without maintenance.
- **Daily rhythm:** "updates every morning 06:47 IST" — a reason to come back,
  and the substrate for a weekly "Drift Report" follow-up post series.

## WHERE IT GOES (promoter executes; nothing sends without your nod)

1. X/Twitter thread (short form) — tag nobody; let it travel on the number
2. dev.to / Medium long form
3. Hacker News (Show HN, long form, weekday ~13:00 UTC)
4. x402 / agent-builder Discords & GitHub discussions — soft-share, answer questions
   with evidence links only

## DO NOT SAY (banned claims)

- No MC 4849 agent-code stat (single-sourced — banned from all materials)
- No "$24M/30d" volume claim (frozen vanity stat per Wayback evidence)
- No "the only" claims about settlement — say "facilitator-free, our own signer"
- No revenue promises, no "Google of x402" — measured, not claimed
