# AGENTS.md — code402 (guidance for AI coding agents)

The settlement layer: Rust/WASM Workers with deterministic paid tools + signed receipts.
Live at https://code402.dev (Base mainnet USDC via x402).

Key facts for agents:
- Paid calls: POST /v1/tools/{tool}/call → 402 challenge → retry with X-PAYMENT (EIP-3009)
- Receipts are secp256k1-signed; verifier address in wrangler.toml [vars]
- Secrets live ONLY in wrangler secret store (COMPANY_WALLET, RECEIPT_SIGNING_KEY, RPC_*)
- Rust workspace: crates/core (payment/receipt/validate — 16 tests), crates/edge (worker)

Rules: integer money; determinism over fluency; never commit keys (history is forever).
