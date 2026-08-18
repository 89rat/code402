import { useEffect, useState } from 'react'

import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Zap, ShieldCheck, FileCheck2 } from 'lucide-react'

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

const pillars = [
  {
    icon: Zap,
    title: 'Native x402 & ACP Ready',
    body: 'HTTP 402 is the contract, not an afterthought. Agents discover price, pay, and retry — autonomously, per call, in USDC on a low-cost L2.',
  },
  {
    icon: ShieldCheck,
    title: 'Strict Determinism',
    body: 'Same input, same output, every time, from every edge location. Our Stateless Cryptographic Verification Engine has no hidden state to drift.',
  },
  {
    icon: FileCheck2,
    title: 'Cryptographic Auditability',
    body: 'Every call returns a signed receipt bound to input and output hashes, written to an Append-Only Audit Ledger. Reconciliation is a hash check, not a spreadsheet.',
  },
]

const catalog = [
  { name: 'UK Entity Validator', status: 'live', desc: 'Companies House number format + structural validation.' },
  { name: 'VAT Modulus-97 Checker', status: 'live', desc: 'HMRC checksum, standard and alternative variants.' },
  { name: 'Address Canonicalizer', status: 'live', desc: 'Deterministic normalization for downstream dedup.' },
  { name: 'PII Scrubber', status: 'soon', desc: 'Deterministic redaction with verifiable output hashes.' },
]

const ecosystem = [
  {
    name: 'x402 Atlas',
    url: 'https://atlas.code402.dev',
    desc: 'Live search engine for machine-payable APIs — probe-verified prices, uptime, and on-chain seller trust scores. MCP-native.',
    badge: 'DISCOVERY',
  },
  {
    name: 'M2M/1 Gateway',
    url: 'https://gateway.code402.dev/v1/services',
    desc: 'Our open commerce protocol: machine-readable storefront, tiered pricing ($0.001–$0.005/call), x402 settlement on Base.',
    badge: 'PROTOCOL',
  },
  {
    name: 'Seller Leaderboard',
    url: 'https://atlas.code402.dev/leaderboard',
    desc: 'x402 sellers ranked by verified on-chain settled USDC volume — measured, not claimed. Claim your profile free.',
    badge: 'TRUST',
  },
  {
    name: 'State of x402',
    url: 'https://atlas.code402.dev/reports/state-of-x402',
    desc: 'Weekly ecosystem report auto-generated from liveness probes and settlement data. Machine-readable (.md available).',
    badge: 'DATA',
  },
]

