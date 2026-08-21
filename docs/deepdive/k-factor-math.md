# The K ≥ 1.2 Math: Engineering the Two-Sided Viral Loop

*Written 2026-08-16. Companion to virality-engineering.md. All numbers are explicit assumptions — test them weekly in kaizen loop 3.*

---

## 1. Why single-sided virality caps at K ≈ 1.0

Classic model: K = exposures per user × conversion rate.

Buyer-side only: each proxy user emits ~600 receipt-header exposures/month, but to **non-unique** counterparties. Unique sellers touched per user per month: **~25–30**. Impression-level conversion is irrelevant; what matters is *unique-actor conversion*. At 0.3% unique-actor conversion: K = 30 × 0.003 = **0.09**. Pathetic. Even the optimistic bundle of all single-sided carriers summed to K ≈ 0.8–1.0.

The lever isn't more exposures. It's **conversion rate on the exposed party** — and the exposed party (a seller) has no reason to install a buyer-side firewall... unless you give them one.

## 2. The structural fix: the loop must be two-sided

Give the seller a reason to activate, and give the *seller's* responses a carrier that exposes new buyers. Then buyers create sellers and sellers create buyers — and the math multiplies instead of adding.

**The two carriers (both just headers):**
- **Buyer → Seller:** `x-agent-ledger-receipt: <hash>` (+ block citations when a quote is rejected)
- **Seller → Buyer:** `x-agent-ledger-verified: P47 · agentledger.dev/p/<endpoint>` on every 402 response from activated sellers (+ the README badge)

**The seller activation product ("Claim Your Endpoint," free):** sellers register to (a) see which agents were blocked and why, (b) get their percentile + badge, (c) dispute mischaracterization. The hook is loss aversion: **"3 agents rejected your quote this week — here's why"** is the highest-converting message available in this market, because blocked quotes = lost revenue.

## 3. The two-type K matrix (this is the hard math)

Two populations: buyers (B) and sellers (S). Growth is governed by the next-generation matrix:

```
| B' |   | K_bb  K_sb |   | B |
| S' | = | K_bs  K_ss | × | S |
```

K_xy = new x's produced per y per month. Growth rate = dominant eigenvalue λ of the matrix. K ≥ 1.2 means **λ ≥ 1.2**.

**Assumptions (per active user per month):**

| Parameter | Value | Basis |
|---|---|---|
| Unique sellers touched per buyer | 30 | ~40 payments/mo, ~75% unique endpoints |
| Seller activation rate from exposure | **4%** | loss-aversion hook ("you're losing sales") + free claim page. Dev-tool norm for high-intent exposure: 2–5% |
| **K_bs (sellers created per buyer)** | **1.2** | 30 × 0.04 |
| Unique agents served per activated seller | 60 | paywalled endpoint serving agents; response header + badge on every reply |
| Buyer activation rate from seller exposure | **2%** | "protect your agent from overcharges" — medium intent |
| **K_sb (buyers created per seller)** | **1.2** | 60 × 0.02 |
| K_bb (direct buyer→buyer: OSS, MCP tool, posts) | 0.25 | GitHub/MCP virality from virality-engineering.md |
| K_ss (seller→seller: directory, badge culture) | 0.15 | verified-seller directory competition |

Matrix:
```
M = | 0.25  1.2 |
    | 1.2   0.15 |
```

**Dominant eigenvalue:**
λ = (0.40 + √(0.10² + 4×1.44)) / 2 = (0.40 + √(0.01 + 5.76)) / 2 = (0.40 + 2.402) / 2 = **1.40**

## 4. K = 1.4, with sensitivity (so you know what must be true)

| Seller activation (K_bs driver) | Buyer activation (K_sb driver) | λ |
|---|---|---|
| 2% (K_bs=0.6) | 1% (K_sb=0.6) | 0.79 — flywheel fails |
| 3% (0.9) | 1.5% (0.9) | 1.10 — self-sustaining |
| **4% (1.2)** | **2% (1.2)** | **1.40 — target** |
| 5% (1.5) | 2.5% (1.5) | 1.72 |

**The two numbers that must be true: ≥4% seller activation and ≥2% buyer activation.** Both sit inside normal dev-tool ranges for high-intent exposures — but they are NOT free. They require: (a) the block-notification email/page to be excellent (loss aversion, one-click claim), (b) the claim flow to take <5 minutes, (c) the seller header/badge to be one line of middleware.

Margin of safety: even at half-target conversion (3%/1.5%), λ = 1.10 > 1 — the loop still self-sustains. The design goal isn't hitting exactly 1.4; it's building enough headroom that missing assumptions by 50% still leaves K > 1.

## 5. Cycle time — the hidden multiplier that matters as much as K

Viral growth ≈ K^(t/cycle_length). Two ways to raise effective growth: raise K, or **shorten the cycle**.

- Baseline cycle (exposure → activation): ~30 days if sellers notice headers organically.
- With **weekly "blocked-quote digest"** to unclaimed sellers (you have their endpoint from the crawl; a polite one-time notification — useful, not spam: "your endpoint was quoted/rejected N times this week, median deviation +X%"): cycle compresses to **~7–10 days**.

Effect: λ=1.4 at 30-day cycles → 1.4×/mo. At 9-day cycles → 1.4^(30/9) ≈ **3.1×/month effective**. Cycle-time compression is worth more than any further K tuning. (Rule: one notification, unsubscribe honored instantly, data always useful — spam once and the trust asset burns.)

## 6. The ceiling — honest math on carrying capacity

K > 1 does not grow forever; the market is finite. Relevant graph size (paywalled endpoints + serious agent builders): **~5,000–20,000 actors** in 2026, growing with the ecosystem. Logistic model: N(t) = C / (1 + e^(−r(t−t₀))), r = ln(λ)/cycle.

Starting from 50 activated users at λ=1.4, 9-day cycles: **~50% of a 10,000-actor graph reached in ~4–5 months.** After saturation, K collapses (everyone's already exposed) and growth shifts to (a) ecosystem growth riding new entrants, (b) retention/monetization. This is fine — as stated in the virality doc, the goal was never millions; it's **total name-ownership of the graph**, after which the moat (data, records, spec authorship) does the earning.

K ≥ 1.2's real job: get to saturation *before a copycat does*. It's a race parameter, not a revenue parameter.

## 7. Build list to hit the math (in order)

1. **Response-side header + claim page** (week 3–4): one-line seller middleware, `<5 min` claim flow, percentile + "blocked N times" dashboard. Without this, K_sb = 0 and the matrix collapses to single-sided λ ≈ 0.5.
2. **Block-citation + weekly digest** (week 5–6): the 4% seller-activation engine.
3. **Verified-seller directory** (month 3): feeds K_ss and gives sellers a rank to compete for.
4. **MCP price tool** (month 3, per virality doc): feeds K_bb.
5. **Weekly instrumentation:** track unique-sellers-exposed per buyer, claim rate, header-activated buyers per seller — the four matrix parameters — in kaizen loop 3. If any parameter runs below 50% of target for 4 consecutive weeks, the loop gets a kaizen improvement before anything else does.

## 8. The math in one line

**Single-sided:** K = 30 × 0.003 + ε ≈ 0.1–1.0 (caps out).
**Two-sided:** λ = eigenvalue of [[0.25, 1.2],[1.2, 0.15]] = **1.40**, with 50% miss-tolerance still > 1, and cycle compression to 9 days making it ≈ **3×/month effective** until graph saturation at ~4–5 months.

The difference between the two lines is one product decision: **build the free seller-side claim page.** Everything else in this document already exists in the plan.
