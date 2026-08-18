import { useEffect, useState } from 'react'
import { ShieldCheck, ShieldAlert, Copy, Check, ExternalLink, Activity } from 'lucide-react'

type TrustRecord = {
  domain: string
  level: string
  fidelity_pct: number | null
  days_measured: number
  observations: number
  settled_observations: number
  self_trades_disclosed: number
  drift_events: number
  first_measured: string | null
  last_run: string
  evidence_root_hash: string
  methodology_url: string
  badge_url: string
}

const SETTLEMENT_TX = '0xc6478aea46f82fb9bde295e052c5c26e42e4c80ceb6f44db35a4896cd2c7672d'

// Snapshot of the live drift wall, measured by our crawler (updated daily).
// Full feed ships with the trust API; these are real rows from 2026-08-17.
const DRIFT_WALL = [
  { url: 'stableupload.dev/api/upload', catalog: '$0.005', live: '$2.00', note: '400× stale catalog price' },
  { url: 'blockrun.ai/api/v1/exa/search', catalog: '$0.011', live: '"0.0110" (string)', note: 'spec-violating amount' },
  { url: 'x402.tavily.com/search', catalog: '$0.016', live: '$0.01', note: 'catalog self-corrected' },
  { url: 'api.exa.ai/contents', catalog: '$0.001', live: '$0', note: 'silently free / broken' },
]

const EMBED_MD =
  '[![code402 trust](https://code402.dev/v1/trust/code402.dev/badge.svg)](https://code402.dev/trust)'

