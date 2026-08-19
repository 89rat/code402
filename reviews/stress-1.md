# Stress campaign 1 — 2026-08-19 (local dev worker + REAL CDP settles)

Driver: tests/vectors/gen/stress.mjs (viem-signed = the C1 client semantics;
concurrent via Promise.all). Environment: wrangler dev (single isolate —
real edge concurrency NOT simulated; DO serialization IS). Ledger verified
post-run: 27 V2_SETTLED events = 27 settlements rows, 1:1, zero duplicates.

## Results (all design-claims held)
- **A fuzz-flood**: 150 malformed PAYMENT-SIGNATUREs (random/oversized/
  wrong-version/garbage-json) in 1.14s — ALL 400, ZERO 5xx/panics.
  Structural gate holds under flood; ~131 rps on local dev.
- **B replay-storm**: 25 PARALLEL replays of a settled payment — all 200,
  BYTE-IDENTICAL bodies (1 distinct), 296ms total. G2b perfect.
- **C same-payment race**: 10 PARALLEL requests, ONE payment — 1x200 +
  9x503(in-progress, retryable), identical 200 output, EXACTLY ONE settle
  (ledger-confirmed). The claim machine is correct under real concurrency.
- **D G3 collision**: two payers, SAME nonce — both traverse the claim
  machine independently (funded=200, empty=400 insufficient_funds).
  UNIQUE(payer,nonce) isolation proven live.
- **E burst settles**: 12 PARALLEL REAL CDP settles — ALL 200 with 12 real
  on-chain txs, wall 2.31s TOTAL (latencies clustered 1.9-2.3s = one block
  window), zero 429s from CDP at 12-parallel. The system parallelizes
  settles; throughput is block-width, not block-count.

## Learnings
1. Parallel settle throughput: N concurrent settles complete in ~ONE block
   window (~2s) — capacity is far beyond sequential intuition.
2. Race losers get 503-retryable immediately (correct, but a long-poll /
   await-winner pattern would convert them to identical 200s client-side
   with one retry) — recorded as polish, not a defect.
3. Fuzz flood costs the facilitator NOTHING (structural gate rejects all
   locally) — the G4 quota-guard claim is now stress-verified.
4. Local-dev caveat: single isolate; production edge adds dispersion the
   DO model already accounts for (single-threaded per claim key).
