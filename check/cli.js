#!/usr/bin/env node
/**
 * x402check — CLI. Usage:
 *   node cli.js <url> [--json]
 *   npx x402check <url>          (once published)
 *
 * Exit codes: 0 = grade A/B (CI-pass), 1 = C/D (warn), 2 = F/unreachable (CI-fail).
 */
import { check } from './index.js';

const args = process.argv.slice(2);
const url = args.find(a => !a.startsWith('--'));
const asJson = args.includes('--json');
const bodyIdx = args.indexOf('--body');
const body = bodyIdx >= 0 ? args[bodyIdx + 1] : undefined;

if (!url) {
  console.error('usage: x402check <url> [--json] [--body \'{"input":{...}}\']');
  process.exit(64);
}

const r = await check(url, { body });

if (asJson) {
  console.log(JSON.stringify(r, null, 2));
} else {
  const mark = { blocker: '✖', major: '▲', minor: '·', info: 'ℹ' } ;
  console.log(`\nx402check — ${r.url}`);
  console.log(`dialect: ${r.dialect}   grade: ${r.grade}   score: ${r.score}/100   probed: ${r.probedAt}\n`);
  if (r.findings.length === 0) {
    console.log('  ✔ no findings — an autonomous agent can discover and parse this challenge.\n');
  } else {
    for (const f of r.findings) {
      console.log(`  ${mark[f.severity]} [${f.severity}] ${f.check}`);
      console.log(`      ${f.detail}`);
      console.log(`      ↳ ${f.citation}`);
    }
    console.log('');
  }
  if (r.grade === 'F') console.log('  Verdict: NOT PAYABLE by a conformant agent today.');
  else if (r.grade === 'N/A') console.log('  Verdict: challenge gated behind valid input — re-run with --body for a full grade.');
  else if (r.grade === 'A') console.log('  Verdict: conformant. List it: x402scan + your manifest generator output.');
  console.log('');
}

process.exitCode = r.grade === 'F' ? 2 : (r.grade === 'C' || r.grade === 'D') ? 1 : 0;
