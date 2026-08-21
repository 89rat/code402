import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { FileSearch, ShieldCheck, Banknote, Bug } from 'lucide-react'

const deliverables = [
  {
    icon: FileSearch,
    title: 'Structural audit of your live 402 flow',
    body: 'Challenge shape, header dialect, envelope conformance — cited to the spec text, section by section. Not our opinion: the spec.',
  },
  {
    icon: Bug,
    title: 'The attack taxonomy, run against you',
    body: 'Replay, grant-before-settle, signature bypass, nonce misuse, quota/gas abuse, discovery drift — each attack demonstrably stopped or demonstrated, with file:line or request-sequence evidence.',
  },
  {
    icon: Banknote,
    title: 'Money-state audit',
    body: 'Where do funds go in every failure mode — timeout-after-settle, already-used, tool failure after payment, facilitator outage? Every ambiguous state must resolve to a defined, reconciled outcome. Most deployments fail this section.',
  },
  {
    icon: ShieldCheck,
    title: 'Agent-UX proof + fixtures',
    body: 'Can an autonomous agent discover → 402 → pay → verify using only your public manifests? We run it and show the transcript. Every defect ships as an executable test vector your CI can hold forever.',
  },
]

export default function Review() {
  return (
    <div className="mx-auto max-w-6xl px-6 py-16">
      <Badge variant="outline" className="border-[#27272A] font-mono text-xs text-[#06B6D4]">
        fixed scope · 5 days · fixed fee · paid in USDC over x402
      </Badge>
      <h1 className="mt-4 text-3xl font-bold tracking-tight">x402 Deployment Review</h1>
      <p className="mt-4 max-w-3xl text-lg leading-relaxed text-[#A1A1AA]">
        Your buyers are autonomous agents. They cannot file a support ticket, cannot ask for a
        refund, and will simply never come back after a failed payment. We review your deployment
        the way an adversarial buyer experiences it — and hand you every finding as a failing
        test you can keep.
      </p>

      <div className="mt-10 grid gap-6 md:grid-cols-2">
        {deliverables.map((d) => (
          <Card key={d.title} className="border-[#27272A] bg-[#18181B]">
            <CardHeader>
              <d.icon className="h-5 w-5 text-[#06B6D4]" />
              <CardTitle className="text-base text-[#FAFAFA]">{d.title}</CardTitle>
            </CardHeader>
            <CardContent className="text-sm leading-relaxed text-[#A1A1AA]">{d.body}</CardContent>
          </Card>
        ))}
      </div>

      <div className="mt-10 grid gap-6 md:grid-cols-2">
        <Card className="border-[#27272A] bg-[#18181B]">
          <CardHeader>
            <CardTitle className="text-base text-[#FAFAFA]">Terms</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2 text-sm text-[#A1A1AA]">
            <p>· Scope: one deployment, one payment path. Fixed.</p>
            <p>· Time: 5 business days from kickoff.</p>
            <p>· Fee: fixed, quoted on scope confirmation (US$7.5–15K by surface).</p>
            <p>· Paid in USDC on Base, via x402 — you experience our conformance as your first deliverable.</p>
            <p>· We never ask for keys. All testing uses our own funds, capped and disclosed.</p>
          </CardContent>
        </Card>
        <Card className="border-[#27272A] bg-[#18181B]">
          <CardHeader>
            <CardTitle className="text-base text-[#FAFAFA]">Why us</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2 text-sm text-[#A1A1AA]">
            <p>· We run code402.dev — a production x402 merchant gateway whose receipts reconcile to the chain.</p>
            <p>· Our review process is adversarial by construction: multi-model panel, citations mandatory, tests adjudicate.</p>
            <p>· Our own postmortems are public — including the day our checker graded our own production endpoint F.</p>
            <p>· Independence: findings are cited or discarded. That rule applies to us too.</p>
          </CardContent>
        </Card>
      </div>

      <div className="mt-10 rounded-lg border border-[#06B6D4] bg-[#18181B] p-8 text-center">
        <p className="font-mono text-sm text-[#A1A1AA]">Scope confirmation by email — tell us the host and the payment path.</p>
        <a href="mailto:review@code402.dev" className="mt-4 inline-block rounded-md bg-[#06B6D4] px-6 py-3 text-sm font-semibold text-[#09090B] hover:bg-[#22d3ee] transition-colors">
          review@code402.dev →
        </a>
      </div>
    </div>
  )
}
