# Legacy snapshot (Stage 0 baseline)

`fixtures.json` records the CURRENT (pre-v2) HTTP behavior of code402-edge,
captured 2026-08-19 against `wrangler dev` on the feat/x402-v2 branch BEFORE
any refactoring. 13 fixtures: discovery/manifest routes, routing validation,
and the legacy payment path (402 challenge shape, 401 signature-error taxonomy).

## Volatile fields (ignore when diffing)

The `challenge_402` body contains per-request values: `nonce`,
`expires_at`, `expires_at_rfc3339`, `request_id`, `payment_intent_id`.
Regression comparison = status + headers + body with these fields masked.

## Regenerating

```bash
npx wrangler dev --port 8799 &
bash tests/legacy-snapshot/run.sh 8799
```

Requires local PRICING KV seeded (vat-mod97-check et al.) and `.dev.vars`
(dummy secrets). Note: `wrangler kv key put --local` crashes on this Windows
setup (libuv assert); seed via the miniflare store files if the namespace is
empty — see blobs under `.wrangler/state/v3/kv/<namespace-id>/blobs/`.

## Why 500s are absent

Earlier capture attempts 500'd on payment paths ("pricing missing") until the
staging KV namespace was seeded. The committed fixtures reflect fully-seeded
behavior: 402/401/400s only. If a regression run produces 5xx here, that is
itself a failure of the baseline contract.
