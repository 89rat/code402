# code402

**Machine-payable APIs for AI agents — the settlement layer that reconciles to the chain.**

An x402 v2 (HTTP 402) payment gateway on Cloudflare Workers (Rust/WASM), USDC on Base,
non-custodial by construction: the payer signs an EIP-3009 authorization directly to the
merchant. We never touch the funds — which forces every failure mode to be answered with
engineering instead of a refund.

```
agent ──POST──► 402 + PAYMENT-REQUIRED (HMAC-stamped)
      ──retry──► PAYMENT-SIGNATURE (EIP-712 / EIP-3009)
                │ structural gate → claim (Durable Object, exactly-once per (from,nonce))
                │ → facilitator /settle (CDP) → execute → XDR-1 receipt
      ◄──200────┘ output + offline-verifiable receipt + PAYMENT-RESPONSE
```

## Why this one is different

Most 402 gateways answer *"did the client pay?"* None of them answer what happens when
the answer is *maybe*. This repo is the full engineering for the ambiguous-money problem:

- **Settle-before-serve** — nothing is served before the transfer is on-chain
- **Exactly-once per (from, nonce)** — Durable Object claim machine with lease recovery
  for crashed isolates; race losers receive the byte-identical stored response
- **A reconciler that treats the chain as root of truth** — hourly sweep reads
  `authorizationState` + `AuthorizationUsed/AuthorizationCanceled` logs and resolves every
  stale claim three ways: settled / canceled / expired, with deep-scan escalation
- **Paid-but-unserved → entitlement, not apology** — if the chain proves you paid and we
  never served you, your next identical request executes free, bound to the original input
- **XDR-1 receipts** — RFC 8785 (JCS) canonical, domain-separated, signed `payment_ref`
  binding; the spec's §7 vector reproduces byte-for-byte in this repo's tests
- **Ambiguity classification as code** — facilitator failure shapes (live-observed CDP
  responses) map to fail-closed receipt-pending, never a guessed terminal state

## Live proof

- Production: **https://code402.dev** (v1 wire, Base mainnet; v2 route deployed dark pending flip)
- Live telemetry: **https://code402.dev/v1/ops/stats** (reconciler runs, backlog, breaker)
- Evidence page: **https://code402.dev/proof** — 1,000 real settles, the 132-phantom
  corpus, all four reconciler scenarios proven with transaction hashes
- A real mainnet receipt with its on-chain settlement: `reviews/` in this repo

## Try it

```bash
curl -X POST https://code402.dev/v1/tools/vat-mod97-check/call \
  -H 'content-type: application/json' \
  -d '{"input":{"vat_number":"GB123456789"}}'
# → 402 + challenge (price, chain, recipient, EIP-712 domain)
```

Machine discovery: `/.well-known/x402.json` · `/llms.txt` · `/.well-known/openapi.yaml` ·
`/.well-known/mcp.json`. The v2 wire (`PAYMENT-REQUIRED` / `PAYMENT-SIGNATURE` /
`PAYMENT-RESPONSE` headers) is live on staging and verified against the official
`@x402/fetch` client end-to-end (real settle, receipt, replay).

## Repo map

| path | what |
|---|---|
| `crates/core` | pure payment logic: x402 v2 wire types, EIP-712/3009 verification, claim state machine (model-checked), three-way reconciler, XDR-1 receipts + JCS |
| `crates/edge` | Cloudflare Worker: routes, stamped challenges, facilitator seam (CDP), Durable Objects, hourly reconciliation cron |
| `specs/x402` | vendored x402 specification, SHA-pinned (SPEC-VERSION checked by CI) |
| `reviews/` | the paper trail: stress reports, panel gate verdicts, reconciler spec + e2e evidence |
| `plans/` | design invariants (I1–I6), roadmap, monetization playbook |
| `PANEL.md` | the engineering constitution: multi-AI panel review, gates, every-defect-becomes-a-vector |

## How it's built (the process is half the repo)

Every payment-path change ships through a panel: independent AI reviewers (wide-angle +
red-team) attack the diff, the builder adjudicates every finding — prove it wrong or
concede — and tests adjudicate first. This caught three real production bugs the builder
had shipped green (including an entitlement hole and an unreachable code path that only
live e2e exposed). The full verdict trail is in `reviews/`.

## Links

Live: code402.dev · Proof: code402.dev/proof · Telemetry: code402.dev/v1/ops/stats
Social: [github.com/code402dev](https://github.com/code402dev) · [@code402dev](https://x.com/code402dev) · [LinkedIn](https://www.linkedin.com/company/code402dev)

## License

MIT. The receipts specification (XDR-1) is CC0.
