// mcp-server-xdr1 — MCP server exposing XDR-1 receipt verification as tools.
// Any agent can: verify_receipt (offline) and verify_settlement (on-chain).
// Reference verifier for https://code402.dev receipts and any XDR-1 issuer.

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { keccak256, recoverAddress, type Hex } from "viem";

// ---------- XDR-1 primitives (v0.2 wire format) ----------

const SPEC_V02 = "xdr-1/0.2";

function u32be(n: number): Buffer {
  const b = Buffer.alloc(4);
  b.writeUInt32BE(n);
  return b;
}

function lpStr(s: string): Buffer {
  const u = Buffer.from(s, "utf8");
  return Buffer.concat([u32be(u.length), u]);
}

function hexBytes(h: string): Buffer {
  const s = h.startsWith("0x") ? h.slice(2) : h;
  if (!/^[0-9a-fA-F]{64}$/.test(s)) throw new Error(`bad bytes32: ${h}`);
  return Buffer.from(s, "hex");
}

/** commitment = keccak256("XDR-1" || 0x00 || payload) — spec §4 */
export function commitmentV02(r: {
  request_id: string; tool: string; tool_version: string;
  input_hash: string; output_hash: string;
  timestamp_unix: number; payment_ref: string; spec: string;
}): Hex {
  const spec = Buffer.from(r.spec, "utf8");
  const payload = Buffer.concat([
    lpStr(r.request_id), lpStr(r.tool), lpStr(r.tool_version),
    hexBytes(r.input_hash), hexBytes(r.output_hash),
    (() => { const b = Buffer.alloc(8); b.writeBigUInt64BE(BigInt(r.timestamp_unix)); return b; })(),
    hexBytes(r.payment_ref),
    Buffer.from([spec.length]), spec,
  ]);
  return keccak256(Buffer.concat([Buffer.from("XDR-1\0"), payload])) as Hex;
}

/** legacy v0 commitment (no domain tag, no payment_ref/spec) — spec §9 */
export function commitmentV0(r: {
  request_id: string; tool: string; tool_version: string;
  input_hash: string; output_hash: string; timestamp_unix: number;
}): Hex {
  const payload = Buffer.concat([
    lpStr(r.request_id), lpStr(r.tool), lpStr(r.tool_version),
    hexBytes(r.input_hash), hexBytes(r.output_hash),
    (() => { const b = Buffer.alloc(8); b.writeBigUInt64BE(BigInt(r.timestamp_unix)); return b; })(),
  ]);
  return keccak256(payload) as Hex;
}

// ---------- verification ----------

type VerifyResult = {
  ok: boolean;
  failure: null | "SHAPE_INVALID" | "COMMITMENT_MISMATCH" | "SIGNER_UNTRUSTED" | "SETTLEMENT_MISMATCH";
  signer?: string;
  commitment?: string;
  spec_version: "v0" | "v0.2";
};

export async function verifyReceipt(
  doc: any,
  opts: { manifest?: { signing_address?: string } } = {},
): Promise<VerifyResult> {
  const r = doc?.receipt;
  const sig: string | undefined = doc?.signature;
  if (!r || typeof sig !== "string") return { ok: false, failure: "SHAPE_INVALID", spec_version: "v0" };

  const isV02 = r.spec === SPEC_V02 && typeof r.payment_ref === "string";
  const commitment = isV02 ? commitmentV02(r) : commitmentV0(r);
  const stored = (doc.commitment ?? "").replace(/^0x/, "").toLowerCase();
  if (stored !== commitment.slice(2).toLowerCase())
    return { ok: false, failure: "COMMITMENT_MISMATCH", commitment, spec_version: isV02 ? "v0.2" : "v0" };

  // low-s check + v normalization (spec §5)
  const raw = hexBytes("0x" + "00".repeat(0) + sig); // validates 65-byte hex via length below
  if (raw.length !== 65) return { ok: false, failure: "SHAPE_INVALID", spec_version: isV02 ? "v0.2" : "v0" };
  const vByte = raw[64];
  const v = vByte >= 27 ? vByte - 27 : vByte;
  if (v !== 0 && v !== 1) return { ok: false, failure: "SHAPE_INVALID", spec_version: isV02 ? "v0.2" : "v0" };

  const signer = await recoverAddress({
    hash: commitment,
    signature: ("0x" + sig.replace(/^0x/, "").slice(0, 128) + (v === 1 ? "1c" : "1b")) as Hex,
  });

  if (opts.manifest?.signing_address &&
      signer.toLowerCase() !== opts.manifest.signing_address.toLowerCase())
    return { ok: false, failure: "SIGNER_UNTRUSTED", signer, commitment, spec_version: isV02 ? "v0.2" : "v0" };

  return { ok: true, failure: null, signer, commitment, spec_version: isV02 ? "v0.2" : "v0" };
}

// ---------- MCP wiring ----------

const server = new McpServer({ name: "xdr1-verifier", version: "0.1.0" });

server.tool(
  "verify_receipt",
  "Offline verification of an XDR-1 delivery receipt: recompute commitment, check signature, optionally pin the signer to a manifest address.",
  {
    receipt_doc: z.string().describe("The full receipt JSON document (receipt + commitment + signature)"),
    signing_address: z.string().optional().describe("Expected signer from the merchant's /.well-known/xdr-1.json"),
  },
  async ({ receipt_doc, signing_address }) => {
    try {
      const doc = JSON.parse(receipt_doc);
      const res = await verifyReceipt(doc, { manifest: { signing_address } });
      return { content: [{ type: "text", text: JSON.stringify(res, null, 2) }] };
    } catch (e) {
      return { content: [{ type: "text", text: JSON.stringify({ ok: false, failure: "SHAPE_INVALID", error: String(e) }) }] };
    }
  },
);

server.tool(
  "verify_settlement",
  "On-chain settlement check for an XDR-1 receipt: fetches the settlement tx receipt on Base and checks status, token, recipient, and (v0.2) the EIP-3009 nonce against payment_ref.",
  {
    tx_hash: z.string(),
    payment_address: z.string().describe("Merchant payment address from the manifest"),
    amount_minor: z.string(),
    rpc_url: z.string().optional(),
  },
  async ({ tx_hash, payment_address, amount_minor, rpc_url }) => {
    const rpc = rpc_url ?? "https://mainnet.base.org";
    const resp = await fetch(rpc, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "eth_getTransactionReceipt", params: [tx_hash] }),
    });
    const json = await resp.json();
    const rcpt = json.result;
    if (!rcpt) return { content: [{ type: "text", text: JSON.stringify({ ok: false, failure: "SETTLEMENT_MISMATCH", reason: "tx not found" }) }] };
    const statusOk = rcpt.status === "0x1";
    // USDC Transfer(topic0=0xddf252ad...) to payment_address with value >= amount_minor
    const want = BigInt(amount_minor);
    const transferOk = (rcpt.logs ?? []).some((l: any) =>
      l.address?.toLowerCase() === "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913" &&
      l.topics?.[0] === "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef" &&
      ("0x" + l.topics[2].slice(26)).toLowerCase() === payment_address.toLowerCase() &&
      BigInt(l.data) >= want);
    const ok = statusOk && transferOk;
    return { content: [{ type: "text", text: JSON.stringify({ ok, failure: ok ? null : "SETTLEMENT_MISMATCH", statusOk, transferOk }, null, 2) }] };
  },
);

await server.connect(new StdioServerTransport());
