// Stage 2 reverse direction: verify Rust-generated signatures with viem
// (verifyTypedData) — and cross-check the Rust digest derivation.
// Run: node tests/vectors/gen/verify-rust-vectors.mjs <corpusdir>

import { verifyTypedData, getAddress } from 'viem';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

const dir = process.argv[2];
if (!dir) { console.error('usage: verify-rust-vectors.mjs <corpusdir>'); process.exit(1); }

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

let pass = 0, fail = 0;
for (const f of readdirSync(dir).filter(n => n.endsWith('.json')).sort()) {
  const fx = JSON.parse(readFileSync(join(dir, f), 'utf-8'));
  const ok = await verifyTypedData({
    domain: { ...fx.domain, verifyingContract: getAddress(fx.domain.verifyingContract) },
    types: TYPES,
    primaryType: 'TransferWithAuthorization',
    message: {
      from: getAddress(fx.authorization.from),
      to: getAddress(fx.authorization.to),
      value: BigInt(fx.authorization.value),
      validAfter: BigInt(fx.authorization.validAfter),
      validBefore: BigInt(fx.authorization.validBefore),
      nonce: fx.authorization.nonce,
    },
    address: getAddress(fx.authorization.from),
    signature: fx.signature,
  });
  if (ok) { pass++; console.log(`${fx.name}: viem verifies Rust signature ✓`); }
  else { fail++; console.error(`${fx.name}: viem REJECTED Rust signature ✗`); }
}
if (fail > 0) process.exit(1);
console.log(`reverse-direction vectors: ${pass} verified, ${fail} failed`);
