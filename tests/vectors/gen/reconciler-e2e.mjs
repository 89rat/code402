// RECONCILER-SPEC v1 §6 test 6 — live Sepolia e2e driver (scenarios a–d).
//   a) oob:         settle OUT-OF-BAND on-chain, then pay the route with the
//                   same (now consumed) authorization -> facilitator reports
//                   already-used -> receipt_pending wedge (the 122-phantom
//                   production shape). Cron resolves -> settled_reconciled
//                   + entitlement; `repost` then serves FREE, once.
//   b) cancel:      cancelAuthorization on-chain first -> route wedge ->
//                   cron resolves -> failed_canceled.
//   c) wedge-short: valid window ~40s; against a broken-facilitator dev the
//                   settle transport-errors -> settling wedge; cron (after
//                   expiry) -> failed_expired.
//   d) wedge-long:  far validity; broken-facilitator wedge; on a healthy
//                   instance the cron RE-DRIVES the settle (real CDP), then
//                   `repost` serves FREE via the entitlement.
//
// Usage: node reconciler-e2e.mjs <BASE> <payerKeyHex> <mode> <headerFile.json>
import { privateKeyToAccount } from 'viem/accounts';
import { getAddress, createPublicClient, createWalletClient, http } from 'viem';
import { baseSepolia } from 'viem/chains';
import { writeFileSync, readFileSync } from 'node:fs';

const [BASE, KEY, MODE, ART] = [process.argv[2], process.argv[3], process.argv[4], process.argv[5]];
const ROUTE = `${BASE}/v2/tools/vat-mod97-check/call`;
const BODY = JSON.stringify({ input: { vat_number: 'GB123456789' } });
const RPC = 'https://sepolia.base.org';
const account = MODE === 'repost' ? null : privateKeyToAccount(KEY.startsWith('0x') ? KEY : '0x' + KEY);
const pub = createPublicClient({ chain: baseSepolia, transport: http(RPC) });
const wc = createWalletClient({ account, chain: baseSepolia, transport: http(RPC) });

const TW_TYPES = { TransferWithAuthorization: [
  { name: 'from', type: 'address' }, { name: 'to', type: 'address' }, { name: 'value', type: 'uint256' },
  { name: 'validAfter', type: 'uint256' }, { name: 'validBefore', type: 'uint256' }, { name: 'nonce', type: 'bytes32' },
]};
const CX_TYPES = { CancelAuthorization: [
  { name: 'authorizer', type: 'address' }, { name: 'nonce', type: 'bytes32' },
]};
const USDC_ABI = [
  { name: 'transferWithAuthorization', type: 'function', stateMutability: 'nonpayable', inputs: [
    { name: 'from', type: 'address' }, { name: 'to', type: 'address' }, { name: 'value', type: 'uint256' },
    { name: 'validAfter', type: 'uint256' }, { name: 'validBefore', type: 'uint256' },
    { name: 'nonce', type: 'bytes32' }, { name: 'signature', type: 'bytes' }], outputs: [] },
  { name: 'cancelAuthorization', type: 'function', stateMutability: 'nonpayable', inputs: [
    { name: 'authorizer', type: 'address' }, { name: 'nonce', type: 'bytes32' },
    { name: 'v', type: 'uint8' }, { name: 'r', type: 'bytes32' }, { name: 's', type: 'bytes32' }], outputs: [] },
];

const enc = (o) => Buffer.from(JSON.stringify(o)).toString('base64');
const dec = (s) => JSON.parse(Buffer.from(s, 'base64').toString());
const rndHex = (n) => '0x' + [...crypto.getRandomValues(new Uint8Array(n))].map(b => b.toString(16).padStart(2, '0')).join('');

async function get402() {
  const r = await fetch(ROUTE, { method: 'POST', headers: { 'content-type': 'application/json' }, body: BODY });
  const prH = r.headers.get('payment-required');
  if (!prH) throw new Error('no 402: ' + r.status);
  return dec(prH);
}

async function signAuth(req, nonce, validBefore) {
  return account.signTypedData({
    domain: { name: req.extra.name, version: req.extra.version, chainId: Number(req.network.split(':')[1]), verifyingContract: getAddress(req.asset) },
    types: TW_TYPES, primaryType: 'TransferWithAuthorization',
    message: { from: account.address, to: getAddress(req.payTo), value: BigInt(req.amount), validAfter: 0n, validBefore, nonce },
  });
}

