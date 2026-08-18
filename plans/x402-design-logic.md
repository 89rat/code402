# x402 Design Logic — The Unified Reasoning

> Provenance: parallel Claude Cowork session, adopted by operator directive
> 2026-08-19. This is the "why" standard against which every future proposal
> is judged. Its Rev 3 amendments (typestate, Facilitator/Rail traits, TLA+
> model, constant-time MAC, parked-with-triggers register) are adopted in
> plans/integrated-roadmap.md.

The "why" document under `x402-v2-plan-rev3.md` (merchant) and `paying-crawler-plan.md` (buyer). The plans say what and when; this says why, and it is the standard against which every future proposal — including clever ones — gets judged. Runtime: Cloudflare Workers (V8 isolates, WASM, single-threaded per isolate), D1, KV, Durable Objects. Wire: x402 spec v2.0 exactly. Signatures: EIP-712/secp256k1 for payments, Ed25519 for Web Bot Auth identity. Anything specifying hardware we don't run or wire formats the spec doesn't define is out of scope by construction.

## 1. The physics — where time actually goes

| Layer | Latency | Share of a paid call |
|---|---|---|
| On-chain settlement (Base block) | ~2,000 ms | ~95% |
| Facilitator `/verify` | ~100 ms | ~5% |
| Edge compute (parse, policy, sign, store) | ~1–5 ms | <0.5% |
| Cryptographic ops within that | ~µs | noise |

**Law 1: money moves at block speed; compute must merely be boring.** Optimization in this system means: fewer round trips per paid call, facilitator quota never wasted (it is the scarce resource, not CPU), crawler cache-hit rate (never re-pay unchanged content — that is the gross margin), and replay instead of re-execute. Any proposal measured in nanoseconds is optimizing 0.002% of the path and is rejected on arrival. Correctness of the state machine is worth 1000× more than speed of any instruction in it.

## 2. The six invariants

Everything in both plans is a consequence of these:

- **I1 — No serve without settle** (merchant). The resource ships only after `/settle` succeeds or a prior settlement record proves it already did.
- **I2 — No spend without policy** (crawler). A signature exists only if the policy engine approved the exact requirement first. Deny by default.
- **I3 — Exactly-once per (from, nonce)** — on both sides. The merchant settles each authorization once (DO claim, `UNIQUE(payer, nonce)`); the crawler signs each resource-attempt once (nonce ledger, resend-same-payment on ambiguity, never a fresh nonce until the prior outcome is known).
- **I4 — The chain is the root of truth.** D1 ledgers on both sides are caches of chain state; hourly reconciliation against `AuthorizationUsed` events closes every ambiguous window (settle-ok-timeout ghosts, receipt-pending rows, forged receipts).
- **I5 — Fail closed on money, fail open on meta.** Breaker trips, budget exhaustion, kill-switch → payments stop. Reputation lookups, telemetry, observatory writes fail → payments proceed, degradation logged. The meta-layer may never block or break the payment path. (The one architectural idea worth keeping from the compute drafts, placed here as an invariant.)
- **I6 — Content never touches payment decisions.** Both directions: response bodies provably cannot influence the crawler's policy engine (payment-layer injection defense, tested); payment status gates execution but never alters what the merchant's tool computes. Policy engines read protocol fields only.

## 3. Identity logic — three keys, three jobs

Web Bot Auth Ed25519 keypair = *who we are* (crawler identity across both rails; public directory at `/.well-known/http-message-signatures-directory`). Wallet secp256k1 key = *what we can spend* (lives only in the signer service; crawl workers request signatures and receive them or refusals; zeroize-on-drop; grant-bounded). HMAC secret = *what we offered* (merchant requirement stamps; Worker secret). The keys never share storage, never cross layers, and compromise of any one is survivable: identity key → re-register; HMAC → rotate, grace window absorbs it; wallet key → capped by the policy engine's spend ceilings, which is why the signer holds the policy and not just the key.

## 4. Wire logic — the spec is the ABI

The wire format is x402 v2's JSON envelopes, base64 in headers, byte-for-byte per the vendored spec. No bespoke binary layouts, no protobuf on the wire, no "optimized" headers: our entire commercial position (conformance suite, certification, audits) rests on being the implementation that matches the spec exactly, and §1 says wire-level serialization cost is noise. The codec is size-capped, allocation-bounded, and panic-free (`#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` — a panic aborts the whole Worker request).

Inside the code, the pipeline is enforced by typestate: `Payment<Received> → Payment<StructurallyValid> → Payment<Verified> → Payment<Claimed> → Payment<Settled> → Response<Served>` on the merchant; `Requirement<Parsed> → Requirement<PolicyApproved> → Payment<Signed> → Outcome<Receipted|Disputed>` on the crawler. Illegal transitions don't compile; the states erase at compile time and cost nothing at runtime. This is the correct, zero-cost version of "discrete execution."

## 5. The mirror principle

The two products are one threat model read in opposite directions. Every defense has a dual; building one side hands you the test for the other:

| Merchant (don't get robbed) | Crawler (don't get drained) |
|---|---|
| Settle before serve (I1) | Policy before sign (I2) |
| Field-verify echoed requirement vs HMAC stamp | Verify requirement vs asset allowlist + ceilings |
| Structural gate guards facilitator quota | Structural gate guards wallet + loop detection |
| Replay stored response to retries/race losers | Resend same payment on ambiguity; never double-sign |
| "Already used" → success only with our D1 record | Receipt trusted only when reconciled to chain |
| KV kill-switch, breaker fail-closed | Budget caps, per-domain breakers, kill-switch |
| Claim lease frees wedged nonces | Delivery scoring blacklists pay-no-serve domains |
| NonceGuard retired: client owns nonces | Nonce ledger: we own our nonces |

