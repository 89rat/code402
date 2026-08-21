# Adversarial Review: XDR-1 Receipt Spec (v0.1)

**Date:** 2026-08-17 · **Reviewer:** fresh-context independent agent (no shared context with the author)
**Method:** recomputed all digests with keccak256 + secp256k1 recovery in Python; queried Base mainnet RPC directly.

## What was verified independently

- §7 test vector: all three digests (`input_hash`, `output_hash`, `commitment`) reproduce **exactly** under the §4 algorithm.
- §3 example: commitment recomputes byte-exact from the receipt fields; the 65-byte signature recovers to the claimed signer `0x12138883…072e`; `v=0`, low-s.
- Settlement tx `0xc647…672d` is real: status `0x1`, block `0x2fc0ca2`, USDC contract `0x833589…2913`, exactly 5000 minor units (0.005 USDC), recipient `0xdcd0fe97…fdcf`.

The usual "spec was written by an LLM and the vectors are fabricated" attack surface is clean. The problems were in design and wording.

## Findings (v0.1) — all addressed in v0.2

1. **MAJOR — §5's core justification was false: the commitment was not domain-separated.** Nothing in the v0 §4 byte layout tagged it as XDR-1; a raw secp256k1 prehash signature over a bare digest is replayable into any other protocol that ecrecovers bare digests. *Fixed in v0.2: `"XDR-1" || 0x00` domain prefix (§4).*
2. **MAJOR — the `settlement` block was unsigned, so payment was not bound to delivery.** Anyone holding any valid receipt could attach any real payment tx and pass the old step 5. *Fixed in v0.2: signed `payment_ref` = the 402-challenge nonce; step 5 checks the on-chain authorization nonce against it (§3, §6).*
3. **MAJOR — `canonical_json` was not canonical**, contradicting the "Deterministic" design goal ("insertion order as produced by the merchant" is implementation-defined). *Fixed in v0.2: pinned RFC 8785 (JCS) (§3).*
4. **MAJOR — the old §6 step 5 referenced a `recipient` that existed nowhere in the receipt** (in the §3 tx, USDC went to the payment address, not the signing address). Log-parsing rules were unspecified. *Fixed in v0.2: recipient defined as manifest `payment_address`; exact Transfer/EIP-3009 log matching specified (§5.3, §6 step 5).*
5. **MAJOR — the trust root for "merchant's published key" was undefined.** *Fixed in v0.2: `/.well-known/xdr-1.json` manifest, origin-scoped for v0.2; rotation/revocation marked future work (§5.3).*
6. **MINOR — §7 vector had no signature**, leaving the recovery path untestable. *Fixed: full signature vector with the public test key.*
7. **MINOR — signature malleability unspecified.** *Fixed: low-s required, high-s rejected (§5).*
8. **MINOR — `v` normalization (27/28) unspecified.** *Fixed (§5).*
9. **MINOR — "verify offline without trusting the merchant" overclaimed.** *Fixed: reworded to attestation semantics; scope-and-honesty paragraph added (§1).*
10. **NIT — erratum contradicted §3** (unprefixed vs prefixed `commitment` hex). *Fixed: §3 states hex was normalized for publication.*
11. **NIT — timestamp semantics unstated.** *Fixed: informational; no clock-skew rejection (§6).*
12. **NIT — no error taxonomy.** *Fixed: `SHAPE_INVALID` / `COMMITMENT_MISMATCH` / `SIGNER_UNTRUSTED` / `SETTLEMENT_MISMATCH` (§6).*

## Verdict on v0.1

**FIX-FIRST** — no fabricated values, but five real design holes that would have produced divergent or insecure independent implementations. All fixed in v0.2 without breaking the worked examples.
