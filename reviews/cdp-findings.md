# CDP x402 Facilitator findings — 2026-08-19 (plan-rev3 G8)

Source: docs.cdp.coinbase.com/x402/seller/facilitator + API reference
(`/api-reference/v2/rest-api/x402-facilitator/x402-facilitator`). Verified by
web fetch 2026-08-19; re-verify at Stage 4 implementation time.

## Endpoints (v2 REST)
- `POST /v2/x402/verify` — verify payment (scheme+network aware)
- `POST /v2/x402/settle` — on-chain settlement
- Supported-schemes/networks endpoint exists ("programmatic source of truth")
  → G7 health probe target; exact path to confirm at implementation.

## Authentication
- "Authenticates with your CDP API key ID and secret." Exact wire mechanism
  (header vs JWT; ES256 vs Ed25519) NOT stated on fetched pages — confirm from
  the API reference at Stage 4 before choosing the WASM signing crate
  (plan G10: `ed25519-dalek`/`p256`, not `ring`).

## Economics (updates G4 sizing)
- **Verification is always free.** Fees apply only to onchain activity.
- 1,000 onchain facilitator transactions/month free; then **$0.001 per onchain
  settlement**.
- ⇒ The budget the structural gate protects is primarily the **settle** path
  (settle must only ever follow verify-success + DO claim), not verify volume.
  Rate limits on verify still matter for latency/abuse, not cost.
- `batch-settlement` scheme: thousands verified offchain free, one onchain tx —
  relevant to later cost optimization (out of scope for now).

## Networks & schemes
- EVM (ERC-20 via EIP-3009 or Permit2): Base `eip155:8453`, Base Sepolia
  `eip155:84532`, Polygon `eip155:137`, Arbitrum `eip155:42161`,
  World `eip155:480`, World Sepolia `eip155:4801`.
- Solana: mainnet + devnet (`exact` only).
- Schemes on EVM: `exact`, `upto`, `batch-settlement` (all three CDP-supported;
  we implement `exact` only per locked decision 4).
- Same provider serves testnet and mainnet.

## Sanctions/KYT (G8 one-line finding)
- **OFAC + KYT screening is the facilitator's built-in responsibility** — it
  "identifies and declines payments involving sanctioned or high-risk
  addresses." Record: screening liability sits with CDP, not code402.

## Rate limits
- Not published on fetched pages (FAQ points to CDP SLO page: 99.9%
  availability target for verify/settle). Treat as unknown; breaker thresholds
  (G8) sized conservatively until observed. TODO Stage 4: measure and record.

## Legacy traffic measurement (G1) — DECISION: HARD-CUT
D1 remote query 2026-08-19:
- staging `code402-ledger-staging`: 2 CHALLENGED, 1 SETTLED (self-tests only)
- prod `code402-ledger-prod`: 40 CHALLENGED (unpaid probes), 1
  PENDING_SETTLEMENT (never confirmed), **0 SETTLED**
⇒ Zero completed legacy paid traffic ever, in either environment. Per plan G1:
**hard-cut the legacy X-PAYMENT route at Stage 5; no 90-day sunset machinery.**
(The 40 prod challenges are unpaid probes; they receive the v2 402 going
forward, which standard clients can actually answer.)

## PRE-CUT OBLIGATIONS (audit Q4, before the Stage 5 hard cut)
1. **Reconcile prod's 1 PENDING_SETTLEMENT row on-chain** (`AuthorizationUsed`
   / `TransferWithAuthorization` for that nonce): if the payment actually
   moved, serve or make that payer whole before cutting their redemption path.
2. **Re-run the traffic query at Stage 5** — this measurement is a point-in-time
   snapshot; re-verify ~zero settled traffic immediately before the cut.

## Auth mechanism CONFIRMED (2026-08-19, checklist #4 CLOSED)
Source: docs.cdp.coinbase.com/api-reference/v2/authentication.md + verify-payment.md.
- Auth = **JWT Bearer**: `Authorization: Bearer $JWT`. The Secret API Key never
  leaves our side — it locally signs JWTs (EdDSA/**Ed25519**, `kid`=key_id;
  claims sub=key_id, iss=cdp, aud=[cdp_service], nbf/exp, uri="METHOD host/path";
  120s lifetime). Wallet ops add X-Wallet-Auth (ES256) — NOT needed for
  verify/settle. => Implement JWT minting in the worker with ed25519-dalek
  (exactly Rev 3 G10's prediction; ring stays out). CDP_API_KEY secret =
  TWO parts (key id + secret), one wrangler secret as `id:secret`.
- **Base URL: https://api.cdp.coinbase.com/platform** — our client appends
  /v2/x402/verify|settle; verified against the live reference.
- `GET /supported` endpoint CONFIRMED (G7 health probe target is real).
- Payout config: CDP's DEFAULT settles into a provisioned CDP Server Wallet
  (custodial); `payToConfig type:"address"` = OUR OWN wallet (self-custody,
  what we use — COMPANY_WALLET). The "Coinbase Business account" marketing
  flow corresponds to their custodial default — NOT a protocol requirement.
- Account requirement: **CDP developer credentials only** (console.cdp.coinbase.com).
  The "Coinbase Business APIs" docs family (checkout APIs) is a separate
  product; x402 facilitator needs no Business account.
