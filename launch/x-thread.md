# X launch thread — evidence-first (@code402dev first posts, one tweet per block)

**1/10**
We run a payment system for AI agents (x402 / HTTP 402, USDC on Base, non-custodial).

Last week it moved money on-chain 1,000 times while we stress-tested it. It also
"lost" 122 of those payments.

That second number is why I'm posting. 🧵

**2/10**
"Lost" precisely: the transfer settled on-chain, but our side timed out before recording
it. Money moved; no receipt; the payer paid for nothing.

Every payments engineer knows this class. Most systems hope. We built a reconciler.

**3/10**
The rule: the chain is the root of truth. Every hour, a sweep reads
authorizationState for every unresolved claim, then the AuthorizationUsed /
AuthorizationCanceled logs, and resolves each one three ways:
settled / canceled / expired. Never a guess — unknown stays unknown until the chain speaks.

**4/10**
We tested it the only way that counts — with real money on Sepolia, four scenarios:

• settle out-of-band, then retry → resolved with the exact tx; the retry ran FREE
• payer cancels on-chain → failed_canceled; retry correctly refused
• window expires unused → failed_expired, exactly per spec
• our settle never landed → the cron itself re-drove it and settled (real tx)

**5/10**
The best part of #4: "retry ran free" is not a refund. We're non-custodial — we
CAN'T refund. It's a cryptographic entitlement: chain-proved you paid → your next
identical request executes free, bound to the original input. Different input → 400.

**6/10**
Live testing caught what unit tests couldn't, three times:
- the facilitator's real already-used error shape matched NOTHING we'd tested
- the entitlement was unreachable after 5 minutes (grace window bug) — our e2e passed
  only because it was fast enough
- the entitlement served ANY input until review caught the compute-oracle hole

All three stories are public in the repo's reviews/ directory. Failures included.

**7/10**
Every paid call returns an XDR-1 receipt: RFC 8785 canonical JSON, domain-separated
commitment, signed payment reference. You can verify it offline — no call to us —
with any keccak + secp256k1. The spec's test vector reproduces byte-for-byte.

https://code402.dev

**8/10**
How is this built? A multi-AI panel reviews every payment-path change: independent
red-team + wide-angle reviewers attack the diff; the builder must prove each finding
wrong or concede. It caught 3 real bugs I'd shipped green. The verdicts are published.

The process assumes the builder is the last person to see their own bugs. It usually holds.

**9/10**
All of it is open: the gateway, the reconciler, the receipts spec, the stress data,
the 132-phantom regression corpus, the review trail.

github.com/code402dev — start with reviews/reconciler-e2e-report.md

**10/10**
Live telemetry, not marketing dashboards: code402.dev/v1/ops/stats

We're building the settlement layer that assumes everything fails and answers each
failure with engineering. More receipts than promises, as a matter of policy.

(DM's open — especially if you're monetizing an API for agents, or breaking our stuff.)
