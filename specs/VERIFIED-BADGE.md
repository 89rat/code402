# code402 Verified — Trust Badge Spec (v0.1)

**Status:** Draft for promoter approval — INTERNAL until launch
**Author:** CEO-function (Kimi) · **Date:** 2026-08-17
**Depends on:** DIS-1 crawler (`code402/crawler/`), daily pipeline automation, `observations.db`

---

## 1. What the badge certifies

One sentence, verifiable by anyone:

> **"This seller's published prices match their live 402 challenges, continuously measured by an independent crawler since {date}."**

The badge certifies *pricing honesty over time* — not quality, not uptime, not legality. Narrow claim, aggressively enforced.

Three measured properties:

| Property | Definition | Data source |
|---|---|---|
| **Quote fidelity** | live 402 amount == catalog/manifest amount | drift report (catalog-vs-live) |
| **Quote stability** | price changes are infrequent and never silent-spiky | time series of `amount_minor` per endpoint |
| **Settlement honesty** *(phase 2)* | settled amounts == quoted amounts | `kind=settled` rows vs preceding `quoted` rows |

## 2. Why this is defensible (and self-issued badges usually aren't)

1. **Evidence is append-only and hash-chained.** Every observation carries `raw_sha256` of the exact response. We publish methodology + hashes; anyone can re-probe and recompute. We cannot quietly rewrite history.
2. **Self-trades are excluded and disclosed.** Rows with `source=paid-probe` / `self_trade=true` never count toward fidelity stats. The exclusion is in the published methodology, not a promise.
3. **Revocation is automatic.** Fail the criteria, lose the badge at the next daily run. No human override. (Including for ourselves — code402.dev lives under the same rules.)
4. **We measured first.** The time series starts 2026-08-17. Nobody can backfill a competing record.

## 3. Badge levels

| Level | Criteria (rolling 30 days) |
|---|---|
| **Verified** | ≥7 consecutive daily observations AND quote fidelity ≥ 99% AND ≥ 1 observed live 402 |
| **Verified Gold** | ≥30 days AND fidelity ≥ 99.5% AND zero unresolved drift events |
| **Unrated** | < 7 days of data (honest default — most of the ecosystem starts here) |
| **Flagged** | any unresolved drift event > 48h old, or a spec-violating quote (e.g. decimal string in `amount`) — public, with evidence hash |

Levels are computed by code, from the DB, daily. No manual assignment.

**Definitions:**
- *Drift event:* catalog quote ≠ live quote for the same endpoint+asset, persisting across two consecutive daily runs.
- *Resolved:* catalog and live agree again. **Time-to-correction** (first-seen → resolved) is recorded per event and published per seller — this is the metric nobody else has.

## 4. Phase plan

**Phase 1 — Self-application (now).**
code402.dev is seller #1. We publish our own trust record first: `/v1/trust/code402.dev` serving signed JSON + an embeddable SVG badge. This is honest because the data is real and the methodology is public. It also dogfoods the pipeline.

**Phase 2 — Open directory (next).**
Every crawler-tracked seller gets a public trust page automatically (we already measure them unpaid — that's the point of the crawler). Sellers claim/enrich their page optionally. The badge becomes a *directory of measured sellers*, not a pay-to-display logo.

**Phase 3 — Settlement honesty (needs funded wallet).**
Compare settled vs quoted on our own endpoints (and any seller who opts in with read-only proof). Locked until the India onramp is resolved.

## 5. Technical delivery (Phase 1)

- `GET https://code402.dev/v1/trust/{domain}` → JSON: `{level, fidelity_pct, days_measured, drift_events, time_to_correction_avg_h, last_run, methodology_url, evidence_root_hash}`
- `GET https://code402.dev/v1/trust/{domain}/badge.svg` → shields-style badge embedding level + fidelity
- Nightly job (extend `daily.py`) computes trust records from `observations.db` into a `trust_records` table; worker serves it. No new infrastructure.
- Methodology page at `/trust/methodology` — PUBLIC tier content per the operating contract.

## 6. Hard rules (from OPERATING-CONTRACT.md)

- Methodology public; raw dataset INTERNAL; keys SEALED.
- Never mix self-trade or our own endpoints' stats into "organic demand" claims.
- A competitor who meets the criteria gets the badge. The day we fudge that, the product is dead.
- No outreach/marketing claims beyond what the data shows. The badge speaks; we don't inflate it.

## 7. Revenue hook (later, not now)

The badge is free. The monetizable layers on top, when volume warrants: API access to the full time series, drift alerting for wallets ("this seller's quote changed 400× since the catalog"), and Verified-Gold audit reports. Do not charge for trust itself — that poisons it.

---

*Next decision needed from promoter: approve Phase 1 build (trust_records job + worker endpoints).*
