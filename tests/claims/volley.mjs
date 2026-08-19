// Claims-verification battery — turns cdp-findings claims into measurements.
// Two CDP keys (A/B) let us volley: per-key limits, cross-key independence.
// Keys come from env/files — NEVER hardcoded. Testnet only, our own wallets.
//
// Claims under test (reviews/cdp-findings.md):
//   C1  verify is always free           -> 50 verifies, watch for quota errors
//   C2  rate limits UNKNOWN             -> burst /supported + verify, record 429s
//   C3  limits bind per key             -> same burst on A then B
//   C4  JWT exp 120s enforced           -> stale JWT must 401
//   C5  latency is block-bound (Law 1)  -> N paid calls, distribution
//   C6  insufficient_funds taxonomy     -> empty-wallet settle, exact errorReason
//   C7  /supported shape stability      -> full kinds/extensions snapshot
//
// Usage: node tests/claims/volley.mjs <keyIdA> <secretA> <keyIdB> <secretB>
//        SETTLE_* envs optional for C5/C6 (see run-claims.sh).

import { createPrivateKey, sign } from 'node:crypto';

const [keyIdA, secA, keyIdB, secB] = process.argv.slice(2);
const keys = { A: { id: keyIdA, sec: secA }, B: { id: keyIdB, sec: secB } };
const HOST = 'api.cdp.coinbase.com';
const BASE = `https://${HOST}/platform`;
const results = {};
const log = (...a) => console.log(...a);

function jwt(k, uri, { skew = 0 } = {}) {
  const kp = Buffer.from(k.sec, 'base64');
  const pkcs8 = Buffer.concat([Buffer.from('302e020100300506032b657004220420', 'hex'), kp.subarray(0, 32)]);
  const priv = createPrivateKey({ key: pkcs8, format: 'der', type: 'pkcs8' });
  const b64u = (o) => Buffer.from(JSON.stringify(o)).toString('base64url');
  const now = Math.floor(Date.now() / 1000);
  const header = { alg: 'EdDSA', typ: 'JWT', kid: k.id, nonce: crypto.randomUUID() };
  const claims = { sub: k.id, iss: 'cdp', aud: ['cdp_service'], nbf: now, exp: now + 120 + skew, uri };
  const signing = b64u(header) + '.' + b64u(claims);
  return signing + '.' + sign(null, Buffer.from(signing), priv).toString('base64url');
}

async function call(k, path, body, { skew } = {}) {
  const method = body ? 'POST' : 'GET';
  const uri = `${method} ${HOST}/platform${path}`;
  const r = await fetch(BASE + path, {
    method,
    headers: { 'Authorization': 'Bearer ' + jwt(k, uri, { skew }), 'content-type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await r.text();
  return { status: r.status, body: text };
}

// C7: supported snapshot
{
  const a = await call(keys.A, '/v2/x402/supported');
  const b = await call(keys.B, '/v2/x402/supported');
  const ja = JSON.parse(a.body), jb = JSON.parse(b.body);
  results.C7_supported = {
    both_200: a.status === 200 && b.status === 200,
    identical: JSON.stringify(ja) === JSON.stringify(jb),
    extensions: ja.extensions,
    kinds_count: ja.kinds.length,
    v2_kinds: ja.kinds.filter(x => x.x402Version === 2).map(x => `${x.network}:${x.scheme}`),
    v1_kinds: ja.kinds.filter(x => x.x402Version === 1).map(x => `${x.network}:${x.scheme}`).slice(0, 6),
  };
  log('C7 supported:', JSON.stringify(results.C7_supported, null, 1).slice(0, 600));
}

// C4: stale JWT must 401
{
  const stale = await call(keys.A, '/v2/x402/supported', undefined, { skew: -300 });
  results.C4_stale_jwt = { status: stale.status, rejected: stale.status === 401 };
  log('C4 stale JWT ->', stale.status);
}

// C1+C2+C3: verify freeness + rate limits, per key, volleyed
// (verify with a dummy-but-wellformed payload: invalid payment -> 200 isValid:false
//  is still a VERIFY served; quota errors would surface as 429/402)
{
  const dummyVerify = {
    x402Version: 2,
    paymentPayload: {
      x402Version: 2,
      accepted: { scheme: 'exact', network: 'eip155:84532', amount: '1', asset: '0x036CbD53842c5426634e7929541eC2318f3dCF7e', payTo: '0x0000000000000000000000000000000000000001', maxTimeoutSeconds: 60 },
      payload: { signature: '0x' + '00'.repeat(65), authorization: { from: '0x0000000000000000000000000000000000000001', to: '0x0000000000000000000000000000000000000001', value: '1', validAfter: '0', validBefore: '9999999999', nonce: '0x' + '00'.repeat(32) } },
    },
    paymentRequirements: { scheme: 'exact', network: 'eip155:84532', amount: '1', asset: '0x036CbD53842c5426634e7929541eC2318f3dCF7e', payTo: '0x0000000000000000000000000000000000000001', maxTimeoutSeconds: 60 },
  };
  const burst = async (k, tag, n) => {
    const codes = {};
    const t0 = Date.now();
    for (let i = 0; i < n; i++) {
      const r = await call(k, '/v2/x402/verify', dummyVerify);
      codes[r.status] = (codes[r.status] || 0) + 1;
    }
    return { key: tag, n, ms: Date.now() - t0, codes };
  };
  results.C1C2_verify = [];
  results.C1C2_verify.push(await burst(keys.A, 'A', 25));
  results.C1C2_verify.push(await burst(keys.B, 'B', 25));
  // volley: alternate rapid-fire A/B 30x total
  const volleyCodes = {};
  for (let i = 0; i < 30; i++) {
    const k = i % 2 === 0 ? keys.A : keys.B;
    const r = await call(k, '/v2/x402/verify', dummyVerify);
    volleyCodes[`${i % 2 === 0 ? 'A' : 'B'}:${r.status}`] = (volleyCodes[`${i % 2 === 0 ? 'A' : 'B'}:${r.status}`] || 0) + 1;
  }
  results.C1C2_volley = volleyCodes;
  log('C1/C2/C3 verify bursts + volley:', JSON.stringify(results.C1C2_verify), JSON.stringify(volleyCodes));
}

// settle + latency (C5/C6) are driven from the worker e2e, not here —
// this script covers the direct-API claims. See run-claims.sh for the rest.
console.log('RESULT_JSON_START');
console.log(JSON.stringify(results, null, 2));
console.log('RESULT_JSON_END');
