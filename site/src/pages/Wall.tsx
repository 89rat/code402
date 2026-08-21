import { Badge } from '@/components/ui/badge'
import { Card, CardContent } from '@/components/ui/card'

type Brick = { name: string; desc: string; rent: string; status: 'live' | 'building' | 'planned' }
type Shelf = { id: string; title: string; promise: string; bricks: Brick[] }

const SHELVES: Shelf[] = [
  {
    id: 'sellers',
    title: 'Shelf 1 — Sellers: agents must be able to pay you',
    promise: 'Everything between your API and a paying machine.',
    bricks: [
      { name: 'Gateway template', desc: '“Deploy to Cloudflare” — a payable endpoint tonight.', rent: 'Free OSS → hosted tier', status: 'building' },
      { name: 'Hosted merchant gateway', desc: 'We run your payment path; you watch the ledger.', rent: 'Subscription + volume', status: 'building' },
      { name: 'Conformance test kit', desc: 'Codec + golden vectors for x402 v2. The missing test kit.', rent: 'Free · Apache-2.0', status: 'building' },
      { name: 'Certification badge', desc: 'Monthly probed conformance badge. Expires — that’s the point.', rent: 'Subscription', status: 'planned' },
      { name: 'Deployment Review', desc: '5-day adversarial audit; defects delivered as executable vectors.', rent: '$7.5–15K fixed', status: 'live' },
      { name: 'Manifest generator', desc: 'x402.json, llms.txt, openapi, mcp.json — one source of truth.', rent: 'Free tool; hosted sub', status: 'planned' },
      { name: 'Claim machine', desc: 'Exactly-once settlement per (payer, nonce). D1-backed state machine.', rent: 'Free crate + integration', status: 'building' },
      { name: 'Reconciler', desc: 'Hourly sweep that treats the chain as root of truth.', rent: 'Free crate + integration', status: 'live' },
      { name: 'XDR-1 receipts', desc: 'JCS-canonical, offline-verifiable proof of service.', rent: 'CC0 spec · metered verify API', status: 'live' },
      { name: 'OFAC payout screen', desc: 'Sanctions screening on payers before settlement.', rent: 'Subscription', status: 'building' },
      { name: 'Credit notes', desc: 'Paid-but-unserved becomes signed, amount-bound bearer credit. Never a refund.', rent: 'Protocol feature', status: 'planned' },
      { name: 'Pricing optimizer', desc: 'KV-instant repricing + 402→paid conversion analytics.', rent: 'Subscription', status: 'building' },
    ],
  },
  {
    id: 'buyers',
    title: 'Shelf 2 — Buyers: your agent spends money safely',
    promise: 'Never sign blind. Policy before signature, reputation before spend.',
    bricks: [
      { name: 'x402-paying-client', desc: 'Five lines to pay any conformant endpoint.', rent: 'Free OSS', status: 'building' },
      { name: 'Signer service', desc: 'Grant-bounded key custody; zeroize-on-drop; the agent never sees the key.', rent: 'Free self-host; hosted sub', status: 'building' },
      { name: 'Spend policy engine', desc: 'Deny-by-default predicates over protocol fields only.', rent: 'Free core; policy packs sub', status: 'building' },
      { name: 'Safe-to-pay reputation', desc: 'Who actually serves after taking money — measured, not claimed.', rent: 'Metered per lookup', status: 'planned' },
      { name: 'Price index API', desc: 'Cheapest alive endpoint for X, right now.', rent: 'Free web · metered API', status: 'building' },
      { name: 'Delivery-reliability dataset', desc: 'Counterparty due diligence from real paid probes.', rent: 'Metered / snapshots', status: 'planned' },
      { name: 'Agent budget ledger', desc: 'Channels and credit for repeat spend — per-call fees compressed.', rent: 'Subscription', status: 'planned' },
    ],
  },
  {
    id: 'publishers',
    title: 'Shelf 3 — Publishers: the Sept-15 shelf',
    promise: 'Cloudflare blocks agent & training crawlers by default on ad pages from Sept 15, 2026. These bricks turn a block into a paywall.',
    bricks: [
      { name: 'x402 paywall brick', desc: 'Retrofit any site: blocked agents get a 402 and a price instead of a wall.', rent: 'Subscription or % of crawl revenue', status: 'planned' },
      { name: 'Bot identity setup', desc: 'Web Bot Auth keypair, directory registration, verified-bot filing — done for you.', rent: 'Fixed fee', status: 'live' },
      { name: 'Readiness audit', desc: 'Will your traffic survive the flip? Measured answer, five days.', rent: 'Fixed fee', status: 'live' },
      { name: 'Pay Per Use integration', desc: 'Cloudflare’s Monetization Gateway (x402-settled) wired into your stack.', rent: 'Fixed fee', status: 'planned' },
    ],
  },
  {
    id: 'enterprise',
    title: 'Shelf 4 — Enterprise & ecosystem',
    promise: 'The heavy iron, licensed.',
    bricks: [
      { name: 'Self-hosted facilitator', desc: 'Own verify + settle + confirm. No quota, no per-settle toll.', rent: 'License + support', status: 'planned' },
      { name: 'Conformance suite site license', desc: 'The full vector corpus in your CI.', rent: 'Annual license', status: 'building' },
      { name: 'Ambiguous-money forensics', desc: '“Money moved, state unknown” — 24h incident retainer.', rent: 'Retainer', status: 'building' },
      { name: 'x402 v2 internals workshop', desc: 'Your team, up the curve, from the people who wrote the vectors.', rent: 'Per seat', status: 'planned' },
      { name: 'Custom vector packs', desc: 'Prove your edge case is handled — permanently.', rent: 'Fixed per pack', status: 'planned' },
    ],
  },
]

