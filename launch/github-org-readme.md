# GitHub org profile README (put in repo `code402dev/.github/README.md`)

## Code402

**Machine-payable APIs for AI agents — the settlement layer that reconciles to the chain.**

Non-custodial x402 (HTTP 402) payments in USDC on Base. We never hold the funds — so every
failure mode gets answered with engineering instead of a refund.

### What's here

| repo | what |
|---|---|
| **code402** | The gateway: Rust/WASM Cloudflare Worker, settle-before-serve, exactly-once claims, hourly chain reconciliation, XDR-1 receipts. Fully open, failures included. |
| m2m-exchange | The M2M/1 commerce protocol spec layered above payment rails. |
| x402-atlas | The neutral index of machine-payable endpoints. |

### The receipts standard

XDR-1: offline-verifiable delivery receipts — RFC 8785 canonical, domain-separated,
signed payment binding. Spec + reference implementation + vectors in the main repo.
CC0, vendor-neutral, third-party verifiable with no call to us.

### Live

- https://code402.dev — production (Base mainnet)
- https://code402.dev/proof — the evidence page: real settles, real failures, real tx hashes
- https://code402.dev/v1/ops/stats — live operational telemetry

### How it's built

Every payment-path change passes a multi-AI panel review with published verdicts
(`reviews/` in the main repo). Every defect becomes a test vector. The process is
designed on the assumption that the builder is the last person to see their own bugs.
