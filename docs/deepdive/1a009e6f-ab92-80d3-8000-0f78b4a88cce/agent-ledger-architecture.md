# Agent Ledger — Technical Architecture
**Date:** 2026-08-16 · **Audience:** founding engineering team · **Status:** v1.0

Mission: a cross-rail spend control plane for AI agents. Cloudflare is the *edge plane* (distribution + low-latency execution). The *record plane* (evidence graph, signing keys, verifier) is sovereign — Cloudflare-free by design. Cloudflare is a deployment target, never a dependency.

---

## 1. System Overview

```
                        ┌──────────────────── EDGE PLANE (portable) ────────────────────┐
                        │                                                               │
 Agent ──HTTP msg sig──►│  Spend Firewall (CF Worker | Lambda@Edge | self-hosted proxy) │
 (x402, AP2, MPP, card) │   ├─ Identity verify (HTTP Message Signatures, RFC 9421)      │
                        │   ├─ Policy check: budget (Durable Object / portable state)   │
                        │   ├─ Price check: quoted vs baseline (<2ms, local snapshot)   │
                        │   └─ Decision: ALLOW / BLOCK(402) / REQUIRE-ESCALATION        │
                        └───────┬──────────────────────────────┬────────────────────────┘
                                │ payment executes             │ receipt + telemetry
                                ▼ (unchanged, zero custody)    ▼
                        Rails: x402 facilitators,         ┌── RECORD PLANE (sovereign) ──┐
                        Stripe MPP, AP2, cards,           │ Receipt Signer (KMS/HSM)      │
                        agent wallets                     │ Append-only Evidence Graph    │
                                                          │  (hash-chained, merkleized)   │
                                ┌───────────────────────► │ Anchoring: public chains +    │
                                │                         │  independent data trust       │
   Telemetry ──► Analytics Engine (price index feed)      │ Public Verifier (stateless)   │
                                                          └──────────┬────────────────────┘
                                                                     ▼
                                          Dashboards · Public Price Index · Audit Bundles
```

**Data flow:** (1) Agent issues a payment request through the firewall endpoint (reverse proxy or SDK-injected fetch wrapper). (2) Firewall verifies the agent's HTTP Message Signature, resolves policy (agent → vendor → task budget), and compares the quoted price against the rolling baseline for `(endpoint, rail)`. (3) Decision in <5ms p50; if ALLOW, request proceeds to the rail. (4) On payment completion (tx hash / Stripe event / AP2 mandate), a receipt is canonicalized, hash-chained, and signed by the record plane — *not* by the edge. (5) The signed receipt hash returns synchronously in a response header (`Agent-Ledger-Receipt:`), which is the viral carrier. (6) Telemetry flows to Analytics Engine for the public price index; receipts flow to the evidence graph, merkle-anchored hourly to Base and daily to Bitcoin (via OpenTimestamps).

**Key split:** the edge *observes and enforces*; the record plane *attests and preserves*. The edge never holds signing keys. If Cloudflare disappears, receipts keep verifying; if our company disappears, anchors + escrowed data keep verifying.

---

## 2. Cloudflare Layer — the Edge Plane

| Component | CF Primitive | Why |
|---|---|---|
| 402 firewall / proxy | **Workers** (standard tier; no Smart Placement needed — co-locate via `placement: smart` only if baseline store is regional) | 0ms cold start, HTTP Message Signatures via WebCrypto Ed25519 at the edge, deployable as transparent reverse proxy on `agentledger.dev/v1/proxy/*` |
| Budget/session state | **Durable Objects** (one DO per agent-id; SQLite-backed) | Strongly consistent, single-writer budget counters — exactly what you need to prevent double-spend racing across concurrent agent tasks |
| Price telemetry | **Analytics Engine** (`writeDataPoint` per observed quote) | Free, high-cardinality time series feeding the public price index without hitting our own infra |
| Ingestion fan-out | **Queues** (batch size 100, 5s) between Workers and record plane | Decouples receipt emission from signing latency; retries for rail webhooks |
| Receipt bodies / audit bundles | **R2** with a *mirror* (see §3) | Cheap bulk storage — but R2 is a cache/replica, never the system of record |
| Baseline snapshots | **KV** (global, 60s TTL) + DO-cached hot set | KV's eventual consistency is fine for baselines (a 60s-stale percentile doesn't change a 2σ outlier verdict) |
| Agent identity | **HTTP Message Signatures (RFC 9421)** natively in Workers | Cloudflare's existing support aligns; agent keys are Ed25519, registered in an agent directory |
| Distribution | **Workers Templates + Marketplace**, **Agents SDK** plugin (`@cloudflare/agents` middleware hook) | One-click deploy = the growth wedge |
| LLM-assisted anomaly scoring (later) | **Workers AI** (Llama guard-class model on flagged txns) | Async only, never inline |

