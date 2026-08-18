// @code402/sdk — pay-per-call client for code402 tool endpoints.
//
// One function does the whole loop:
//   1. POST the tool call unpaid
//   2. Receive HTTP 402 with a payment challenge (authoritative: recipient,
//      amount, chain, nonce all come from the challenge — never hardcoded)
//   3. Sign an EIP-3009 TransferWithAuthorization (EIP-712) with your wallet
//   4. Retry the call with X-PAYMENT
//   5. Verify the returned receipt's commitment + signature against the
//      server's advertised receipt-signing address
//
// Dependency: viem (signing + keccak). Works in Node >= 18 and browsers —
// pass any viem LocalAccount (privateKeyToAccount, or a custom signer).

import {
  keccak256,
  toBytes,
  hexToBytes,
  bytesToHex,
  concat,
  recoverAddress,
} from "viem";

const TRANSFER_TYPES = {
  TransferWithAuthorization: [
    { name: "from", type: "address" },
    { name: "to", type: "address" },
    { name: "value", type: "uint256" },
    { name: "validAfter", type: "uint256" },
    { name: "validBefore", type: "uint256" },
    { name: "nonce", type: "bytes32" },
  ],
};

const SUPPORTED_CHAINS = new Set([8453, 84532]); // Base mainnet, Base Sepolia

export class Code402Error extends Error {
  constructor(message, { status, code, body } = {}) {
    super(message);
    this.name = "Code402Error";
    this.status = status;
    this.code = code;
    this.body = body;
  }
}

// Length-prefixed strings + fixed-width hashes + big-endian u64 timestamp,
// keccak256 — mirrors m2m-core Receipt::commitment() byte for byte.
export function receiptCommitment(r) {
  const parts = [];
  for (const s of [r.request_id, r.tool, r.tool_version]) {
    const b = toBytes(s);
    const len = new Uint8Array(4);
    new DataView(len.buffer).setUint32(0, b.length, false);
    parts.push(len, b);
  }
  parts.push(hexToBytes(r.input_hash), hexToBytes(r.output_hash));
  const ts = new Uint8Array(8);
  new DataView(ts.buffer).setBigUint64(0, BigInt(r.timestamp_unix), false);
  parts.push(ts);
  return keccak256(concat(parts));
}

// Recover the receipt signer. The server signs the raw 32-byte commitment
// (no EIP-191 prefix), so we normalize v to 27/28 and recover over the hash.
export async function recoverReceiptSigner({ commitment, signature }) {
  const sig = hexToBytes(signature);
  if (sig.length !== 65) throw new Code402Error("receipt signature must be 65 bytes");
  if (sig[64] < 27) sig[64] += 27;
  const hash = commitment.startsWith("0x") ? commitment : "0x" + commitment;
  return await recoverAddress({ hash, signature: bytesToHex(sig) });
}

