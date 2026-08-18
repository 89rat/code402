# Operating Protocol — Panel + Week One

> Adopted 2026-08-19 by operator directive. This is the constitution for the
> three-track build (merchant × crawler × monetization). The four plan
> documents it references live in `plans/` and `reviews/`.

## The five rules

1. **The repo is the memory.** No model's context window holds state. Every panel exchange, critique, and decision lands in `reviews/` as a file. Panelists read files, never relayed summaries.
2. **Tests adjudicate; models argue.** CI (vectors, fixtures, e2e, claim-machine model check) holds a veto no model or human overrides casually. A critique that can be expressed as a failing test must be.
3. **Citations or it didn't happen.** Every finding names a file:line, spec section, or test. Format: `lib.rs:382` / `spec §5.1.2` / `fixture drain_03`. Uncited findings are discarded unread.
4. **Blind, adversarial review.** Reviewers get the artifact stripped of authorship and enthusiasm. The question is always "what breaks," never "is this good."
5. **The builder never merges its own work.** Orchestrator writes; panel reviews; you merge.

## Roster

| Role | Model | Reads | Writes | May not |
|---|---|---|---|---|
| Orchestrator | Z code | Stage plan + design-logic doc | Code, tests, PR | Merge; touch payment path without a failing test first |
| Wide-angle reviewer | Kimi | Entire diff + full repo in one pass | Line-cited findings → `reviews/` | Summarize without citations |
| Red team | DeepSeek | Diff + threat model (§5 mirror table) + invariants I1–I6 | Invariant-violation attempts → `reviews/` | Vague concerns; findings must name the violated invariant |
| Advisor / gatekeeper | Claude | Stage artifacts + both plan docs + design logic | Gate verdict → `reviews/` | — |
| Judge | CI suite | Everything | Pass/fail | Be skipped |
| Owner | You | Gate verdicts | Merge; money decisions | Delegate spend approvals to any model |

> Status note 2026-08-19: Claude's CLI auth is pending the operator's terminal
> `claude /login`; until then its gate-verdict role is covered by the
> consolidated ZCode verdict and its critiques append retroactively. Codex is
> quota-limited and sits outside the roster (DeepSeek holds its slot).

## Role prompts

**Orchestrator (Z code):**
> Execute Stage {N} of `{plan-file}` exactly. Constraints: small diffs, one stage-item per commit, conventional commit messages referencing the stage item. Any change on the payment path requires writing the failing test first. Never modify `SPEC-VERSION`, vendored specs, or golden vectors without an explicit instruction. Stop and report — do not improvise — when: a test you didn't write fails, the plan is ambiguous, or a dependency is missing. Do not open a PR as complete unless `cargo test` and the e2e snapshot are green locally. You do not merge.

**Wide-angle reviewer (Kimi):**
> You are reviewing a diff against this full repository, both loaded in your context. Output findings only — no summary, no praise, no restatement of what the diff does. Each finding: severity (blocker/major/minor), file:line citation, the concrete failure scenario, and where relevant the spec section (`specs/` is vendored in-repo) or design-logic invariant (I1–I6) it violates. Look specifically for cross-file inconsistencies a single-file review would miss: type changes not propagated, error taxonomy drift, test coverage gaps versus the stage's gate list, divergence between code and the vendored spec. If you find nothing at blocker or major severity, say exactly that in one line.

**Red team (DeepSeek):**
> Attached: a diff, the threat-model mirror table, and invariants I1–I6. Your only task: construct concrete attacks that violate an invariant through this code. For each attempt: the invariant targeted, the exact request sequence or input, the file:line where the defense fails or holds, and — if it fails — the fixture you would add to make this attack a permanent regression test. Attacks that depend on hardware or infrastructure this system does not run on (threads, NUMA, raw sockets — the runtime is Cloudflare Workers/WASM) are out of scope. Report holds as one line each; report breaks in full.

**Advisor gate (Claude):**
> Stage {N} gate audit. Inputs: the stage's diff, test results, Kimi's findings, DeepSeek's findings, the stage definition in the plan, and `x402-design-logic.md`. Verdict structure: (1) findings both reviewers missed, cited; (2) disposition of every blocker/major finding — fixed, refuted with citation, or open; (3) invariant check against I1–I6; (4) gate verdict: pass / pass-with-conditions / fail. A stage with any open blocker fails. File the verdict in `reviews/stage-{N}-gate.md`.

## Disagreement rule

Ground truth wins: spec line beats model opinion, test result beats both, chain state beats everything. If two panelists still disagree after citations, design the smallest reversible experiment that discriminates, run it, commit the result to `reviews/`. Never resolve by seniority of model or confidence of prose.

## Kaizen loop

After every gate: each defect that reached review becomes a vector or fixture (rule: **every defect becomes a vector**); one line in `reviews/gate-retro.md` answering "what leaked through the previous gate and which checklist line now catches it."

## Week one runbook (status 2026-08-19)

- **Day 1 — filings:** ⏳ operator-owned, still open: crawler UA string, Web Bot
  Auth keypair + directory, verified-bot form, Pay Per Crawl signup, Builder
  Code claim, Basename profile. (Orchestrator can draft all inputs on request.)
- **Days 2–3 — Stage 0:** ✅ complete (commits 56ceccb…1b2287d), gated.
- **Days 3–5 — Stages 1–2:** Stage 1 ✅ complete and gated; Stage 2 code
  complete, panel-audited, corrections in flight (this commit window).
  Crawler C1 unlocks when the Stage-2 gate closes.
- **Post #1 draft:** ⏳ 30 min/day, operator-owned or orchestrator-drafted.
- **Panel protocol exercised:** twice for real (Stages 1 and 2) — this
  document formalizes what those gates actually did.