**Developer experience (<10 lines):**

```bash
npm create cloudflare@latest -- --template agent-ledger/firewall
wrangler secret put AGENT_LEDGER_KEY
```

```ts
// worker.ts — full integration
import { agentLedger } from "@agent-ledger/cloudflare";
export default agentLedger({
  apiKey: env.AGENT_LEDGER_KEY,          // attests identity, NOT a signing key
  policy: { defaultBudget: "50.00 USDC/day", inflationTolerance: 0.25 },
  upstream: "https://api.vendor.com",
});
```

Every response carries the viral header:

```
Agent-Ledger-Receipt: al1:7f3a…c2; chain=1892341; verify=https://verify.agentledger.dev/r/7f3a…c2
```

---

## 3. The Sovereign Evidence Core — the Record Plane

**Hard rule:** nothing here requires Cloudflare to operate, verify, or recover. CF may *replicate* data but never *owns* it.

**3.1 Signing key custody.**
Receipt signing keys live in **AWS KMS (asymmetric ECDSA P-256, `KeyUsage: SIGN_VERIFY`, non-extractable)** with a second root in **GCP Cloud HSM** — dual-cloud so no single provider is a hard dependency, and neither is Cloudflare. Key ceremony: 3-of-5 threshold operator approval (Shamir over the KMS grant policy), quarterly rotation of the *receipt-signing subkey*, long-lived root published in the transparency log. All public keys published at a static, domain-independent location: DNS TXT on our domain **plus** IPFS **plus** a `.well-known` document escrowed with the Internet Archive — verification must not depend on our DNS.

**3.2 Evidence graph.**
Each receipt is hash-chained: `receipt_hash[i] = SHA-256(canonical(receipt[i]) || receipt_hash[i-1])`. Hourly, receipts are merkleized; the merkle root is:
1. Written to **AWS S3 Object Lock (compliance mode, 7y)** and **GCP Cloud Storage with retention policy** — dual-cloud WORM;
2. Anchored on-chain: OP_RETURN-style commit on **Bitcoin via OpenTimestamps** (daily) and a cheap calldata write on **Base** (hourly) — public notaries no one can revoke;
3. Snapshotted quarterly to a **data trust / escrow** (e.g., Iron Mountain digital escrow or a university partner) holding the full graph + verifier source code.

Storage backend is boring on purpose: **Postgres (AWS Aurora) as the indexed ledger + content-addressed Parquet in WORM object storage as the immutable substrate**. No blockchain for the graph itself — anchoring only. Throughput target: 5k receipts/sec sustained, append-only tables, `INSERT`-only DB role.

**3.3 Verifier.**
The verifier is a **stateless, dependency-light binary** (single Go/Rust static binary, also WASM) that checks: schema → chain linkage → signature → merkle inclusion → on-chain anchor. It runs against *only* the public anchors + escrowed data. Escape hatch test: given nothing but (a) the OTS proofs, (b) the Base anchor calldata, (c) the escrow data dump, a third party can fully re-verify history. This is the company's existential insurance and the customers' audit guarantee.

**3.4 Migration continuity.** Receipt history and verification are untouched by any edge migration — the edge is stateless w.r.t. evidence (only DO budget state must be exported; see §6).

---

## 4. Firewall Engine Design

