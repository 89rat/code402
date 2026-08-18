## Audit verdicts

### 1. VERIFY MODULE — ISSUE (minor)

**Domain construction is correct for the EIP-3009 path.**

- `domain_separator_from_requirement()` in `crates/core/src/payment/x402v2_verify.rs` correctly uses:
  - `extra.name` / `extra.version` from the issued requirement
  - CAIP-2 `eip155:<id>` → EIP-712 `chainId`
  - `req.asset_addr()` → EIP-712 `verifyingContract`
- The EIP-2 high-s check is correct: `k256::ecdsa::Signature::normalize_s()` returns `Some(_)` when `s` is high, so `ecsig.normalize_s().is_some()` correctly rejects high-s before recovery.

**Two issues remain:**

1. **No `assetTransferMethod` guard.**
   `prefilter()` unconditionally builds the EIP-3009 `TransferWithAuthorization` digest. Per `scheme_exact_evm.md`, exact EVM also defines `permit2` and `erc7710`. If this prefilter is invoked for those methods, it would:
   - hash the wrong EIP-712 struct, and/or
   - LocalReject a valid non-EIP-3009 payload.
   The caller may already gate this, but `prefilter()` itself does not enforce it.

2. **`v` validity is implicit.**
   The EOA signature `v` byte is passed to `eip712::recover_address()`. No explicit check is visible in `x402v2_verify.rs`. If `recover_address()` accepts recovery ids outside Ethereum’s 27/28 convention, an invalid-`v` signature might not be rejected locally. Add explicit `v == 27 || v == 28` or equivalent before recovery.

---

### 2. PREFILTER CLASSIFICATION — ISSUE

The local-reject/pass-through boundary spends facilitator quota on locally detectable garbage:

```rust
if body.len() > EOA_SIG_HEX {
    return Ok(VerifyOutcome::PassThrough);
}
```

Any hex signature longer than 65 bytes passes through, even if it is not a well-formed EIP-6492 envelope. An attacker can send a plausible-looking `0x` + 66 random hex bytes and force a facilitator call. This is locally detectable if the prefilter performs even a minimal 6492 envelope shape/prefix check.

The documented 65-byte ERC-1271 trade-off is acceptable.

**Required:** Change `PassThrough` for `>65` bytes to require at least an ERC-6492 envelope structure check, or add a strict upper bound and local-reject malformed over-long signatures.

---

### 3. TAXONOMY — ISSUE

The §9 wire strings in `x402v2_errors.rs` match the vendored spec exactly.

But `map_error()` does not actually make all spec taxonomy variants reachable from internal errors.

#### Missing reachable mappings

- `Taxonomy::InvalidValidAfter` is declared and string-correct, but no `X402Error` arm maps to it. The spec requires `invalid_exact_evm_payload_authorization_valid_after` for a future `validAfter`. Unless `X402Error` has a valid-after variant not shown, this spec code can never be emitted.
- `Taxonomy::InvalidRecipientMismatch` is similarly declared, but no `X402Error` arm maps to it. A `to != payTo` mismatch should map to `invalid_exact_evm_payload_recipient_mismatch`, not fall through as `InvalidPayload`.

#### BadAddress overload

In `x402v2_verify.rs`, signer mismatch is reported as:

```rust
Err(X402Error::BadAddress(format!(
    "signer mismatch: recovered {recovered:?} != declared {:?}",
    twa.from
)))
```

Then in `x402v2_errors.rs`:

```rust
X402Error::BadAddress(_) => Taxonomy::InvalidPayload,
```

That means a cryptographic signer mismatch is emitted as `invalid_payload` instead of the spec’s `invalid_exact_evm_payload_signature`. This is wrong for the prefilter and for any other path that uses `BadAddress` to mean “recovered address != declared from”.

**Required:** Split `BadAddress` into:
- malformed address → `InvalidPayload` / `InvalidPaymentRequirements`
- signer mismatch → `InvalidSignature`

And add reverse coverage: every `Taxonomy` variant must either be produced by at least one `X402Error` mapping or be documented as facilitator-origin.

#### settlement_pending

The taxonomy string and doc comment match §5.3.2/§9. The actual enforcement in `SettleResponse::validate()` is outside the inlined files. If it does not already reject a `settlement_pending` response with empty `transaction` or missing `network`, that is a required fix.

---

### 4. VECTOR COVERAGE — ISSUE

`validAfter-future` living in Stage-1 gate tests is acceptable. The crypto prefilter deliberately does not inspect `validAfter`; that is a structural-gate concern, not a signature-verification concern.

However, the Rust crypto vector test only enforces:

