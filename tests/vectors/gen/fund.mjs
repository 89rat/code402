// Fund both payer wallets via the CDP faucet (alternating keys A/B to also
// probe per-key faucet limits). Usage: node fund.mjs <addr1> <addr2> <drips-each>
import { createPrivateKey, sign } from 'node:crypto';
const KEYS = [
  ['08df756f-dfa4-4c74-b1f5-bd9f5216c64b', 'hAskp2x3BEPY1XSBcGlYW0GFuyw9UKGdp6a5elVrHuutUWYMUoC3vfI1UJJi1neif3JNiB0Tp7AceLIiP0IMJw=='],
  ['880fa796-bb4e-4cb0-b5e3-0e7fdfc7293f', 'w5mDSJqOzwDA/cVl2de5Q5zbeTFHZP0/t/0SQ5S7UY7Va0k7P2Fuxc61NGWR81orJVgp1f1ou9o1TvaFOqfFRw=='],
];
import { privateKeyToAccount } from 'viem/accounts';
import { readFileSync } from 'node:fs';
const addr1 = privateKeyToAccount('0x' + readFileSync(process.argv[2], 'utf8').trim()).address;
const addr2 = privateKeyToAccount('0x' + readFileSync(process.argv[3], 'utf8').trim()).address;
const nEach = parseInt(process.argv[4] || '9');
const mint = (id, sec, uri) => {
  const kp = Buffer.from(sec, 'base64');
  const pkcs8 = Buffer.concat([Buffer.from('302e020100300506032b657004220420','hex'), kp.subarray(0,32)]);
  const priv = createPrivateKey({ key: pkcs8, format: 'der', type: 'pkcs8' });
  const b64u = (o) => Buffer.from(JSON.stringify(o)).toString('base64url');
  const now = Math.floor(Date.now()/1000);
  const sg = b64u({alg:'EdDSA',typ:'JWT',kid:id,nonce:crypto.randomUUID()})+'.'+b64u({sub:id,iss:'cdp',aud:['cdp_service'],nbf:now,exp:now+120,uri});
  return sg+'.'+sign(null, Buffer.from(sg), priv).toString('base64url');
};
const drip = async (addr, i) => {
  const [id, sec] = KEYS[i % 2];
  const jwt = mint(id, sec, 'POST api.cdp.coinbase.com/platform/v2/evm/faucet');
  const r = await fetch('https://api.cdp.coinbase.com/platform/v2/evm/faucet', {
    method:'POST', headers:{'Authorization':'Bearer '+jwt,'content-type':'application/json'},
    body: JSON.stringify({ address: addr, network:'base-sepolia', token:'usdc' }) });
  const t = await r.text();
  console.log(`drip ${i}: ${addr.slice(0,8)} key=${i%2?'B':'A'} -> ${r.status} ${t.slice(0,90)}`);
};
for (let i = 0; i < nEach; i++) { await drip(addr1, i); await drip(addr2, i + 1); await new Promise(r => setTimeout(r, 1500)); }
console.log('done');
