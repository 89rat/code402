# Spec verification 2026-08-19 — Rev 3 wire claims vs vendored spec text

Verified by ZCode against `specs/x402/x402-specification-v2.md` at commit
`ddf98ee511d30031f1923915f2d46084bb62cd4f` (raw fetch, pre-vendoring).
All five load-bearing Rev 3 claims CONFIRMED:

1. **PaymentRequired envelope** — `x402Version` (number, required, "must be 2"),
   `error` (string, optional), `resource` (ResourceInfo object, **required**),
   `accepts` (array, required), `extensions` (object, optional). ✔
2. **§5.1.2 extension echo semantics** — "The client must include at least the
   info received; it may append additional info but cannot delete or overwrite
   existing info." (G6's HMAC-stamp survives client-side appending.) ✔
3. **validAfter/validBefore are strings** — Authorization table types both as
   `string`; example shows quoted values ("1740672089"). ✔
4. **Response types** — `VerifyResponse {isValid req, invalidReason?, payer?,
   extra?}`; `SettleResponse {success req, errorReason?, payer?, transaction
   REQUIRED, network REQUIRED, amount?, extensions?}`. Addition adopted into
   G2: the replay record stores `transaction` + `network` by construction. ✔
5. **CAIP-2 networks** — §11.1: `eip155:8453` (Base), `eip155:84532` (Sepolia);
   non-EVM example `solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp`. ✔

Correction to Rev 3: the live canonical repo resolved as
`github.com/x402-foundation/x402` (Rev 3 cited `coinbase/x402`, the historical
origin). SPEC-VERSION pins the foundation repo.
