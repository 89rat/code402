// Fix 3: OFFICIAL @x402/fetch client round-trip against our /v2 route.
// The reference SDK reads our 402, signs, retries — third-party proof.
import { wrapFetchWithPayment, x402Client } from '@x402/fetch';
import { ExactEvmScheme } from '@x402/evm';
import { createWalletClient, http } from 'viem';
import { baseSepolia } from 'viem/chains';
import { privateKeyToAccount } from 'viem/accounts';
import { readFileSync } from 'node:fs';

const BASE = process.argv[2] || 'http://127.0.0.1:8870';
const key = '0x' + readFileSync(process.argv[3] || '../../.staging/payer-secret.txt', 'utf8').trim();
const account = privateKeyToAccount(key);
const walletClient = createWalletClient({ account, chain: baseSepolia, transport: http('https://sepolia.base.org') });
const client = new x402Client();
const signer = { address: account.address, signTypedData: (args) => walletClient.signTypedData(args) };
client.register('eip155:84532', new ExactEvmScheme(signer));
client.register('eip155:8453', new ExactEvmScheme(signer));
const fetchWithPayment = wrapFetchWithPayment(fetch, client);

console.log('payer:', account.address);
const t0 = Date.now();
try {
  const res = await fetchWithPayment(`${BASE}/v2/tools/vat-mod97-check/call`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ input: { vat_number: 'GB123456789' } }),
  });
  const ms = (Date.now() - t0);
  console.log(`status: ${res.status} (${ms}ms)`);
  const pr = res.headers.get('payment-response');
  console.log('PAYMENT-RESPONSE:', pr ? pr.slice(0, 50) + '...' : 'ABSENT');
  const j = await res.json();
  console.log('settlement tx:', j.settlement?.transaction?.slice(0, 20) || 'NONE');
  console.log(res.status === 200 && j.settlement ? 'OFFICIAL-CLIENT-E2E: PASS' : 'OFFICIAL-CLIENT-E2E: CHECK');
} catch (e) {
  console.log('ERROR:', String(e).slice(0, 200));
}
