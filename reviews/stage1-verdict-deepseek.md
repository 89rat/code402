### Verdict per question

1. **TYPE FIDELITY — ISSUE**
   The field order and renames mostly line up with the §5.1.2/§5.2.2/§5.3.2/§5.4.2 tables, but the validators are narrower than the spec types, and some wire types are looser than the spec says.

   Specifics:
   - `PaymentRequirements.validate_spec()` in `x402v2.rs` forces `asset` and `payTo` to be EVM addresses via `asset_addr()`/`pay_to_addr()`. Spec §5.1.2 says `asset` may be a token contract address **or ISO 4217 currency code**, and `payTo` may be a wallet address **or role constant like `"merchant"`**. So `PaymentRequired::validate()` is not a true spec-level validator and would reject otherwise spec-conformant objects.
   - `PaymentRequired::validate()` only validates `self.accepts[0]`, not every element of `accepts[]`. §5.1.2 defines `accepts` as an array of requirement objects; a missing/structurally invalid second entry should not pass spec validation.
   - `PaymentRequirements.extra` is typed `Option<serde_json::Value>`, but the spec table says the wire type is `object`. Serde will happily accept a JSON string/number there.
   - `ExtensionData.info` and `ExtensionData.schema` are `serde_json::Value`, but the spec says both are `object`. Invalid scalar values are accepted by deserialization and not rejected by `validate()`.
   - `ExactEvmPayload` only models the EIP-3009 shape (`signature` + `authorization`). The exact EVM scheme in `scheme_exact_evm.md` also defines Permit2 (`permit2Authorization`) and ERC-7710. Stage 3 will need those variants.
   - `ResourceInfo.validate()` checks `iconUrl` only by `starts_with("https://")` or `starts_with("http://")`, not that it is an absolute URL. Spec §5.1.2 says absolute `https`/`http` URL.
   - `SettleResponse::validate()` does not enforce that `errorReason` is omitted when `success == true`, and does not validate `payer` as an address or `amount` as a decimal string.

2. **CODEC — OK**
   `decode_b64_json()` uses RFC 4648 standard base64 (`base64::engine::general_purpose::STANDARD`) and enforces canonical form by requiring `B64.encode(B64.decode(s)) == s`. That correctly rejects URL-safe alphabet, non-padded values, embedded whitespace, and non-canonical trailing bits.

   The 24,000 base64-character cap is implementation policy, not a spec limit. It sits below the stated ~32KB Cloudflare header total, so it is reasonable as DoS protection, though it can reject large but spec-valid payloads. That is acceptable if intentional.

3. **STRUCTURAL GATE — ISSUE**
   The gate is not complete enough to justify its “everything cheap and local that must hold BEFORE any facilitator call” comment.

   Specifics:
   - **`validAfter` is never checked.** `Authorization::valid_after_unix()` exists but is never called in `structural_gate()`. A payload whose authorization is not yet valid can pass the gate as long as `validBefore` satisfies the margin.
   - **`authorization.from` is never validated.** `from_addr()` exists but is not called. An invalid `from` address passes the structural gate.
   - **Echo comparison omits `extra`.** The function compares `scheme`, `network`, `amount`, `asset`, `payTo`, and `maxTimeoutSeconds`, but not `accepted.extra`. The comment says `extra` is HMAC-verified upstream, but the function itself does not enforce that; if the caller has not already verified a stamp, a client can tamper with `extra` and pass the gate.
   - **`0x` prefix not enforced for nonce or signature.** `nonce_bytes()` optionally strips `0x` and `structural_gate` strips `0x` from the signature, so a nonce/signature without `0x` but with the right hex length can pass. The code docs and spec examples use `0x`.
   - **Resource URL check can fail open.** If `ctx.route_url` is `None`, no resource URL binding occurs. If `ctx.route_url` is required in practice, it should not be optional at this layer.
   - **`p.resource.validate()` is not called.** If `resource` is present with invalid `serviceName`, `tags`, or `iconUrl`, the gate only checks `url`.
   - **EIP-6492 pass-through claim is mostly okay for Stage 1**, since `>=130` hex chars allows longer smart-account signatures. But exactly-65-byte ERC-1271/EIP-6492 signatures are indistinguishable from plain EOAs here; Stage 2 must not assume all 65-byte signatures are EOAs.
   - No panic/crash is obvious in the visible code.

4. **VECTORS — ISSUE**
   The positive golden vectors match the spec examples in field values, with `settle-response.json` intentionally canonicalized to table/struct field order.

   Issues:
   - The visible negative vector set is incomplete. Missing negatives include:
     - `validAfter` in the future.
     - invalid `authorization.from`.
     - `auth.to != expected.payTo`.
     - malformed `value`/`validAfter`/`validBefore`.
     - missing `0x` on nonce/signature.
     - resource URL mismatch.
     - `accepted.extra` tampering.
     - non-canonical base64, URL-safe base64, missing padding, oversized header.
     - invalid CAIP-2 network.
   - `gate_rejects_amount_tamper` contains a dead `tampered` variable and actually modifies `payload.authorization.value`, not the echoed `accepted.amount`. It tests a real tamper class, but the test name/intent is muddled.
   - `read_vec()` first builds a path through `specs/x402/../../tests/vectors`, which resolves to `repo/tests/vectors`, not `crates/core/tests/vectors`. It works only because of the fallback branch.
   - The inlined test file is truncated, so the full negative suite and any SPEC-VERSION drift test cannot be fully assessed from the visible portion.

