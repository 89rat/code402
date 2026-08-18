# code402 — Deployment Runbook

**The last blocker between the build and revenue. Execute in order. Stop at any red check.**

Prerequisites: Cloudflare account, `npm i -g wrangler`, `wrangler login`, Rust toolchain with `cargo install worker-build`, a dedicated Base wallet (staging) and a Safe multi-sig (production, when balance > $1K).

---

## Phase 0 — Local verification (done ✅)

- `cargo test -p m2m-core` → 16/16 pass (incl. context-distill)
- Edge crate builds via `worker-build --release` (run during first deploy)

## Phase 1 — Staging (Base Sepolia, test USDC)

```bash
cd code402

# 1. Create staging bindings
wrangler kv:namespace create PRICING
#   → paste id into wrangler.toml [[kv_namespaces]] id (REPLACE_STAGING_KV_ID)
wrangler d1 create code402-ledger-staging
#   → paste database_id (REPLACE_STAGING_D1_ID)
wrangler r2 bucket create code402-receipts-staging
wrangler queues create settlement-confirm

# 2. Apply D1 schema
wrangler d1 execute code402-ledger-staging --file=migrations/0001_init.sql

# 3. Set secrets (NEVER in files)
wrangler secret put COMPANY_WALLET        # staging receiving address
wrangler secret put RECEIPT_SIGNING_KEY   # hex secp256k1 key for receipt sigs
wrangler secret put RPC_PRIMARY           # Base Sepolia RPC
wrangler secret put RPC_FALLBACK

# 4. Seed pricing KV (0.005 USDC = 5000 minor units per tool)
wrangler kv:key put --binding=PRICING vat-mod97-check '{"amount_minor":5000}'
wrangler kv:key put --binding=PRICING company-number-format '{"amount_minor":5000}'
wrangler kv:key put --binding=PRICING context-distill '{"amount_minor":5000}'

# 5. Deploy (builds WASM edge via worker-build)
wrangler deploy
```

### Phase 1 acceptance (ONE verified paid loop — all must pass)

```bash
# a. Unpaid call → 402 challenge with correct payTo
curl -i -X POST https://code402-edge.<your-subdomain>.workers.dev/v1/tools/context-distill/call \
  -H 'Content-Type: application/json' \
  -d '{"input":{"html":"<p>hello <b>world</b></p>"}}'
# expect: HTTP 402, challenge body with recipient = COMPANY_WALLET, amount 5000, nonce

# b. Paid call (sign EIP-3009 with a test wallet holding Sepolia USDC,
#    retry with X-PAYMENT voucher) → 200 + output + signed receipt
# c. Replay SAME voucher → 409 REPLAYED_NONCE
# d. Same idempotency_key twice → second returns idempotent_replay:true, no double charge
# e. Receipt verifies against RECEIPT_SIGNING_KEY public key
# f. D1 ledger row written; R2 receipt object exists; settlement queue consumed
```

**If any of a–f fails: fix, redeploy, restart acceptance. Do not proceed to mainnet.**

## Phase 2 — Production (Base mainnet)

```bash
# 1. Create production bindings (env.production in wrangler.toml)
wrangler kv:namespace create PRICING --env production
wrangler d1 create code402-ledger-prod --env production
wrangler r2 bucket create code402-receipts-prod
wrangler queues create settlement-confirm-prod
wrangler d1 execute code402-ledger-prod --file=migrations/0001_init.sql --env production

# 2. Production secrets — COMPANY_WALLET = Safe multi-sig when balance > $1K
wrangler secret put COMPANY_WALLET --env production
wrangler secret put RECEIPT_SIGNING_KEY --env production   # DIFFERENT key than staging
wrangler secret put RPC_PRIMARY --env production
wrangler secret put RPC_FALLBACK --env production

# 3. Seed pricing (prod)
for t in vat-mod97-check company-number-format context-distill; do
  wrangler kv:key put --env production --binding=PRICING "$t" '{"amount_minor":5000}'
done

# 4. Deploy production
wrangler deploy --env production

# 5. DNS: code402.dev → the worker (Custom Domain in dashboard or routes)
#    Site (code402/site/dist) → Cloudflare Pages or same worker static assets
```

### Phase 2 acceptance (first REAL dollar)

- One real paid call from an external wallet → USDC lands in COMPANY_WALLET
- Receipt on R2 verifies; D1 ledger variance == 0
- `/.well-known/mcp.json`, `/.well-known/openapi.yaml`, `/llms.txt` all 200 on code402.dev
- Heartbeat day-1 entry written (even if mostly zeros — see AUTOPILOT.md)

## Phase 3 — Distribution (after Phase 2 acceptance)

1. Submit MCP manifest to registries (Smithery, PulseMCP) — via publisher hand
2. Publish `openapi.yaml` + llms.txt (already in site/public)
3. Activate GTM hands per AUTOPILOT.md schedule

## Rollback

- `wrangler rollback --env production` to previous deployment.
- Canary rule: if settlement failure rate > 5% or any net_final ≤ 0 event,
  rollback automatically and page human.

## Hard rules

- Secrets only via `wrangler secret put`. Never in git, files, or logs.
- Staging and production signing keys are DIFFERENT.
- Never claim "live" until Phase 2 acceptance passes with a real external payment.