**4.1 Baseline computation.**
Baseline per `(endpoint_host+path_template, rail, currency)`:
- Rolling **7-day exponentially-decayed P50/P90** of observed accepted quotes, updated from telemetry (Analytics Engine → hourly rollup → published baseline bundle).
- Minimum sample floor: **n ≥ 30**; below that, endpoint is in **cold-start mode**: default policy `fail-open-but-flag` (allow, log, receipt marked `baseline: provisional`) and seed from the public index cross-endpoint medians and vendor-published price lists (AP2 mandates and x402 `accepts` payloads are crawled).
- Attack verdict: quote > `P90 × (1 + tolerance)` **and** quote > `P50 × 2` → BLOCK with 402 + explanation body. Two-condition AND prevents single-print spoofing of P90.

**4.2 Latency budget.**
Inline path target: **≤2ms p50, ≤8ms p99 added latency**. Achieved because the baseline bundle is a compact (~200KB) binary snapshot cached in the Worker isolate (KV refreshes every 60s) and budget state is a DO in the same colo. Decisions:
- **Inline:** signature verify, budget check, price-vs-baseline.
- **Async (post-decision, via Queues):** receipt signing, anomaly scoring, index telemetry.
- **Fail policy is per-policy-bundle, not global:** default `fail-closed` for budget exceeded (money), `fail-open` for baseline-service unavailability (availability) — but fail-open events are themselves receipted and rate-capped (max 100 fail-open txns/min per agent, then closed).

**4.3 Abuse handling.**
- **Baseline poisoning** (an attacker drip-feeds inflated quotes to move P50): baselines only ingest *completed, rail-confirmed* payments (x402 tx hash verified on-chain, Stripe event with signature), never raw quotes; plus per-endpoint contribution cap (no single agent-id contributes >5% of samples) and trimmed-mean outlier rejection.
- **Gas-abuse on x402**: facilitator response includes `maxAmountRequired`; firewall pins the permit amount to quoted price and rejects permits with headroom >10%.
- **Receipt flooding**: record-plane ingestion is idempotent on `payment_uid` (rail-native id); rate limit 100 receipts/min/agent at the queue; receipt signing is the scarce resource and is credit-metered per customer.
- **DO hot-key**: a single agent's DO saturating → automatic shard by task-id with deterministic rollup.

---

## 5. Cross-Rail Ingestion

**Design principle: prefer rail-native webhooks; poll only as reconciliation.** All connectors are read-only; we never initiate fund movement.

| Rail | Mechanism | Identity of payment (`payment_uid`) | Pattern |
|---|---|---|---|
| x402 (Base/Solana USDC) | Facilitator webhooks (Coinbase CDP, PayAI, Dexter) + direct on-chain confirmation via Base RPC / Solana `getSignaturesForAddress` | `chain:txhash:logIndex` | Webhook primary; 5-min polling reconciler against facilitator `/settle` history |
| Stripe MPP | Stripe event webhook `mpp.payment.*` with Stripe signature verification | `stripe:evt_id` / payment intent | Webhook + daily Balance Transactions API pull for reconciliation |
| Google AP2 | Signed AP2 mandates are **W3C Verifiable Credentials** — ingested as the mandate itself (agent submits via SDK or we observe via AP2 endpoint) | `ap2:mandate_id` (VC `id`) | Push (SDK) + VC signature verification against issuer DID |
| Card disputes (MC 4849) | Processor webhook (via Stripe/Adyen dispute events mapped to reason code 4849) | `card:dispute_id` | Webhook only; dispute receipts join the evidence graph as `type: dispute` linked to original payment receipt |
| Agent wallets (Coinbase Agentic Wallets, AWS AgentCore, MoonPay) | Read-only OAuth-scoped APIs / webhook where offered | `wallet:provider_tx_id` | Polling (1–5 min) with cursor pagination; webhook where available |

Connector output normalizes into the receipt schema (§6) through a per-rail `Normalizer` — the only rail-specific code in the system. Target: **receipt latency <30s p99 from payment confirmation**.

---

## 6. Multi-Edge Portability Contract

The contract has three artifacts, all versioned and content-addressed:

**6.1 Policy bundle** (`policy.alpb`, signed JSON): budgets, tolerances, fail-mode, rail config. Compiled once, deployable to any edge. Edges attest the bundle hash in every receipt (`policy_hash` field), so enforcement is auditable.

