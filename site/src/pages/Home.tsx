import { useEffect, useState } from 'react'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Zap, ShieldCheck, FileCheck2, Compass, Store, Code2 } from 'lucide-react'

const SCRIPT: { text: string; cls: string }[] = [
  { text: '$ agent pay-and-call vat-mod97-check', cls: 'text-[#A1A1AA]' },
  { text: '→ POST /v1/tools/vat-mod97-check/call', cls: 'text-[#A1A1AA]' },
  { text: '← 402 Payment Required', cls: 'text-[#F59E0B]' },
  { text: '  price: 0.005 USDC · chain: base · nonce: 0x9f3a…', cls: 'text-[#F59E0B]' },
  { text: '→ sign EIP-3009 TransferWithAuthorization', cls: 'text-[#A1A1AA]' },
  { text: '→ retry with X-PAYMENT voucher', cls: 'text-[#A1A1AA]' },
  { text: '← 200 OK', cls: 'text-[#06B6D4]' },
  { text: '  output: { valid: true, canonical: "123456782" }', cls: 'text-[#06B6D4]' },
  { text: '  receipt: { commitment: 0x7c1e…, signature: 0xbd42… }', cls: 'text-[#06B6D4]' },
  { text: '✓ settled · receipt anchored · audit complete', cls: 'text-[#FAFAFA]' },
]

function TerminalLoop() {
  const [lines, setLines] = useState(0)
  useEffect(() => {
    const t = setInterval(() => {
      setLines((n) => (n >= SCRIPT.length + 4 ? 0 : n + 1))
    }, 700)
    return () => clearInterval(t)
  }, [])
  return (
    <div className="overflow-hidden rounded-lg border border-[#27272A] bg-[#18181B]">
      <div className="flex items-center gap-1.5 border-b border-[#27272A] px-4 py-2.5">
        <span className="h-2.5 w-2.5 rounded-full bg-[#27272A]" />
        <span className="h-2.5 w-2.5 rounded-full bg-[#27272A]" />
        <span className="h-2.5 w-2.5 rounded-full bg-[#06B6D4]" />
        <span className="ml-2 font-mono text-xs text-[#A1A1AA]">agent — x402 loop</span>
      </div>
      <div className="min-h-[280px] p-4 font-mono text-xs leading-6">
        {SCRIPT.slice(0, Math.min(lines, SCRIPT.length)).map((l, i) => (
          <div key={i} className={l.cls}>{l.text}</div>
        ))}
        <span className="inline-block h-4 w-2 animate-pulse bg-[#06B6D4]" />
      </div>
    </div>
  )
}

interface LiveStats { services: number; alive: number; endpoints_probed: number }

function LiveStatsStrip() {
  const [stats, setStats] = useState<LiveStats | null>(null)
  useEffect(() => {
    fetch('https://atlas.code402.dev/v1/stats')
      .then((r) => r.json())
      .then(setStats)
      .catch(() => {})
  }, [])
  const items = [
    { k: 'payable services indexed', v: stats ? stats.services.toLocaleString() : '—' },
    { k: 'endpoints verified alive', v: stats ? stats.alive.toLocaleString() : '—' },
    { k: 'hourly probes run', v: stats ? stats.endpoints_probed.toLocaleString() : '—' },
    { k: 'seller fee', v: '2%' },
    { k: 'buyer fee', v: '$0 · always' },
    { k: 'custody risk', v: 'zero' },
  ]
  return (
    <div className="grid grid-cols-2 gap-px overflow-hidden rounded-lg border border-[#27272A] bg-[#27272A] md:grid-cols-6">
      {items.map((i) => (
        <div key={i.k} className="bg-[#18181B] px-4 py-4">
          <p className="font-mono text-xl text-[#06B6D4]">{i.v}</p>
          <p className="mt-1 text-[11px] leading-tight text-[#71717A]">{i.k}</p>
        </div>
      ))}
    </div>
  )
}

