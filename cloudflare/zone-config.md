# Cloudflare Zone Configuration — code402.dev

**Status:** operator-action required · **Why not automated:** the wrangler OAuth token on this
machine has `zone (read)` only — no Rulesets write. Everything below is exact, validated-shape
configuration for the dashboard (or API with a zone-scoped token with `Rulesets: Edit`).

## 1. WAF skip rule — keep agents out of bot challenges

**Security → WAF → Custom rules → Create rule** (or via Rulesets API):

- Name: `api-agents-skip-challenges`
- Expression (use "Edit expression"):
  ```
  (starts_with(http.request.uri.path, "/v1/") or starts_with(http.request.uri.path, "/v2/")) and not cf.bot_management.score eq 1
  ```
  Simplify to just the path clause if bot-management fields aren't in your plan:
  ```
  (starts_with(http.request.uri.path, "/v1/") or starts_with(http.request.uri.path, "/v2/"))
  ```
- Action: **Skip** → tick: All remaining custom rules, Browser Integrity Check, Bot Fight Mode /
  Super Bot Fight Mode challenges, Rate limiting rules.
- Order: place **above** any block/challenge rules.

**Design note — deliberately path-scoped, NOT header-scoped.** The tempting version
(`http.request.headers["payment-signature"] ne ""`) is an EDoS amplifier: anyone can send an empty
payment header and skip your protections. Our API paths are machine-only by design; the worker's
own structural gate is the real filter, and payment failures 4xx there.

## 2. Cache rule — API responses never cached at zone level

The worker already sends `Cache-Control: private, no-store` on every payment-negotiated response
(402s, paid 200s, replays — deployed 2026-08-19). Belt-and-braces zone rule:

**Caching → Cache Rules → Create rule**:
- Name: `api-bypass-cache`
- Expression:
  ```
  (starts_with(http.request.uri.path, "/v1/") or starts_with(http.request.uri.path, "/v2/"))
  ```
- Action: **Bypass cache**

Keep caching ON for everything else (the SPA assets and `/llms.txt`, `/.well-known/*` are static
and benefit from edge caching; `/v1/ops/stats` self-limits with `max-age=30`).

## 3. API curl equivalents (needs zone token with Rulesets Edit)

```bash
ZONE_ID=<code402.dev zone id>
API_TOKEN=<zone-scoped, Rulesets:Edit>

# skip rule (entry point: http_request_firewall_custom phase)
curl -X POST "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/rulesets/phases/http_request_firewall_custom/entrypoint/rules" \
  -H "Authorization: Bearer $API_TOKEN" -H "Content-Type: application/json" \
  -d '{"expression":"(starts_with(http.request.uri.path, \"/v1/\") or starts_with(http.request.uri.path, \"/v2/\"))","action":"skip","action_parameters":{"phases":["http_request_firewall_custom","http_request_sbfm"]},"description":"api-agents-skip-challenges"}'

# cache bypass (http_request_cache_settings phase)
curl -X POST "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/rulesets/phases/http_request_cache_settings/entrypoint/rules" \
  -H "Authorization: Bearer $API_TOKEN" -H "Content-Type: application/json" \
  -d '{"expression":"(starts_with(http.request.uri.path, \"/v1/\") or starts_with(http.request.uri.path, \"/v2/\"))","action":"set_cache_settings","action_parameters":{"cache":false},"description":"api-bypass-cache"}'
```

(Verify the exact `action_parameters` shape against the current Rulesets API docs before running —
skip-phase lists have changed across API versions.)

## 4. Already applied (no action needed)

- `Cache-Control: private, no-store` on all payment paths (worker code, both envs)
- `run_worker_first` for `/v1/*`, `/v2/*`, `/.well-known/*`, `/llms.txt` (assets router never
  shadows the API)
- Custom domains `code402.dev` / `www` bound to `code402-edge-prod` (re-confirmed on deploy)
