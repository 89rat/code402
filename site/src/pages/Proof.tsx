import { useEffect, useState } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card'
import { Badge } from '../components/ui/badge'

interface OpsStats {
  x402v2_enabled: boolean
  facilitator_breaker: string | null
  pending_settlement_events: string | null
  reconciler: {
    last_success_ms: string | null
    stale_backlog: string | null
    oldest_stale_age_s: string | null
    canceled_last_run: string | null
  }
}

function LiveOpsStrip() {
  const [ops, setOps] = useState<OpsStats | null>(null)
  useEffect(() => {
    fetch('/v1/ops/stats')
      .then((r) => r.json())
      .then(setOps)
      .catch(() => {})
  }, [])
  const ago = ops?.reconciler.last_success_ms
    ? `${Math.max(0, Math.round((Date.now() - Number(ops.reconciler.last_success_ms)) / 60000))} min ago`
    : '—'
  const cells = [
    { k: 'reconciler last run', v: ago },
    { k: 'stale claims backlog', v: ops?.reconciler.stale_backlog ?? '—' },
    { k: 'oldest stale age', v: ops?.reconciler.oldest_stale_age_s ? `${ops.reconciler.oldest_stale_age_s}s` : '—' },
    { k: 'cancels this run', v: ops?.reconciler.canceled_last_run ?? '0' },
    { k: 'facilitator breaker', v: ops?.facilitator_breaker ?? 'closed' },
    { k: 'settlements pending', v: ops?.pending_settlement_events ?? '—' },
  ]
  return (
    <div className="mb-12 rounded-lg border bg-muted/30 p-4">
      <p className="mb-3 font-mono text-xs uppercase tracking-widest text-muted-foreground">
        live from production — /v1/ops/stats, refreshed per visit
      </p>
      <div className="grid grid-cols-2 gap-4 md:grid-cols-6">
        {cells.map((c) => (
          <div key={c.k}>
            <p className="font-mono text-lg font-bold">{c.v}</p>
            <p className="text-[11px] leading-tight text-muted-foreground">{c.k}</p>
          </div>
        ))}
      </div>
    </div>
  )
}

const stats = [
  { label: 'Real on-chain settles', value: '1,000+', note: 'Base Sepolia, Coinbase CDP facilitator' },
  { label: 'Wrong answers', value: '0', note: 'Zero 5xx, zero panics, zero double-serves across all campaigns' },
  { label: 'Burst throughput', value: '~6.2/s', note: 'Per facilitator key, clean windows' },
  { label: 'Settle latency p50', value: '3.5s', note: 'Sustained 25-wide load (1.4s single-shot)' },
  { label: '402 challenge latency', value: '137ms', note: 'The free tier of the protocol' },
  { label: 'Replay determinism', value: '100%', note: '25-parallel replay storm, byte-identical responses' },
]

