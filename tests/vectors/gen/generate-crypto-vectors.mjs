// Stage 2 crypto vector generator — TS→Rust direction.
// Signs EIP-3009 TransferWithAuthorization authorizations with viem (the
// reference wallet stack the official SDK uses), across both real USDC
// domains, in every v encoding, plus forgeries and envelopes.
// Output: crates/core/tests/vectors/crypto/*.json — consumed by the Rust
// test suite (x402v2_crypto_vectors.rs). Fixtures embed keys/values: the
// files ARE the determinism; rerunning regenerates only if deps change.
//
// Run: node tests/vectors/gen/generate-crypto-vectors.mjs
// (requires: npm i viem@2 @x402/fetch@2.22.0 — see gen/package.json)

import { privateKeyToAccount } from 'viem/accounts';
import { getAddress } from 'viem';
import { writeFileSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = join(HERE, '..', '..', '..', 'crates', 'core', 'tests', 'vectors', 'crypto');

// Real token domains (from code402 wrangler.toml + scheme spec):
const SEPOLIA = {
  name: 'USDC', version: '2', chainId: 84532,
  verifyingContract: getAddress('0x036cbd53842c5426634e7929541ec2318f3dcf7e'),
};
const BASE = {
  name: 'USD Coin', version: '2', chainId: 8453,
  verifyingContract: getAddress('0x833589fcd6edb6e08f4c7c32d4f71b54bda02913'),
};

const TYPES = {
  TransferWithAuthorization: [
    { name: 'from', type: 'address' },
    { name: 'to', type: 'address' },
    { name: 'value', type: 'uint256' },
    { name: 'validAfter', type: 'uint256' },
    { name: 'validBefore', type: 'uint256' },
    { name: 'nonce', type: 'bytes32' },
  ],
};

// Deterministic payer keys (test-only; fixtures embed them openly).
const PAYER_A = '0x' + '4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318';
const PAYER_B = '0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d'; // anvil test key #1 (valid secp256k1)

function requirement(domain, payTo, amount = '10000') {
  return {
    scheme: 'exact',
    network: `eip155:${domain.chainId}`,
    amount,
    asset: domain.verifyingContract,
    payTo,
    maxTimeoutSeconds: 60,
    extra: {
      name: domain.name, version: domain.version,
      assetTransferMethod: 'eip3009', paymentFlow: 'upfront',
    },
  };
}

function authorization(account, to, value = '10000') {
  return {
    from: account.address,
    to,
    value,
    validAfter: '1740672000',
    validBefore: '1740672400',
    nonce: '0x' + 'f3746613c2d920b5fdabc0856f2aeb2d4f88ee6037b8cc5d04a71a4462f13480',
  };
}

async function sign(account, domain, auth) {
  return await account.signTypedData({
    domain,
    types: TYPES,
    primaryType: 'TransferWithAuthorization',
    message: {
      from: auth.from, to: auth.to, value: BigInt(auth.value),
      validAfter: BigInt(auth.validAfter), validBefore: BigInt(auth.validBefore),
      nonce: auth.nonce,
    },
  });
}

// v-encoding transforms on a 65-byte sig
function withV(sig, vByte) {
  return sig.slice(0, -2) + vByte.toString(16).padStart(2, '0');
}
// EIP-2 malleable twin: s' = n - s, v flipped (0<->1)
const N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141n;
function malleableTwin(sig) {
  const r = BigInt('0x' + sig.slice(2, 66));
  const s = BigInt('0x' + sig.slice(66, 130));
  const vRaw = parseInt(sig.slice(130, 132), 16); // 0/1, 27/28, 28(0x1c)/29(0x1d)
  const yParity = vRaw === 0 || vRaw === 27 || vRaw === 28 ? (vRaw % 27) : vRaw - 28; // normalize to 0/1
  const s2 = N - s;
  const v2 = (1 - yParity) + 27; // flipped parity, 27/28 form
  return '0x' + r.toString(16).padStart(64, '0') + s2.toString(16).padStart(64, '0') + v2.toString(16).padStart(2, '0');
}

const PAY_TO = getAddress('0x3bca128282a1de2f74efc16fa44a32a6f88a72ff');
const fixtures = [];

const payerA = privateKeyToAccount(PAYER_A);
const payerB = privateKeyToAccount(PAYER_B);

async function base_case(name, note) {
  const auth = authorization(payerA, PAY_TO);
  const sig = await sign(payerA, SEPOLIA, auth);
  return { name, note, domain: SEPOLIA, requirement: requirement(SEPOLIA, PAY_TO), authorization: auth, signature: sig, expected: 'local_pass', payerKey: PAYER_A };
}

// 1. Sepolia USDC pass
fixtures.push(await base_case('sepolia_usdc_pass', 'canonical EOA pass, v as emitted by viem'));
// 2. Base USD Coin pass (domain divergence)
{
  const auth = authorization(payerA, PAY_TO);
  const sig = await sign(payerA, BASE, auth);
  fixtures.push({ name: 'base_usdcoin_pass', note: 'mainnet domain name divergence (USD Coin vs USDC)', domain: BASE, requirement: requirement(BASE, PAY_TO), authorization: auth, signature: sig, expected: 'local_pass', payerKey: PAYER_A });
}
// 3+4. v-normalization: 27/28 and 0/1 encodings both pass
{
  const c = await base_case('v_27form', 'same signature re-encoded with EIP-155 style v=27/28');
  const vRaw = parseInt(c.signature.slice(130, 132), 16);
  const vv = vRaw <= 1 ? vRaw + 27 : 27 + ((vRaw - 27) % 2); // to 27/28 form
  c.signature = withV(c.signature, vv);
  fixtures.push(c);
  const c2 = await base_case('v_0form', 'same signature re-encoded with compact v=0/1');
  const v0 = vv - 27;
  c2.signature = withV(c2.signature, v0);
  fixtures.push(c2);
}
// 5. wrong chain: signed Sepolia, requirement says Base
{
  const auth = authorization(payerA, PAY_TO);
  const sig = await sign(payerA, SEPOLIA, auth);
  fixtures.push({ name: 'wrong_chain', note: 'signature for Sepolia domain verified under Base requirement -> signer mismatch', domain: SEPOLIA, requirement: requirement(BASE, PAY_TO), authorization: auth, signature: sig, expected: 'local_reject', payerKey: PAYER_A });
}
// 6. wrong token: same chain, different verifyingContract
{
  const auth = authorization(payerA, PAY_TO);
  const sig = await sign(payerA, SEPOLIA, auth);
  const fakeReq = requirement(SEPOLIA, PAY_TO);
  fakeReq.asset = getAddress('0x833589fcd6edb6e08f4c7c32d4f71b54bda02913'); // base USDC addr on sepolia chain id
  fixtures.push({ name: 'wrong_token', note: 'domain verifyingContract mismatch -> signer mismatch', domain: SEPOLIA, requirement: fakeReq, authorization: auth, signature: sig, expected: 'local_reject', payerKey: PAYER_A });
}
// 7. wrong signer: from = payerB, signed by payerA
{
  const auth = authorization(payerB, PAY_TO);
  const sig = await sign(payerA, SEPOLIA, auth);
  fixtures.push({ name: 'wrong_signer', note: 'declared from != actual signer', domain: SEPOLIA, requirement: requirement(SEPOLIA, PAY_TO), authorization: auth, signature: sig, expected: 'local_reject', payerKey: PAYER_A });
}
// 8. malleable twin: high-s variant must be REJECTED (EIP-2)
{
  const c = await base_case('high_s_malleable', 'EIP-2 malleable twin (s->n-s, v flipped) must be rejected');
  c.signature = malleableTwin(c.signature);
  c.expected = 'local_reject';
  fixtures.push(c);
}
// 9. garbage 65 bytes
{
  const c = await base_case('garbage_sig', 'random 65 bytes -> recovery failure');
  c.signature = '0x' + 'ab'.repeat(65);
  c.expected = 'local_reject';
  fixtures.push(c);
}
// 10. 6492 envelope: sig + magic suffix -> pass through
{
  const c = await base_case('envelope_6492', 'EOA sig + EIP-6492 magic suffix -> facilitator pass-through');
  c.signature = c.signature + '6492649264926492649264926492649264926492649264926492649264926492';
  c.expected = 'pass_through';
  fixtures.push(c);
}

// 11. long non-magic hex: NOT a 6492 envelope -> local reject (quota guard)
{
  const c = await base_case('long_non_magic', '>65B hex without the EIP-6492 magic suffix must be locally rejected');
  c.signature = '0x' + 'ab'.repeat(100); // 100 bytes, no magic
  c.expected = 'local_reject';
  fixtures.push(c);
}
// 12. invalid v byte (valid r/s shape, v=2) -> local reject
{
  const c = await base_case('invalid_v', 'recovery id outside {0,1,27,28} must be rejected');
  c.signature = withV(c.signature, 2);
  c.expected = 'local_reject';
  fixtures.push(c);
}

mkdirSync(OUT, { recursive: true });
for (const f of fixtures) {
  writeFileSync(join(OUT, f.name + '.json'), JSON.stringify(f, null, 2));
}
console.log(`wrote ${fixtures.length} fixtures to ${OUT}`);