const audiences = [
  {
    icon: Store,
    tag: 'SELLERS',
    title: 'Sell any API to AI agents',
    body: 'Register your wallet, list your endpoint. Every x402-capable agent on earth can pay you per call — USDC straight to your wallet. We add discovery, receipts, reputation, invoices.',
    cta: 'Start selling →',
    href: 'https://gateway.code402.dev/v1/sellers',
    code: `curl -X POST gateway.code402.dev/v1/sellers
  { "id": "you", "wallet": "0x…",
    "name": "Your API" }`,
  },
  {
    icon: Compass,
    tag: 'AGENTS',
    title: 'Search before you pay',
    body: '209+ payable endpoints with probe-verified prices, uptime, and on-chain seller trust. A deterministic purchase-policy gate (ACCEPT / REJECT / ESCALATE) before any signature.',
    cta: 'Search the index →',
    href: 'https://atlas.code402.dev',
    code: `POST atlas.code402.dev/mcp
  → search_x402("web search")
  → plan_purchase(url, budget_usd)`,
  },
  {
    icon: Code2,
    tag: 'BUILDERS',
    title: 'Build on the open protocol',
    body: 'M2M/1: discovery → quotes → orders → receipts, layered above any payment rail — x402 today, fiat and prepaid specified in v1.1. Open spec, reference gateway, conformance-tested.',
    cta: 'Read the spec →',
    href: 'https://github.com/89rat/m2m-exchange',
    code: `"accepts": [
  { "scheme": "exact", "asset": "USDC" },
  { "scheme": "prepaid", "asset": "USD6" } ]`,
  },
]

const pillars = [
  {
    icon: Zap,
    title: 'Native x402 & multi-rail',
    body: 'HTTP 402 is the contract. Crypto rail owns micropayments; fiat and prepaid rails (M2M/1.1) reach buyers whose budgets live in bank accounts. Same protocol, same receipt, same reputation.',
  },
  {
    icon: ShieldCheck,
    title: 'Strict determinism',
    body: 'Same input, same output, every time, from every edge location. Integer money only. LLMs may propose; deterministic code decides, signs, and settles.',
  },
  {
    icon: FileCheck2,
    title: 'Cryptographic auditability',
    body: 'Every call returns a signed receipt bound to input and output hashes. An append-only ledger with a verifiable conservation invariant. Reconciliation is a hash check, not a spreadsheet.',
  },
]

const catalog = [
  { name: 'x402 Endpoint Prober', status: 'live', desc: 'Verify any URL\u2019s 402 paywall; normalized terms. $0.005/call.' },
  { name: 'VAT Modulus-97 Checker', status: 'live', desc: 'ISO 7064 checksum validation, machine-verifiable. $0.005/call.' },
  { name: 'Context Distill', status: 'live', desc: 'Deterministic content extraction with signed receipts. $0.005/call.' },
  { name: 'UK Entity Validator', status: 'live', desc: 'Companies House format + structural validation.' },
  { name: 'Marketplace (beta)', status: 'live', desc: 'Third-party endpoints via gateway.code402.dev — any seller, direct payTo.' },
  { name: 'Fiat rails + escrow', status: 'soon', desc: 'M2M/1.1 Stripe adapter and procurement escrow.' },
]

