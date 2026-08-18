# CHANGELOG

## 0.1.0 — Phase 1 (m2m-core) — 2026-08-12

### Acceptance
- `cargo test -p m2m-core` → **9 passed; 0 failed** (doc-tests 0).

### Deviations from directive (rule 6 decisions — conservative, deterministic)
1. **`alloy-primitives` `serde` feature enabled** (workspace deps). Required:
   `TransferWithAuthorization` / `PaymentVoucher` derive `Serialize`/`Deserialize`
   over `Address`/`U256`/`B256`, whose serde impls are feature-gated in
   alloy-primitives 0.8. No dependency added beyond the listed set.
2. **`into_word()` call sites take `.as_slice()`** (eip712.rs, erc3009.rs).
   alloy-primitives 0.8 returns `FixedBytes<32>` from `Address::into_word()`;
   `Vec::extend_from_slice` requires `&[u8]`. No semantic change — identical
   bytes hashed.
3. **Test `roundtrip` uses `SigningKey::from_slice(&bytes)`** instead of
   `from_bytes(&vec.into())`. k256 0.13 `from_bytes` takes
   `&FieldBytes` (`GenericArray<u8, U32>`); `Vec<u8>` does not implement
   `Into<GenericArray<..>>`. Same key material, same assertion.
4. **VAT modulus-97 semantics fixed**: `canonicalise_vat` strips an optional
   case-insensitive `GB` prefix, requires exactly 9 ASCII digits, then accepts
   if the standard check pair (`97 - sum%97`) OR the alternative pair
   (`+55`, only when ≤ 99) matches the final two digits. Test vectors are
   computed from the algorithm, not copied from unverifiable external lists.
5. **`Receipt::commitment()` length-prefixes** the variable-length string
   fields (request_id, tool, tool_version) before concatenation, preventing
   field-boundary ambiguity in the commitment preimage. Hashes fixed-width,
   timestamp big-endian u64.
6. **Package names**: core = `m2m-core`, edge = `m2m-edge` (Acceptance 1
   requires `cargo test -p m2m-core`).
7. **`m2m-edge` is a stub cdylib/rlib crate** pending Phase 2 so the
   workspace resolves; it carries no Cloudflare dependencies yet, preserving
   the "core compiles with zero Cloudflare deps" invariant.

### Environment note
- Rust toolchain at `%USERPROFILE%\.cargo\bin` (not on PATH by default in
  this shell). No Cloudflare/wrangler credentials available in this session;
  Phases requiring deploy (Acceptance 2 staging flow) are blocked on that.

## 0.1.0 — Phase 2+3 (m2m-edge, wrangler.toml, migrations) — 2026-08-12

### Acceptance
- `cargo check -p m2m-edge --target wasm32-unknown-unknown` → **0 errors, 0 warnings**.
- Acceptance 2 (staging scripted flow) is BLOCKED: requires `wrangler` auth,
  resource IDs (KV/D1/R2/Queue) and the four secrets. wrangler.toml ships with
  `REPLACE_*` placeholders.

### Decisions (rule 6)
8. **workers-rs anchored at `worker = "0.4"`** (crates.io latest confirmed
   2026-08-12), features `d1` + `queue` enabled; `wasm-bindgen` and
   `async-trait` added — both required by the `#[durable_object]` macro
   expansion in worker-macros 0.4.
9. **`getrandom` "js" feature unified** (transitive via k256) — required for
   wasm32-unknown-unknown. Feature-only change.
10. **`X-PAYMENT` header carries the raw JSON PaymentVoucher** (staging).
    The x402 reference transport is base64(JSON); adding a base64 crate would
    exceed the approved dependency set, so the deterministic choice is raw
    JSON. Revisit at production hardening.
11. **request_id = cf-ray header** (globally unique per request), fallback
    `req-{epoch_ms}`.
12. **Idempotency wired to the D1 `idempotency` table**: replayed key returns
    `{idempotent_replay, receipt_ref}`; successful calls `INSERT OR IGNORE`.
