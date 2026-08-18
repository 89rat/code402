#!/usr/bin/env bash
# Legacy snapshot e2e — records CURRENT (pre-v2) HTTP behavior of code402-edge.
# Run: tests/legacy-snapshot/run.sh [port]   (expects `wrangler dev --port <port>`)
# Output: tests/legacy-snapshot/fixtures.json (deterministic fields only).
#
# This is the Stage-0 regression net (plan-rev3): the v2 refactor must not
# change these behaviors on the legacy route until the Stage-5 hard cut
# (traffic data in reviews/cdp-findings.md: 0 settled legacy payments ever).
set -u
PORT="${1:-8799}"
BASE="http://127.0.0.1:$PORT"
OUT="$(dirname "$0")/fixtures.json"

snap() { # name method path [curl args...]
  local name="$1" method="$2" path="$3"; shift 3
  local body headers status
  body=$(curl -s --max-time 10 -X "$method" "$BASE$path" "$@" 2>/dev/null)
  status=$(curl -s -o /dev/null -w '%{http_code}' --max-time 10 -X "$method" "$BASE$path" "$@" 2>/dev/null)
  headers=$(curl -s -D - -o /dev/null --max-time 10 -X "$method" "$BASE$path" "$@" 2>/dev/null \
    | tr -d '\r' | grep -iE '^(HTTP|x-schema-version|content-type|cache-control)' | sort)
  python - "$name" "$status" "$headers" "$body" "$OUT" << 'PYEOF'
import json, sys, os
name, status, headers, body, path = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]
try: body_json = json.loads(body)
except Exception: body_json = body[:2000]
entry = {"status": int(status), "headers": [h for h in headers.splitlines() if h], "body": body_json}
data = {}
if os.path.exists(path):
    data = json.load(open(path, encoding="utf-8"))
data[name] = entry
tmp = path + ".tmp"
json.dump(data, open(tmp, "w", encoding="utf-8"), indent=2, sort_keys=True)
os.replace(tmp, path)
print(f"snap {name}: {status}")
PYEOF
}

# --- free/discovery surfaces ---
snap manifest GET /.well-known/x402.json
snap llms_txt GET /llms.txt
snap openapi GET /.well-known/openapi.yaml
snap mcp_manifest GET /.well-known/mcp.json
snap trust_missing GET /v1/trust/does-not-exist.example
snap trust_badge_unrated GET /v1/trust/does-not-exist.example/badge.svg

# --- routing/validation (pre-payment) ---
snap wrong_route GET /v1/tools
snap unknown_tool POST /v1/tools/not-a-tool/call -H 'content-type: application/json' -d '{"input":{"x":"y"}}'
snap bad_body POST /v1/tools/vat-mod97-check/call -H 'content-type: application/json' -d 'not json'
snap missing_field POST /v1/tools/vat-mod97-check/call -H 'content-type: application/json' -d '{"input":{"wrong":"field"}}'

# --- payment path (no valid signature available offline; deterministic error paths) ---
snap challenge_402 POST /v1/tools/vat-mod97-check/call -H 'content-type: application/json' -d '{"input":{"vat_number":"GB123456789"}}'
snap garbage_payment POST /v1/tools/vat-mod97-check/call -H 'content-type: application/json' -H 'X-PAYMENT: not-json' -d '{"input":{"vat_number":"GB123456789"}}'
snap bad_voucher POST /v1/tools/vat-mod97-check/call -H 'content-type: application/json' -H 'X-PAYMENT: {"auth":{"from":"0x1","to":"0x2","value":"1","valid_after":0,"valid_before":1,"nonce":"0x00"},"signature":"00"}' -d '{"input":{"vat_number":"GB123456789"}}'

# --- preflight / status route (current behavior; v2 changes CORS by design) ---
snap options_preflight OPTIONS /v1/tools/vat-mod97-check/call
snap status_route GET /v1/requests/req-some-request-id

# --- semantic-failure voucher: properly signed, wrong recipient (audit Q5).
# Requires: cargo run --manifest-path crates/keygen/Cargo.toml --bin paytest
# > semantic-voucher.txt (regenerate when expired; file is gitignored).
V="$OUT.dir/../legacy-snapshot/semantic-voucher.txt"
if [ -f "$(dirname "$0")/semantic-voucher.txt" ]; then
  snap semantic_voucher_wrong_recipient POST /v1/tools/vat-mod97-check/call \
    -H 'content-type: application/json' \
    -H "X-PAYMENT: $(cat "$(dirname "$0")/semantic-voucher.txt")" \
    -d '{"input":{"vat_number":"GB123456789"}}'
else
  echo "skip semantic_voucher (no semantic-voucher.txt — see README)"
fi

echo "fixtures written to $OUT"
