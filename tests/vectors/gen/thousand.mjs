// The big run: N real settles in waves of W. Telemetry per settle (JSONL).
// Usage: node thousand.mjs <workerUrl> <payerKeyHex> <N> <W> <out.jsonl>
import { privateKeyToAccount } from 'viem/accounts';
import { getAddress } from 'viem';
import { createWriteStream } from 'node:fs';

const [BASE, PAYER, N, W, OUT] = [process.argv[2], process.argv[3], parseInt(process.argv[4]), parseInt(process.argv[5] || '25'), process.argv[6] || 'thousand.jsonl'];
const ROUTE = `${BASE}/v2/tools/vat-mod97-check/call`;
const BODY = JSON.stringify({ input: { vat_number: 'GB123456789' } });
const account = privateKeyToAccount(PAYER.startsWith('0x') ? PAYER : '0x' + PAYER);
const TYPES = { TransferWithAuthorization: [
  { name: 'from', type: 'address' }, { name: 'to', type: 'address' }, { name: 'value', type: 'uint256' },
  { name: 'validAfter', type: 'uint256' }, { name: 'validBefore', type: 'uint256' }, { name: 'nonce', type: 'bytes32' },
]};
const rndHex = (n) => '0x' + [...crypto.getRandomValues(new Uint8Array(n))].map(b => b.toString(16).padStart(2, '0')).join('');
const enc = (o) => Buffer.from(JSON.stringify(o)).toString('base64');
const dec = (s) => JSON.parse(Buffer.from(s, 'base64').toString());
const out = createWriteStream(OUT, { flags: 'w' });

async function oneSettle(i) {
  const rec = { i };
  try {
    const t0 = performance.now();
    const r402 = await fetch(ROUTE, { method: 'POST', headers: { 'content-type': 'application/json' }, body: BODY });
    const prH = r402.headers.get('payment-required');
    rec.ch_ms = +(performance.now() - t0).toFixed(1);
    if (!prH) { rec.err = 'no-402:' + r402.status; out.write(JSON.stringify(rec) + '\n'); return rec; }
    const pr = dec(prH);
    const req = pr.accepts[0];
    const t1 = performance.now();
    const nonce = rndHex(32);
    const sig = await account.signTypedData({
      domain: { name: req.extra.name, version: req.extra.version, chainId: Number(req.network.split(':')[1]), verifyingContract: getAddress(req.asset) },
      types: TYPES, primaryType: 'TransferWithAuthorization',
      message: { from: account.address, to: getAddress(req.payTo), value: BigInt(req.amount), validAfter: 0n, validBefore: 4102444800n, nonce },
    });
    rec.sign_ms = +(performance.now() - t1).toFixed(1);
    const t2 = performance.now();
    const r = await fetch(ROUTE, { method: 'POST', headers: { 'content-type': 'application/json', 'PAYMENT-SIGNATURE': enc({ x402Version: 2, accepted: req, payload: { signature: sig, authorization: { from: account.address, to: req.payTo, value: req.amount, validAfter: '0', validBefore: '4102444800', nonce } }, extensions: pr.extensions }) }, body: BODY });
    rec.status = r.status; rec.pay_ms = +(performance.now() - t2).toFixed(1);
    if (r.status === 200) { const j = await r.json(); rec.tx = j.settlement?.transaction?.slice(0, 18); }
    else { try { const j = await r.json(); rec.err = j.error?.code; } catch { rec.err = 'body?'; } }
  } catch (e) { rec.err = 'exc:' + String(e).slice(0, 60); }
  out.write(JSON.stringify(rec) + '\n');
  return rec;
}

const t0 = Date.now();
let done = 0;
const waves = Math.ceil(N / W);
for (let wv = 0; wv < waves; wv++) {
  const n = Math.min(W, N - done);
  const wt = Date.now();
  const rs = await Promise.all(Array.from({ length: n }, (_, k) => oneSettle(wv * W + k)));
  done += n;
  const ok = rs.filter(r => r.status === 200).length;
  const errs = {};
  rs.filter(r => r.status !== 200).forEach(r => { const e = r.err || r.status; errs[e] = (errs[e] || 0) + 1; });
  console.log(`wave ${wv + 1}/${waves}: ${ok}/${n} ok, wave ${((Date.now() - wt) / 1000).toFixed(1)}s, total ${done}, elapsed ${((Date.now() - t0) / 1000).toFixed(0)}s${Object.keys(errs).length ? ' errs=' + JSON.stringify(errs) : ''}`);
  if (rs.length && rs.every(r => r.err === 'insufficient_funds')) { console.log('WALLET EMPTY — stopping'); break; }
}
console.log('RUN_DONE total_ms=' + (Date.now() - t0));
out.end();