13. **Queue consumer ack/retry semantics**: message acked only when the
    facilitator tx receipt shows status 0x1; otherwise retried (max_retries 5).
    Messages without a tx_hash are retried, not acked.
14. **Treasury sweep is an operator action**: the hourly cron only records the
    unsettled count to KV (`ops:pending_settlement`); no treasury keys live in
    the Worker beyond RECEIPT_SIGNING_KEY. Conservative reading of
    "sweep-policy check".
15. **Staging env targets Base Sepolia** (chain 84532, USDC
    0x036CbD53842c5426634e7929541eC2318f3dCF7e); production = Base mainnet
    (8453, 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913) per §5.

## 0.1.0 — Phase 4+5 (site + machine discovery) — 2026-08-12

### Acceptance
- `npm run build` (tsc + vite) → green. dist: 1.5 kB HTML, 13.6 kB CSS gz, 90.9 kB JS gz.
- Obfuscation grep-verify over public copy (site/src, site/public, index.html,
  dist/index.html, dist/llms.txt, dist/.well-known/*): **zero forbidden terms**.
  Note: the minified framework bundle contains coincidental 2-char JS
  identifiers (`d1`, `kv`) from react/react-router codegen — machine noise, not
  site copy; content files are clean.
- Fundamentals memo applied where applicable: spec/conformance section and
  self-describing versioned payloads (x402_version everywhere, manifests
  versioned), docs lead with the reconciliation wedge. PQ crypto / mesh / CBDC
  / hardware-root items recorded as out-of-scope for this build (would violate
  the approved dependency set and the directive's architecture).

### Decisions (rule 6)
16. **Site stack**: React + TypeScript + Vite + Tailwind + shadcn/Radix +
    lucide-react, static build deployable to Pages. Design tokens per §7
    (zinc palette = exact spec hexes; cyan #06B6D4 accent, amber #F59E0B
    payment, Inter/Geist headings, JetBrains Mono code).
17. **x402.json recipient** = "RUNTIME_ENV:COMPANY_WALLET" placeholder — the
    recipient is authoritative in the live 402 challenge, never hardcoded in a
    static file (§8 says "recipient from env at runtime").
18. **Lighthouse ≥95**: not run here (no headless Chrome in this session);
    the build is a static, no-framework-blocking page with system-preconnected
    fonts — expected headroom is high, but this checkbox needs a real run
    post-deploy.

## 2026-08-17 — Verified trust badge Phase 1 + first mainnet settlement

- First REAL mainnet settlement: $0.005 self-trade, facilitator-free (own Rust
  EIP-1559 signer), tx 0xc6478aea...7672d, block 50072738. API 200 + signed receipt.
  Recorded as kind=settled, source=paid-probe, self_trade=true (never organic).
- Worker: added GET /v1/trust/{domain}, GET /v1/trust/{domain}/badge.svg,
  POST /v1/trust-ingest (Bearer TRUST_INGEST_KEY, wrangler secret).
- crawler/trust.py: computes trust records from observations.db (per-endpoint
  fidelity, days_measured, evidence root hash); ingests to worker KV.
- daily.py: pipeline now discover -> crawl:self -> crawl:external -> trust.
- Tooling: keygen bins payprod/settleprod; crawler/paid_call.py orchestrator.

## 2026-08-17 (b) — Storefront live + enterprise hardening

- Worker now serves the full site via [assets] (SPA); run_worker_first keeps
  /v1/*, /.well-known/*, /llms.txt dynamic. Homepage was 400 before this — fixed.
- New /trust page: live badge + trust record, first-settlement proof, Drift Wall,
  methodology, embeddable badge snippet (the viral loop).
- Zone hardening (Pro): rate limit 300 rpm/IP on /v1/* (block 60s); Cloudflare
  Managed WAF on web traffic (API paths excluded — worker does schema-first
  validation; HTML payloads would false-positive); zone-wide security headers
  (nosniff, Referrer-Policy, Permissions-Policy, HSTS 180d).
- Verified: site 200, trust JSON 200, manifest dynamic 200, paywall 402 intact,
  HTML payload not blocked.