```rust
assert!(fixtures.len() >= 10);
```

and pins two specific pass fixtures via `domain_divergence_is_real`. It does not enforce the Stage-1 required vector classes. A future deletion or rename of, for example, the high-s or invalid-v fixture would not necessarily fail this suite.

**Required:** Add a manifest/enum of required fixture classes and assert presence for at least:
- domain binding / wrong-domain rejection
- recovers-to-from pass
- low-s pass / high-s reject
- invalid `v` reject
- wrong-chain reject
- 6492-wrapped valid pass-through
- signer-mismatch reject

`validAfter-future` may remain in Stage-1 gate tests.

---

### 5. DIFFERENTIAL FINDING — OK

The reasoning is sound.

JSON object key order is semantically irrelevant. If the only byte divergence is `extra` key order caused by `BTreeMap` alphabetizing versus the SDK insertion order, then semantic equality holds.

The key question is whether any cryptographic layer signs raw wire bytes. The prompt states the G6 MAC stamps the canonical serialization. If that is true:
- facilitator forwarding that re-encodes from parsed objects is safe, because canonical serialization is stable
- SettleResponse echo is safe because JSON response key order is not semantic
- the official SDK decoding Rust output and comparing sorted semantics confirms interop

Caveat: if any future MAC or signature is over the raw original header bytes rather than canonical JSON, any byte divergence would matter. That is not the case described here.

---

### 6. FUZZ HARNESS — ISSUE / NOT CI-READY

The current harness is a useful smoke test but not sufficient for CI/nightly readiness.

Problems:
- `differential.mjs` default Rust executable path is `target/debug/examples/codec_roundtrip.exe`, which is wrong on Unix.
- `N=200` with a fixed seed is small for nightly fuzzing.
- It only covers `PaymentRequired` header codec, not malformed inputs.
- It does not fuzz payment payload signatures in either direction.
- It requires Node and `@x402/core`; the provided snippets do not show hermetic dependency setup.
- Field coverage is narrow: always valid addresses, always one exact accept, always valid `eip155` networks, always `assetTransferMethod: "eip3009"`, empty `extensions`.

**Missing:**
- malformed-input fuzz: random bytes, invalid base64/hex/JSON, truncation, huge headers, bad Unicode
- payload-signature-direction fuzz: viem-signed random EIP-3009 payloads → Rust prefilter, plus mutated signatures for negative cases
- reverse direction Rust → viem at fuzz scale
- codec fuzz for `PaymentPayload`, `VerifyResponse`, `SettleResponse`

---

## Required corrections

1. **Gate prefilter by assetTransferMethod.**
   In `x402v2_verify.rs`, if `req.extra.assetTransferMethod != "eip3009"`, return `PassThrough` or ensure the caller never invokes EIP-3009 verification for `permit2`/`erc7710`.

2. **Add explicit ECDSA `v` validation.**
   Accept only Ethereum EOA recovery ids (27/28 or 0/1 depending internal convention) before calling `recover_address`.

3. **Replace blanket `>65 → PassThrough`.**
   Add minimal ERC-6492 envelope shape/prefix detection for signatures longer than 65 bytes. Malformed over-long hex should be `LocalReject`, not forwarded.

4. **Fix taxonomy reachability.**
   Add internal errors for:
   - future `validAfter` → `InvalidValidAfter`
   - recipient mismatch → `InvalidRecipientMismatch`
   - signer mismatch → `InvalidSignature`
   Do not overload `BadAddress` for signer mismatch.

5. **Enforce `SettleResponse::validate()` settlement-pending invariant.**
   Confirm and test that `settlement_pending` requires non-empty `transaction` and `network`.

6. **Enforce vector classes explicitly.**
   Replace or augment `len >= 10` with required fixture-class assertions.

7. **Make fuzz harness CI-ready.**
   Fix Rust exe path resolution, add malformed-input fuzz, add signature-direction fuzz vs viem, increase corpus size and seed control.

8. **Add reverse taxonomy coverage test.**
   Ensure every §9 code is either emittable from an internal error or documented as facilitator-origin.

---

## Nice-to-haves

- Add a maximum `signature` hex length in the structural gate so pass-through payloads cannot be arbitrarily large.
- Extend differential fuzz to `SettleResponse`, `VerifyResponse`, and `PaymentPayload`.
- Add wrong-token-domain and wrong-`extra.name`/`version` fixtures, not only wrong-chain.
- Add a conformance vector if the documented 1271 plain-65-byte trade-off ever causes a false reject in production.
- Wire a small fuzz smoke test into `cargo test` so regressions are visible without a separate nightly job.