const campaigns = [
  {
    name: 'Stress I — Design Claims Under Load',
    date: 'Aug 19, 2026',
    tag: 'Correctness',
    findings: [
      '150 malformed payment headers: all rejected locally (400), facilitator never touched, zero panics',
      '25-parallel replay of a settled payment: every response byte-identical to the first',
      '10-way same-payment race: exactly one on-chain settle (ledger-verified), losers retry cleanly',
      'Two payers, same nonce: both traversed independently — per-authorizer idempotency proven live',
      '12 parallel real settles completed in 2.31s total — one block window',
    ],
  },
  {
    name: 'Stress II — 1,000 Settles Sustained',
    date: 'Aug 19, 2026',
    tag: 'Scale & Degradation',
    findings: [
      '717 confirmed settles + 283 retryable timeouts + zero other outcomes, in 310 seconds',
      'Bimodal wave structure: CDP burst queue at ~6 settles/s, graceful degradation past it, full recovery',
      '~122 "phantom settles" discovered: on-chain movement exceeded confirmed responses — the ambiguous-outcome class is real, and the reconciliation design (chain as root of truth) exists precisely for it',
      'Every failure was fail-closed and retryable per design; no unpaid serve, no double charge',
    ],
  },
  {
    name: 'Claims Battery — Facilitator Verified',
    date: 'Aug 19, 2026',
    tag: 'Conformance',
    findings: [
      '80 volleyed verify calls across two API keys: zero throttling (verification is free, as specified)',
      'JWT expiry (120s) enforced by facilitator: stale tokens rejected with 401',
      'Error taxonomy byte-matched: insufficient_funds from the live facilitator matches the spec §9 string exactly',
      'Two client-side bugs found and fixed by the battery itself — deterministic rejections were being misclassified as ambiguous timeouts',
    ],
  },
  {
    name: 'Reconciler E2E — All Four Scenarios',
    date: 'Aug 19, 2026',
    tag: 'Chain Truth',
    findings: [
      'Out-of-band settle then retry: wedge resolved to settled_reconciled with the exact on-chain tx; the retry executed FREE, bound to the original input (different input → 400)',
      'On-chain cancelAuthorization: resolved to failed_canceled; retry correctly terminal',
      'Expired-unused: failed_expired on the second sweep, exactly per spec',
      'Re-drive: the hourly sweep itself re-submitted a wedged payment to the live facilitator and settled it (real tx) — the payer paid once, was served once',
      '132 real phantom settles exported as a standing regression corpus; every defect became a test',
    ],
  },
  {
    name: 'Receipts — XDR-1 v0.2 Conformance',
    date: 'Aug 19, 2026',
    tag: 'Receipts',
    findings: [
      'RFC 8785 (JCS) canonicalization: the spec test vector reproduces byte-for-byte — hashes, commitment, signature, signer recovery',
      'Domain-separated commitment with a signed payment_ref: a receipt is cryptographically bound to one payment authorization',
      'Issued live through the official @x402/fetch client round-trip; the receipt verifies offline against the published signing address',
    ],
  },
]

export default function Proof() {
  return (
    <div className="mx-auto max-w-5xl px-4 py-16">
      <div className="mb-12 text-center">
        <Badge variant="outline" className="mb-4">Live data · not marketing</Badge>
        <h1 className="text-4xl font-bold tracking-tight">Proof, not promises</h1>
        <p className="mt-4 text-lg text-muted-foreground max-w-2xl mx-auto">
          Every number on this page was measured against the live protocol — real wallets,
          real signatures, real on-chain settlement through the Coinbase CDP facilitator
          on Base Sepolia. Full methodology and raw telemetry are in the{' '}
          <a href="https://github.com/89rat/code402" className="underline hover:text-foreground">open repository</a>{' '}
          (reviews/stress-1.md, reviews/stress-2.md, reviews/claims-verification.md).
        </p>
      </div>

      <LiveOpsStrip />

      <div className="mb-16 grid grid-cols-2 gap-4 md:grid-cols-3">
        {stats.map((s) => (
          <Card key={s.label}>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">{s.label}</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold">{s.value}</div>
              <p className="mt-1 text-xs text-muted-foreground">{s.note}</p>
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="space-y-8">
        {campaigns.map((c) => (
          <Card key={c.name}>
            <CardHeader>
              <div className="flex items-center gap-3">
                <CardTitle className="text-xl">{c.name}</CardTitle>
                <Badge>{c.tag}</Badge>
              </div>
              <p className="text-sm text-muted-foreground">{c.date}</p>
            </CardHeader>
            <CardContent>
              <ul className="space-y-3">
                {c.findings.map((f, i) => (
                  <li key={i} className="flex gap-3 text-sm leading-relaxed">
                    <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-primary" />
                    <span>{f}</span>
                  </li>
                ))}
              </ul>
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="mt-12 rounded-lg border bg-muted/50 p-6 text-center">
        <p className="text-sm text-muted-foreground">
          The phantom-settle finding is the point, not an embarrassment: ambiguous outcomes
          are unavoidable in any system that talks to a blockchain, and the honest engineering
          answer is reconciliation against chain state — designed in from day one, and now
          proven with real work items.
        </p>
      </div>
    </div>
  )
}
