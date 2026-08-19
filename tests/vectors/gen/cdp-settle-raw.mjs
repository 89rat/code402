// Raw CDP /settle probe: replays a (consumed) payment to capture the EXACT
// facilitator response shape that the route must classify.
// Usage: node cdp-settle-raw.mjs <headerFile>
import { readFileSync } from 'node:fs';
import { ed25519 } from '@noble/curves/ed25519';

const header = readFileSync(process.argv[2], 'utf8').trim();
const payload = JSON.parse(Buffer.from(header, 'base64').toString());
const KID = readFileSync(process.env.HOME + '/.zcode/secrets/cdp-key-id', 'utf8').trim();
const SEC = readFileSync(process.env.HOME + '/.zcode/secrets/cdp-secret', 'utf8').trim();
const kp = Buffer.from(SEC, 'base64');
const seed = kp.slice(0, 32);
const b64u = (b) => Buffer.from(b).toString('base64url');
const now = Math.floor(Date.now() / 1000);
const headerJwt = { alg: 'EdDSA', typ: 'JWT', kid: KID, nonce: 'n' + now };
const claims = { sub: KID, iss: 'cdp', aud: ['cdp_service'], nbf: now, exp: now + 120, uri: 'POST api.cdp.coinbase.com/platform/v2/x402/settle' };
const signing = `${b64u(JSON.stringify(headerJwt))}.${b64u(JSON.stringify(claims))}`;
const jwt = `${signing}.${b64u(Buffer.from(ed25519.sign(Buffer.from(signing), seed)))}`;

const body = JSON.stringify({ x402Version: 2, paymentPayload: payload, paymentRequirements: payload.accepted });
const r = await fetch('https://api.cdp.coinbase.com/platform/v2/x402/settle', {
  method: 'POST', headers: { 'content-type': 'application/json', Authorization: `Bearer ${jwt}` }, body,
});
const text = await r.text();
console.log('HTTP', r.status);
try { console.log(JSON.stringify(JSON.parse(text), null, 1).slice(0, 900)); } catch { console.log(text.slice(0, 900)); }