export default function Home() {
  return (
    <div>
      <section className="mx-auto grid max-w-6xl items-center gap-12 px-6 py-20 lg:grid-cols-2">
        <div>
          <Badge variant="outline" className="mb-6 border-[#27272A] font-mono text-xs text-[#06B6D4]">
            open protocol · non-custodial · USDC on Base · live now
          </Badge>
          <h1 className="text-4xl font-bold leading-tight tracking-tight md:text-5xl">
            The open commerce layer for the machine economy.
          </h1>
          <p className="mt-6 text-lg leading-relaxed text-[#A1A1AA]">
            Agents discover, budget, pay, and prove. Sellers plug any API in and earn per
            call — paid direct, never custodied. One protocol above every rail: x402 today,
            fiat and prepaid already specified.
          </p>
          <div className="mt-8 flex flex-wrap gap-4">
            <a href="https://gateway.code402.dev/v1/sellers" className="rounded-md bg-[#06B6D4] px-5 py-2.5 text-sm font-semibold text-[#09090B] hover:bg-[#22d3ee] transition-colors">
              Sell your API →
            </a>
            <a href="https://atlas.code402.dev" className="rounded-md border border-[#27272A] px-5 py-2.5 font-mono text-sm text-[#A1A1AA] hover:border-[#06B6D4] hover:text-[#FAFAFA] transition-colors">
              agents: search the index →
            </a>
            <a href="https://github.com/89rat/m2m-exchange" className="rounded-md border border-[#27272A] px-5 py-2.5 font-mono text-sm text-[#A1A1AA] hover:border-[#06B6D4] hover:text-[#FAFAFA] transition-colors">
              ★ the protocol
            </a>
          </div>
        </div>
        <TerminalLoop />
      </section>

      <section className="border-y border-[#27272A] bg-[#09090B]">
        <div className="mx-auto max-w-6xl px-6 py-10">
          <p className="mb-4 font-mono text-xs uppercase tracking-widest text-[#52525B]">
            live from the index — updated hourly, verified by probes not claims
          </p>
          <LiveStatsStrip />
        </div>
      </section>

      <section className="border-b border-[#27272A]">
        <div className="mx-auto max-w-6xl px-6 py-20">
          <h2 className="text-2xl font-semibold tracking-tight">Three ways in</h2>
          <div className="mt-8 grid gap-6 md:grid-cols-3">
            {audiences.map((a) => (
              <Card key={a.tag} className="flex flex-col border-[#27272A] bg-[#18181B]">
                <CardHeader>
                  <div className="flex items-center justify-between">
                    <a.icon className="h-5 w-5 text-[#06B6D4]" />
                    <Badge variant="outline" className="border-[#27272A] font-mono text-[10px] text-[#06B6D4]">{a.tag}</Badge>
                  </div>
                  <CardTitle className="text-base text-[#FAFAFA]">{a.title}</CardTitle>
                </CardHeader>
                <CardContent className="flex flex-1 flex-col gap-4">
                  <p className="text-sm leading-relaxed text-[#A1A1AA]">{a.body}</p>
                  <pre className="overflow-x-auto rounded-md bg-[#09090B] p-3 font-mono text-[11px] leading-5 text-[#A1A1AA]">{a.code}</pre>
                  <a href={a.href} target="_blank" rel="noopener noreferrer" className="mt-auto font-mono text-xs text-[#06B6D4] hover:underline">
                    {a.cta}
                  </a>
                </CardContent>
              </Card>
            ))}
          </div>
        </div>
      </section>

      <section className="border-b border-[#27272A]">
        <div className="mx-auto max-w-6xl px-6 py-20">
          <h2 className="text-2xl font-semibold tracking-tight">Why machines choose this stack</h2>
          <div className="mt-8 grid gap-6 md:grid-cols-3">
            {pillars.map((p) => (
              <Card key={p.title} className="border-[#27272A] bg-[#18181B]">
                <CardHeader>
                  <p.icon className="h-6 w-6 text-[#06B6D4]" />
                  <CardTitle className="text-base text-[#FAFAFA]">{p.title}</CardTitle>
                </CardHeader>
                <CardContent className="text-sm leading-relaxed text-[#A1A1AA]">{p.body}</CardContent>
              </Card>
            ))}
          </div>
        </div>
      </section>

      <section className="border-b border-[#27272A]">
        <div className="mx-auto max-w-6xl px-6 py-20">
          <div className="flex items-end justify-between">
            <h2 className="text-2xl font-semibold tracking-tight">Platform</h2>
            <a href="https://gateway.code402.dev/v1/services" className="font-mono text-xs text-[#06B6D4] hover:underline">machine-readable: /v1/services →</a>
          </div>
          <div className="mt-8 divide-y divide-[#27272A] rounded-lg border border-[#27272A]">
            {catalog.map((t) => (
              <div key={t.name} className="flex items-center justify-between bg-[#18181B] px-5 py-4 first:rounded-t-lg last:rounded-b-lg">
                <div>
                  <p className="font-medium text-[#FAFAFA]">{t.name}</p>
                  <p className="text-sm text-[#A1A1AA]">{t.desc}</p>
                </div>
                <Badge variant="outline" className={t.status === 'live' ? 'border-[#06B6D4] font-mono text-xs text-[#06B6D4]' : 'border-[#F59E0B] font-mono text-xs text-[#F59E0B]'}>
                  {t.status === 'live' ? 'LIVE' : 'SOON'}
                </Badge>
              </div>
            ))}
          </div>
          <p className="mt-6 font-mono text-xs text-[#52525B]">
            Public repos: <a className="text-[#06B6D4] hover:underline" href="https://github.com/89rat/code402">code402</a> · <a className="text-[#06B6D4] hover:underline" href="https://github.com/89rat/m2m-exchange">m2m-exchange</a> · <a className="text-[#06B6D4] hover:underline" href="https://github.com/89rat/x402-atlas">x402-atlas</a>
          </p>
        </div>
      </section>
    </div>
  )
}