export function createClient({
  baseUrl,
  account,
  receiptSigner,          // expected receipt-signing address; if omitted,
                          // fetched from {baseUrl}/.well-known/x402.json
  validitySeconds = 3600, // EIP-3009 validBefore window
  fetchImpl,              // override fetch (tests, React Native, etc.)
} = {}) {
  if (!baseUrl) throw new Code402Error("createClient: baseUrl is required");
  if (!account) throw new Code402Error("createClient: a viem account is required");
  const base = baseUrl.replace(/\/+$/, "");
  const doFetch = fetchImpl ?? globalThis.fetch;
  let signerCache = receiptSigner ?? null;

  async function expectedSigner() {
    if (signerCache) return signerCache;
    const res = await doFetch(`${base}/.well-known/x402.json`);
    if (!res.ok) throw new Code402Error("failed to fetch x402 manifest", { status: res.status });
    const manifest = await res.json();
    if (!manifest.receipt_signing_address) {
      throw new Code402Error("manifest has no receipt_signing_address; pass receiptSigner explicitly");
    }
    signerCache = manifest.receipt_signing_address;
    return signerCache;
  }

  async function post(tool, payload, payment) {
    const headers = { "content-type": "application/json" };
    if (payment) headers["x-payment"] = payment;
    const res = await doFetch(`${base}/v1/tools/${tool}/call`, {
      method: "POST",
      headers,
      body: JSON.stringify(payload),
    });
    const text = await res.text();
    let body = null;
    try { body = text ? JSON.parse(text) : null; } catch { /* non-JSON edge page */ }
    return { status: res.status, body, text };
  }

  async function callTool(tool, input, { idempotencyKey, maxAmountMinor } = {}) {
    const payload = { input };
    if (idempotencyKey) payload.idempotency_key = idempotencyKey;

    // 1 — unpaid attempt
    const first = await post(tool, payload, null);
    if (first.status === 200 && first.body?.idempotent_replay) {
      return { replayed: true, receiptRef: first.body.receipt_ref };
    }
    if (first.status !== 402 || !first.body) {
      throw new Code402Error(`unexpected status ${first.status}`, {
        status: first.status, code: first.body?.error?.code, body: first.body ?? first.text,
      });
    }

    // 2 — challenge is authoritative
    const ch = first.body;
    const chainId = ch.network?.chain_id;
    if (!SUPPORTED_CHAINS.has(chainId)) {
      throw new Code402Error(`unsupported chain ${chainId}`, { code: "UNSUPPORTED_CHAIN" });
    }
    const amount = BigInt(ch.price.amount);
    if (maxAmountMinor != null && amount > BigInt(maxAmountMinor)) {
      throw new Code402Error(`price ${amount} exceeds maxAmountMinor ${maxAmountMinor}`, { code: "PRICE_CAP" });
    }
    if (Math.floor(Date.now() / 1000) > ch.expires_at) {
      throw new Code402Error("challenge already expired", { code: "CHALLENGE_EXPIRED" });
    }

    // EIP-712 domain comes from the challenge (Sepolia USDC = "USDC",
    // mainnet USDC = "USD Coin"). Fall back to the manifest if absent.
    let domain = ch.eip712;
    if (!domain?.name || !domain?.version) {
      const res = await doFetch(`${base}/.well-known/x402.json`);
      const manifest = res.ok ? await res.json() : null;
      domain = manifest?.eip712_domain;
    }
    if (!domain?.name) throw new Code402Error("no EIP-712 domain in challenge or manifest", { code: "DOMAIN_MISSING" });

    // 3 — sign EIP-3009 authorization. Challenge nonce binds payment to request.
    const validBefore = Math.floor(Date.now() / 1000) + validitySeconds;
    const authorization = {
      from: account.address,
      to: ch.recipient,
      value: amount,
      validAfter: 0n,
      validBefore: BigInt(validBefore),
      nonce: ch.nonce,
    };
    const signature = await account.signTypedData({
      domain: {
        name: domain.name,
        version: domain.version,
        chainId,
        verifyingContract: ch.price.token_address,
      },
      types: TRANSFER_TYPES,
      primaryType: "TransferWithAuthorization",
      message: authorization,
    });

    // 4 — paid retry. Wire format matches the server's PaymentVoucher serde.
    const voucher = {
      auth: {
        from: authorization.from.toLowerCase(),
        to: authorization.to.toLowerCase(),
        value: "0x" + amount.toString(16),
        valid_after: 0,
        valid_before: validBefore,
        nonce: ch.nonce,
      },
      signature: Array.from(hexToBytes(signature)),
    };
    const second = await post(tool, payload, JSON.stringify(voucher));
    if (second.status !== 200 || !second.body) {
      throw new Code402Error(`paid call failed with status ${second.status}`, {
        status: second.status, code: second.body?.error?.code, body: second.body ?? second.text,
      });
    }

    // 5 — verify receipt: recompute commitment, recover signer, compare.
    const { output, receipt } = second.body;
    let receiptVerified = false;
    if (receipt?.receipt && receipt?.commitment && receipt?.signature) {
      const recomputed = receiptCommitment(receipt.receipt);
      if (recomputed !== "0x" + receipt.commitment && recomputed !== receipt.commitment) {
        throw new Code402Error("receipt commitment mismatch", { code: "RECEIPT_COMMITMENT_MISMATCH" });
      }
      const signer = await recoverReceiptSigner(receipt);
      const expected = (await expectedSigner()).toLowerCase();
      if (signer.toLowerCase() !== expected) {
        throw new Code402Error(`receipt signer ${signer} != expected ${expected}`, { code: "RECEIPT_SIGNER_MISMATCH" });
      }
      receiptVerified = true;
    }

    return {
      replayed: false,
      output,
      receipt,
      receiptVerified,
      requestId: receipt?.receipt?.request_id ?? ch.request_id,
      settlement: "PENDING_SETTLEMENT",
    };
  }

  return { callTool, expectedSigner };
}
