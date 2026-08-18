# @code402/sdk

Pay-per-call client for code402 tool endpoints. One function runs the whole
machine-payment loop:

1. `POST /v1/tools/{tool}/call` unpaid → server replies **402** with a payment challenge
2. SDK signs an **EIP-3009 TransferWithAuthorization** (EIP-712, USDC on Base) with your wallet
3. SDK retries with the `X-PAYMENT` voucher → server verifies, executes, and returns the output
4. SDK **verifies the receipt**: recomputes the commitment and recovers the
   receipt-signing key, comparing it to the server's advertised address

No API keys. No accounts. No billing dashboard. The wallet is the login.

## Install

```bash
npm install @code402/sdk viem
```

## Quickstart

```js
import { privateKeyToAccount } from "viem/accounts";
import { createClient } from "@code402/sdk";

const client = createClient({
  baseUrl: "https://code402-edge.akrivis.workers.dev", // staging (Base Sepolia, test USDC)
  account: privateKeyToAccount(process.env.PAYER_KEY),
  receiptSigner: "0x7b885c42b47671a91b3d81694ce18d38e25e7149", // staging receipt key
});

const res = await client.callTool(
  "vat-mod97-check",
  { vat_number: "GB947292996" },
  { idempotencyKey: "my-request-1", maxAmountMinor: 10000 }
);

console.log(res.output);          // deterministic tool output
console.log(res.receiptVerified); // true — cryptographic proof of execution
```

## Safety rails built in

- **Challenge is authoritative.** Recipient, amount, chain, and nonce are read
  from the server's 402 challenge — nothing is hardcoded, so a config mistake
  can't send funds to a stale address.
- **`maxAmountMinor` price cap.** The call throws `PRICE_CAP` before signing if
  the server quotes more than you allowed.
- **Replay-safe.** `idempotencyKey` returns a stored receipt reference instead
  of charging twice. On-chain, every payment nonce can be used exactly once.
- **Receipt verification is on by default.** A response that fails commitment
  or signer verification throws — you never mistake an unproven answer for a
  paid one.

## Errors

All failures throw `Code402Error` with `.status` (HTTP) and `.code`
(server error code such as `REPLAYED_NONCE`, `EXPIRED_PAYMENT`,
`INVALID_SIGNATURE`, or SDK-side `PRICE_CAP`, `RECEIPT_SIGNER_MISMATCH`).

## Environments

| | staging | production |
|---|---|---|
| chain | Base Sepolia (84532) | Base mainnet (8453) |
| asset | test USDC (no value) | USDC |
| receipt signer | `0x7b885c42b47671a91b3d81694ce18d38e25e7149` | see `/.well-known/x402.json` |

If `receiptSigner` is omitted, the SDK fetches `/.well-known/x402.json` from
the server and reads `receipt_signing_address`.

Testnet USDC: https://faucet.circle.com (select USDC → Base Sepolia).
