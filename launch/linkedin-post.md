# LinkedIn company launch post (Code402 page, first post)

---

We just put a payment system through the kind of testing you rarely see announced —
and we're publishing the failures, not just the wins.

**What Code402 is:** a settlement layer for machine-to-machine payments. AI agents pay
APIs per call over HTTP 402 (the x402 standard) in USDC on Base. Funds move directly
from payer to merchant — we never hold them. Non-custodial by construction.

**Why that's hard:** when you don't hold the funds, you can't refund mistakes. Every
failure mode — the payer paid but wasn't served, the settlement landed but the record
didn't, two requests raced for one authorization — has to be answered with engineering,
not an apology and a chargeback.

**What we did about it:**

• **Exactly-once settlement** per payment authorization, enforced by a Durable Object
  claim machine — race losers receive the byte-identical stored response
• **A reconciler that treats the blockchain as the source of truth** — every hour it
  resolves each unresolved claim against on-chain state: settled, canceled, or expired.
  We proved all four resolution scenarios with real money and transaction hashes
• **Cryptographic entitlements** — if the chain proves a payer paid and we never served
  them, their next identical request executes free, bound to the original input
• **XDR-1 delivery receipts** — every paid call returns a signed receipt that any third
  party can verify offline: canonical JSON (RFC 8785), domain-separated commitment,
  payment reference bound into the signature

**And the part we're most proud of:** during stress testing with 1,000 real settlements,
the system "lost" 122 payments — money moved on-chain, our record timed out. Instead of
hiding it, we exported all 132 such cases into a public regression corpus and built the
reconciler that resolves them. Every defect became a test.

**How it's built:** every payment-path change passes an independent multi-AI review
panel — red-team and wide-angle reviewers attack each diff, and every finding must be
proven wrong or conceded, in public verdicts. It recently caught three real production
bugs the builder had shipped green. We consider that process the product.

Everything is open: the gateway, the reconciler, the receipts specification, the stress
data, the review trail.

🔗 Live: https://code402.dev
🔗 Evidence: https://code402.dev/proof
🔗 Code: https://github.com/code402dev

If you're monetizing an API for AI agents — or you'd like to try breaking our
settlement guarantees — we'd like to hear from you.

#AgenticPayments #x402 #AIagents #FinTechEngineering #OpenSource #Base #USDC
