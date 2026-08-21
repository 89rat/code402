# Proposal: Content-Addressed Delivery Binding for Offer & Receipt

**Type:** Extension amendment (backward-compatible, opt-in)
**Target:** `@x402/extensions/offer-receipt` — Offer & Receipt Extension Specification
**Author:** JUANA LIMITED (code402.dev) — x402 merchant since 2026-08, Base mainnet
**Status:** Draft for TSC discussion

---

## 1. Motivation — the dispute-resolution gap

The Offer & Receipt extension proves **that** a delivery happened: the signed
receipt carries `resourceUrl`, `payer`, `network`, `issuedAt`, and optionally
`txHash`. This closes the "was there an interaction?" question.

It does not close the question disputes are actually about: **what was
delivered.**

Concretely, today's receipt cannot distinguish between:

- a validator API returning `{"valid": true}` and the same API returning
  `{"valid": false}` — opposite answers, identical receipts;
- a data endpoint returning fresh data vs. stale data vs. an error object
  with HTTP 200;
- a correct response and a corrupted/truncated one.

For deterministic services (validators, normalizers, transforms, verifiers —
a large and growing class of x402 sellers), *the output bytes are the
product*. A receipt that doesn't bind to them supports only the weakest
dispute claim ("I paid and got nothing") and not the strong ones ("I paid and
got the wrong answer" / "this output was reproducible" / "the merchant
altered results after the fact").

This proposal adds an **optional content binding** to the receipt payload:
hashes of the exact request input and response output, canonically encoded.
Because the payload is what gets signed (EIP-712 or JWS), the binding
inherits the extension's existing signature and identity machinery with **no
new key management, no new wire fields outside the payload, and no breaking
changes**.

## 2. Proposal

Add an optional `content` object to the **receipt payload**:

```json
{
  "resourceUrl": "https://api.example.com/v1/validate",
  "payer": "0x…",
  "network": "eip155:8453",
  "issuedAt": 1786934823,
  "content": {
    "input_hash":  "0x903d7dd8de69f0a4618c92477ca60cb692fe0103aaa5fe8b3f1703914e2f67f5",
    "output_hash": "0xe0a7d443f12051fd841e5d532d4f88126687a97c25a8b0deb88632d60f61f88b",
    "encoding": "jcs",
    "algorithm": "keccak256"
  }
}
```

| Field | Requirement | Meaning |
|---|---|---|
| `input_hash` | MUST if `content` present | `keccak256(jcs(request_body))` — the exact input the merchant processed |
| `output_hash` | MUST if `content` present | `keccak256(jcs(response_body))` — the exact output the client received |
| `encoding` | MUST | `"jcs"` (RFC 8785). Pinned, not negotiated — canonicalization agility is how receipt ecosystems fracture |
| `algorithm` | MUST | `"keccak256"` for v1; the field exists so a future revision can add hashes without re-versioning payloads |

Servers opt in per route, mirroring the existing `declareOfferReceiptExtension`
pattern:

```ts
declareOfferReceiptExtension({
  includeTxHash: false,
  contentBinding: true,   // new — hashes request/response bodies into the receipt
})
```

## 3. Why this design

**a. Hashes, not content.** Receipts stay small and privacy-preserving;
nothing about the input/output is revealed beyond what the parties already
hold. The client proves possession of specific bytes by revealing them;
anyone can then verify the hash.

**b. JCS pinned (RFC 8785).** "Canonical JSON" left implementation-defined
(key order, number formatting) makes hashes irreproducible across languages
and quietly voids the guarantee. JCS has mature implementations in JS/TS,
Go, Python, and Rust.

**c. Inside the signed payload, not beside it.** The binding inherits both
signature formats (EIP-712 `did:pkh`, JWS `did:web`) and all existing key
management — including the extension's signer-authorization and rotation
story. An unsigned sidecar field would be detachable and worthless.

**d. Optional, per route.** Non-deterministic or streaming services simply
don't enable it. No existing receipts change meaning.

## 4. Verification additions

Verifiers that encounter `content` perform, after the existing
`verifyReceiptSignature*` steps:

1. `algorithm` == `"keccak256"` and `encoding` == `"jcs"`, else
   `CONTENT_UNSUPPORTED`.
2. If the verifier holds the claimed input: JCS-encode, hash, compare to
   `input_hash` — mismatch ⇒ `CONTENT_INPUT_MISMATCH`.
3. If it holds the claimed output: likewise for `output_hash` — mismatch ⇒
   `CONTENT_OUTPUT_MISMATCH`.

A receipt that passes proves the signer attested to *those exact bytes*.
For deterministic services, a third party can go further: re-execute the
published algorithm on the input and check the output hash — turning
"the merchant attested" into "the merchant was *right*," without trusting
anyone.

## 5. Test vectors

Computed with a production Rust implementation (alloy-primitives keccak256,
k256 secp256k1) running behind code402.dev:

```
input        = {"company_number":"12345678"}
output       = {"valid":true}

input_hash   = 0x903d7dd8de69f0a4618c92477ca60cb692fe0103aaa5fe8b3f1703914e2f67f5
output_hash  = 0xe0a7d443f12051fd841e5d532d4f88126687a97c25a8b0deb88632d60f61f88b
```

(JCS and compact serde encodings coincide for these flat-ASCII documents;
the vectors are reproducible with any RFC 8785 encoder. A fuller vector set —
nested objects, non-ASCII, float edge cases — ships with the implementation PR.)

## 6. Production evidence

This binding has been live since 2026-08-15 on code402.dev (Cloudflare
Workers, Base mainnet), where every paid call returns a receipt carrying
input/output hashes. A real settled example is publicly auditable:
tx `0xc6478aea46f82fb9bde295e052c5c26e42e4c80ceb6f44db35a4896cd2c7672d`
(Base, 0.005 USDC). The format also survived an independent adversarial
review (fresh-context reviewer recomputed all vectors and the on-chain
settlement; findings fixed before publication) — review available on request.

## 7. What we are offering

- A PR to `@x402/extensions` implementing `contentBinding` for both EIP-712
  and JWS paths, with the test-vector suite, behind the opt-in flag shown in §2.
- Client-side verification helpers (`verifyReceiptContent(receipt, { input,
  output })`) matching the existing `verifyReceiptMatchesOffer` ergonomics.
- Documentation updates to the extension page, including a "deterministic
  services" section.

No governance changes, no new dependencies, no breaking behavior.

## 8. Open questions for the TSC

1. Should offers gain an optional `expected_output_schema` commitment so
   buyers can dispute "wrong *shape*" without revealing content? (We lean no
   for v1 — hash binding covers the strong cases without the complexity.)
2. Is there appetite for an optional `payment_ref` field binding the receipt
   to the specific 402-challenge nonce, hardening the receipt↔settlement
   link for audit trails? We have this running in production and can
   contribute it separately or fold it in here.
