# Kimi ⇄ Claude Code bridge (headless dispatch)

## One-time setup (human does this, single click)

Double-click `setup-claude-bridge.bat` in the workspace root. It will:

1. Check Node.js/npm (offers winget install if missing)
2. `npm install -g @anthropic-ai/claude-code`
3. Verify `claude` is callable
4. Create `code402\collab\{inbox,outbox,logs}`
5. Run one tiny headless auth probe (`BRIDGE_OK`)

Success marker: `code402\collab\READY` exists.
If the probe fails: run `claude` once in a terminal, log in, re-run the .bat.

## The working loop (Kimi does this)

1. Kimi writes a task file: `collab\inbox\YYYYMMDD-HHMM-<slug>.md` — objective, files in scope, acceptance criteria.
2. Kimi dispatches from git-bash:
   `cmd //c "code402\\collab\\dispatch.cmd YYYYMMDD-HHMM-<slug>.md"`
3. Claude Code runs inside `code402\` under the rules in `AGENT-BRIEF.md` (no secrets, no deploys, bounded turns).
4. Output lands in `collab\outbox\<slug>.result.md` plus `<slug>.exitcode`; raw stderr in `collab\logs\<slug>.log`.
5. Kimi reviews the result, runs the project's gates/tests, and reports back to the human.

## Rules of engagement

- **One owner per task.** If Claude is implementing, Kimi does not touch those files until the run finishes. Avoids edit collisions.
- **Cost:** every dispatch spends Claude subscription/API quota. `dispatch.cmd` caps runs at `--max-turns 25`; tighten in that one line if needed.
- **Secrets:** Claude never sees `.staging\` or key material (enforced by `AGENT-BRIEF.md`). Tasks needing secrets come back as `SECRET-NEEDED`.
- **Review before merge:** Kimi inspects every diff before anything is committed or deployed. Nothing auto-deploys, ever.
