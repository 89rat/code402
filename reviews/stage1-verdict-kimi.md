• I'll audit Stage 1 against the vendored spec. First, let me size up the files.

• Now the scheme spec, transport spec, tests, vectors, and SPEC-VERSION.

• All files read. Let me verify the test suite actually passes and confirm the package layout.

• The picture is now clear — let me get the full test summary to quantify the state.

• The tree is being edited concurrently — the previously failing test now passes. Let me re-read the current gate code to audit the latest state.

• The in-flight changes got committed mid-audit. Let me confirm the current HEAD state.

• Audit complete. Important context first: **the tree changed mid-audit.** While I was working, commit `51d6850` ("stage-1 fix: EIP-6492 pass-through + echo timeout compare") landed on top of `5d988b8`, fixing two defects I had independently identified at the commit you named. I also ran the tests: at `5d988b8` they were green (42); during the transition the new 6492 regression test was briefly red (a broken odd-nibble case: `"ab".repeat(65)` = 130 nibbles, not 131); at current HEAD `51d6850` everything is green (27 unit + 16 integration in `m2m-core`). Verdicts below are against current HEAD, with the `5d988b8`-era defects marked FIXED.

  ---

  ## 1. TYPE FIDELITY — OK (with minor issues)

  Every struct matches its spec table for names, requiredness, wire types, and **field order** (struct declaration order == table order, so the serializer emits spec order):

  - `PaymentRequired` / `PaymentRequirements` / `ResourceInfo` ↔ §5.1.2 ✓ (incl. `serviceName`/`tags`/`iconUrl` constraints in `ResourceInfo::validate`, `crates/core/src/payment/x402v2.rs:108-131`)
  - `PaymentPayload` (`resource` Optional ✓) / `Authorization` (all strings incl. `validAfter`/`validBefore` ✓) / `ExactEvmPayload` ↔ §5.2.2 ✓
  - `SettleResponse` ↔ §5.3.2 ✓ (incl. the `settlement_pending` ⇒ non-empty `transaction` rule, §5.3.2 + §9, enforced at `x402v2.rs:366-370`)
  - `VerifyResponse` ↔ §5.4.2 ✓; `Extensions` = map of `{info, schema}` ✓

  Issues (all minor):

  - **1a.** `validate_spec` (`x402v2.rs:181-192`) parses `asset`/`payTo` as EVM addresses, but §5.1.2 explicitly allows ISO 4217 fiat codes and role constants (`"merchant"`). So it's exact-EVM validation, not spec-generic — fine for code402, but the name/comment overclaims. Rename or annotate.
  - **1b.** `PaymentRequired::validate` (`x402v2.rs:243`) only validates `accepts[0]`; other entries are unchecked on the decode path.
  - **1c.** `extra` and `ExtensionData.info`/`schema` are typed `serde_json::Value`; spec says `object`. `"extra": "foo"` deserializes cleanly. No exploit, but it's a fidelity gap.
  - **1d.** `x402_version: u8` vs spec `number`: `300` or `2.0` fails with `InvalidJson` instead of `WrongVersion`. Cosmetic.
  - **1e. Missing for Stage 3:** the §7.1/7.2 facilitator request envelope `{x402Version, paymentPayload, paymentRequirements}` is not modeled anywhere — Stage 3 cannot call CDP `/v2/x402/verify|settle` without it. Also unmodeled: §7.3 `SupportedResponse` (`kinds`/`extensions`/`signers`) — presumably Stage 4. §9 error strings are deliberately deferred (comment at `x402v2.rs:36`). Permit2/ERC-7710 payload shapes not modeled — acceptable, `BadScheme` declares "only exact".

  ## 2. CODEC — OK (one documentation nit)

  - 24KB cap enforced **before** decode on the b64 string, and on encode (`x402v2.rs:392-412`). Sits safely below Cloudflare's ~32KB total-header ceiling. Sound.
  - Re-encode equality is a correct canonicality check: it rejects embedded whitespace, non-zero trailing bits, missing/incorrect padding, and the URL-safe alphabet (the `STANDARD` engine also rejects `-`/`_` at decode). The spec and `http.md` only say "Base64-encoded" without naming an alphabet, but all spec examples and `@x402/fetch` 2.x (`Buffer.toString('base64')`) emit RFC 4648 standard **with padding** — so the strict check is compatible with the reference SDK.
  - Nit: non-TS clients using unpadded or URL-safe base64 (Go `RawStdEncoding`, Python `urlsafe_b64encode`) will be hard-rejected. That's a defensible strictness choice, but it's an interop decision made silently — document it in the module docs (the `NotCanonicalBase64` error text already hints at it).

  ## 3. STRUCTURAL GATE — ISSUES at 5d988b8, both FIXED in 51d6850; two residual notes

  - **3a. EIP-6492 contradiction — WAS A REAL DEFECT, NOW FIXED.** At `5d988b8` the comment said long 6492 envelopes "intentionally pass this length check as pass-through" while the code (`sig.len() != 130`) rejected them. `51d6850` changed it to `< 130 || odd || non-hex` (`x402v2.rs:479`) with regression tests. Code now matches the stated G4 design. ✓
  - **3b. Echo completeness — PARTIALLY FIXED, WITH A HARD DEPENDENCY.** At `5d988b8` the echo compare omitted `maxTimeoutSeconds` and `extra`. `51d6850` added `maxTimeoutSeconds` and deliberately excluded `extra` with the rationale that it carries the G6 HMAC stamp verified upstream (`x402v2.rs:451-454`). That is sound **only if** (i) the edge crate actually MAC-verifies `extra` before this gate runs, and (ii) Stage 3 forwards our MAC-verified `expected` — never the client's echoed `accepted` — to the facilitator. If either slips, an attacker can rewrite `extra.assetTransferMethod`/`paymentFlow` in the echo unchecked. This dependency should be recorded as a blocking requirement for the edge/Stage-3 work, not just a comment.
  - **3c. validBefore margin** — `saturating_add` is correct and fail-closed (overflow saturates → reject; >u64 timestamps fail parse → reject). ✓
  - **3d. Recipient binding / echo compares** are case-sensitive string compares (`x402v2.rs:487-493`). Fail-closed — a lowercase variant of the same address is *rejected*, never accepted — so no security hole, but a client that normalizes EIP-55 case gets a false rejection. Nice-to-have: compare parsed `Address` values.
  - **3e. resource.url check** is skip-if-absent; §5.2.2 makes `resource` Optional, so omission is legal and unbypassable-by-design — but harmless, because amount/recipient binding comes from route-derived `expected`, not the echo. ✓ Exact-string URL compare can false-reject on normalization differences (trailing slash); acceptable for now since we control both sides.
  - **3f. value == amount** string equality against our canonical issued amount is correct for `exact` (kills the legacy `>=`). ✓
  - **3g. Crash audit: clean.** `#![deny(unwrap_used, expect_used, panic)]`, every parse checked, `strip_prefix` safe, no allocation hazards under the 24KB cap. I could not construct a panic path.
  - **3h. Two residual gaps (nice-to-have):** `validAfter ≤ now` is never checked (not-yet-valid authorizations pass the gate and burn a facilitator call); and **any** ≥65-byte hex blob now passes the signature check — for sigs > 65 bytes, consider verifying the EIP-6492 magic suffix (`…6492⁶` ×3…`6492`) so garbage blobs don't get a free ride to the paid facilitator endpoint.
  - **3i.** Cosmetic: echo mismatches on `scheme`/`network`/`asset` report `ExactAmountMismatch` — will matter when the §9 taxonomy mapping lands.

  ## 4. VECTORS — OK

  - **Fidelity:** `payment-required.json` is byte-faithful to the §5.1.1 example (including `serviceName`/`tags`/`iconUrl` and `"extensions": {}`); `payment-payload.json` to §5.2.1. Both minified, field order preserved; the byte-exact roundtrip asserts (`x402v2_vectors.rs:41,64,73`) pin our serializer to spec order — strong.
  - **settle-response.json** uses the §7.2 success-example order (`success, payer, transaction, network`), not §5.3.1's (`success, transaction, network, payer`). This is the documented canonicalization and the struct reproduces it. Note §5.3.1's example would *not* roundtrip byte-identically — worth one comment line in the vector or test so a future reader doesn't "fix" it.
  - **Negatives are the right negatives:** echo tamper, amount tamper, margin violation, nonce/sig shape, numeric timestamps (v1 shape), wrong version, URL mismatch, non-canonical b64, oversize, amount syntax/overflow, `settlement_pending` rules, reserved-key issuance, 6492 pass-through (post-fix). Matches the G4/G5/G9/G10 surface.
  - **Stage 2's crypto vectors must cover:** EIP-712 domain binding (`extra.name`/`version` vs token contract, chainId derived from `accepted.network` vs signed domain), signature recovers to `authorization.from`, low-s/malleability, `v` validity, wrong-chain signature, 6492-wrapped valid signature (verify via facil), and `validAfter`-future. None of these belong in Stage 1 — just don't let Stage 2 skimp.
  - Cosmetics: `read_vec()`'s first path (`specs/x402/../../tests/vectors`) is dead; the fallback is the real one. And your "42 green" matches `5d988b8` (27+15); HEAD now has 43.

  ## 5. validate_spec vs validate_for_issue — CORRECT READING, one real omission

  The split is right. §6.1: omitted reserved keys resolve to mechanism defaults, explicit declaration is always legal, and — critically — *"When the resolved payment flow is not `authorization`, `accepts[].extra.paymentFlow` MUST be present."* code402 resolves to `upfront` (non-default), so `validate_for_issue` requiring `paymentFlow: "upfront"` is spec-mandated, not just house policy. Requiring `assetTransferMethod: "eip3009"` is permitted (scheme spec: if present, MUST be `"eip3009"`). The roundtrip test asserting the spec example **fails** `validate_for_issue` (`x402v2_vectors.rs:39`) locks the distinction in. ✓

  - **ISSUE:** `validate_issued` (`x402v2.rs:196-213`) does **not** require `extra.name` / `extra.version`. `scheme_exact_evm.md` §1 (lines 71-73): for `eip3009`, `extra.name` and `extra.version` are **required** — they're the EIP-712 domain parameters; without them a client cannot construct a valid signature. This is blocking for Stage 2.

  ## 6. SPEC-VERSION DRIFT TEST — OK, two holes worth closing

  - I recomputed all five SHA-256 pins — **all match** the files on disk. The test parses every `sha256_*` key, asserts content equality, and asserts `checked == 5` (`x402v2_vectors.rs:264-281`), so silently dropping a pin fails. Good design.
  - **Hole A:** no detection of *new unpinned files* under `specs/x402/` — a re-vendor adding e.g. `transports/` or `extensions/` passes silently. Nice-to-have: assert the directory file set equals the pinned set.
  - **Hole B:** `spec_commit` is informational only — nothing ties content hashes to that upstream commit (offline, by design; rule 2's reviews/ note is the process control). Acceptable, but acknowledge it's trust-on-bump.
  - **Hole C (minor):** the golden vectors aren't pinned to the spec text — someone editing spec + pins together leaves stale vectors green. Nice-to-have: a test asserting each vector's minified JSON appears in/derives from the pinned `.md`.
  - `ts_sdk_ver = 2.22.0` has no test consumer yet (Stage 2's bidirectional vectors) — fine, noted.

  ---

  ## Required corrections

  1. **(FIXED in `51d6850`)** 6492 pass-through contradiction in `structural_gate` — keep the regression test; it was briefly red mid-fix due to a broken odd-length case (`"ab".repeat(65)` = 130 nibbles, not 131).
  2. **(FIXED in `51d6850`)** Add `maxTimeoutSeconds` to the echo comparison.
  3. **`validate_issued` must require `extra.name` and `extra.version`** (scheme_exact_evm.md lines 71-73) — blocks Stage 2 signing.
  4. **Model the §7.1/7.2 facilitator request envelope** `{x402Version, paymentPayload, paymentRequirements}` before Stage 3.
  5. **Record the hard dependency** behind the gate's `extra` exclusion: G6 MAC verification upstream in the edge crate, and Stage 3 forwarding `expected` (never the client echo) to the facilitator. Make it a launch checklist item, not just a code comment.

  ## Nice-to-haves

  6. Rename `validate_spec` → `validate_exact_evm` (or document EVM-only scope); it rejects spec-legal ISO 4217 assets and `"merchant"` payTo.
  7. Compare addresses semantically (parsed `Address`) in recipient binding and echo compare.
  8. Add `validAfter ≤ now` to the gate; reject >65-byte signatures lacking the EIP-6492 magic suffix.
  9. Type `extra` / `ExtensionData.{info,schema}` as JSON objects, not `Value`; validate all `accepts[]` entries, not just `[0]`.
  10. Drift test: fail on unpinned files in `specs/x402/`; consider pinning vectors to spec text.
  11. `x402_version: u8` → wider int so out-of-range versions yield `WrongVersion`, not `InvalidJson`; stop reporting `ExactAmountMismatch` for scheme/network echo mismatches (matters at §9 taxonomy time).
  12. Clean up `read_vec()`'s dead first path; add a one-line comment on the settle vector's field-order canonicalization.

  No files were edited.

