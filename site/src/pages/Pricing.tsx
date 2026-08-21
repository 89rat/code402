import { Link } from 'react-router'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'

const tools = [
  { name: 'company-number-format', price: '0.002', why: 'commodity check — the habit-former' },
  { name: 'vat-mod97-check', price: '0.005', why: 'incumbent replacement, priced ≥10% under paid validators' },
  { name: 'context-distill', price: '0.010', why: 'saves ~4k tokens of LLM context per 16KB page' },
]

export default function Pricing() {
  return (
    <div className="mx-auto max-w-6xl px-6 py-16">
      <h1 className="text-3xl font-bold tracking-tight">Pricing</h1>
      <p className="mt-4 max-w-2xl text-[#A1A1AA]">
        Payment happens per call, in-protocol, via x402 — your agent never talks to a billing
        page. Value-based per tool, repriced without deploys. Every price below is the live
        challenge price, not a marketing number.
      </p>

      <div className="mt-10 grid gap-6 md:grid-cols-3">
        {tools.map((t) => (
          <Card key={t.name} className="border-[#27272A] bg-[#18181B]">
            <CardHeader>
              <CardTitle className="font-mono text-sm text-[#FAFAFA]">{t.name}</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="font-mono text-3xl font-semibold text-[#06B6D4]">
                {t.price} <span className="text-base text-[#A1A1AA]">USDC / call</span>
              </p>
              <p className="mt-3 text-sm text-[#A1A1AA]">{t.why}</p>
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="mt-10 grid gap-6 md:grid-cols-2">
        <Card className="border-[#06B6D4] bg-[#18181B]">
          <CardHeader>
            <CardTitle className="text-[#FAFAFA]">Deployment Review</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="font-mono text-3xl font-semibold text-[#06B6D4]">
              $7.5–15K <span className="text-base text-[#A1A1AA]">fixed · 5 days</span>
            </p>
            <ul className="mt-6 space-y-2 text-sm text-[#A1A1AA]">
              <li>· Adversarial review of your x402 deployment, spec-cited</li>
              <li>· Money-state audit — every failure mode reconciled</li>
              <li>· Every defect delivered as an executable test vector</li>
              <li>· Paid in USDC over x402</li>
            </ul>
            <Link to="/review" className="mt-6 inline-block rounded-md border border-[#06B6D4] px-4 py-2 text-sm text-[#06B6D4] hover:bg-[#06B6D4] hover:text-[#09090B] transition-colors">
              Scope it →
            </Link>
          </CardContent>
        </Card>
        <Card className="border-[#27272A] bg-[#18181B]">
          <CardHeader>
            <CardTitle className="text-[#FAFAFA]">Enterprise</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="font-mono text-3xl font-semibold text-[#F59E0B]">
              Licensed <span className="text-base text-[#A1A1AA]">bricks + support</span>
            </p>
            <ul className="mt-6 space-y-2 text-sm text-[#A1A1AA]">
              <li>· Self-hosted facilitator — own verify/settle/confirm</li>
              <li>· Conformance suite site license for CI</li>
              <li>· Ambiguous-money incident retainer (24h)</li>
              <li>· Workshops and custom vector packs</li>
            </ul>
            <Link to="/wall" className="mt-6 inline-block rounded-md border border-[#27272A] px-4 py-2 text-sm text-[#FAFAFA] hover:border-[#F59E0B] transition-colors">
              Browse the wall →
            </Link>
          </CardContent>
        </Card>
      </div>

      <p className="mt-10 border-t border-[#27272A] pt-6 font-mono text-xs text-[#52525B]">
        Free forever: the conformance test kit, x402check, the price-index web view, and the
        weekly State of x402 report. They are the storefront, not the product.
      </p>
    </div>
  )
}
