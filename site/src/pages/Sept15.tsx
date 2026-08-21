import { Link } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { AlarmClock, CheckCircle2, XCircle } from 'lucide-react'

const conditions = [
  'Applies to new domains onboarding to Cloudflare, and to existing free-tier zones that never changed the setting',
  'Applies on pages that display ads — not whole sites',
  'Training and Agent class crawlers: blocked by default',
  'Search class crawlers: still allowed by default',
  'Mixed-purpose crawlers (Googlebot, Applebot, BingBot) are judged by their most restrictive behavior — block Training and they are blocked too',
]

const agentSteps = [
  'Register a Web Bot Auth identity (Ed25519 keypair + public directory) so publishers can verify you',
  'Separate your crawlers by declared purpose — search, agent, training — one bot, one job',
  'Carry a way to pay: x402 wallet + spend policy, so a 402 is a purchase, not a dead end',
  'Grade the endpoints you depend on (x402check) — a supplier that can’t be paid is an outage waiting',
]

const publisherSteps = [
  'Decide deliberately per class (Search / Agent / Training) — don’t inherit the default by accident',
  'Keep Google: set the opt-out for mixed search+training crawlers before the deadline if search traffic matters',
  'Turn the block into revenue: put an x402 paywall on premium paths so blocked agents get a price instead of a void',
  'Watch logs for two weeks after the flip for crawlers blocked by accident',
]

export default function Sept15() {
  return (
    <div className="mx-auto max-w-6xl px-6 py-16">
      <Badge variant="outline" className="border-[#3a2f0b] font-mono text-xs text-[#F59E0B]">
        deadline briefing · verified against Cloudflare’s July 1, 2026 announcements
      </Badge>
      <h1 className="mt-4 flex items-center gap-3 text-3xl font-bold tracking-tight">
        <AlarmClock className="h-8 w-8 text-[#F59E0B]" />
        September 15, 2026
      </h1>
      <p className="mt-4 max-w-3xl text-lg leading-relaxed text-[#A1A1AA]">
        Cloudflare sets new defaults: on pages with ads, <span className="text-[#FAFAFA]">Agent</span> and{' '}
        <span className="text-[#FAFAFA]">Training</span> crawlers are blocked unless the site allows them;{' '}
        <span className="text-[#FAFAFA]">Search</span> stays allowed. Alongside it, Cloudflare’s Monetization
        Gateway pays publishers in stablecoins <span className="text-[#FAFAFA]">over the x402 protocol</span>.
        The agentic web must now identify itself — and pay.
      </p>

      <Card className="mt-10 border-[#27272A] bg-[#18181B]">
        <CardHeader>
          <CardTitle className="text-base text-[#FAFAFA]">Exactly what changes (all five conditions — most retellings get this wrong)</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {conditions.map((c) => (
            <p key={c} className="flex gap-3 text-sm leading-relaxed text-[#A1A1AA]">
              <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0 text-[#06B6D4]" /> {c}
            </p>
          ))}
          <p className="flex gap-3 text-sm leading-relaxed text-[#A1A1AA]">
            <XCircle className="mt-0.5 h-4 w-4 shrink-0 text-[#F59E0B]" />
            What it is NOT: a whole-web flip. Existing paid plans keep dashboard control; non-ad pages are untouched.
          </p>
        </CardContent>
      </Card>

      <div className="mt-10 grid gap-6 md:grid-cols-2">
        <Card className="border-[#27272A] bg-[#18181B]">
          <CardHeader>
            <CardTitle className="text-base text-[#FAFAFA]">If you run agents or crawlers</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {agentSteps.map((s, i) => (
              <p key={s} className="flex gap-3 text-sm leading-relaxed text-[#A1A1AA]">
                <span className="font-mono text-[#06B6D4]">{i + 1}.</span> {s}
              </p>
            ))}
            <Link to="/wall#buyers" className="mt-2 inline-block font-mono text-xs text-[#06B6D4] hover:underline">
              buyer-side bricks →
            </Link>
          </CardContent>
        </Card>
        <Card className="border-[#27272A] bg-[#18181B]">
          <CardHeader>
            <CardTitle className="text-base text-[#FAFAFA]">If you publish content</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {publisherSteps.map((s, i) => (
              <p key={s} className="flex gap-3 text-sm leading-relaxed text-[#A1A1AA]">
                <span className="font-mono text-[#06B6D4]">{i + 1}.</span> {s}
              </p>
            ))}
            <Link to="/wall#publishers" className="mt-2 inline-block font-mono text-xs text-[#06B6D4] hover:underline">
              publisher-side bricks →
            </Link>
          </CardContent>
        </Card>
      </div>

      <Card className="mt-10 border-[#06B6D4] bg-[#18181B]">
        <CardContent className="flex flex-col items-start gap-4 p-8 md:flex-row md:items-center md:justify-between">
          <div>
            <h3 className="text-lg font-semibold text-[#FAFAFA]">The Sept 15 Scoreboard</h3>
            <p className="mt-2 max-w-xl text-sm leading-relaxed text-[#A1A1AA]">
              On deadline day we publish the measured answer: which payable endpoints are alive,
              which crawlers are registered, which sellers reconcile to the chain. Probes, not
              press releases. Get graded before the snapshot.
            </p>
          </div>
          <Link to="/check" className="shrink-0 rounded-md bg-[#06B6D4] px-5 py-2.5 text-sm font-semibold text-[#09090B] hover:bg-[#22d3ee] transition-colors">
            Grade your endpoint →
          </Link>
        </CardContent>
      </Card>
    </div>
  )
}
