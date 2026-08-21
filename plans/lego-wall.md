# The LEGO Wall — modularity doctrine (operator rule, 2026-08-19)

> "One rule of all: modular, retrofittable, expandable LEGOs for the gold
> rush — they pick and choose what they want from the wall."
> — operator directive, binding on every artifact we ship

## The rule

Every product, crate, API, and template we ship is a **brick**:

1. **Standalone** — a brick works alone. No brick requires another brick to
   be useful. (The vectors don't need the gateway; the receipt module doesn't
   need the reconciler.)
2. **Retrofittable** — a brick drops into a stack that already exists.
   Nobody rewrites anything to adopt us. If adoption requires more than an
   afternoon, the brick is malformed.
3. **Expandable** — a brick has a declared extension seam (trait, adapter,
   or config), never a fork. Spec moves → new adapter, never a rewrite
   (atlas moat #2 applied to ourselves).
4. **Composable only through the wire** — bricks connect via the x402 spec,
   CAIP-2 strings, and the XDR-1 receipt format. The spec is the ABI; the
   ABI is the stud-and-tube. No internal couplings, no shared-state bricks.

## The wall (current inventory)

| Brick | Form | Status |
|---|---|---|
| **x402v2 codec + golden vectors** | OSS crate (Apache-2.0) | Stage 2 — critical path |
| **Claim machine** (D1-only, pure state machine) | crate + migration SQL | R1 redesign approved in principle |
| **XDR-1 receipts** (JCS, offline-verifiable) | crate + CC0 spec | built |
| **Reconciler** (chain-as-truth sweep + inline) | crate + worker module | built; R2 inline variant pending |
| **Structural gate / policy engine** (deny-by-default predicates) | crate, merchant + crawler instances | built |
| **Gateway template** ("Deploy to Cloudflare") | GitHub template repo | Stage 5 + 2 weeks |
| **Conformance prober + badge** | paid API (x402-priced) | atlas exhaust, productize |
| **Price/trust index** | free web + paid API | atlas |
| **Deployment Review** (service) | fixed-scope, 5 days | launch/deployment-review.md ✔ |
| **Signer service** (grant-bounded, zeroize-on-drop) | crawler-side service | C2 |
| **Credit notes** (entitlement → bearer credit) | XDR-1 extension | R4 — counsel check first |

## Brick engineering rules

- One repo per public brick, or one crate per brick in a published workspace —
  semver'd, SPEC-VERSION pinned, changelog per brick.
- Every brick ships with **its own vectors**; adopting a brick means adopting
  its test kit first. (The test kit IS the brick's storefront.)
- A brick's docs answer in one page: what it does, what it refuses to do,
  the three lines to integrate.
- No brick ever phones home, holds keys it doesn't own, or requires our
  infra to keep running. A customer who leaves keeps working — that is what
  makes the next brick an easy yes.
- Pricing is per brick; bundles are a discount, never a lock.

## Why this wins the gold rush

Miners don't buy platforms — they buy the one tool that solves tonight's
problem, from someone whose other tools they'll consider later. The wall
converts every "no" to the platform into a "yes" to a brick, and every brick
adopted is a wire-level dependency on our spec interpretation — which is the
quiet mechanism by which the referee of the gold rush gets chosen.