Corollary: **our crawler paying our merchant on Sepolia is the highest-value test in the system** — every mirror pair exercises live, both directions, before any stranger's money is involved.

## 6. Settlement sequence logic — why each step sits where it does

`structural gate → /verify → full request validation → DO claim (lease) → /settle → execute → persist → respond`

Structural gate first: facilitator quota is the scarce resource (I5's fail-closed breaker makes quota exhaustion a DoS vector, so garbage never reaches CDP). Verify before claim: verification is read-only; claiming first would mint junk DO state for invalid payments. Full validation before settle: never take money for a request we can already reject — "paid but failed" must be unreachable for rejectable requests. Claim before settle: the DO serializes concurrent same-(from,nonce) attempts; losers wait and replay rather than error. Persist before respond: the stored response is what makes retries, race losers, and duplicate deliveries idempotent. The claim machine (`claimed → settling → settled | failed`, alarm-based lease) is small enough to model-check — a TLA+/model of its interleavings is a cheap Stage 4 artifact and the right home for formal methods in this system (not dependent-typed balance arithmetic; there are no balances here, EIP-3009 moves exact amounts).

## 7. Policy logic — one engine shape, two instances

Both policy engines are the same construction: an ordered list of pure predicates over protocol fields only (I6), versioned and reviewed like code, deny-by-default, every decision logged with the rule that made it. Merchant instance: header well-formedness, size caps, single-header, nonce = 32 bytes, echo-vs-stamp equality, validBefore ≥ now + settle margin, EOA-ecrecover prefilter (6492/1271 pass through). Crawler instance: canonical-USDC-per-CAIP-2 allowlist, price ceiling per content class, per-domain/hour/day caps, no cross-domain redirect follow, loop detection, remaining-budget check. A rejected predicate anywhere = no signature, no facilitator call, no spend — and the log line is the audit product's raw material.

## 8. Data logic — the flywheel is exhaust

Neither side generates data as a feature; both generate it as exhaust, and the exhaust is the inventory. Merchant side: settlement ledger + failure-reason taxonomy = who pays correctly. Crawler side: delivery outcomes at pennies of COGS = who serves after taking money. Union: the observatory — endpoint quality, conformance status, safe-to-pay reputation — sold back to the market as a paid x402 API through our own spec-perfect endpoint, and doubling as certification evidence and audit outreach ammunition. The kaizen rule closes the loop: **every defect becomes a vector** — every bug, client finding, reconciliation divergence, and probe anomaly ends as a fixture in the conformance suite, which is simultaneously the product, the credential, and the regression net. Integrity constraint: no manufactured volume — self-paid transactions are labeled smoke tests, minimal, never Builder-Code-farmed.

## 9. Failure logic — ambiguity resolves to idempotence

Every failure in this system is one of three kinds. Deterministic rejection (policy/verify says no): cheap, logged, final. Ambiguous outcome (timeout after settle sent; network death after payment sent): resolved by idempotent replay — same nonce, same payment, same stored response — and whatever remains ambiguous is closed by reconciliation against the chain (I4). Systemic failure (facilitator down, budget gone, kill-switch): fail closed on money (I5), page a human, never improvise. There is no fourth kind; any failure path that doesn't map to one of these three is a design bug.

## 10. Future-proofing — pin, abstract, park

**Pinned:** vendored spec files + `SPEC-VERSION` (spec commit, SDK version, CDP API version) read by CI; golden vectors as the compatibility contract; monthly drift watch.

**Abstracted now (cheap, earns its keep):** `Facilitator` trait over verify/settle (CDP today; self-hosted or secondary tomorrow — this is the enterprise-facilitator product seam); `Rail` trait (x402-native vs Pay-Per-Crawl); scheme enum with `upto` headroom in types only; CAIP-2 network strings everywhere (Solana is a new match arm, not a redesign).

**Parked, with entry triggers — not built:** Solana rail → when a paying customer requires it. Self-hosted facilitator product → when an enterprise LOI exists. State channels / batching → only if per-settle fees return (CDP is currently free; the problem doesn't exist). ZK compliance proofs → only if a regulated buyer demands PII-free attestations and pays for them. Binary telemetry serialization → only past ~10M events/day, and never on the x402 wire. Exchange (m2m-exchange) → after toll-operation relationships supply liquidity. A parked idea with a trigger is strategy; the same idea without one is procrastination with extra steps.

## 11. Where hard systems rigor actually lands

The legitimate low-level concerns, at their correct addresses: signer service — keys zeroized on drop, never serialized, signing bounded by grants; WASM payment path — panic-deny lints, bounded allocations, U256 via alloy-primitives, no floats anywhere near money; MAC comparison — constant-time; codec — differential fuzz Rust↔TS nightly plus static bidirectional vectors; DO claim machine — model-checked. That is the complete list. Rigor spent anywhere else is rigor taken from here.

## 12. The one-line law

**Be byte-exact at the wire, exactly-once with money, deny-by-default on spend, fail-closed on payment and fail-open on everything else, reconcile to the chain, and turn every defect and every probe into sellable data.** Any proposal that doesn't serve one of those clauses — whatever hardware it name-drops — is decoration.
