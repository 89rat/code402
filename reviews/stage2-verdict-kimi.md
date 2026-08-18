• I'll audit the Stage 2 crypto conformance. Starting with the core verify module, error taxonomy, and the vendored spec.

• Now the spec's verification logic, §9 taxonomy, and the supporting crypto modules.

• Now the test fixtures, generators, and fuzz harness.

• Let me check the gen package wiring, the Stage-1 gate tests for validAfter, and verify the claimed green test run.

• All 52 tests confirmed green (30+3+19). Let me verify the differential fuzz claim by actually running it.

• Audit complete — I read the verify module, taxonomy, both vector directions, the fuzz harness, the vendored spec, and re-ran the suite (`cargo test -p m2m-core`: 30+3+19 = 52 green) and the differential harness live (`N=200 semantic_fail=0 byte_diverge=200 rust_err=0`). Verdicts below.

  ## 1. Verify module — OK

  - Domain construction is correct per EIP-712 + `scheme_exact_evm.md:69-73`: typehash string `EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)` (`eip712.rs:6`), name/version hashed, chainId as uint256 BE, contract as left-padded word; digest `\x19\x01‖ds‖sh` (`eip712.rs:15-19`); struct type string matches USDC's `TransferWithAuthorization` (`erc3009.rs:8`). Sources are right: name/version from `extra`, chainId from CAIP-2 (`x402v2_verify.rs:51-57`), verifyingContract = `asset` — all from the issued requirement, never the echo. Confirmed against viem in both directions.
  - No forged-LocalPass path: `LocalPass` requires ecrecover == `authorization.from` under our domain (`x402v2_verify.rs:141-145`). An attacker validly signing with their own key passes, but that is a genuine signature from the declared `from` — the prefilter never claims more (G4 comment is honest about this).
  - No false-LocalReject beyond the documented 1271 trade-off: v ∈ {0,1,27,28} all accepted (`eip712.rs:24-28`); high-s rejection is safe because OZ ECDSA (USDC's verifier) rejects s > n/2 on-chain, so no settleable payload is ever high-s.
  - EIP-2 check semantics correct: `k256::ecdsa::Signature::from_slice` rejects r/s = 0 or ≥ n, and `normalize_s()` returns `Some` iff s was in the upper half (`x402v2_verify.rs:136-140`). The malleable twin (which recovers to the *same* address with flipped v) is rejected pre-recovery — verified by the `high_s_malleable` fixture.

  ## 2. Prefilter classification — ISSUE

  - **Quota leak:** any hex string longer than 65 bytes is `PassThrough` (`x402v2_verify.rs:120-122`) with no check for the EIP-6492 magic suffix. Random 200-hex-char garbage spends a facilitator /verify call — exactly what the prefilter exists to prevent. The gate has the same hole (`x402v2.rs:562`: any even-length hex ≥130 passes). The 32-byte magic `0x6492…6492` is locally checkable, and the code's own PassThrough note already accepts classification strictness for non-standard smart-wallet encodings.
  - Reverse direction (LocalReject when it should pass through): none found beyond the documented 1271 dummy-65-byte trade-off.
  - Minor: no max signature length below the 24 KB header cap — a ~23 KB "signature" passes through to the facilitator.

  ## 3. Taxonomy — ISSUE

  - §9 completeness: 16/16 codes, verbatim vs spec lines 581-596 ✓; exhaustiveness compile-enforced ✓.
  - **Bug A — validAfter mis-mapped:** the not-yet-valid check reuses `ValidBeforeMargin` (`x402v2.rs:571-574`) → maps to `invalid_exact_evm_payload_authorization_valid_before`, but §9:582 assigns not-yet-valid to `invalid_exact_evm_payload_authorization_valid_after`. `Taxonomy::InvalidValidAfter` is unreachable, and the gate test codifies the wrong variant (`x402v2_vectors.rs:304`).
  - **Bug B — recipient mismatch mis-mapped:** `auth.to != pay_to` returns `BadAddress` (`x402v2.rs:583-585`) → `invalid_payload`, while §9:586 has `invalid_exact_evm_payload_recipient_mismatch` — that variant is also unreachable. Similarly the prefilter's signer mismatch (`x402v2_verify.rs:148-151`) → `BadAddress` → `invalid_payload`, but facilitator semantics for a signature recovering to the wrong address is `invalid_exact_evm_payload_signature` (§9:585). Plain unparseable `BadAddress` → `InvalidPayload` is sensible ✓.
  - `settlement_pending` non-terminal handling is consistent: `SettleResponse::validate` enforces non-empty `transaction` when `errorReason == "settlement_pending"` (`x402v2.rs:433-437`) and `network` is always CAIP-2-validated — matching §9:596 and §5.3.2. Retry/reconcile behavior is a Stage-4 caller concern; nothing contradictory here.

  ## 4. Vector coverage — OK (one gap)

  Against the Stage-1 list: domain binding ✓ (`wrong_chain`, `wrong_token`, `base_usdcoin_pass` + the `domain_divergence_is_real` sanity test); recovers-to-from ✓ (`wrong_signer`); low-s/malleability ✓ (`high_s_malleable`); wrong-chain ✓; 6492-wrapped-valid ✓ (`envelope_6492` — classification-level, which is all a local prefilter can assert; real 6492 verification is the facilitator's). Gap: **no explicit invalid-v reject fixture** — v ∈ {2, 37, …} rejection is only incidentally covered by `garbage_sig`'s v=0xab. validAfter-future living in the Stage-1 gate tests is acceptable (it's a timestamp compare, not crypto) — no need to move it — but see Bug A: the test asserts the wrong error variant.

  ## 5. Differential finding — OK (verified)

  I re-ran the harness; the divergence detail confirms it is *only* `extra` key order: Rust emits `assetTransferMethod,name,paymentFlow,version` (alphabetical — serde_json default `Map` is BTreeMap, no `preserve_order` feature in the workspace), the SDK emits insertion order. The G6 reasoning is sound: the MAC (plan-rev3:54-55) stamps our canonical serialization, which we control at both stamp and verify time, so client echo byte-order cannot matter. Nowhere does extra byte-order matter: the facilitator request forwards our issued requirement, not the echo (`x402v2.rs:376-379`); Settle/VerifyResponse carry no order-sensitive maps we re-stamp; EIP-712 signs typed fields, not JSON bytes. Caveat: G6 is not yet implemented (launch-checklist #1, Stage 3) — the Stage-3 note should explicitly forbid MAC-over-raw-wire-bytes.

  ## 6. Fuzz harness — ISSUE (readiness)

  - **Not CI-wired:** no `.github/workflows/` exists; `tests/fuzz/run.sh` is manual-only despite its own "CI: schedule nightly" comment. It also requires `npm ci` in `tests/vectors/gen`.
  - **Pin drift:** `package.json` uses `^2.22.0` while SPEC-VERSION pins `ts_sdk_ver = 2.22.0` and rule 1 demands conformance read SPEC-VERSION and fail on drift — `differential.mjs` never reads it (only the Rust hash test checks the spec files).
  - Corpus: seeded xorshift, deterministic ✓; PaymentRequired-only; decent field spread but `extensions` always `{}` and `extra` keys fixed.
  - Missing: malformed-input fuzz (corrupt/non-canonical b64, wrong version, missing fields, scalar `extra` — only valid envelopes are fed); PaymentPayload/SettleResponse differential; payload-signature-direction fuzz (random viem-signed auths must LocalPass, byte-mutated sigs must never LocalPass); the reverse direction (`gen_rust_vectors` + `verify-rust-vectors.mjs`) is not wired into `run.sh`.

  ## Required corrections

  1. **6492 magic check** (Q2): in `x402v2_verify.rs` (and the gate's sig-shape check), require >65-byte signatures to end with the 32-byte `0x6492…6492` magic, else `LocalReject(BadSignature)`. Add a reject fixture (long hex without magic); keep `envelope_6492` as pass-through.
  2. **validAfter variant** (Q3-A): add `X402Error::ValidAfterFuture(u64, u64)` → `Taxonomy::InvalidValidAfter`; use it at `x402v2.rs:571-574`; update `x402v2_vectors.rs:297-306` to assert it.
  3. **Recipient/signer mapping** (Q3-B): add a dedicated recipient-mismatch error → `InvalidRecipientMismatch` for `auth.to != payTo`; map the prefilter's signer mismatch to `InvalidSignature`. This makes all 16 §9 variants reachable and matches facilitator semantics.
  4. **CI + pin enforcement** (Q6): add a nightly workflow (build `codec_roundtrip`, `npm ci`, `bash tests/fuzz/run.sh`, plus reverse-direction gen+verify); pin `@x402/fetch` exactly and have the harness parse `ts_sdk_ver` from SPEC-VERSION and fail on drift.

  ## Nice-to-haves

  - Explicit invalid-v fixture (valid signature, v=2 → `local_reject`).
  - Malformed-input differential corpus; PaymentPayload/SettleResponse differential coverage.
  - Validate facilitator-supplied `errorReason`/`invalidReason` against the §9 strings in `SettleResponse::validate`/a VerifyResponse validator (any string currently passes).
  - Cap pass-through signature length (e.g. 1 KB) well below the 24 KB header cap.
  - Remove dead code in `expiry_boundary_is_exact` (`x402v2_crypto_vectors.rs:120` unused `now`; line 132 discarded struct construction).
  - Stage-3 note: G6 MAC must be computed over our canonical serialization, never raw echoed header bytes.