export default function Trust() {
  const [rec, setRec] = useState<TrustRecord | null>(null)
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    fetch('/v1/trust/code402.dev')
      .then((r) => (r.ok ? r.json() : null))
      .then(setRec)
      .catch(() => setRec(null))
  }, [])

  const copyEmbed = () => {
    navigator.clipboard.writeText(EMBED_MD).catch(() => {})
    setCopied(true)
    setTimeout(() => setCopied(false), 1500)
  }

  return (
    <div className="mx-auto max-w-4xl px-6 py-16">
      {/* hero */}
      <div className="mb-4 flex items-center gap-3">
        <ShieldCheck className="h-8 w-8 text-[#06B6D4]" />
        <h1 className="text-3xl font-semibold tracking-tight">code402 Verified</h1>
      </div>
      <p className="mb-6 max-w-2xl text-[#A1A1AA]">
        The trust layer for agent payments. We continuously measure whether sellers'
        published prices match their live 402 challenges — and publish the record,
        including our own. Levels are computed by code from append-only evidence.
        Nobody is rated by hand. Including us.
      </p>

      {/* live record */}
      <div className="mb-10 rounded-lg border border-[#27272A] bg-[#111113] p-6">
        <div className="mb-4 flex flex-wrap items-center gap-4">
          <img src="/v1/trust/code402.dev/badge.svg" alt="code402 trust badge" className="h-5" />
          <span className="font-mono text-xs text-[#A1A1AA]">live — recomputed daily 06:47 IST</span>
        </div>
        {rec ? (
          <div className="grid grid-cols-2 gap-4 font-mono text-sm md:grid-cols-4">
            <Stat label="level" value={rec.level} />
            <Stat label="price fidelity" value={rec.fidelity_pct == null ? '—' : `${rec.fidelity_pct}%`} />
            <Stat label="days measured" value={String(rec.days_measured)} />
            <Stat label="observations" value={String(rec.observations)} />
            <Stat label="settled (self-test)" value={String(rec.settled_observations)} />
            <Stat label="self-trades disclosed" value={String(rec.self_trades_disclosed)} />
            <Stat label="drift events" value={String(rec.drift_events)} />
            <Stat label="measured since" value={rec.first_measured ?? '—'} />
          </div>
        ) : (
          <p className="font-mono text-sm text-[#A1A1AA]">loading live record…</p>
        )}
        {rec && (
          <p className="mt-4 break-all font-mono text-xs text-[#52525B]">
            evidence root: {rec.evidence_root_hash}
          </p>
        )}
      </div>

      {/* settlement proof */}
      <h2 className="mb-3 text-xl font-semibold">First mainnet settlement — proven, not promised</h2>
      <div className="mb-10 rounded-lg border border-[#27272A] bg-[#111113] p-6 font-mono text-sm">
        <p className="text-[#A1A1AA]">
          2026-08-17 · $0.005 USDC · Base mainnet · block 50072738 · facilitator-free
          (our own EIP-1559 signer) · HTTP 200 + signed receipt
        </p>
        <a
          href={`https://basescan.org/tx/${SETTLEMENT_TX}`}
          target="_blank"
          rel="noreferrer"
          className="mt-2 inline-flex items-center gap-1 break-all text-[#06B6D4] hover:underline"
        >
          {SETTLEMENT_TX} <ExternalLink className="h-3 w-3" />
        </a>
        <p className="mt-2 text-xs text-[#52525B]">
          Labeled self_trade forever. It validates our rails; it is never counted as demand.
        </p>
      </div>

      {/* drift wall */}
      <div className="mb-3 flex items-center gap-2">
        <Activity className="h-5 w-5 text-[#F59E0B]" />
        <h2 className="text-xl font-semibold">The Drift Wall</h2>
      </div>
      <p className="mb-4 text-sm text-[#A1A1AA]">
        7.8% of catalog-listed x402 prices did not match their live 402 challenges when we
        measured them (129 comparable endpoints, 2026-08-17). Nobody else publishes this.
        We do — daily.
      </p>
      <div className="mb-10 overflow-x-auto rounded-lg border border-[#27272A]">
        <table className="w-full font-mono text-xs">
          <thead className="bg-[#111113] text-[#A1A1AA]">
            <tr>
              <th className="px-4 py-2 text-left">endpoint</th>
              <th className="px-4 py-2 text-left">catalog</th>
              <th className="px-4 py-2 text-left">live</th>
              <th className="px-4 py-2 text-left">finding</th>
            </tr>
          </thead>
          <tbody>
            {DRIFT_WALL.map((d) => (
              <tr key={d.url} className="border-t border-[#27272A]">
                <td className="px-4 py-2 text-[#FAFAFA]">{d.url}</td>
                <td className="px-4 py-2 text-[#A1A1AA]">{d.catalog}</td>
                <td className="px-4 py-2 text-[#F59E0B]">{d.live}</td>
                <td className="px-4 py-2 text-[#A1A1AA]">{d.note}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* methodology */}
      <h2 className="mb-3 text-xl font-semibold">Methodology</h2>
      <div className="mb-10 space-y-3 rounded-lg border border-[#27272A] bg-[#111113] p-6 text-sm text-[#A1A1AA]">
        <p><b className="text-[#FAFAFA]">What we measure.</b> Unpaid probes of public 402 challenges (a 402 is a public price quote), plus public catalog entries. We never pay third parties; our own endpoints are additionally settled-tested and labeled.</p>
        <p><b className="text-[#FAFAFA]">Fidelity.</b> Per-endpoint share of observations whose quoted amount equals that endpoint's modal quote, over a rolling window.</p>
        <p><b className="text-[#FAFAFA]">Levels.</b> verified = ≥7 measured days, ≥99% fidelity, ≥1 live 402. verified-gold = ≥30 days, ≥99.5%, zero unresolved drift. flagged = unresolved drift &gt;48h or spec-violating quotes, published with evidence hash. Everything else is unrated — the honest default.</p>
        <p><b className="text-[#FAFAFA]">Self-trades.</b> We self-test settlement. Every such row is labeled <span className="font-mono">self_trade=true</span>, disclosed in the record, and never counted as organic demand.</p>
        <p><b className="text-[#FAFAFA]">Evidence.</b> Observations are append-only; each carries a SHA-256 of the raw response, chained into the evidence root above. Recompute it yourself — the methodology is public because trust that can't be audited is marketing.</p>
        <p><b className="text-[#FAFAFA]">Revocation.</b> Automatic, at the next daily run. No human override — including for code402.dev itself.</p>
      </div>

      {/* embed — the viral loop */}
      <h2 className="mb-3 text-xl font-semibold">Carry the badge</h2>
      <p className="mb-3 text-sm text-[#A1A1AA]">
        Sellers: an honest badge is worth more than a landing-page claim. Embed yours —
        it updates itself daily from measured evidence.
      </p>
      <div className="mb-4 flex items-center gap-2 rounded-lg border border-[#27272A] bg-[#111113] p-4">
        <code className="flex-1 break-all font-mono text-xs text-[#06B6D4]">{EMBED_MD}</code>
        <button
          onClick={copyEmbed}
          className="rounded-md border border-[#27272A] p-2 text-[#A1A1AA] hover:border-[#06B6D4] hover:text-[#06B6D4]"
          aria-label="Copy embed code"
        >
          {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
        </button>
      </div>
      <div className="flex items-start gap-2 rounded-lg border border-[#3F3F46] bg-[#111113] p-4 text-xs text-[#A1A1AA]">
        <ShieldAlert className="mt-0.5 h-4 w-4 shrink-0 text-[#F59E0B]" />
        <p>
          Selling an x402 endpoint? You're probably already in our dataset as
          <span className="font-mono"> unrated</span>. Claim your domain, get measured,
          earn verified. The badge is free. Trust is not for sale — that's the point.
        </p>
      </div>
    </div>
  )
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-xs uppercase tracking-wider text-[#52525B]">{label}</p>
      <p className="text-[#FAFAFA]">{value}</p>
    </div>
  )
}
