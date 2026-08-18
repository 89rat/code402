# plans/ provenance + verification (2026-08-19)

These plan documents arrived from a parallel Claude Cowork session (operator
directive: "use these and integrate for best monetisation and scale"). Per
project doctrine, load-bearing external claims were independently re-verified
before adoption:

- ERC-8021 / Base Builder Codes: VERIFIED (blog.base.dev, docs.base.org,
  github.com/base/builder-codes). Attribution suffix = last 16 bytes of
  calldata matching the 8021-pattern; codes registry maps code -> payout
  wallet.
- Cloudflare Sept 15 2026 default-block of mixed AI crawlers on ad-supported
  pages: VERIFIED (multiple outlets). Pay Per Crawl evolving to Pay Per Use:
  VERIFIED. crawler-max-price / crawler-exact-price headers: VERIFIED — and
  since Dec 2025 they MUST be inside the Web Bot Auth signature-input
  components (the crawler plan's design matches this exactly).
- Web Bot Auth = RFC 9421 HTTP Message Signatures, Ed25519, key directory at
  /.well-known/http-message-signatures-directory: VERIFIED (Cloudflare docs +
  github.com/cloudflare/web-bot-auth). Verified-bot registration lead times
  are real and weeks-long — C0 filings go FIRST.
- x402 Foundation grants / Base Builder Grants on Gitcoin / x402scan
  registration: plausible but NOT yet individually verified — verify at
  application time (Phase 2) before relying on them.

Note: the Cowork copy of Rev 3 cites spec v2.0 2025-12-09 at coinbase/x402;
canonical repo is x402-foundation/x402 (corrected at Stage 0, see
reviews/spec-verification-2026-08-19.md). The adopted Rev 3 with all Stage-0/1
amendments lives at reviews/plan-rev3-2026-08-19.md — that file remains the
binding merchant plan; these documents extend it.
