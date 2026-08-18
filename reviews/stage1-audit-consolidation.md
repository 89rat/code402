# Stage 1 audit consolidation — 2026-08-19

Panel: **Kimi** (kimi-k2 CLI, full repo access, watched mid-audit commits land)
· **DeepSeek** (v4-pro API — first panel run after Codex quota-out; composite
60KB inline prompt, 60K-token reasoning budget) · **ZCode** (self-review,
verification, adoption). **Codex** ✗ (ChatGPT usage limit). **Claude** ✗ (CLI
login still pending).

## Commits under audit
`5d988b8` (module+vectors) → `51d6850` (6492 fix, pre-empting Kimi's identical
finding) → `a9e7570` (Kimi round: domain params, §7 envelope, gate additions)
→ `d0515a5` (DeepSeek round: validator split, gate hardening, drift Hole A).
**46 tests green** (27 unit + 19 integration).

## Convergence
- Both agents independently flagged: `extra.name`/`version` required for
  EIP-3009 domain construction (adopted, verified vs scheme spec :71-73);
  `validAfter` unchecked in gate (adopted); accepts[0]-only validation
  (adopted); `validate_spec` EVM-scoped masquerading as spec-generic (adopted
  as a real split per DeepSeek; Kimi had it as a naming nit); dead `read_vec`
  path (adopted); `ExactAmountMismatch` as catch-all echo error (noted — fix
  lands with the §9 taxonomy in Stage 2).
- Kimi unique: §7 facilitator envelope missing (adopted — `FacilitatorRequest`
  modeled); G6 echo-exclusion dependency promoted to launch-checklist item;
  settle-vector field-order canonicalization note (adopted); Stage-2 crypto
  vector coverage list (recorded for the Stage 2 gate).
- DeepSeek unique: strict-object `extra`/`info`/`schema` (adopted); `from`
  validation + mandatory `0x` prefixes (adopted); `route_url` fail-closed
  (adopted); SettleResponse tightening (adopted); drift Hole A unpinned-files
  (adopted); "exactly-65-byte signatures may still be ERC-1271 — Stage 2 must
  not assume all 65-byte sigs are EOAs" (recorded as a Stage-2 requirement).
- Divergences: none material. DeepSeek wanted Permit2/ERC-7710 payload
  variants modeled now — REJECTED per locked decision 4 (scope cut;
  `BadScheme` declares exact-only). DeepSeek's resource-URL fail-open concern
  resolved by making `route_url` required rather than optional.

## Deferred (recorded, not blocking)
Address-semantic (parsed) echo compares; error-variant split for echo
mismatches; URL-parse for iconUrl; eip155-numeric CAIP-2 tightening;
per-variant max signature length. All Stage-2+ cleanup unless the Stage-2
gate disagrees.

## Panel mechanics notes (folded into /cross-verify skill)
DeepSeek API has no file access — composite inline prompts required; reasoning
model needs max_tokens ≥ 60000 (16K produced an empty verdict on the first
run); fallback key stored. Kimi observed the tree changing mid-audit and
re-based itself — CLI agents with repo access tolerate live trees.

**Gate: awaiting operator go for Stage 2** (crypto conformance: bidirectional
TS↔Rust vectors, domain divergence, v-normalization, expiry boundaries,
value==amount already in gate, 6492/1271 classification, nightly differential
fuzz harness).
