// Live smoke test against code402 staging (Base Sepolia, test USDC).
// Uses the persisted staging payer key from ../.staging/payer-secret.txt
// (created by crates/keygen paytest). Run: npm run test:live

import { readFileSync } from "node:fs";
import { privateKeyToAccount } from "viem/accounts";
import { createClient } from "../src/index.js";

const secret = readFileSync(
  new URL("../../.staging/payer-secret.txt", import.meta.url),
  "utf8"
).trim();

const client = createClient({
  baseUrl: "https://code402-edge.akrivis.workers.dev",
  account: privateKeyToAccount("0x" + secret),
  receiptSigner: "0x7b885c42b47671a91b3d81694ce18d38e25e7149",
});

const idem = "sdk-live-" + Date.now();
console.log("payer:", client && "ok", "| idempotency:", idem);

const res = await client.callTool(
  "vat-mod97-check",
  { vat_number: "GB947292996" },
  { idempotencyKey: idem, maxAmountMinor: 10000 }
);

console.log("output:", JSON.stringify(res.output));
console.log("receipt verified:", res.receiptVerified);
console.log("request:", res.requestId);

// Second call with the same idempotency key must replay without payment.
const again = await client.callTool(
  "vat-mod97-check",
  { vat_number: "GB947292996" },
  { idempotencyKey: idem }
);
console.log("idempotent replay:", again.replayed === true, "| ref:", again.receiptRef);

if (!res.receiptVerified || again.replayed !== true) {
  console.error("SMOKE TEST FAILED");
  process.exit(1);
}
console.log("SMOKE TEST PASSED");
