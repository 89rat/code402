import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'

const items = [
  { t: 'Committed-use credits', d: 'Pre-purchase call volume at discounted rates. Credits draw down per call with the same signed receipts — finance gets a single audit trail.' },
  { t: 'Compliance-ready audit', d: 'Bulk exports from the Append-Only Audit Ledger: every request, input hash, output hash, receipt and settlement reference, exportable for your auditors.' },
  { t: 'Determinism SLA', d: 'Contractual guarantee of byte-identical outputs for identical inputs per tool version, backed by Sub-Millisecond Edge Compute across the network.' },
]

export default function Enterprise() {
  return (
    <div className="mx-auto max-w-6xl px-6 py-16">
      <h1 className="text-3xl font-bold tracking-tight">Enterprise</h1>
      <p className="mt-4 max-w-2xl text-[#A1A1AA]">
        For fleets of agents and the finance teams who audit them. Same protocol,
        same receipts — plus volume economics and compliance tooling.
      </p>
      <div className="mt-10 grid gap-6 md:grid-cols-3">
        {items.map((i) => (
          <Card key={i.t} className="border-[#27272A] bg-[#18181B]">
            <CardHeader><CardTitle className="text-base text-[#FAFAFA]">{i.t}</CardTitle></CardHeader>
            <CardContent className="text-sm leading-relaxed text-[#A1A1AA]">{i.d}</CardContent>
          </Card>
        ))}
      </div>
      <div className="mt-12 rounded-lg border border-[#27272A] bg-[#18181B] p-6">
        <p className="font-mono text-sm text-[#A1A1AA]">
          Contact: <span className="text-[#06B6D4]">enterprise@code402.dev</span>
          {' '}· JUANA LIMITED · Company No. 14043409 · Coventry, United Kingdom
        </p>
      </div>
    </div>
  )
}
