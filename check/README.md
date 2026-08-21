# x402check

**Can an autonomous agent actually pay your endpoint?** One command tells you —
with a grade, and every finding cited to the rule it violates.

```bash
node cli.js https://your-endpoint.example/call
node cli.js https://your-endpoint.example/call --body '{"input":{...}}'
node cli.js <url> --json        # machine-readable, for CI
```

Zero dependencies. Node 18+, browsers, Cloudflare Workers. Exit codes:
`0` grade A/B · `1` C/D · `2` F — so it drops straight into CI:
`x402check $URL || exit 1` and a broken payment path fails your build.

## What it checks

- **Discovery** — does an unpaid request return HTTP 402 with a machine-readable
  challenge? (v2 `PAYMENT-REQUIRED` header · v1 `accepts[]` body · bespoke
  dialects detected and named)
- **Money fields** — decimal-string integer amounts (never floats), CAIP-2
  networks, `0x`+40hex asset/payTo, sane timeout bands, EIP-712 domain material
- **Agent-UX** — cache hygiene on challenges, CORS/expose headers for browser
  agents, Deprecation/Sunset signals on legacy dialects
- **Validation-first detection** — endpoints that demand valid input before
  revealing the price get flagged with the trade-off, not a false F

## Why it exists

After September 15, 2026, agents are blocked by default on monetized pages and
must *pay or identify themselves*. "Is my endpoint payable by a conformant
agent?" becomes a question every seller needs answered — continuously, in CI,
not once by hand. Grades make it viral; citations make it actionable; exit
codes make it sticky.

Dogfood note: this checker grades its authors' own production endpoint **F**
today (bespoke v1 dialect, pending the v2 flip) — the report that finding
generated is public in our repo's `reviews/`. We check ourselves first.

Part of the code402 LEGO wall (`plans/lego-wall.md`): standalone, retrofittable,
expandable. Apache-2.0.
