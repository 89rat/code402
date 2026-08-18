import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

const taxonomy: { code: string; http: number; meaning: string; strategy: string }[] = [
  { code: 'PAYMENT_REQUIRED', http: 402, meaning: 'No payment credential supplied.', strategy: 'Parse the challenge body (price, recipient, nonce, expires_at), sign an EIP-3009 TransferWithAuthorization, retry with X-PAYMENT before expiry.' },
  { code: 'INVALID_SIGNATURE', http: 401, meaning: 'Voucher signature failed recovery or does not match the declared payer.', strategy: 'Re-sign with the exact EIP-712 domain (name, version, chainId, verifyingContract) from the challenge; verify the payer key; do not retry the same bytes.' },
  { code: 'REPLAYED_NONCE', http: 409, meaning: 'This payment nonce was already claimed.', strategy: 'Fetch a fresh challenge, use the new nonce. Never reuse a nonce.' },
  { code: 'INSUFFICIENT_PAYMENT', http: 402, meaning: 'Voucher value is below the required price.', strategy: 'Read price.amount from the challenge and re-sign with value >= amount. Treat price as authoritative per request.' },
  { code: 'EXPIRED_PAYMENT', http: 402, meaning: 'Voucher outside its valid_after..valid_before window.', strategy: 'Re-sign with a fresh validity window (<= 5 minutes). Check local clock skew.' },
  { code: 'UNSUPPORTED_CHAIN', http: 400, meaning: 'Chain ID not allowlisted.', strategy: 'Use the chain in network.chain_id from the challenge. Do not guess.' },
  { code: 'UNSUPPORTED_TOKEN', http: 400, meaning: 'Token contract not allowlisted.', strategy: 'Pay in the asset named by price.token_address (USDC). No substitutes.' },
  { code: 'INVALID_RECIPIENT', http: 400, meaning: 'Voucher to-address is not the settlement wallet.', strategy: 'Set auth.to = recipient from the challenge, byte-exact.' },
  { code: 'INPUT_SCHEMA_INVALID', http: 400, meaning: 'Request body failed schema validation.', strategy: 'Fix input against the tool schema in mcp.json / openapi.yaml. Validation runs before any payment logic — this error costs nothing.' },
  { code: 'RATE_LIMITED', http: 429, meaning: 'Too many requests.', strategy: 'Exponential backoff with jitter; honor Retry-After if present.' },
  { code: 'TOOL_INTERNAL_ERROR', http: 500, meaning: 'Deterministic tool fault.', strategy: 'Safe to retry with the same idempotency_key — a replay returns the stored receipt without re-charging.' },
]

const flow = [
  ['1', 'Discover', 'GET /.well-known/mcp.json — tool schemas, prices, auth type'],
  ['2', 'Call unpaid', 'POST /v1/tools/{tool}/call → 402 challenge with price + nonce'],
  ['3', 'Sign', 'EIP-712 / EIP-3009 TransferWithAuthorization in USDC'],
  ['4', 'Retry paid', 'X-PAYMENT voucher → 200 + deterministic output + signed receipt'],
  ['5', 'Reconcile', 'Verify receipt commitment against your input/output hashes'],
]

export default function Docs() {
  return (
    <div className="mx-auto max-w-6xl px-6 py-16">
      <h1 className="text-3xl font-bold tracking-tight">Agent integration docs</h1>
      <p className="mt-4 max-w-3xl leading-relaxed text-[#A1A1AA]">
        Every Code402 tool speaks the same five-step loop. There is no signup, no API key,
        no dashboard session — payment <em>is</em> the credential. All responses are produced
        by a Stateless Cryptographic Verification Engine on a Globally Distributed Edge
        Network and recorded in an Append-Only Audit Ledger.
      </p>

      <h2 className="mt-14 text-xl font-semibold">The 402 loop</h2>
      <div className="mt-6 overflow-hidden rounded-lg border border-[#27272A]">
        {flow.map(([n, t, d]) => (
          <div key={n} className="flex items-baseline gap-4 border-b border-[#27272A] bg-[#18181B] px-5 py-3 last:border-0">
            <span className="font-mono text-sm text-[#06B6D4]">{n}</span>
            <span className="w-28 shrink-0 font-medium text-[#FAFAFA]">{t}</span>
            <span className="font-mono text-xs text-[#A1A1AA]">{d}</span>
          </div>
        ))}
      </div>

      <h2 className="mt-14 text-xl font-semibold">Error taxonomy</h2>
      <p className="mt-2 text-sm text-[#A1A1AA]">
        Stable machine-readable codes. The resolution strategy column is written for autonomous agents.
      </p>
      <div className="mt-6 rounded-lg border border-[#27272A]">
        <Table>
          <TableHeader>
            <TableRow className="border-[#27272A] hover:bg-transparent">
              <TableHead className="font-mono text-xs text-[#A1A1AA]">CODE</TableHead>
              <TableHead className="font-mono text-xs text-[#A1A1AA]">HTTP</TableHead>
              <TableHead className="font-mono text-xs text-[#A1A1AA]">MEANING</TableHead>
              <TableHead className="font-mono text-xs text-[#A1A1AA]">AGENT RESOLUTION STRATEGY</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {taxonomy.map((e) => (
              <TableRow key={e.code} className="border-[#27272A]">
                <TableCell className="font-mono text-xs text-[#F59E0B]">{e.code}</TableCell>
                <TableCell className="font-mono text-xs text-[#FAFAFA]">{e.http}</TableCell>
                <TableCell className="text-sm text-[#A1A1AA]">{e.meaning}</TableCell>
                <TableCell className="text-sm text-[#A1A1AA]">{e.strategy}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      <h2 className="mt-14 text-xl font-semibold">Machine-readable manifests</h2>
      <ul className="mt-4 space-y-2 font-mono text-sm">
        {['/.well-known/mcp.json', '/.well-known/openapi.yaml', '/.well-known/x402.json', '/llms.txt', '/.well-known/security.txt'].map((p) => (
          <li key={p}><a className="text-[#06B6D4] hover:underline" href={p}>{p}</a></li>
        ))}
      </ul>
    </div>
  )
}
