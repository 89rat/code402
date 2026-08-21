/**
 * x402check — core conformance checker (zero-dependency, ESM).
 * Runs in Node 18+, browsers, and Cloudflare Workers (fetch-only).
 *
 * One question answered: "can an autonomous agent actually pay this endpoint?"
 * Every finding cites the rule it violates. Findings-only philosophy:
 * a clean endpoint gets a one-line verdict.
 *
 * Dialects detected:
 *   v2  — 402 + PAYMENT-REQUIRED header (base64 JSON envelope)
 *   v1  — 402 + JSON body {x402Version:1, accepts:[...]}
 *   none — no payable signal
 */

export const SEVERITY = { BLOCKER: 'blocker', MAJOR: 'major', MINOR: 'minor', INFO: 'info' };

const RE_CAIP2_EVM = /^eip155:\d+$/;
const RE_ADDRESS = /^0x[0-9a-fA-F]{40}$/;
const RE_DECIMAL_INT = /^(0|[1-9]\d*)$/;
const KNOWN_SCHEMES = new Set(['exact', 'upto']);

function finding(severity, check, detail, citation) {
  return { severity, check, detail, citation };
}

function* checkAcceptsEntry(a, i, dialect) {
  const at = `accepts[${i}]`;
  if (!a || typeof a !== 'object') {
    yield finding('blocker', 'accepts-shape', `${at} is not an object`, 'envelope');
    return;
  }
  // scheme
  if (typeof a.scheme !== 'string' || !KNOWN_SCHEMES.has(a.scheme)) {
    yield finding('major', 'scheme', `${at}.scheme = ${JSON.stringify(a.scheme)} — unknown scheme`,
      'scheme registry (exact / upto)');
  }
  // network — CAIP-2
  if (typeof a.network !== 'string' || !RE_CAIP2_EVM.test(a.network)) {
    yield finding('blocker', 'network-caip2', `${at}.network = ${JSON.stringify(a.network)} — not CAIP-2 eip155:<chainId>`,
      'spec: network is a CAIP-2 identifier');
  }
  // amount — decimal string integer, no floats, no negatives
  const amount = dialect === 'v2' ? a.amount : a.maxAmountRequired;
  const amountKey = dialect === 'v2' ? 'amount' : 'maxAmountRequired';
  if (typeof amount !== 'string') {
    yield finding('blocker', 'amount-type', `${at}.${amountKey} must be a decimal STRING (got ${typeof amount})`,
      'P1: integer-string minor units; P2: no floats for money');
  } else if (!RE_DECIMAL_INT.test(amount)) {
    yield finding('blocker', 'amount-format', `${at}.${amountKey} = ${JSON.stringify(amount)} — not a non-negative integer string`,
      'P1/P2: decimal-string minor units');
  } else if (amount === '0') {
    yield finding('major', 'amount-zero', `${at}.${amountKey} is 0 — a free endpoint behind a 402 confuses agents`,
      'challenge semantics');
  }
  // addresses
  for (const key of ['asset', 'payTo']) {
    if (typeof a[key] !== 'string' || !RE_ADDRESS.test(a[key])) {
      yield finding('blocker', `${key}-address`, `${at}.${key} = ${JSON.stringify(a[key])} — not a 0x+40hex address`,
        'EVM scheme: address shape');
    }
  }
  // timeout sanity
  const t = a.maxTimeoutSeconds;
  if (typeof t !== 'number' || !Number.isFinite(t)) {
    yield finding('major', 'timeout-missing', `${at}.maxTimeoutSeconds missing/not a number`, 'envelope');
  } else if (t < 30 || t > 3600) {
    yield finding('minor', 'timeout-range', `${at}.maxTimeoutSeconds = ${t} — outside the sane 30–3600s band`,
      'settle-margin practice (validBefore ≥ now + ~30s)');
  }
  // EIP-712 domain material — without name/version the client cannot sign
  const extra = a.extra;
  if (!extra || typeof extra.name !== 'string' || extra.name === '' ||
      typeof extra.version !== 'string' || extra.version === '') {
    yield finding('major', 'eip712-domain', `${at}.extra lacks name/version — agents cannot construct the EIP-712 domain to sign`,
      'scheme_exact_evm: domain {name,version,chainId,verifyingContract}');
  }
}

function* checkEnvelopeV2(env, probedUrl) {
  if (env.x402Version !== 2) {
    yield finding('blocker', 'version', `x402Version = ${JSON.stringify(env.x402Version)} in a v2 (header) dialect envelope`,
      'v2 envelope: {x402Version:2, error?, resource, accepts, extensions?}');
  }
  const r = env.resource;
  if (!r || typeof r !== 'object' || typeof r.url !== 'string' || r.url === '') {
    yield finding('blocker', 'resource-required', 'v2 envelope missing required resource.url',
      'v2 envelope: resource is REQUIRED (ResourceInfo)');
  } else if (probedUrl && r.url !== probedUrl) {
    yield finding('minor', 'resource-url-match', `resource.url ${JSON.stringify(r.url)} ≠ probed URL ${JSON.stringify(probedUrl)}`,
      'route binding: payload.resource.url should route-match');
  }
  if (!Array.isArray(env.accepts) || env.accepts.length === 0) {
    yield finding('blocker', 'accepts-empty', 'v2 envelope has no accepts entries', 'v2 envelope: accepts (non-empty)');
  } else {
    for (let i = 0; i < env.accepts.length; i++) yield* checkAcceptsEntry(env.accepts[i], i, 'v2');
  }
}

