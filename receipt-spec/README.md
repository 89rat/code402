# XDR-1: Delivery Receipts for Machine-Paid APIs

**Status:** v0.2-draft · **Date:** 2026-08-17 · **Reference implementation:** [code402](https://code402.dev) (`m2m-core::receipt`)

---

## 1. Abstract

x402 and similar HTTP-native payment protocols answer *"did the client pay?"*
They do not answer *"did the merchant attest to what was delivered, and can a
third party verify that attestation later?"*

XDR-1 defines a **Delivery Receipt**: a small, signed, content-addressed
document issued by the merchant in the same HTTP 200 response that delivers
the paid output. It binds together the request, the tool and version, hashes
of the exact input and output, a timestamp, and the payment reference from the
402 challenge. Anyone holding the receipt can verify the merchant's
attestation offline; binding the signing key to the merchant's identity
requires the merchant manifest (§5.3).

Design goals:

- **Offline-verifiable attestation.** Steps 1–3 of §6 need only the receipt
  and the merchant manifest — no call back to the merchant.
- **Deterministic.** The commitment is byte-exact and reproducible in any
  language with keccak256 and an RFC 8785 (JCS) JSON encoder.
- **Settlement-linked.** The signed payment reference ties the receipt to one
  specific authorization, closing the loop between payment and delivery.

Scope and honesty: a receipt proves the merchant *attested* to a delivery.
It cannot prove the output was good — a malicious merchant can sign garbage.
What it makes possible is cryptographic accountability: receipts are
non-repudiable, comparable, and auditable.

---

## 2. Lifecycle

```
CHALLENGED            merchant answered 402 with price + payment terms
PENDING_SETTLEMENT    payment accepted, output delivered, receipt issued
SETTLED               settlement confirmed (on-chain receipt observed)
FAILED_SETTLEMENT     settlement reverted or expired unpaid
```

The receipt is issued at `PENDING_SETTLEMENT` — synchronously, inside the
200 response. Settlement confirmation is asynchronous and MUST NOT delay
delivery. A receipt remains cryptographically valid regardless of later
settlement state; settlement state is an accounting fact, not a property of
the signature.

---

## 3. Receipt document

```json
{
  "receipt": {
    "request_id": "a2c55a11cebfd2ba",
    "tool": "vat-mod97-check",
    "tool_version": "1.0.0",
    "input_hash": "0x6c82534c961b7974528381d7ab0279fd622dda98270fdbf9df97dd78f81c6287",
    "output_hash": "0x313cccdad4b6de7a28120d31aee6864128fc60e129d21247cdb8ecb2137aa237",
    "timestamp_unix": 1786934823
  },
  "commitment": "0xfcf1ea426d16a0713e3c29fc12259ff687f0c9741cfcc216e263cd05af76412b",
  "signature": "0x22847f1c33668e3eb0212f5bcb36a0769c7cb877226a5ce0048207284ac30f53705f5e44064b34d35138df6c2bb02e84f5aac63d361c367455c452537ee7c56400",
  "settlement": {
    "chain_id": 8453,
    "token": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
    "tx_hash": "0xc6478aea46f82fb9bde295e052c5c26e42e4c80ceb6f44db35a4896cd2c7672d",
    "amount_minor": "5000"
  }
}
```

The document above is a **real receipt**, issued by code402.dev on 2026-08-17
for a `vat-mod97-check` call paid with 0.005 USDC on Base mainnet, shown
exactly as stored (v0 fields only). Two provenance notes, stated plainly:

- It was issued under the **v0 wire format** — before `payment_ref`, the
  `spec` field, and the domain tag existed (see §9). Its commitment and
  signature are valid for the v0 construction over exactly the fields shown;
  under v0.2 rules its settlement link is informational.
- Hex fields were normalized to `0x`-prefixed form for publication (the v0
  implementation serialized `commitment` unprefixed).

The `settlement` tx is publicly auditable on Base (block `0x2fc0ca2`).
Recovered signer: `0x12138883e159622e22abcdac0e1ecc4b5c0a072e` — the code402
receipt-signing address.

### Fields

| Field | Type | Signed? | Meaning |
|---|---|---|---|
| `request_id` | string | yes | Merchant-generated unique id for the call |
| `tool` / `tool_version` | string | yes | The priced endpoint and its version |
| `input_hash` | bytes32 | yes | `keccak256(jcs(request_input))` |
| `output_hash` | bytes32 | yes | `keccak256(jcs(response_output))` |
| `timestamp_unix` | uint64 | yes | Merchant clock at issuance (informational; see §6) |
| `payment_ref` | bytes32 | yes | The `nonce` from this request's 402 challenge; binds receipt → payment authorization |
| `spec` | string | yes | `"xdr-1/0.2"` |
| `commitment` | bytes32 | — | See §4 |
| `signature` | 65 bytes | — | See §5 |
| `settlement` | object, optional | **no** | Chain reference, attached post-hoc; verified against `payment_ref` in §6 step 5 |

`jcs` is the JSON Canonicalization Scheme, RFC 8785. Pinning JCS (rather than
any implementation-defined serialization) is what makes `input_hash` and
`output_hash` reproducible by third parties.

---

## 4. Commitment construction

```
payload = for each of [request_id, tool, tool_version]:
              uint32_be(len(utf8(s))) || utf8(s)
          || input_hash    (32 raw bytes)
          || output_hash   (32 raw bytes)
          || uint64_be(timestamp_unix)
          || payment_ref   (32 raw bytes)
          || uint8(len("xdr-1/0.2")) || "xdr-1/0.2"

commitment = keccak256( "XDR-1" || 0x00 || payload )
```

- The `"XDR-1" || 0x00` prefix is the **domain separator**: an XDR-1 signature
  can never be confused with, or replayed into, any other protocol that
  ecrecovers bare 32-byte digests.
- String fields are length-prefixed (uint32 big-endian) to eliminate
  field-boundary ambiguity; hashes, the timestamp, and `payment_ref` are
  fixed-width. All multi-byte integers are big-endian.

---

## 5. Signature

- Curve: **secp256k1** (Ethereum-compatible tooling everywhere).
- The merchant signs the 32-byte commitment directly as a prehash. Because
  the commitment is domain-separated (§4), no EIP-191 prefix is needed or
  used.
- `signature` is the 65-byte recoverable signature `r || s || v`.
  - Issuers MUST emit `v ∈ {0,1}` and MUST produce **low-s** signatures
    (`s ≤ n/2`) so each commitment has exactly one valid canonical signature.
  - Verifiers MUST accept `v ∈ {0,1,27,28}` (normalizing 27/28 → 0/1) and
    MUST reject high-s signatures.

### 5.3 Merchant manifest (trust root for v0.2)

The merchant publishes its keys at its own origin:

```
GET /.well-known/xdr-1.json
{
  "signing_address": "0x12138883e159622e22abcdac0e1ecc4b5c0a072e",
  "payment_address": "0xdcd0fe977640add2dbe62ca0fb30c63f2fd9fdcf",
  "receipt_spec": "xdr-1/0.2"
}
```

v0.2 scopes trust to **origin-published keys**: the manifest must come from
the same origin that issued the receipt, over HTTPS. Registries, rotation,
and revocation are explicitly future work; verifiers SHOULD log the manifest
they relied on.

---

## 6. Verification algorithm

A verifier performs, in order, with named failure modes:

1. **Shape check** — required fields present and well-typed; `spec`
   recognized. Failure: `SHAPE_INVALID`.
2. **Commitment recompute** — rebuild §4 bytes from the signed fields;
   MUST equal `commitment`. Failure: `COMMITMENT_MISMATCH`.
3. **Signature recovery & trust** — enforce low-s, normalize `v`,
   `ecrecover` → signer address; MUST equal `signing_address` in the
   merchant's origin manifest (§5.3). Failure: `SIGNER_UNTRUSTED`.
4. *(optional)* **I/O recompute** — if the original input/output are
   available, JCS-encode and re-hash; MUST equal `input_hash`/`output_hash`.
5. *(optional)* **Settlement check** — if `settlement.tx_hash` is present,
   fetch the on-chain receipt. For EIP-3009 settlement: status MUST be `0x1`;
   the `TransferWithAuthorization` nonce MUST equal the signed `payment_ref`;
   the USDC `Transfer` log's `to` MUST equal the manifest `payment_address`;
   `value` MUST be ≥ `amount_minor`. Failure: `SETTLEMENT_MISMATCH`.

Steps 1–3 are fully offline. Step 5 is the only step requiring network, and
because `payment_ref` is signed, the payment is cryptographically bound to
this receipt — a third party cannot attach an unrelated payment transaction.

`timestamp_unix` is informational: verifiers MUST NOT reject on clock skew.

---

## 7. Test vector

```
request_id      = "req-1"
tool            = "uk-entity-validator"
tool_version    = "1.0.0"
input           = {"company_number":"12345678"}
output          = {"valid":true}
timestamp_unix  = 1700000000
payment_ref     = 0x0000000000000000000000000000000000000000000000000000000000000001
spec            = "xdr-1/0.2"

input_hash  = 0x903d7dd8de69f0a4618c92477ca60cb692fe0103aaa5fe8b3f1703914e2f67f5
output_hash = 0xe0a7d443f12051fd841e5d532d4f88126687a97c25a8b0deb88632d60f61f88b
commitment  = 0x0f2e25c4f2736bf8db95f01f99ce0593ded8a4f6b6c14ee6a697f2e3b41e89c5

test signing key (NEVER use in production):
  private_key = 0x0000000000000000000000000000000000000000000000000000000000000001
  address     = 0x7e5f4552091a69125d5dfcb7b8c2659029395bdf
signature   = 0x5000b8e2a3cfa0f57cdc907422a4fa7025ef4948066200561cf76ffd8a5506400f7e26497ee2f98ccee82d095bf38e2013a8a6dded41210de0a59c809545ef3500
```

All digests and the signature were computed with the reference implementation
(`m2m-core`, Rust, `alloy_primitives::keccak256`, `k256`). A conforming
implementation MUST reproduce every value exactly, including signature
recovery to the stated address. The test key is the publicly known anvil/
hardhat account #0 key — it exists to make the recovery path testable.

---

## 8. Implementation notes (learned from production)

These come from running the v0 scheme live on Cloudflare Workers at
code402.dev — including one real mainnet settlement on Base:

1. **Issue the receipt synchronously; confirm settlement asynchronously.**
   Deliver output + receipt in the 200 response the moment payment verifies.
   Never make the buyer wait for chain finality.
2. **Settlement confirmation MUST distinguish three outcomes** — confirmed,
   not-yet-found, and RPC/error. Collapsing error into "not settled"
   (e.g. `unwrap_or(false)`) makes failures invisible.
3. **Bounded retries need a dead-letter sink.** With `max_retries: 5` and
   `retry_delay: 0`, a confirmation message exhausts in seconds and vanishes.
   Use a backoff ≥ 60 s and a DLQ; alert on any event stuck in
   `PENDING_SETTLEMENT` beyond a threshold (10 min works).
4. **Keep an append-only ledger** of lifecycle events (§2). Hourly
   reconciliation (`count(PENDING_SETTLEMENT)` published to an ops key) is
   what caught the only production failure so far.
5. **Do not record payer addresses you don't need.** The ledger row can
   carry `tx_hash` alone; the chain is the payer record.

---

## 9. Versioning & errata

- `xdr-1/0.2` — this document. Changes from v0 (deployed at code402.dev on
  2026-08-17): domain-separated commitment; signed `payment_ref` binding
  payment to receipt; `spec` field; JCS pinned; low-s and v-normalization
  rules; manifest trust root; named verification failure modes.
- **v0 receipts** (like §3's) remain verifiable under their original
  untagged construction; their settlement links are informational only.
  Verifiers encountering a receipt without a `spec` field SHOULD treat it
  as v0.
- **Erratum (known, cosmetic):** the v0 implementation serializes
  `commitment` as hex without the `0x` prefix while `input_hash`/
  `output_hash` carry it. This spec mandates `0x`-prefixed hex for all byte
  fields; verifiers SHOULD accept both forms. The reference implementation
  converges to the v0.2 format in `m2m-core` v1.1.

**Review history.** v0.1 of this document underwent an independent
adversarial review on 2026-08-17 (fresh-context reviewer, recomputed all
vectors and the on-chain settlement). It returned zero fabricated-value
findings and five design findings — domain separation, unsigned settlement
link, non-canonical JSON, undefined recipient check, undefined key trust
root — all of which are addressed in this v0.2. Full review in
`reviews/independent-review-2026-08-17.md`.

## License

CC0 / public domain for the specification. Reference implementation is MIT.
