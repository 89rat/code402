# x402 Deployment Review — fixed-scope, 5 days, fixed fee

**Your agents will find the bugs. We'll find them first — with citations.**

You are shipping (or have shipped) an x402 payable endpoint. The buyers are
autonomous agents that cannot file a support ticket, cannot ask for a refund,
and will simply never come back after a failed payment. We review your
deployment the way an adversarial buyer experiences it — and hand you every
finding as a failing test you can keep.

## What you get

1. **A structural audit of your live 402 flow** — challenge shape, header
   dialect, envelope conformance against the vendored x402 v2 spec (not our
   opinion: the spec text, cited by section).
2. **The attack taxonomy run against your endpoint** — replay, grant-before-
   settle, signature bypass, nonce misuse, quota/gas abuse, discovery drift —
   each published attack either demonstrably stopped or demonstrated, with
   `file:line` or request-sequence evidence.
3. **Money-state audit** — what happens to a payer's funds in every failure
   mode: timeout-after-settle, already-used, tool failure after payment,
   facilitator outage. Every ambiguous state must resolve to a defined,
   reconciled outcome. (This is the section most deployments fail.)
4. **Agent-UX proof** — can an autonomous agent discover → 402 → pay → verify
   using only your public manifests? We run it and show the transcript.
5. **Every defect as a fixture** — findings ship as executable test vectors
   (JSON/HTTP), not prose. Your CI can hold the fixes forever.

## What we don't do

- No code changes to your repo. No retainers baked in. No "strategy."
- We never ask for keys, and you never paste any. All testing is against
  public endpoints with our own funds, capped and disclosed.

## Terms

- **Scope:** one deployment (one host, one payment path). Fixed.
- **Time:** 5 business days from kickoff.
- **Fee:** fixed, quoted on scope confirmation (US$7.5–15K depending on
  surface). Payable in USDC on Base — via x402, naturally. You experience
  our conformance as your first deliverable.
- **Independence:** we operate the public x402 conformance suite and the
  ecosystem price/trust index. Findings are cited or discarded — that rule
  applies to us too.

## Why us

We run code402.dev — a production x402 v2 merchant gateway whose receipts
reconcile to the chain, verified end-to-end with the official client. We
publish the open conformance vectors used across the ecosystem, and our
review process is adversarial by construction: multi-model panel, citations
mandatory, tests adjudicate. Our own postmortems are public in our repo's
`reviews/` — read how we get attacked before you hire us to attack you.

**Contact:** review@code402.dev · https://code402.dev/proof