export default function Home() {
  return (
    <div>
      <section className="mx-auto grid max-w-6xl items-center gap-12 px-6 py-24 lg:grid-cols-2">
        <div>
          <Badge variant="outline" className="mb-6 border-[#27272A] font-mono text-xs text-[#06B6D4]">
            non-custodial · x402 · USDC on Base · open protocol
          </Badge>
          <h1 className="text-4xl font-bold leading-tight tracking-tight md:text-5xl">
            Any API can sell to AI agents. In five minutes. Without touching payment code.
          </h1>
          <p className="mt-6 text-lg leading-relaxed text-[#A1A1AA]">
            Register your wallet, list your endpoint — and every x402-capable agent on earth
            can pay you per call, directly. We add discovery, commerce terms, receipts, and
            reputation. Buyers' USDC goes straight to your wallet. We never hold money.
          </p>
          <div className="mt-8 flex flex-wrap gap-4">
            <a
              href="https://gateway.code402.dev/v1/services"
              className="rounded-md bg-[#06B6D4] px-5 py-2.5 text-sm font-semibold text-[#09090B] hover:bg-[#22d3ee] transition-colors"
            >
              Start selling →
            </a>
            <a
              href="https://atlas.code402.dev"
              className="rounded-md border border-[#27272A] px-5 py-2.5 font-mono text-sm text-[#A1A1AA] hover:border-[#06B6D4] hover:text-[#FAFAFA] transition-colors"
            >
              agents: search 189+ services →
            </a>
          </div>
          <div className="mt-8 grid grid-cols-3 gap-4 font-mono text-xs text-[#52525B]">
            <div><span className="text-[#FAFAFA]">2%</span> seller fee<br />nothing for buyers</div>
            <div><span className="text-[#FAFAFA]">$0.001+</span> per-call pricing<br />settled on-chain</div>
            <div><span className="text-[#FAFAFA]">0</span> chargebacks possible<br />signed receipt every call</div>
          </div>
        </div>
        <TerminalLoop />
      </section>

      <section className="border-t border-[#27272A]">
        <div className="mx-auto max-w-6xl px-6 py-20">
          <h2 className="text-2xl font-semibold tracking-tight">How selling works</h2>
          <div className="mt-8 grid gap-6 md:grid-cols-3">
            {[
              { n: "01", t: "Register your wallet", c: 'curl -X POST /v1/sellers\n{ "id": "you", "wallet": "0x…", "name": "Your API" }' },
              { n: "02", t: "List your endpoint", c: 'POST /v1/sellers/you/services\n{ "serviceId": "my-api",\n  "upstream_url": "https://…",\n  "price_usd": "$0.05" }' },
              { n: "03", t: "Agents pay you direct", c: 'GET /s/you/my-api\n← 402 → agent signs EIP-3009\n← 200 + receipt · USDC → your wallet' },
            ].map((s) => (
              <Card key={s.n} className="border-[#27272A] bg-[#18181B]">
                <CardHeader>
                  <span className="font-mono text-xs text-[#06B6D4]">{s.n}</span>
                  <CardTitle className="text-base text-[#FAFAFA]">{s.t}</CardTitle>
                </CardHeader>
                <CardContent>
                  <pre className="overflow-x-auto rounded-md bg-[#09090B] p-3 font-mono text-[11px] leading-5 text-[#A1A1AA]">{s.c}</pre>
                </CardContent>
              </Card>
            ))}
          </div>
          <p className="mt-6 font-mono text-xs text-[#52525B]">
            Live now on gateway.code402.dev — testnet today, mainnet after the security gate.
          </p>
        </div>
      </section>

      <section className="border-t border-[#27272A]">
        <div className="mx-auto max-w-6xl px-6 py-20">
          <h2 className="text-2xl font-semibold tracking-tight">Why agents choose Code402</h2>
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

      <section className="border-t border-[#27272A]">
        <div className="mx-auto max-w-6xl px-6 py-20">
          <div className="flex items-end justify-between">
            <h2 className="text-2xl font-semibold tracking-tight">API catalog</h2>
            <span className="font-mono text-xs text-[#A1A1AA]">0.005 USDC / call · Sub-Millisecond Edge Compute</span>
          </div>
          <div className="mt-8 divide-y divide-[#27272A] rounded-lg border border-[#27272A]">
            {catalog.map((t) => (
              <div key={t.name} className="flex items-center justify-between bg-[#18181B] px-5 py-4 first:rounded-t-lg last:rounded-b-lg">
                <div>
                  <p className="font-medium text-[#FAFAFA]">{t.name}</p>
                  <p className="text-sm text-[#A1A1AA]">{t.desc}</p>
                </div>
                <Badge
                  variant="outline"
                  className={t.status === 'live'
                    ? 'border-[#06B6D4] font-mono text-xs text-[#06B6D4]'
                    : 'border-[#F59E0B] font-mono text-xs text-[#F59E0B]'}
                >
                  {t.status === 'live' ? 'LIVE' : 'SOON'}
                </Badge>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="border-t border-[#27272A]">
        <div className="mx-auto max-w-6xl px-6 py-20">
          <div className="flex items-end justify-between">
            <h2 className="text-2xl font-semibold tracking-tight">The ecosystem</h2>
            <span className="font-mono text-xs text-[#A1A1AA]">discovery · protocol · trust · data</span>
          </div>
          <p className="mt-3 max-w-2xl text-sm text-[#A1A1AA]">
            Code402 is the settlement layer — but agents need discovery, commerce terms, and
            trust before they pay. We built the full stack.
          </p>
          <div className="mt-8 grid gap-6 md:grid-cols-2">
            {ecosystem.map((e) => (
              <a
                key={e.name}
                href={e.url}
                target="_blank"
                rel="noopener noreferrer"
                className="group rounded-lg border border-[#27272A] bg-[#18181B] p-5 transition-colors hover:border-[#06B6D4]"
              >
                <div className="flex items-center justify-between">
                  <p className="font-medium text-[#FAFAFA]">{e.name}</p>
                  <Badge variant="outline" className="border-[#27272A] font-mono text-xs text-[#06B6D4]">
                    {e.badge}
                  </Badge>
                </div>
                <p className="mt-2 text-sm leading-relaxed text-[#A1A1AA]">{e.desc}</p>
                <p className="mt-3 font-mono text-xs text-[#52525B] group-hover:text-[#06B6D4] transition-colors">
                  {e.url.replace('https://', '')} →
                </p>
              </a>
            ))}
          </div>
        </div>
      </section>
    </div>
  )
}
