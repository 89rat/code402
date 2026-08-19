// CDP Base Sepolia ETH faucet (POST /platform/v2/evm/faucet, EdDSA JWT auth —
// the node twin of crates/edge/src/facilitator.rs mint_jwt).
// Usage: node faucet.mjs <addressHex> [keyIdFile secretFile]
import { readFileSync } from 'node:fs';
import { ed25519 } from '@noble/curves/ed25519';

const ADDR = process.argv[2];
const KID = readFileSync(process.argv[3] || process.env.HOME + '/.zcode/secrets/cdp-key-id', 'utf8').trim();
const SEC = readFileSync(process.argv[4] || process.env.HOME + '/.zcode/secrets/cdp-secret', 'utf8').trim();

const kp = Buffer.from(SEC, 'base64');
if (kp.length !== 64) { console.error('secret must be 64-byte keypair b64'); process.exit(1); }
const seed = kp.slice(0, 32); // noble re-derives the same expanded key
const b64u = (b) => Buffer.from(b).toString('base64url');
const now = Math.floor(Date.now() / 1000);
const header = { alg: 'EdDSA', typ: 'JWT', kid: KID, nonce: 'n' + now };
const claims = { sub: KID, iss: 'cdp', aud: ['cdp_service'], nbf: now, exp: now + 120, uri: 'POST api.cdp.coinbase.com/platform/v2/evm/faucet' };
const signing = `${b64u(JSON.stringify(header))}.${b64u(JSON.stringify(claims))}`;
const sig = ed25519.sign(Buffer.from(signing), seed);
const jwt = `${signing}.${b64u(Buffer.from(sig))}`;

const r = await fetch('https://api.cdp.coinbase.com/platform/v2/evm/faucet', {
  method: 'POST',
  headers: { 'content-type': 'application/json', Authorization: `Bearer ${jwt}` },
  body: JSON.stringify({ address: ADDR, network: 'base-sepolia', token: 'eth' }),
});
const body = await r.text();
console.log('faucet', r.status, body.slice(0, 300));
