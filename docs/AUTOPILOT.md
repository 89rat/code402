# AUTOPILOT.md — The Self-Running Business System

**One company:** code402 is the product. OpenFang is the moat + distribution engine.
This document defines what runs autonomously, on what schedule, where humans
are legally required, and how the viral loop and profit path work mechanically.

Adopted staging discipline (from independent review): **Stage 1 — prove the
service. Stage 2 — SDK distribution when adoption is measured. Stage 3 —
network/marketplace only after traction.** PayMesh-scale ambitions are Stage 3
and stay deferred until Stage 1 revenue is real. Our non-custodial settlement
(agent pays wallet→wallet via facilitator) deliberately avoids the prepaid-
custody money-transmitter trap that network models walk into.

---

## 1. The Autopilot Loops (what runs without you)

### Loop A — Revenue (24/7, edge)

```
Agent request → code402 edge (Cloudflare)
  ├─ no credential → 402 challenge (machine-actionable)
  ├─ paid → verify (EIP-712, k256) → NonceGuard replay check
  │        → deterministic tool → signed receipt → D1 + R2 → settlement queue
  └─ replay/idempotent → 409 or stored result (no double charge)
USDC lands wallet→wallet. No human in the loop. Ever.
```

### Loop B — Distribution (scheduled GTM hands)

| Hand | Schedule | Job |
|---|---|---|
| gtm-lead-harvester | every 2h | Sweep registries/framework graphs (READ-ONLY), score targets ≥50 |
| gtm-researcher | every 2h +17m | Map target pain → verifiable brief (claims cite shipped code only) |
| gtm-publisher | every 4h | Broadcast signed manifests; ≤50 artifact-rich touches/run; trial-first always |
| market-intelligence | real-time queue + 10min + daily | Score inbound signals (WASM), route high-intent to kaizen/human |

### Loop C — Operations (daily)

| Time (UTC) | System | Job |
|---|---|---|
| 00:00 | Kaizen | Analyze yesterday → bounded signed proposals (±10% price, ±quota, rebate bps) |
| 00:08 | Heartbeat | Revenue-integrity checks; any failure ⇒ RED ⇒ all autonomous changes HALT |
| hourly | Edge crons | Settlement confirmation queue drain, ledger reconciliation |
| weekly | Payout batcher | Referral payouts above $5 threshold only; treasury reserve check |
| weekly | Trace refresh | Facilitator contract re-verification; mock regeneration |

### Loop D — Viral mechanics (honest, built-in)

1. **Trial-first**: 25 free calls, wallet-keyed (Sybil-resistant) — agents
   self-onboard from the manifest with zero human contact.
2. **Propagation cards** in every 200/402 — each response is a signed
   rediscovery surface (offer, trial URL, referral policy).
3. **Referral from realized net** — `x-referrer-wallet` accrues rebate;
   self-referral rejected; velocity caps; batched payouts ≥ $5. Machines
   route peers because it pays, within fraud bounds.
4. **Receipt chain** — every paid call yields a verifiable receipt; receipts
   themselves are distribution artifacts (agents pass them to orchestrators).

Measured, never asserted: k_ref > 1 = self-sustaining; k_ref ≤ 1 = paid
acquisition channel, reported as such in the heartbeat.

---

## 2. The Human Boundary (legally required checkpoints)

Autonomous systems may NEVER touch these without human approval:

- Treasury wallet / payout destinations (multi-sig migration at $1K+)
- Signing keys (receipt vs manifest keys, rotation)
- Compliance policy, AUP changes, provider registry entries
- Referral rate changes beyond ±25% bounds
- Any product from the banned list (anti-bot evasion, MITM proxy, email dispatch)

Everything else — pricing within bounds, quotas, experiments, outreach,
manifest publishing, payout batching — runs on autopilot.

## 3. The Profit Path (pragmatic, reality-discounted)

| Milestone | Volume | Net/day | What runs it |
|---|---|---|---|
| Day 1 | 1 verified loop | ~$0 | Deployment acceptance (DEPLOY.md) |
| Day 10 | ~150–200 req | ~$2–4 | Registry listings + first trials |
| Day 100 | ~3.5–5K req | ~$50–95 | GTM hands + retention (cached-route share) |
| Day 300 | portfolio | ~$300–485 | 3–4 tools on the same skeleton + referral loop |

Compounding levers already built: semantic cache (margin % on retries),
kaizen bounded pricing, fail-tax on hostile traffic, referral economics.
Honesty: the architecture converts and retains demand; it cannot create it.

## 4. Failure Responses (autonomous)

| Signal | Response |
|---|---|
| Settlement failure rate > 5% | Rollback deployment, page human |
| ledger variance ≠ 0 | Heartbeat RED; payouts frozen; human required |
| Facilitator down | 503 + Retry-After; discovery + existing grants unaffected |
| Referral farm pattern (velocity caps hit > 10/day) | −25% rebate bps (bounded) + human flag |
| p99 latency regression > 10% vs baseline | Roll back last pricing change; alert |
| Registry delisting | Canonical /.well-known endpoints carry discovery alone |

## 5. Activation Checklist (in order)

1. `code402/DEPLOY.md` Phase 1 (staging) — acceptance a–f pass
2. Phase 2 (mainnet) — first real dollar lands; code402.dev resolves
3. Phase 3 — registry submissions; activate GTM hands on schedule
4. First heartbeat generated; pin chain head
5. Week 2: enable referral payouts (watch velocity caps first week)
6. Week 3–4: add yahoo_finance nectar tool (check redistribution terms first)
7. Month 2: schema-normalizer tool (same skeleton)
8. Month 3: evaluate air-gapped sandbox (AUP + abuse controls gate)

## 6. What "autopilot" honestly means here

- Revenue, distribution, pricing-within-bounds, fraud defense, receipts,
  payouts, and daily ops run without you — 24/7.
- You spend ~30 min/day on the heartbeat (GREEN = glance and go; RED = act),
  and make the gated decisions (treasury, keys, new products, partnerships).
- Virality is machinery (trial + propagation cards + referral-from-net),
  not a promise. The machine measures k_ref and reports it honestly.
