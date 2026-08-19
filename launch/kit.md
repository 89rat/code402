# Code402 Launch Kit — accounts, profiles, first posts

Brand: **Code402** · Handle: **@code402dev** (GitHub org `code402dev` AVAILABLE, X handle
available per probe — verify at signup) · Angle: **evidence-first**.

Everything below is copy-paste ready. Account creation itself is operator-only
(email/identity verification flows). Total time: ~45 minutes.

---

## 1. GitHub org (do this first — repos make the other posts credible)

1. github.com → sign in as `89rat` → **+ → New organization** → Free plan
2. Organization username: **`code402dev`** · Name: **Code402** · Email: your company email
3. Transfer the flagship repo: `89rat/code402` → Settings → Danger Zone → **Transfer
   ownership** → to `code402dev`. GitHub auto-redirects old URLs (no links break).
   Also transfer (optional): `m2m-exchange`, `x402-atlas` if you want them under the org.
4. Create repo **`code402dev/.github`** — make `README.md` in it with the content from
   `launch/github-org-readme.md` (renders on the org profile page).

## 2. X (@code402dev)

1. x.com → create account → handle `code402dev`
2. Bio (≤160 chars):

   > Machine-payable APIs for AI agents. Non-custodial x402 on Base. Our receipts reconcile to the chain — every failure mode answered with engineering, published. code402.dev

3. Pin the launch thread (`launch/x-thread.md`) as your first posts.
4. Follow: @x402_foundation, @CloudflareDev, @coinbasedev, @base — reply-level presence
   in those threads is the discovery engine early on.

## 3. LinkedIn company page

1. linkedin.com → your personal profile → Work grid (top right, 3×3 dots) → **Create a
   Company Page** → Company name: **Code402** · URL: linkedin.com/company/code402dev
2. Tagline (120 chars):

   > Machine-payable APIs for AI agents. Spec-conformant x402, non-custodial USDC settlement, offline-verifiable receipts.

3. Description:

   > Code402 is a settlement layer for machine-to-machine payments: AI agents pay APIs
   > per call over HTTP 402 (x402) in USDC on Base — funds move directly from payer to
   > merchant, never custodied. What makes it different is the correctness engineering:
   > exactly-once settlement per authorization, hourly reconciliation against chain
   > state, cryptographic entitlements for paid-but-unserved callers, and XDR-1 delivery
   > receipts that any third party can verify offline. Built in the open — the failures,
   > the reviews, and the evidence are all public.
   >
   > Live: code402.dev · Proof: code402.dev/proof · Code: github.com/code402dev

4. Industry: Software Development · HQ: Coventry, GB (JUANA LIMITED) · Website: code402.dev
5. First post: `launch/linkedin-post.md`.

## 4. Cross-links (after accounts exist — 5 min)

- Repo READMEs → add social line (I'll patch once handles are live)
- code402.dev site footer → LinkedIn/X icons
- X profile → website field: code402.dev
- LinkedIn page → website + GitHub link

## 5. The 30-day cadence after launch

1 post/week from the evidence backlog (no new writing needed — it's all in `reviews/`):
- Why Cloudflare KV breaks 402 replay checks (132-phantom corpus included)
- The four reconciler scenarios, with tx hashes
- The CDP failure-shape bug: what live e2e caught that units couldn't
- XDR-1 receipts: a receipt that verifies offline
- The 1,000-settle stress: bimodal degradation and graceful recovery