**6.2 Receipt schema (v1):**
```json
{
  "spec": "agent-ledger/1",
  "receipt_id": "ulid",
  "ts": "rfc3339",
  "agent": {"id": "did:key:…", "key_id": "…"},
  "vendor": {"endpoint": "api.x.com/v1/chat", "identity": "…"},
  "task_id": "…", "policy_hash": "sha256:…",
  "rail": "x402-base|x402-sol|stripe-mpp|ap2|card|wallet",
  "payment_uid": "…", "amount": "0.042", "currency": "USDC",
  "quoted_price": "…", "baseline_p50": "…", "baseline_p90": "…",
  "decision": "allow|block|escalate",
  "prev_hash": "sha256:…", "receipt_hash": "sha256:…",
  "sig": {"alg": "ES256", "kid": "al-root-2026q3", "value": "…"},
  "edge": {"type": "cf-worker|lambda-edge|selfhost", "version": "…"}
}
```
Note `edge.type`: the schema *expects* heterogeneous edges. Signing happens only in the record plane; the edge submits a canonical receipt candidate and gets back `sig` — so edge migration changes nothing cryptographic.

**6.3 Edge adapter interface** (3 functions): `verifyIdentity(req)`, `evaluate(req, policyBundle, baselineSnapshot, budgetState) → decision`, `emit(receiptCandidate)`. Reference implementations: CF Worker (canonical), **Lambda@Edge/CloudFront Functions** (baseline snapshot fetched from a public CDN URL; budget state via a lightweight regional API backed by DynamoDB — DO-equivalent), **self-hosted** (single-binary Go proxy + npm/pip SDK middleware `fetch` wrapper).

**Portability budget:** full edge migration = redeploy adapters + export DO budget state (nightly DO→record-plane checkpoint makes this lossless) + DNS cutover. Target: **<2 weeks, demonstrated quarterly.**

---

## 7. Build Plan (3 engineers)

**Weeks 1–8 — the wedge (x402 only, Cloudflare only, receipts from day one):**
- W1–2: Receipt schema + record plane skeleton: Postgres ledger, KMS signing service, hash chain, R2/S3 dual-write. *Evidence first — it is the moat.*
- W2–4: CF Worker proxy: RFC 9421 verify, hard budget via Durable Objects, x402 (Base, Coinbase CDP facilitator) ingestion, viral receipt header.
- W4–6: Baseline engine (Analytics Engine → rollup → KV snapshot), 402-inflation blocking, cold-start mode.
- W6–8: Public price index v1 (Pages + Radar-style API), free verifier (static site + WASM), one-click Workers template in marketplace. **Ship week 8.**
- Gate: 100 real receipts/day, verifier independently passes on escrow dump.

**Months 3–6:**
- Stripe MPP + AP2 connectors; dispute (4849) receipts; anomaly scoring (Workers AI); agent wallets (Coinbase Agentic Wallets first).
- Lambda@Edge adapter + self-hosted proxy (proves the contract), GCP HSM second root, OTS anchoring, dashboard GA.
- Audit bundle exporter (per-vendor, per-period signed tarball + verifier).

**Cloudflare kill-switch drill (quarterly, calendarized):**
1. Simulate CF termination: disable Workers routes for a staging deployment.
2. Cut traffic to Lambda@Edge + self-hosted proxy; export DO budget checkpoints → DynamoDB.
3. Verify: (a) zero receipt-chain gaps, (b) verifier passes fully against public anchors + escrow with no CF involvement, (c) p99 added latency ≤ 15ms on AWS path, (d) price index regenerates from evidence data.
4. Time-boxed: must complete in <5 business days. Publish the drill report — it doubles as enterprise-sales collateral.

**Success metrics:** decision p99 ≤8ms; receipt lag ≤30s p99; migration drill ≤5 days; verifier reproducible from escrow alone.

---

## Appendix — Assumptions & Open Decisions
- **Verified-by-design:** CF primitives used (Workers, DO, KV, R2, Queues, Analytics Engine, Templates, Agents SDK) are current GA products; RFC 9421 supported in Workers.
- **Assumptions to confirm:** exact facilitator webhook availability for PayAI/Dexter (fallback = on-chain polling); Stripe MPP event taxonomy (may require private preview access); AP2 mandate transport (SDK push assumed).
- **Open decisions:** data-trust counterparty; whether the price index API is fully open or keyed; Solana confirmation depth (proposed: 1 confirmed block, receipt marked `finality: soft` until finalized).
