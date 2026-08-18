• I'll read the required files first.

• Read `AGENTS.md`, `wrangler.toml`, `crates/edge/src/lib.rs` (all 638 lines), `crates/core/src/payment/{erc3009,eip712,mod}.rs`, and `crates/core/src/lib.rs` (PaymentError enum). Review below.

  ## (1) Top weaknesses / risks

  **W1. Settle-after-serve is a payment hole, and the plan doesn't say which side of the line Stage 4 lands on.** Today the flow is: local EIP-712 verify → nonce claim → **execute tool → 200 with output** → *async* settlement confirm via queue (`lib.rs:382-428`, consumer at `lib.rs:543-563`). The voucher is never moved on-chain before the client already has the output. If you keep that ordering with a real facilitator, a client can present a valid-looking EIP-3009 authorization from an empty wallet, get the output, and the settle fails afterwards. The canonical facilitator flow is **verify → settle → then deliver resource**. The plan must state explicitly: PAYMENT-RESPONSE (which carries the tx hash) is only emitted after facilitator `settle` succeeds, and the tool executes after settle. That changes latency characteristics (block time on every paid call), which is a product decision, not an implementation detail.

  **W2. Nonce ownership is backwards vs. the spec.** In `challenge()` the *server* mints the nonce (`lib.rs:456`: `keccak256(request_id:now)`) and NonceGuard does replay. In x402 exact/EVM the **client** picks a random 32-byte `nonce` inside the EIP-3009 authorization; replay protection comes from the on-chain `authorizationState` and the facilitator. Server-issued nonces in the challenge will break every spec-compliant client. NonceGuard becomes redundant on the payment path (keep it only if you keep the legacy wire).

  **W3. "x402 v2" is a moving target; pinning a spec commit is necessary but not sufficient.** The v2 header names and payload shapes churned across RCs. You must pin *three* things together: the vendored spec commit, the `@x402/*` TS SDK version used to generate vectors, and the CDP facilitator API version — and verify header names/field names against the vendored files, not from anyone's memory. A conformance checklist (Stage 0) keyed to spec section numbers is the right idea; make it a CI gate, not a document.

  **W4. Your "legacy" isn't x402 v1 — it's a private format.** Current `X-PAYMENT` is a raw JSON `PaymentVoucher` (`lib.rs:336-343`), not the base64 x402 v1 payload. So "v2 alongside legacy" means maintaining *your own* dialect forever for a client population that is, realistically, your own SDK (`sdk/`). Given `AGENTS.md`'s schema-stability doctrine, prefer a parallel versioned route (`/v2/tools/{tool}/call`) over header multiplexing on the same route, with a dated sunset for the bespoke path — rather than a permanent config-flag fork inside one handler.

  **W5. Facilitator is a hard SPOF with new auth surface.** CDP `/verify` + `/settle` needs an API credential (new secret alongside `COMPANY_WALLET`/`RECEIPT_SIGNING_KEY`/`RPC_*`), has rate limits, and does its own screening (requests can be refused for reasons you don't control). The plan has no failure-budget story: what's the user-facing behavior when the facilitator 5xx's after you've already decided to serve? There's no graceful local-verify fallback that doesn't reopen W1.

  **W6. Permit2 is a scope bomb.** "EIP-3009 default and Permit2" doubles the crypto surface: Permit2's EIP-712 domain is the Permit2 contract (`0x0000...78BA3`), not the token; the witness type is a different struct; allowance/expiration semantics differ; your `erc3009.rs:verify` shape doesn't generalize. USDC supports EIP-3009 natively on both chains you run. Cut Permit2 to a later stage or drop it — see (3).

  ## (2) Concrete improvements per stage

  **Stage 0**
  - Record in the vendor dir: upstream URL, commit SHA, date, license, and the exact TS SDK version used for vectors. Add a `SPEC-VERSION` file the conformance tests read, so a spec bump without a code bump fails loudly.
  - Baseline tests: snapshot-test the *current* 402 body and X-PAYMENT flow end-to-end under `wrangler dev` before touching anything — you currently have 16 core unit tests but zero HTTP-level tests, so you have no regression net for the refactor.
  - The conformance checklist should be an executable fixture suite (golden request → expected status/headers/body), not a markdown audit.

  **Stage 1**
  - `amount` must be a decimal **string** parsed into `U256` — never float, never u64. Your current `amount_minor: u64` (`lib.rs:333`) is fine at $0.005 but will silently truncate large prices; make the v2 types `U256`-native per AGENTS.md's integer-money rule.
  - Add `resource` and `maxTimeoutSeconds` to PaymentRequirements and validate them server-side (see traps).
  - Base64 codec: reject non-canonical/oversized input with a size cap. Cloudflare caps total header size (~32KB); a malformed or huge PAYMENT-SIGNATURE must produce a 4xx, not a worker panic (a panic is a 500 with no taxonomy).
  - Golden vectors: pin *byte-exact* fixtures (input JSON → base64 header string → decoded struct), committed under `crates/core/tests/vectors/`.

  **Stage 2**
  - The TS SDK generates vectors; Rust verifies them — good. Also do the **reverse**: Rust generates, TS verifies. One-directional vectors can't catch a shared misreading of the spec.
  - Vectors must cover: both EIP-712 domain name variants (Sepolia `"USDC"` vs mainnet `"USD Coin"` — you already handle this via `TOKEN_NAME`, `wrangler.toml:58/112`; make it a vector, not a runtime prayer), `v` normalization 0/1 vs 27/28 (`eip712.rs:24-28`), expiry boundary (`valid_after == now`, `now == valid_before` — your current check at `erc3009.rs:16` is inclusive on both ends; confirm the spec/facilitator agree), and over- vs exact-amount.
  - Error taxonomy: keep your internal `PaymentError` (`crates/core/src/lib.rs:4-14`) but add an explicit mapping layer to the spec's error strings for the PAYMENT-RESPONSE `errorReason` / 402 `error` field. Don't leak internal names and don't invent new ones.

  **Stage 3**
  - 402 response: PAYMENT-REQUIRED header (base64 `PaymentRequired{x402Version:2, resource, accepts:[...]}`), `accepts` carrying one exact/EVM requirement with `network: "eip155:84532"` (staging) / `"eip155:8453"` (prod), `asset` = the token address, `payTo` = COMPANY_WALLET, `extra: {name, version}` from `TOKEN_NAME`/`TOKEN_VERSION`.
  - **CORS**: add `Access-Control-Expose-Headers: PAYMENT-REQUIRED, PAYMENT-SIGNATURE, PAYMENT-RESPONSE` (and allow PAYMENT-SIGNATURE in preflight). Browser-based agents silently fail without this — easy to miss because curl tests won't catch it. Note your SPA (`site/dist`) is served by the same worker (`wrangler.toml:12-15`).
  - Verify the client-echoed `accepted` requirement **field-by-field against what you offered** (scheme, network, asset, payTo, amount) — never trust it. A client swapping `amount` in the echoed copy is the classic exact-scheme cheat.
  - Remove the nonce from the challenge; keep `validAfter=0`-style guidance out of scope — the client sets the window, you enforce `maxTimeoutSeconds` and a clock-skew buffer on `validBefore`.
  - Update or delete `/.well-known/x402.json` (`lib.rs:198-226`) — it currently advertises `x402_version: 1` and the bespoke challenge. Also `llms.txt`, `openapi.yaml`, and `mcp.json` (embedded at `lib.rs:228-231`) presumably describe X-PAYMENT; the plan doesn't mention them.
  - The `X-Schema-Version: 1.0` header (`lib.rs:45`) and the 402-body-as-authoritative contract (`lib.rs:221`) are your own stability doctrine — v2 changes both. Decide and document which is normative now.

  **Stage 4**
  - Sequence: facilitator `/verify` → **nonce/settlement-idempotency claim** → `/settle` → execute → PAYMENT-RESPONSE. 
  - Don't build settlement idempotency on a naive D1 row. You already have a single-threaded NonceGuard DO pattern (`lib.rs:615-637`); reuse a DO keyed by the payment nonce for the verify→settle critical section so two concurrent requests with the same payload can't double-settle. D1 gets the durable *record* (with a UNIQUE constraint on nonce, new migration `migrations/0002_*.sql` — plan doesn't mention migrations), the DO gets the mutual exclusion.
  - **Retry semantics trap**: queues are at-least-once (`wrangler.toml:44-48`, `max_retries: 5`). If a `/settle` call times out client-side but succeeds server-side, the retry will fail with an "authorization already used"-class error. Map that error to "already settled → success", not to failure.
  - Add a facilitator health metric/alarm (your hourly cron at `lib.rs:593-600` is a natural place) and a circuit-breaker flag in KV so you can fail-closed cleanly instead of serving unsettled output.

  **Stage 5**
  - SDK: publish helpers that mirror the *spec* flow (read PAYMENT-REQUIRED, sign EIP-3009, send PAYMENT-SIGNATURE, parse PAYMENT-RESPONSE) — and keep the old SDK importable at its old version so existing agents don't break.
  - Deploy order: staging (Sepolia) e2e → prod worker with `v2 disabled` flag → enable v2 on prod with legacy route still live → mainnet flip = changing `accepts` network + facilitator env, behind one explicit var. Since `env.production` already isolates bindings (`wrangler.toml:64-114`), make the v2 on/off and network selection pure `[vars]`, no code change for the flip.
  - Update `AGENTS.md` (it documents X-PAYMENT as the contract), `DEPLOY.md`, `specs/VERIFIED-BADGE.md` if it references the wire, and the `sdk/README.md`.

  ## (3) Add / cut

  Add:
  - **A "settle-before-serve vs serve-before-settle" decision** as an explicit Stage 4 gate, with risk acceptance written down if you keep optimistic serving (e.g., settle fails → clawback = none, you ate the cost).
  - **Spec/SDK drift watch**: a monthly check (you already have cron infrastructure and an intel pipeline in `intel/`) against the pinned commit.
  - **Secret provisioning step**: CDP API credential into `wrangler secret put` for both envs; document rotation.
  - **Rollback**: KV/env kill-switch to disable v2 acceptance without a redeploy.
  - **Migration file** for the settlement table (`migrations/0002_*.sql`).

  Cut or defer:
  - **Permit2** — defer indefinitely. Zero demand signal, doubles crypto surface, USDC has native EIP-3009 on both chains.
  - **Legacy flag** — prefer the parallel `/v2/` route with a sunset date over a permanent in-handler fork. If real external legacy clients don't exist (the crawler/`sdk/` traffic is yours), cut legacy entirely after migration and delete the code.
  - **Bazaar/discovery extensions and multi-`accepts`** — offer exactly one requirement; skip extension plumbing until a consumer asks.

  ## (4) Spec-compliance traps you may be missing

  - **Exact means exact.** Your verifier accepts overpayment (`erc3009.rs:15`: `value < required` fails, so `>=` passes). The exact scheme (and CDP's facilitator) expects `value == amount`. Decide whether you keep `>=` locally (generous, but divergent) or match the facilitator — mismatch between your pre-check and the facilitator's check produces confusing failures after you've already told the client to proceed.
  - **CAIP-2 everywhere**: `network` is `"eip155:8453"`, not `"base"` — your current challenge emits `{"chain_id":..., "name":"base"}` (`lib.rs:473`). Every place that formats or compares networks needs the CAIP-2 string, including D1 records if you store it.
  - **Echoed-requirement verification** (above, Stage 3) — the spec puts the accepted requirement inside the signed payload envelope; the server must still re-check it.
  - **EIP-6492 / smart-wallet signatures**: local `recover_address` (`eip712.rs:20-32`) only handles 65-byte ECDSA. Counterfactual smart wallets (common in the x402 ecosystem) sign via 6492 envelopes. If you pre-verify locally, you'll 401 legitimate payers. Either skip local signature recovery and let the facilitator `/verify` be authoritative, or explicitly document EOA-only support.
  - **`asset`/`extra.name` provenance**: `extra.name`/`extra.version` must match the token contract's on-chain EIP-712 domain, which is exactly the Sepolia-vs-mainnet divergence you handle in `wrangler.toml:58-59`. Trap: some tooling reads `name()` via RPC rather than trusting `extra`; inconsistent values cause facilitator rejections that look like signature bugs.
  - **Header budget**: base64 inflates payloads ~4/3; Permit2 payloads are large. Stay under Cloudflare's per-request header ceiling; define max accepted PAYMENT-SIGNATURE length and 431/400 behavior.
  - **402 body vs header**: v2 clients read the header; some intermediaries strip bodies on 402. Put everything normative in the header; treat the body as human-readable decoration only.
  - **PAYMENT-RESPONSE semantics**: only on success path, carries `transaction`, `network`, `payer`. A 5xx after payment is taken must still let the client recover the tx reference (your idempotency replay path at `lib.rs:314-325` returns only `receipt_ref` — extend it to carry settlement info).

  ## (5) Testing gaps

  - **No HTTP-level tests at all today** — 16 unit tests in core, nothing exercising routing, headers, status codes, or the DO. Add a `wrangler dev`/`miniflare` e2e suite before Stage 1, snapshotting the current behavior as the legacy baseline.
  - **Concurrency**: two simultaneous requests with the same payment nonce must yield exactly one settle and one 200 (test against the DO, not D1). Current NonceGuard claim (`lib.rs:622-637`) is correct only because DOs are single-threaded — preserve that property in whatever replaces it.
  - **At-least-once delivery**: duplicate queue messages and duplicate client retries (same idempotency key, same nonce) — assert idempotent outcomes on both paths.
  - **Facilitator failure matrix**: timeout, 5xx, 4xx rejection, verify-ok/settle-fail, settle-ok/timeout-on-response. Each maps to a specific client-visible status and a specific ledger state — enumerate the matrix as fixtures.
  - **Malformed-input fuzzing**: non-base64, truncated, oversized headers; `amount` strings with leading zeros/huge values/non-digits; wrong `x402Version`; v1-shaped payloads on the v2 route. None of these may panic the worker.
  - **Golden vectors both directions** (TS→Rust and Rust→TS), including the domain-name divergence and `v`-normalization cases.
  - **Expiry/clock tests**: `validBefore` boundary, skewed `validAfter`, challenge TTL.
  - **CORS test** from an actual browser context (or at least assert the expose-headers on 402/200) — curl-based e2e won't catch this class.
  - **Regression on non-payment surfaces**: the trust registry (`lib.rs:248-288`), badge SVG, and embedded manifests share the worker; the v2 refactor shouldn't touch them but nothing currently tests them either.

  Bottom line: the staging is sensible and the vendored-spec + cross-implementation-vector approach is the right spine. The two decisions the plan currently dodges — settle-before-serve, and whether "legacy" lives on as a route or a flag — are the ones that determine the shape of Stage 3–4, so force them in Stage 0. And cut Permit2.