function* checkEnvelopeV1(body) {
  if (body.x402Version !== 1) {
    yield finding('minor', 'version', `body x402Version = ${JSON.stringify(body.x402Version)} (expected 1 for body dialect)`,
      'v1 body envelope');
  }
  if (!Array.isArray(body.accepts) || body.accepts.length === 0) {
    yield finding('blocker', 'accepts-empty', 'v1 body has no accepts entries', 'v1 body envelope: accepts');
  } else {
    for (let i = 0; i < body.accepts.length; i++) yield* checkAcceptsEntry(body.accepts[i], i, 'v1');
  }
}

function* checkEnvelopeBespoke(b) {
  yield finding('blocker', 'non-spec-dialect',
    'challenge is a bespoke v1 dialect (price/recipient/proof shape), not the spec envelope {x402Version, accepts:[...]} — the official x402 clients cannot pay this endpoint; only clients written for this dialect can',
    'spec envelope (v1 body / v2 PAYMENT-REQUIRED header); cf. the B2 finding: teach the dialect you will not hard-cut');
  const amt = b.price && b.price.amount;
  if (typeof amt !== 'string' || !RE_DECIMAL_INT.test(amt)) {
    yield finding('major', 'amount-format', `price.amount = ${JSON.stringify(amt)} — not a decimal-string integer`, 'P1/P2: integer-string minor units');
  }
  for (const [label, v] of [['recipient', b.recipient], ['price.token_address', b.price && b.price.token_address]]) {
    if (typeof v !== 'string' || !RE_ADDRESS.test(v)) {
      yield finding('major', 'address-shape', `${label} = ${JSON.stringify(v)} — not 0x+40hex`, 'EVM scheme: address shape');
    }
  }
  if (!b.eip712 || typeof b.eip712.name !== 'string' || typeof b.eip712.version !== 'string') {
    yield finding('major', 'eip712-domain', 'missing eip712 {name,version} — agents cannot construct the signing domain',
      'scheme_exact_evm domain material');
  }
  if (typeof b.nonce !== 'string' || !/^0x[0-9a-fA-F]{64}$/.test(b.nonce)) {
    yield finding('minor', 'nonce-shape', `nonce = ${JSON.stringify(b.nonce)} — not 0x+64hex (32 bytes)`, 'EIP-3009 nonce shape');
  }
  const exp = typeof b.expires_at === 'number' ? b.expires_at : NaN;
  if (!Number.isFinite(exp)) {
    yield finding('minor', 'expiry-missing', 'no numeric expires_at — agents cannot bound voucher validity', 'RFC 3339 / unix expiry (P4)');
  } else if (exp * 1000 < Date.now()) {
    yield finding('major', 'expired', `challenge already expired (expires_at ${new Date(exp * 1000).toISOString()})`, 'P4');
  }
  if (!b.status_url) {
    yield finding('info', 'status-url', 'no status_url — payers cannot poll ambiguous outcomes', 'ambiguous-money recovery path');
  }
}

function* checkResponseHygiene(res, dialect) {
  const cc = res.headers.get('cache-control') || '';
  if (!/no-store|private/i.test(cc)) {
    yield finding('major', 'cache-hygiene', `402 response Cache-Control is ${JSON.stringify(cc || '(absent)')} — payment challenges must never be cached by intermediaries`,
      'cache hygiene: stamps/challenges are per-issuance');
  }
  const expose = (res.headers.get('access-control-expose-headers') || '').toLowerCase();
  const want = dialect === 'v2'
    ? ['payment-required', 'payment-signature', 'payment-response']
    : ['x-payment', 'x-payment-response'];
  const missing = want.filter(h => !expose.includes(h));
  const acao = res.headers.get('access-control-allow-origin');
  if (!acao) {
    yield finding('minor', 'cors-absent', 'no Access-Control-Allow-Origin — browser-based agents cannot call this endpoint at all',
      'agent-UX: CORS for browser agents');
  } else if (missing.length > 0) {
    yield finding('minor', 'cors-expose', `CORS is on but does not expose ${missing.join(', ')} — browser agents cannot read the payment headers`,
      'agent-UX: Expose-Headers must include the payment dialect');
  }
  if (dialect === 'v1') {
    const dep = res.headers.get('deprecation') || res.headers.get('sunset');
    if (!dep) {
      yield finding('info', 'legacy-sunset', 'v1 (body/X-PAYMENT) dialect without Deprecation/Sunset headers — agents trained on it have no migration signal',
        'RFC 8594; v2 migration practice');
    }
  }
}