async function postPayment(header) {
  const headers = { 'content-type': 'application/json', 'PAYMENT-SIGNATURE': header };
  const r = await fetch(ROUTE, { method: 'POST', headers, body: BODY });
  let j = {}; try { j = await r.json(); } catch {}
  return { status: r.status, err: j.error?.code || null, tx: j.settlement?.transaction || null, has_output: !!j.output };
}

switch (MODE) {
  case 'oob': {
    const pr = await get402();
    const req = pr.accepts[0];
    const nonce = rndHex(32);
    const vb = 4102444800n; // 2100: far validity
    const sig = await signAuth(req, nonce, vb);
    const hash = await wc.writeContract({
      address: req.asset, abi: USDC_ABI, functionName: 'transferWithAuthorization',
      nonce: await pub.getTransactionCount({ address: account.address, blockTag: 'latest' }),
      args: [account.address, getAddress(req.payTo), BigInt(req.amount), 0n, vb, nonce, sig],
    });
    const rcpt = await pub.waitForTransactionReceipt({ hash });
    const header = enc({
      x402Version: 2, accepted: req, extensions: pr.extensions,
      payload: { signature: sig, authorization: { from: account.address, to: req.payTo, value: req.amount, validAfter: '0', validBefore: String(vb), nonce } },
    });
    writeFileSync(ART, header);
    const out = await postPayment(header);
    console.log(JSON.stringify({ mode: MODE, payer: account.address, nonce, oob_tx: hash, oob_status: rcpt.status, ...out }));
    break;
  }
  case 'cancel': {
    const pr = await get402();
    const req = pr.accepts[0];
    const nonce = rndHex(32);
    const vb = 4102444800n;
    const sig = await signAuth(req, nonce, vb);
    const cxSig = await account.signTypedData({
      domain: { name: req.extra.name, version: req.extra.version, chainId: Number(req.network.split(':')[1]), verifyingContract: getAddress(req.asset) },
      types: CX_TYPES, primaryType: 'CancelAuthorization',
      message: { authorizer: account.address, nonce },
    });
    const v = parseInt(cxSig.slice(130, 132), 16);
    const hash = await wc.writeContract({
      address: req.asset, abi: USDC_ABI, functionName: 'cancelAuthorization',
      nonce: await pub.getTransactionCount({ address: account.address, blockTag: 'latest' }),
      args: [account.address, nonce, v, ('0x' + cxSig.slice(2, 66)), ('0x' + cxSig.slice(66, 130))],
    });
    const rcpt = await pub.waitForTransactionReceipt({ hash });
    const header = enc({
      x402Version: 2, accepted: req, extensions: pr.extensions,
      payload: { signature: sig, authorization: { from: account.address, to: req.payTo, value: req.amount, validAfter: '0', validBefore: String(vb), nonce } },
    });
    writeFileSync(ART, header);
    const out = await postPayment(header);
    console.log(JSON.stringify({ mode: MODE, payer: account.address, nonce, cancel_tx: hash, cancel_status: rcpt.status, ...out }));
    break;
  }
  case 'wedge-short':
  case 'wedge-long': {
    const pr = await get402();
    const req = pr.accepts[0];
    const nonce = rndHex(32);
    const vb = MODE === 'wedge-short' ? BigInt(Math.floor(Date.now() / 1000) + 40) : 4102444800n;
    const sig = await signAuth(req, nonce, vb);
    const header = enc({
      x402Version: 2, accepted: req, extensions: pr.extensions,
      payload: { signature: sig, authorization: { from: account.address, to: req.payTo, value: req.amount, validAfter: '0', validBefore: String(vb), nonce } },
    });
    writeFileSync(ART, header);
    const out = await postPayment(header);
    console.log(JSON.stringify({ mode: MODE, payer: account.address, nonce, valid_before: String(vb), ...out }));
    break;
  }
  case 'repost': {
    const header = readFileSync(ART, 'utf8').trim();
    const out = await postPayment(header);
    console.log(JSON.stringify({ mode: MODE, ...out }));
    break;
  }
  default:
    console.error('modes: oob | cancel | wedge-short | wedge-long | repost');
    process.exit(2);
}
