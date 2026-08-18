// Differential codec fuzz — our Rust codec vs the official @x402/core codec.
// Seeded PRNG (deterministic corpus); asserts semantic equality both ways and
// reports byte-level divergence (informational: JSON object order is
// semantically irrelevant; our Stage-1 vectors pin OUR order to the spec).
// Lives in tests/vectors/gen/ (its package.json holds the deps).
// Run: node tests/vectors/gen/differential.mjs [rust_exe] [N]

import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';

const require = createRequire(import.meta.url);
const core = require('@x402/core/http');
// official http module exports encode/decode for the headers
const { encodePaymentRequiredHeader, decodePaymentRequiredHeader } = core;

const rustExe = process.argv[2] || resolve('target/debug/examples/codec_roundtrip.exe');
const N = parseInt(process.argv[3] || '200', 10);

// seeded xorshift — deterministic corpus
let s = 0x402402 >>> 0 || 1;
const r = () => { s ^= s << 13; s >>>= 0; s ^= s >> 17; s ^= s << 5; s >>>= 0; return s; };
const pick = (arr) => arr[r() % arr.length];
const hexAddr = () => '0x' + Array.from({ length: 40 }, () => '0123456789abcdef'[r() % 16]).join('');
const dec = (n) => String(r() % n);

function randomEnvelope() {
  const e = {
    x402Version: 2,
    resource: {
      url: `https://api.example.com/tool-${r() % 1000}/call`,
      description: 'differential fuzz resource',
      mimeType: 'application/json',
    },
    accepts: [{
      scheme: 'exact',
      network: pick(['eip155:84532', 'eip155:8453']),
      amount: dec(10_000_000),
      asset: hexAddr(),
      payTo: hexAddr(),
      maxTimeoutSeconds: 30 + (r() % 300),
      extra: {
        name: pick(['USDC', 'USD Coin']),
        version: '2',
        assetTransferMethod: 'eip3009',
        paymentFlow: 'upfront',
      },
    }],
    extensions: {},
  };
  if (r() % 2) e.error = 'PAYMENT-SIGNATURE header is required';
  return e;
}

const corpus = Array.from({ length: N }, randomEnvelope);
const official = corpus.map((e) => encodePaymentRequiredHeader(e));

// Rust roundtrip: official-encoded -> Rust decode -> Rust re-encode
const rustOut = execFileSync(rustExe, { input: official.join('\n') + '\n', maxBuffer: 64 * 1024 * 1024 })
  .toString().trim().split('\n');

let semanticFail = 0, byteDiverge = 0, rustErr = 0;
for (let i = 0; i < N; i++) {
  const line = rustOut[i] || 'ERR missing';
  if (!line.startsWith('OK ')) { rustErr++; console.error(`[${i}] rust error: ${line}`); continue; }
  const rustB64 = line.slice(3);
  if (rustB64 !== official[i]) {
    byteDiverge++;
    if (byteDiverge <= 3) console.log(`[${i}] byte divergence (informational)`);
  }
  // semantic check: order-insensitive deep equality (JSON object order is
  // semantically irrelevant; Stage-1 vectors pin OUR order separately)
  const sortKeys = (v) => {
    if (Array.isArray(v)) return v.map(sortKeys);
    if (v && typeof v === 'object') {
      const o = {}; for (const k of Object.keys(v).sort()) o[k] = sortKeys(v[k]); return o;
    }
    return v;
  };
  const decoded = decodePaymentRequiredHeader(rustB64);
  const da = JSON.stringify(sortKeys(decoded)), db = JSON.stringify(sortKeys(corpus[i]));
  if (da !== db) {
    semanticFail++;
    if (semanticFail <= 3) {
      for (let j = 0; j < Math.max(da.length, db.length); j++) {
        if (da[j] !== db[j]) { console.error(`[${i}] SEMANTIC DIFF at ${j}:\n  rust: ...${da.slice(Math.max(0, j - 30), j + 50)}\n  orig: ...${db.slice(Math.max(0, j - 30), j + 50)}`); break; }
      }
    }
  }
  if (rustB64 !== official[i] && byteDiverge === 1) {
    // surface the first byte-divergence detail for the record
    const plain = Buffer.from(rustB64, 'base64').toString();
    const officialPlain = Buffer.from(official[i], 'base64').toString();
    for (let j = 0; j < Math.max(plain.length, officialPlain.length); j++) {
      if (plain[j] !== officialPlain[j]) { console.log(`byte-divergence detail [${i}] at ${j}:\n  rust: ...${plain.slice(Math.max(0, j - 30), j + 50)}\n  ofic: ...${officialPlain.slice(Math.max(0, j - 30), j + 50)}`); break; }
    }
  }
}

console.log(`differential: N=${N} semantic_fail=${semanticFail} byte_diverge=${byteDiverge} rust_err=${rustErr}`);
if (semanticFail > 0 || rustErr > 0) process.exit(1);
console.log('PASS: Rust codec semantically identical to official SDK in both directions');