const statusStyle: Record<Brick['status'], string> = {
  live: 'border-[#06B6D4] text-[#06B6D4]',
  building: 'border-[#F59E0B] text-[#F59E0B]',
  planned: 'border-[#52525B] text-[#A1A1AA]',
}

export default function Wall() {
  return (
    <div className="mx-auto max-w-6xl px-6 py-16">
      <Badge variant="outline" className="border-[#27272A] font-mono text-xs text-[#06B6D4]">the LEGO wall</Badge>
      <h1 className="mt-4 text-3xl font-bold tracking-tight">Pick what you need. Leave the rest.</h1>
      <p className="mt-4 max-w-3xl leading-relaxed text-[#A1A1AA]">
        Every brick is standalone, retrofittable into your stack in an afternoon, and composes
        with the others only through the wire — the x402 spec, CAIP-2 networks, and XDR-1
        receipts. No hostage dependencies: a customer who leaves keeps working. That is what
        makes the next brick an easy yes.
      </p>

      {SHELVES.map((s) => (
        <section key={s.id} id={s.id} className="mt-14">
          <h2 className="text-xl font-semibold tracking-tight text-[#FAFAFA]">{s.title}</h2>
          <p className="mt-1 text-sm text-[#71717A]">{s.promise}</p>
          <div className="mt-6 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {s.bricks.map((b) => (
              <Card key={b.name} className="border-[#27272A] bg-[#18181B]">
                <CardContent className="flex h-full flex-col gap-2 p-5">
                  <div className="flex items-center justify-between gap-2">
                    <p className="font-medium text-[#FAFAFA]">{b.name}</p>
                    <Badge variant="outline" className={`shrink-0 font-mono text-[10px] uppercase ${statusStyle[b.status]}`}>
                      {b.status}
                    </Badge>
                  </div>
                  <p className="text-sm leading-relaxed text-[#A1A1AA]">{b.desc}</p>
                  <p className="mt-auto pt-2 font-mono text-[11px] text-[#06B6D4]">{b.rent}</p>
                </CardContent>
              </Card>
            ))}
          </div>
        </section>
      ))}

      <p className="mt-14 border-t border-[#27272A] pt-8 font-mono text-xs text-[#52525B]">
        Doctrine: standalone · retrofittable · expandable · composable only through the wire.
        Free bricks (test kit, scoreboard, weekly State-of-x402) are never sold — they are why
        the wall has foot traffic.
      </p>
    </div>
  )
}