/** Decode a base64 (or base64url) header value to parsed JSON, or null. */
function decodeHeaderJson(b64) {
  try {
    const norm = b64.replace(/-/g, '+').replace(/_/g, '/');
    const bin = atob(norm);
    const bytes = Uint8Array.from(bin, c => c.charCodeAt(0));
    return JSON.parse(new TextDecoder().decode(bytes));
  } catch { return null; }
}

/**
 * Check one endpoint. `fetchImpl` injectable for tests/Workers.
 * @param {string} url  the tool/call URL
 * @param {object} opts body: JSON string to POST (default '{}'); method override;
 *                      fetchImpl for tests/Workers
 * @returns {Promise<{url, dialect, grade, score, findings[], probedAt}>}
 */
export async function check(url, { fetchImpl = fetch, method, body = '{}' } = {}) {
  const probedAt = new Date().toISOString();
  const findings = [];
  const doFetch = (m) => fetchImpl(url, {
    method: m,
    headers: { 'content-type': 'application/json', 'accept': 'application/json' },
    body: m === 'POST' ? body : undefined,
    redirect: 'manual',
  });

  let res;
  try {
    res = await doFetch(method || 'POST');
    // many x402 resources are GETs: if POST is not allowed, retry as GET
    if (!method && res.status === 405) res = await doFetch('GET');
  } catch (e) {
    findings.push(finding('blocker', 'reachable', `request failed: ${e.message}`, 'agent must be able to reach the endpoint'));
    return report(url, 'none', findings, probedAt);
  }

  if (res.status === 400 || res.status === 422) {
    // Validation-first (G2 pattern): the endpoint rejects bad input before
    // issuing a challenge. Defensible — but it means price discovery requires
    // a schema-valid request, which generic agents cannot guess.
    findings.push(finding('major', 'validation-first',
      `unpaid probe with a generic body returned HTTP ${res.status} (input validated before challenge) — agents cannot discover the price without already knowing the input schema. Re-probe with a valid body (x402check <url> --body '{...}')`,
      'G2 trade-off: validation-before-challenge vs. price discoverability; publish the schema in /.well-known/openapi.yaml'));
    return report(url, 'gated', findings, probedAt);
  }

  if (res.status !== 402) {
    findings.push(finding('blocker', 'status-402',
      `unpaid request returned HTTP ${res.status}, not 402 — agents cannot discover the payment requirement`,
      'HTTP 402 Payment Required is the discovery signal'));
    return report(url, 'none', findings, probedAt);
  }

  // dialect detection: v2 header wins; then v1 body
  const prHeader = res.headers.get('payment-required');
  let dialect = 'none';
  let env = null;
  if (prHeader) {
    if (prHeader.includes(', ')) {
      findings.push(finding('blocker', 'duplicate-header', 'PAYMENT-REQUIRED appears duplicated (comma-joined) — ambiguous which challenge to sign',
        'single-header rule'));
    }
    env = decodeHeaderJson(prHeader);
    dialect = 'v2';
    if (!env) {
      findings.push(finding('blocker', 'header-decode', 'PAYMENT-REQUIRED is not base64 JSON', 'v2: base64(envelope) in header'));
    }
  } else {
    let body = null;
    try { body = await res.json(); } catch { /* not json */ }
    if (body && Array.isArray(body.accepts)) { dialect = 'v1'; env = body; }
    else if (body && body.price && (body.recipient || body.payTo)) { dialect = 'v1-bespoke'; env = body; }
  }

  if (dialect === 'none') {
    findings.push(finding('blocker', 'dialect',
      'HTTP 402 but neither a PAYMENT-REQUIRED header (v2) nor a body {accepts:[...]} (v1) — the challenge is undiscoverable',
      'challenge must be machine-readable'));
    return report(url, dialect, findings, probedAt);
  }

  findings.push(...(dialect === 'v2' ? checkEnvelopeV2(env, url)
    : dialect === 'v1' ? checkEnvelopeV1(env)
    : checkEnvelopeBespoke(env)));
  findings.push(...checkResponseHygiene(res, dialect));
  return report(url, dialect, findings, probedAt);
}

function report(url, dialect, findings, probedAt) {
  const weights = { blocker: 40, major: 12, minor: 4, info: 0 };
  const score = Math.max(0, 100 - findings.reduce((s, f) => s + weights[f.severity], 0));
  const grade = dialect === 'gated' ? 'N/A'
    : findings.some(f => f.severity === 'blocker') ? 'F'
    : score >= 90 ? 'A' : score >= 75 ? 'B' : score >= 60 ? 'C' : 'D';
  return { url, dialect, grade, score, findings, probedAt };
}