5. **validate_spec vs validate_for_issue split — ISSUE**
   The intent is right, but the implementation has two problems.

   - **Correct part:** `validate_spec()` deliberately does not enforce reserved `extra.assetTransferMethod`/`extra.paymentFlow`; this matches §6.1 omission-default semantics. The vector test correctly asserts that the spec example is spec-valid but rejected by `validate_for_issue()` under the stricter issuance policy.
   - **Problem 1:** `validate_spec()` is not actually spec-level. It enforces `scheme == "exact"`, parses `asset`/`payTo` as EVM addresses, and only checks the first `accepts[]` entry. That contradicts the §6.1/default-tolerant framing and the `PaymentRequired::validate()` doc comment.
   - **Problem 2:** `validate_issued()` enforces the reserved keys but misses required exact/eip3009-specific keys. `scheme_exact_evm.md` says `extra.name` and `extra.version` are **required** for EIP-3009, because clients need them for EIP-712 domain construction. An issued `PaymentRequired` with only `assetTransferMethod` and `paymentFlow` would pass `validate_for_issue()` but violate the exact/EVM scheme.

6. **SPEC-VERSION drift — UNKNOWN/ISSUE from inlined materials**
   The visible code does not include the `SPEC-VERSION` file contents or an explicit SPEC-VERSION drift test body. The test file comment claims “offline SPEC-VERSION drift detection,” but it is not shown in the inlined excerpt.

   Potential holes to verify:
   - Hash pins are only effective if the drift test covers **all** pinned spec files and vectors, not just `*.json` or just one markdown file.
   - If the pin file itself is editable without a separate commit-consistency check, the drift test can be trivially updated.
   - The vector-loader fallback can mask a missing/misplaced vector path.
   - The actual tested files must be the same paths listed in `SPEC-VERSION`.

### Required corrections

1. **Split spec-structural validation from exact/EVM validation.**
   `PaymentRequirements::validate_spec()` and `PaymentRequired::validate()` must not reject ISO-4217 `asset` values or role-constant `payTo` values. EVM address parsing belongs in an exact/EVM-specific validator, not the generic spec validator. Validate all `accepts[]` entries, not only `accepts[0]`.

2. **Strict-object wire types for `extra`, `info`, and `schema`.**
   Deserialization/validation should reject `extra` values that are not JSON objects, and `Extensions` entries whose `info`/`schema` are not JSON objects.

3. **Complete exact/eip3009 issuance validation.**
   In `validate_issued()`, require `extra.name` and `extra.version` for EIP-3009, in addition to `assetTransferMethod == "eip3009"` and `paymentFlow == "upfront"`. Validate EVM `asset`/`payTo` addresses in this exact path, not in the generic spec validator.

4. **Structural gate must check `validAfter <= now`.**
   Call and enforce `authorization.valid_after_unix()` in `structural_gate()`; reject authorizations that are not yet valid.

5. **Structural gate must validate the authorization fields locally.**
   Call `from_addr()`, `to_addr()`, `value_u256()`, and `valid_after_unix()` before the facilitator path, not only `nonce_bytes()` and `valid_before_unix()`.

6. **Enforce `0x` prefix and exact hex lengths for nonce and signature.**
   This matches the code’s own documented forms and the spec examples. Do not silently accept missing-`0x` values.

7. **Make `accepted.extra` echo comparison explicit or cryptographically preconditioned.**
   Either compare the full `extra` object field-by-field against the issued requirement, or require/verify an upstream HMAC/authenticated stamp before `structural_gate()` runs. Do not leave it as an unenforced comment.

8. **Make resource URL checking fail closed.**
   If `ctx.route_url` is needed for the resource binding, require it and reject `None`; call `p.resource.validate()` when `resource` is present.

9. **Tighten `SettleResponse::validate()`.**
   Reject `errorReason` when `success == true`; validate `payer` as an address and `amount` as a decimal string when present.

10. **Add missing negative vectors.**
    Cover at least `validAfter` future, invalid `from`, `to` mismatch, malformed value/timestamps, missing `0x`, base64 canonicality, oversized header, resource URL mismatch, `extra` tamper, and invalid CAIP-2.

11. **Fix `read_vec()` path handling.**
    Read directly from `CARGO_MANIFEST_DIR/tests/vectors`; do not rely on a wrong fallback-first path.

12. **Ensure SPEC-VERSION drift test is fail-closed and complete.**
    It must cover the actual spec markdown files and all vectors used by tests, and must not allow the pin file to be edited without detecting drift/commit inconsistency.

13. **Stage 3: model non-EIP-3009 exact/EVM variants.**
    Add Permit2 and ERC-7710 payload shapes, or introduce a tagged/enum representation for `payload`.

### Nice-to-haves

- Replace `iconUrl` prefix validation with a real URL parse/absolute-URL check.
- Tighten `is_caip2()` for `eip155:*` references to require a numeric chain ID.
- Introduce distinct error variants for echo mismatches; `ExactAmountMismatch` is misleading for scheme/network/asset/payTo mismatches.
- Add a maximum signature length beyond the 24KB header cap if the facilitator has stricter expectations.
- Document the chosen base64 variant explicitly in the local spec pinning/README, since `scheme_exact_evm.md`/§5-6 excerpts did not show that detail.
- Add tests for `PaymentPayload.resource` validation and `SettleResponse` error/shape negatives.