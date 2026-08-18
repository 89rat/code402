# Launch checklist — blocking dependencies before each stage ships

Stage-1 audit (Kimi 2026-08-19) promoted these from code comments to
first-class launch blockers. Each must be verifiably true before the stage
gate opens.

## Before Stage 3 (edge wire flow) ships
1. **G6 MAC verification is UPSTREAM of `structural_gate`.** The gate
   deliberately does NOT compare `accepted.extra` against the issued
   requirement (`x402v2.rs` echo-compare). This is sound ONLY if the edge
   crate MAC-verifies the echoed `extensions`/`extra` against the issued
   HMAC stamp BEFORE the gate runs. If this slips, an attacker can rewrite
   `extra.assetTransferMethod`/`paymentFlow`/`name`/`version` unchecked.
2. **Stage 3 forwards OUR requirement — never the client echo — to the
   facilitator.** `FacilitatorRequest.payment_requirements` must be the
   MAC-verified issued requirement (route-derived pricing), not
   `payload.accepted`. Otherwise the echo-compare exclusion becomes a
   settlement-integrity hole.
3. `PAYMENT-REQUIRED` header emitted only via `encode_payment_required`
   (which enforces `validate_for_issue`: reserved keys + EIP-712 domain
   name/version present).

## Before Stage 4 (facilitator settlement) ships
4. CDP auth mechanism confirmed from the live API reference (JWT? header?)
   before choosing the WASM signing crate (`ed25519-dalek`/`p256`; no ring).
5. Failure-matrix fixtures green: timeout / 5xx / 4xx / verify-ok-settle-fail /
   settle-ok-response-lost (`settlement_pending` path) — each mapping to a
   defined ledger state.
6. Already-used-on-settle-retry maps to success ONLY with a matching D1
   settlements row; otherwise `receipt_pending` + cron backfill.

## Before Stage 5 (mainnet flip)
7. Pre-cut obligations in `reviews/cdp-findings.md` discharged (reconcile the
   prod PENDING row on-chain; re-run traffic query).
8. Mainnet enable = KV `ops:x402v2_network` write only (`[vars]` defaults
   stay staging); retreat = KV kill-switch, no redeploy.

## Stage 3 additions (from Stage-2 audit, Kimi Q5 caveat)
9. **G6 MAC is computed over OUR canonical serialization** (serde struct
   order at stamp-time and verify-time), never over raw echoed header bytes
   — `extra` key order differs between implementations (BTreeMap alphabetize
   vs SDK insertion order; Stage-2 differential finding) and must never
   enter any MAC or signature input.
