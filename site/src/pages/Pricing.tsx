import { Link } from 'react-router'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'

export default function Pricing() {
  return (
    <div className="mx-auto max-w-6xl px-6 py-16">
      <h1 className="text-3xl font-bold tracking-tight">Pricing</h1>
      <p className="mt-4 max-w-2xl text-[#A1A1AA]">
        No tiers to decode, no seats, no minimums. Payment happens per call, in-protocol,
        via x402 — your agent never talks to a billing page.
      </p>
      <div className="mt-10 grid gap-6 md:grid-cols-2">
        <Card className="border-[#06B6D4] bg-[#18181B]">
          <CardHeader>
            <CardTitle className="text-[#FAFAFA]">Standard</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="font-mono text-4xl font-semibold text-[#06B6D4]">
              $0.005 <span className="text-base text-[#A1A1AA]">USDC / call</span>
            </p>
            <ul className="mt-6 space-y-2 text-sm text-[#A1A1AA]">
              <li>· Every tool in the catalog, one flat price</li>
              <li>· Settled on Base — low-fee L2, ~2s finality</li>
              <li>· Signed cryptographic receipt on every call</li>
              <li>· Append-Only Audit Ledger access for your requests</li>
              <li>· No account, no key management, no invoice</li>
            </ul>
          </CardContent>
        </Card>
        <Card className="border-[#27272A] bg-[#18181B]">
          <CardHeader>
            <CardTitle className="text-[#FAFAFA]">Enterprise</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="font-mono text-4xl font-semibold text-[#F59E0B]">
              Committed-use <span className="text-base text-[#A1A1AA]">credits</span>
            </p>
            <ul className="mt-6 space-y-2 text-sm text-[#A1A1AA]">
              <li>· Pre-purchased call credits at volume rates</li>
              <li>· Dedicated rate limits and SLA</li>
              <li>· Bulk audit exports for compliance teams</li>
              <li>· Private tool endpoints on request</li>
            </ul>
            <Link
              to="/enterprise"
              className="mt-6 inline-block rounded-md border border-[#27272A] px-4 py-2 text-sm text-[#FAFAFA] hover:border-[#F59E0B] transition-colors"
            >
              Talk to us →
            </Link>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
